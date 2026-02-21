//! Procedure call parser for GQL.
//!
//! This module implements parsing for GQL procedural composition features:
//! - CALL procedure statements (inline and named)
//! - Variable scope clauses
//! - Nested procedure specifications
//! - Variable definition blocks (graph, binding table, value variables)
//! - Statement blocks with NEXT chaining
//! - AT schema clauses
//! - YIELD clauses for result projection
//!
//! # Grammar References
//!
//! This parser implements grammar rules from lines 138-200 and 727-775 of GQL.g4.

use crate::ast::procedure::*;
use crate::ast::references::BindingVariable;
use crate::ast::{Expression, ProcedureStatement, Span};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::base::TokenStream;
use crate::parser::InternalParseResult;
use crate::parser::expression::parse_expression;
use crate::parser::mutation::parse_linear_data_modifying_statement;
use crate::parser::program::parse_catalog_statement_kind;
use crate::parser::query::parse_query_legacy;
use crate::parser::references::{parse_procedure_reference, parse_schema_reference};
use crate::parser::types::{
    parse_binding_table_reference_value_type, parse_graph_reference_value_type, parse_value_type,
};
use smol_str::SmolStr;

/// Parse result with optional value and diagnostics.
pub(crate) type ParseResult<T> = InternalParseResult<T>;

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse an identifier or identifier-like keyword as SmolStr.
fn parse_identifier(stream: &mut TokenStream) -> Result<(SmolStr, Span), Box<Diag>> {
    if stream.check(&TokenKind::Eof) {
        let pos = stream.position();
        return Err(Box::new(
            Diag::error("Expected identifier")
                .with_primary_label(pos..pos, "expected identifier here"),
        ));
    }

    let token = stream.current();
    match &token.kind {
        TokenKind::Identifier(name) => {
            let result = (name.clone(), token.span.clone());
            stream.advance();
            Ok(result)
        }
        TokenKind::DelimitedIdentifier(name) => {
            let result = (name.clone(), token.span.clone());
            stream.advance();
            Ok(result)
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let result = (SmolStr::new(kind.to_string()), token.span.clone());
            stream.advance();
            Ok(result)
        }
        _ => Err(Box::new(
            Diag::error(format!("Expected identifier, found {}", token.kind))
                .with_primary_label(token.span.clone(), "expected identifier here"),
        )),
    }
}

/// Parse a regular identifier (non-delimited), including non-reserved keywords.
fn parse_regular_identifier(
    stream: &mut TokenStream,
) -> Result<(SmolStr, Span), Box<Diag>> {
    if stream.check(&TokenKind::Eof) {
        let pos = stream.position();
        return Err(Box::new(
            Diag::error("Expected regular identifier")
                .with_primary_label(pos..pos, "expected regular identifier here"),
        ));
    }

    let token = stream.current();
    match &token.kind {
        TokenKind::Identifier(name) => {
            let result = (name.clone(), token.span.clone());
            stream.advance();
            Ok(result)
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let result = (SmolStr::new(kind.to_string()), token.span.clone());
            stream.advance();
            Ok(result)
        }
        _ => Err(Box::new(
            Diag::error(format!("Expected regular identifier, found {}", token.kind))
                .with_primary_label(token.span.clone(), "expected regular identifier here"),
        )),
    }
}

/// Find the boundary of an expression in a token stream.
fn find_expression_boundary(stream: &TokenStream) -> usize {
    let tokens = stream.tokens();
    let start_pos = stream.position();
    let mut pos = start_pos;
    let mut depth = 0;

    while pos < tokens.len() {
        let token = &tokens[pos];

        match &token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                depth += 1;
                pos += 1;
                continue;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                if depth > 0 {
                    depth -= 1;
                    pos += 1;
                    continue;
                } else {
                    break;
                }
            }
            _ => {}
        }

        if depth == 0 {
            match &token.kind {
                // Statement and clause keywords that terminate expressions
                TokenKind::Match
                | TokenKind::Filter
                | TokenKind::Let
                | TokenKind::For
                | TokenKind::Order
                | TokenKind::Limit
                | TokenKind::Offset
                | TokenKind::Skip
                | TokenKind::Return
                | TokenKind::Select
                | TokenKind::Finish
                | TokenKind::Insert
                | TokenKind::Set
                | TokenKind::Remove
                | TokenKind::Delete
                | TokenKind::Detach
                | TokenKind::Nodetach
                | TokenKind::Call
                | TokenKind::Union
                | TokenKind::Except
                | TokenKind::Intersect
                | TokenKind::Otherwise
                | TokenKind::From
                | TokenKind::Where
                | TokenKind::Group
                | TokenKind::Having
                | TokenKind::By
                | TokenKind::With
                | TokenKind::As
                | TokenKind::Asc
                | TokenKind::Ascending
                | TokenKind::Desc
                | TokenKind::Descending
                | TokenKind::Nulls
                | TokenKind::Semicolon
                | TokenKind::Eof
                | TokenKind::Next
                | TokenKind::Yield => {
                    break;
                }
                TokenKind::Comma => {
                    break;
                }
                _ => {
                    pos += 1;
                }
            }
        } else {
            pos += 1;
        }
    }

    pos - start_pos
}

/// Parse an expression starting at the current position.
fn parse_expression_at(stream: &mut TokenStream) -> Result<Expression, Box<Diag>> {
    let start_pos = stream.position();
    let count = find_expression_boundary(stream);

    if count == 0 {
        return Err(Box::new(
            Diag::error("Expected expression").with_primary_label(
                stream.current().span.clone(),
                "expected expression here",
            ),
        ));
    }

    let tokens = stream.tokens();
    let expr_tokens = &tokens[start_pos..start_pos + count];
    let result = parse_expression(expr_tokens)?;

    // Advance stream position
    stream.set_position(start_pos + count);

    Ok(result)
}

// ============================================================================
// CALL Procedure Statement (Task 9)
// ============================================================================

/// Parse a CALL procedure statement.
///
/// Grammar: `callProcedureStatement: OPTIONAL? CALL procedureCall`
///
/// # Examples
///
/// ```text
/// CALL my_procedure()
/// OPTIONAL CALL risky_operation()
/// CALL (x, y) { MATCH (n) RETURN n }
/// ```
pub fn parse_call_procedure_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<CallProcedureStatement> {
    let mut stream = TokenStream::new(tokens);
    stream.set_position(*pos);

    let start_pos = stream.position();
    let mut diags = vec![];

    // Check for OPTIONAL keyword
    let optional = stream.consume(&TokenKind::Optional);

    // Expect CALL keyword
    if let Err(diag) = stream.expect(TokenKind::Call) {
        diags.push(*diag);
        *pos = stream.position();
        return (None, diags);
    }

    // Parse procedure call (inline or named)
    let (call_opt, call_diags) = parse_procedure_call(&mut stream);
    diags.extend(call_diags);

    *pos = stream.position();

    if let Some(call) = call_opt {
        let end_span = call.span().end;
        let stmt = CallProcedureStatement {
            optional,
            call,
            span: start_pos..end_span,
        };
        (Some(stmt), diags)
    } else {
        (None, diags)
    }
}

/// Parse a procedure call (inline or named).
///
/// Grammar: `procedureCall: inlineProcedureCall | namedProcedureCall`
fn parse_procedure_call(stream: &mut TokenStream) -> ParseResult<ProcedureCall> {
    let start_pos = stream.position();

    // Try inline procedure call first (looks for LParen or LBrace)
    if stream.check(&TokenKind::LParen) || stream.check(&TokenKind::LBrace) {
        let (inline_opt, diags) = parse_inline_procedure_call(stream);
        if let Some(inline) = inline_opt {
            return (Some(ProcedureCall::Inline(inline)), diags);
        }
        // If inline parse failed, try named
        stream.set_position(start_pos);
    }

    // Try named procedure call
    let (named_opt, diags) = parse_named_procedure_call(stream);
    if let Some(named) = named_opt {
        (Some(ProcedureCall::Named(named)), diags)
    } else {
        (None, diags)
    }
}

/// Helper to get the span of a ProcedureCall.
impl ProcedureCall {
    fn span(&self) -> &Span {
        match self {
            ProcedureCall::Inline(c) => &c.span,
            ProcedureCall::Named(c) => &c.span,
        }
    }
}

// ============================================================================
// Inline Procedure Calls (Task 10)
// ============================================================================

/// Parse an inline procedure call with optional variable scope.
///
/// Grammar: `inlineProcedureCall: variableScopeClause? nestedProcedureSpecification`
///
/// # Examples
///
/// ```text
/// () { MATCH (n) RETURN n }
/// (x, y) { MATCH (n WHERE n.id = x) RETURN n }
/// { MATCH (n) RETURN n }
/// ```
pub fn parse_inline_procedure_call(
    stream: &mut TokenStream,
) -> ParseResult<InlineProcedureCall> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Parse optional variable scope clause
    let (variable_scope, scope_diags) = if stream.check(&TokenKind::LParen) {
        let (scope_opt, scope_diags) = parse_variable_scope_clause(stream);
        (scope_opt, scope_diags)
    } else {
        (None, vec![])
    };
    diags.extend(scope_diags);

    // Parse nested procedure specification
    let (spec_opt, spec_diags) = parse_nested_procedure_specification(stream);
    diags.extend(spec_diags);

    if let Some(specification) = spec_opt {
        let end_span = specification.span.end;
        let call = InlineProcedureCall {
            variable_scope,
            specification,
            span: start_pos..end_span,
        };
        (Some(call), diags)
    } else {
        (None, diags)
    }
}

/// Parse a variable scope clause.
///
/// Grammar: `variableScopeClause: LPAREN bindingVariableReferenceList? RPAREN`
///
/// # Examples
///
/// ```text
/// ()
/// (x)
/// (x, y, z)
/// ```
pub fn parse_variable_scope_clause(
    stream: &mut TokenStream,
) -> ParseResult<VariableScopeClause> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect opening parenthesis
    if let Err(diag) = stream.expect(TokenKind::LParen) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse optional binding variable reference list
    let (variables, var_diags) = if stream.check(&TokenKind::RParen) {
        (vec![], vec![])
    } else {
        let (vars_opt, diags) = parse_binding_variable_reference_list(stream);
        (vars_opt.unwrap_or_default(), diags)
    };
    diags.extend(var_diags);

    // Expect closing parenthesis
    let end_span = if let Ok(span) = stream.expect(TokenKind::RParen) {
        span.end
    } else {
        diags.push(
            Diag::error("Expected closing parenthesis in variable scope clause")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected ')' here",
                ),
        );
        start_pos
    };

    let clause = VariableScopeClause {
        variables,
        span: start_pos..end_span,
    };
    (Some(clause), diags)
}

/// Parse a binding variable reference list (comma-separated).
///
/// Grammar: `bindingVariableReferenceList: bindingVariableReference (COMMA bindingVariableReference)*`
pub fn parse_binding_variable_reference_list(
    stream: &mut TokenStream,
) -> ParseResult<Vec<BindingVariable>> {
    let mut variables = vec![];
    let mut diags = vec![];

    loop {
        // Parse binding variable reference
        let (var_opt, var_diags) = parse_binding_variable(stream);
        diags.extend(var_diags);

        if let Some(var) = var_opt {
            variables.push(var);
        } else {
            break;
        }

        // Check for comma
        if !stream.consume(&TokenKind::Comma) {
            break;
        }
        if stream.check(&TokenKind::RParen) || stream.check(&TokenKind::Eof) {
            diags.push(
                Diag::error("Expected binding variable after ','").with_primary_label(
                    stream.current().span.clone(),
                    "expected binding variable here",
                ),
            );
            break;
        }
    }

    (Some(variables), diags)
}

/// Parse a binding variable.
fn parse_binding_variable(stream: &mut TokenStream) -> ParseResult<BindingVariable> {
    match parse_regular_identifier(stream) {
        Ok((name, span)) => {
            let var = BindingVariable { name, span };
            (Some(var), vec![])
        }
        Err(diag) => (None, vec![*diag]),
    }
}

// ============================================================================
// Named Procedure Calls (Task 11)
// ============================================================================

/// Parse a named procedure call with arguments and yield clause.
///
/// Grammar: `namedProcedureCall: procedureReference procedureArgumentList yieldClause?`
///
/// # Examples
///
/// ```text
/// my_procedure()
/// my_procedure(arg1, arg2)
/// my_procedure(arg1) YIELD result1, result2
/// ```
pub fn parse_named_procedure_call(
    stream: &mut TokenStream,
) -> ParseResult<NamedProcedureCall> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Parse procedure reference
    // We need to slice the tokens for the procedure reference parser
    let proc_ref_start = stream.position();

    // Find the end of the procedure reference (before '(' or YIELD or end)
    let tokens = stream.tokens();
    let mut proc_ref_end = proc_ref_start;
    while proc_ref_end < tokens.len() {
        match &tokens[proc_ref_end].kind {
            TokenKind::LParen | TokenKind::Yield | TokenKind::Eof | TokenKind::Semicolon => break,
            _ => proc_ref_end += 1,
        }
    }

    if proc_ref_end == proc_ref_start {
        diags.push(
            Diag::error("Expected procedure reference").with_primary_label(
                stream.current().span.clone(),
                "expected procedure reference here",
            ),
        );
        return (None, diags);
    }

    let proc_ref_tokens = &tokens[proc_ref_start..proc_ref_end];
    let procedure = match parse_procedure_reference(proc_ref_tokens) {
        Ok(p) => {
            stream.set_position(proc_ref_end);
            p
        }
        Err(diag) => {
            diags.push(*diag);
            return (None, diags);
        }
    };

    // Parse required procedure argument list
    if !stream.check(&TokenKind::LParen) {
        diags.push(
            Diag::error("Expected '(' after procedure reference").with_primary_label(
                stream.current().span.clone(),
                "expected '(' here",
            ),
        );
        return (None, diags);
    }

    let (arguments_opt, arg_diags) = parse_procedure_argument_list(stream);
    diags.extend(arg_diags);
    let Some(arguments) = arguments_opt else {
        return (None, diags);
    };

    // Parse optional YIELD clause
    let (yield_clause, yield_diags) = if stream.check(&TokenKind::Yield) {
        parse_yield_clause(stream)
    } else {
        (None, vec![])
    };
    diags.extend(yield_diags);

    let end_span = yield_clause
        .as_ref()
        .map(|y| y.span.end)
        .or(Some(arguments.span.end))
        .unwrap_or(procedure.span().end);

    let call = NamedProcedureCall {
        procedure,
        arguments: Some(arguments),
        yield_clause,
        span: start_pos..end_span,
    };
    (Some(call), diags)
}

/// Parse a procedure argument list.
///
/// Grammar: `procedureArgumentList: LPAREN (procedureArgument (COMMA procedureArgument)*)? RPAREN`
pub fn parse_procedure_argument_list(
    stream: &mut TokenStream,
) -> ParseResult<ProcedureArgumentList> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect opening parenthesis
    if let Err(diag) = stream.expect(TokenKind::LParen) {
        diags.push(*diag);
        return (None, diags);
    }

    let mut arguments = vec![];

    // Parse arguments until closing parenthesis
    while !stream.check(&TokenKind::RParen) && !stream.check(&TokenKind::Eof) {
        // Parse procedure argument using expression parser
        let arg_start = stream.position();

        match parse_expression_at(stream) {
            Ok(expression) => {
                let arg = ProcedureArgument {
                    expression,
                    span: arg_start..stream.position(),
                };
                arguments.push(arg);
            }
            Err(diag) => {
                diags.push(*diag);
                break;
            }
        }

        // Check for comma
        if !stream.consume(&TokenKind::Comma) {
            break;
        }
        if stream.check(&TokenKind::RParen) || stream.check(&TokenKind::Eof) {
            diags.push(
                Diag::error("Expected argument expression after ','").with_primary_label(
                    stream.current().span.clone(),
                    "expected argument expression here",
                ),
            );
            break;
        }
    }

    // Expect closing parenthesis
    let end_span = if let Ok(span) = stream.expect(TokenKind::RParen) {
        span.end
    } else {
        diags.push(
            Diag::error("Expected closing parenthesis in procedure argument list")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected ')' here",
                ),
        );
        stream.position()
    };

    let list = ProcedureArgumentList {
        arguments,
        span: start_pos..end_span,
    };
    (Some(list), diags)
}

/// Parse a YIELD clause.
///
/// Grammar: `yieldClause: YIELD yieldItemList`
pub fn parse_yield_clause(stream: &mut TokenStream) -> ParseResult<YieldClause> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect YIELD keyword
    if let Err(diag) = stream.expect(TokenKind::Yield) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse yield item list
    let (items_opt, items_diags) = parse_yield_item_list(stream);
    diags.extend(items_diags);

    if let Some(items) = items_opt {
        let end_span = items.span.end;
        let clause = YieldClause {
            items,
            span: start_pos..end_span,
        };
        (Some(clause), diags)
    } else {
        (None, diags)
    }
}

/// Parse a yield item list (comma-separated).
pub fn parse_yield_item_list(stream: &mut TokenStream) -> ParseResult<YieldItemList> {
    let start_pos = stream.position();
    let mut items = vec![];
    let mut diags = vec![];

    loop {
        let item_start = stream.position();

        // yieldItemName is a field name (identifier), not a general expression.
        let (name, name_span) = match parse_identifier(stream) {
            Ok(value) => value,
            Err(diag) => {
                diags.push(*diag);
                break;
            }
        };

        // Check for optional alias (AS bindingVariable).
        let alias = if stream.consume(&TokenKind::As) {
            match parse_regular_identifier(stream) {
                Ok((alias_name, alias_span)) => Some(YieldItemAlias {
                    name: alias_name,
                    span: alias_span,
                }),
                Err(diag) => {
                    diags.push(*diag);
                    None
                }
            }
        } else {
            None
        };

        let item = YieldItem {
            expression: Expression::VariableReference(name, name_span),
            alias,
            span: item_start..stream.position(),
        };
        items.push(item);

        // Check for comma.
        if !stream.consume(&TokenKind::Comma) {
            break;
        }
        if stream.check(&TokenKind::Eof)
            || stream.check(&TokenKind::RParen)
            || stream.check(&TokenKind::RBrace)
            || stream.check(&TokenKind::Semicolon)
            || stream.check(&TokenKind::Next)
        {
            diags.push(
                Diag::error("Expected yield item after ','").with_primary_label(
                    stream.current().span.clone(),
                    "expected yield item here",
                ),
            );
            break;
        }
    }

    if items.is_empty() && diags.is_empty() {
        diags.push(
            Diag::error("Expected yield items")
                .with_primary_label(start_pos..start_pos, "expected yield items here"),
        );
        return (None, diags);
    }

    let list = YieldItemList {
        items,
        span: start_pos..stream.position(),
    };
    (Some(list), diags)
}

// ============================================================================
// Nested Procedure Specifications (Task 12)
// ============================================================================

/// Parse a nested procedure specification.
///
/// Grammar: `nestedProcedureSpecification: LBRACE procedureBody RBRACE`
pub fn parse_nested_procedure_specification(
    stream: &mut TokenStream,
) -> ParseResult<NestedProcedureSpecification> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect opening brace
    if let Err(diag) = stream.expect(TokenKind::LBrace) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse procedure body
    let (body_opt, body_diags) = parse_procedure_body(stream);
    diags.extend(body_diags);

    // Expect closing brace
    let end_span = if let Ok(span) = stream.expect(TokenKind::RBrace) {
        span.end
    } else {
        diags.push(
            Diag::error("Expected closing brace in nested procedure specification")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected '}' here",
                ),
        );
        stream.position()
    };

    if let Some(body) = body_opt {
        let spec = NestedProcedureSpecification {
            body,
            span: start_pos..end_span,
        };
        (Some(spec), diags)
    } else {
        (None, diags)
    }
}

/// Parse a nested data-modifying procedure specification.
pub fn parse_nested_data_modifying_procedure_specification(
    stream: &mut TokenStream,
) -> ParseResult<NestedDataModifyingProcedureSpecification> {
    let (spec_opt, diags) = parse_nested_procedure_specification(stream);
    if let Some(spec) = spec_opt {
        return (
            Some(NestedDataModifyingProcedureSpecification {
                body: spec.body,
                span: spec.span,
            }),
            diags,
        );
    }
    (None, diags)
}

/// Parse a nested query specification.
pub fn parse_nested_query_specification(
    stream: &mut TokenStream,
) -> ParseResult<NestedQuerySpecification> {
    let (spec_opt, diags) = parse_nested_procedure_specification(stream);
    if let Some(spec) = spec_opt {
        return (
            Some(NestedQuerySpecification {
                body: spec.body,
                span: spec.span,
            }),
            diags,
        );
    }
    (None, diags)
}

/// Parse a procedure body.
///
/// Grammar: `procedureBody: atSchemaClause? bindingVariableDefinitionBlock? statementBlock`
pub fn parse_procedure_body(stream: &mut TokenStream) -> ParseResult<ProcedureBody> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Parse optional AT schema clause
    let (at_schema, at_diags) = if stream.check(&TokenKind::At) {
        parse_at_schema_clause(stream)
    } else {
        (None, vec![])
    };
    diags.extend(at_diags);

    // Parse optional variable definition block
    let (variable_definitions, var_diags) = if is_variable_definition_start(stream) {
        parse_binding_variable_definition_block(stream)
    } else {
        (None, vec![])
    };
    diags.extend(var_diags);

    // Parse statement block
    let (statements, stmt_diags) = parse_statement_block(stream);
    diags.extend(stmt_diags);

    if let Some(statements) = statements {
        let end_span = statements.span.end;
        let body = ProcedureBody {
            at_schema,
            variable_definitions,
            statements,
            span: start_pos..end_span,
        };
        (Some(body), diags)
    } else {
        (None, diags)
    }
}

/// Check if the current position starts a variable definition.
fn is_variable_definition_start(stream: &TokenStream) -> bool {
    if stream.check(&TokenKind::Eof) {
        return false;
    }

    is_variable_definition_keyword(&stream.current().kind)
}

fn is_variable_definition_keyword(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Graph
            | TokenKind::Table
            | TokenKind::Value
            | TokenKind::Property
            | TokenKind::Binding
    )
}

fn is_procedure_statement_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Match
            | TokenKind::Optional
            | TokenKind::Use
            | TokenKind::Filter
            | TokenKind::Let
            | TokenKind::For
            | TokenKind::Order
            | TokenKind::Limit
            | TokenKind::Offset
            | TokenKind::Skip
            | TokenKind::Return
            | TokenKind::Finish
            | TokenKind::Select
            | TokenKind::From
            | TokenKind::Call
            | TokenKind::Insert
            | TokenKind::Set
            | TokenKind::Remove
            | TokenKind::Delete
            | TokenKind::Detach
            | TokenKind::Nodetach
            | TokenKind::Create
            | TokenKind::Drop
    )
}

fn is_at_schema_follow_boundary(kind: &TokenKind) -> bool {
    is_variable_definition_keyword(kind)
        || is_procedure_statement_start(kind)
        || matches!(
            kind,
            TokenKind::Next | TokenKind::RBrace | TokenKind::Eof | TokenKind::Semicolon
        )
}

fn consume_typed_marker(stream: &mut TokenStream) -> bool {
    if stream.check(&TokenKind::DoubleColon) || stream.check(&TokenKind::Typed) {
        stream.advance();
        true
    } else {
        false
    }
}

fn find_type_annotation_end(stream: &TokenStream) -> usize {
    let tokens = stream.tokens();
    let start = stream.position();
    let mut cursor = start;
    let mut depth = 0usize;

    while cursor < tokens.len() {
        match tokens[cursor].kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::Lt => {
                depth += 1;
                cursor += 1;
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace | TokenKind::Gt => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                cursor += 1;
            }
            TokenKind::Eq if depth == 0 => break,
            TokenKind::Graph
            | TokenKind::Table
            | TokenKind::Value
            | TokenKind::Property
            | TokenKind::Binding
            | TokenKind::Match
            | TokenKind::Next
            | TokenKind::Semicolon
            | TokenKind::Eof
                if depth == 0 =>
            {
                break;
            }
            _ => cursor += 1,
        }
    }

    cursor
}

// ============================================================================
// Variable Definition Blocks (Task 13)
// ============================================================================

/// Parse a binding variable definition block.
///
/// Grammar: `bindingVariableDefinitionBlock: bindingVariableDefinition+`
pub fn parse_binding_variable_definition_block(
    stream: &mut TokenStream,
) -> ParseResult<BindingVariableDefinitionBlock> {
    let start_pos = stream.position();
    let mut definitions = vec![];
    let mut diags = vec![];

    // Parse one or more variable definitions
    while is_variable_definition_start(stream) {
        let (def_opt, def_diags) = parse_binding_variable_definition(stream);
        diags.extend(def_diags);

        if let Some(def) = def_opt {
            definitions.push(def);
        } else {
            break;
        }
    }

    if definitions.is_empty() {
        return (None, diags);
    }

    let block = BindingVariableDefinitionBlock {
        definitions,
        span: start_pos..stream.position(),
    };
    (Some(block), diags)
}

/// Parse a binding variable definition.
///
/// Grammar: `bindingVariableDefinition: graphVariableDefinition | bindingTableVariableDefinition | valueVariableDefinition`
pub fn parse_binding_variable_definition(
    stream: &mut TokenStream,
) -> ParseResult<BindingVariableDefinition> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    // Dispatch based on keyword
    match &stream.current().kind {
        TokenKind::Graph | TokenKind::Property => {
            let (def_opt, diags) = parse_graph_variable_definition(stream);
            (def_opt.map(BindingVariableDefinition::Graph), diags)
        }
        TokenKind::Table | TokenKind::Binding => {
            let (def_opt, diags) = parse_binding_table_variable_definition(stream);
            (def_opt.map(BindingVariableDefinition::BindingTable), diags)
        }
        TokenKind::Value => {
            let (def_opt, diags) = parse_value_variable_definition(stream);
            (def_opt.map(BindingVariableDefinition::Value), diags)
        }
        _ => (None, vec![]),
    }
}

/// Parse a graph variable definition.
///
/// Grammar: `graphVariableDefinition: PROPERTY? GRAPH bindingVariable optTypedGraphInitializer?`
pub fn parse_graph_variable_definition(
    stream: &mut TokenStream,
) -> ParseResult<GraphVariableDefinition> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Check for optional PROPERTY keyword
    let is_property = stream.consume(&TokenKind::Property);

    // Expect GRAPH keyword
    if let Err(diag) = stream.expect(TokenKind::Graph) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse binding variable
    let (variable, var_diags) = parse_binding_variable(stream);
    diags.extend(var_diags);

    let variable = match variable {
        Some(v) => v,
        None => {
            return (None, diags);
        }
    };

    // Parse optional type annotation and required initializer.
    let mut type_annotation = None;

    if !stream.check(&TokenKind::Eq) {
        let had_typed_marker = consume_typed_marker(stream);
        let type_start = stream.position();
        let type_end = find_type_annotation_end(stream);
        let tokens = stream.tokens();

        if type_end > type_start {
            let type_tokens = &tokens[type_start..type_end];
            match parse_graph_reference_value_type(type_tokens) {
                Ok(ty) => {
                    type_annotation = Some(ty);
                    stream.set_position(type_end);
                }
                Err(diag) => diags.push(*diag),
            }
        } else if had_typed_marker {
            diags.push(
                Diag::error("Expected graph type after typed marker").with_primary_label(
                    stream.current().span.clone(),
                    "expected graph type here",
                ),
            );
            return (None, diags);
        }
    }

    if !stream.consume(&TokenKind::Eq) {
        diags.push(
            Diag::error("Expected '=' in graph variable definition").with_primary_label(
                stream.current().span.clone(),
                "expected '=' here",
            ),
        );
        return (None, diags);
    }

    let (init_opt, init_diags) = parse_graph_initializer(stream);
    diags.extend(init_diags);
    let Some(init) = init_opt else {
        return (None, diags);
    };
    let initializer = Some(init);

    let def = GraphVariableDefinition {
        is_property,
        variable,
        type_annotation,
        initializer,
        span: start_pos..stream.position(),
    };
    (Some(def), diags)
}

/// Parse a graph initializer.
fn parse_graph_initializer(stream: &mut TokenStream) -> ParseResult<GraphInitializer> {
    let start_pos = stream.position();

    // For now, we parse simple graph expressions
    // Check for CURRENT GRAPH tokens, CURRENT_GRAPH identifier, or variable reference.
    if !stream.check(&TokenKind::Eof) {
        let token = stream.current();
        match &token.kind {
            TokenKind::Current => {
                if let Some(next_token) = stream.peek() {
                    if next_token.kind == TokenKind::Graph {
                        stream.advance();
                        stream.advance();
                        let expr = GraphExpression::CurrentGraph(start_pos..stream.position());
                        return (
                            Some(GraphInitializer {
                                expression: expr,
                                span: start_pos..stream.position(),
                            }),
                            vec![],
                        );
                    }
                }
            }
            TokenKind::Identifier(name) => {
                if name.eq_ignore_ascii_case("CURRENT_GRAPH")
                    || name.eq_ignore_ascii_case("CURRENT_PROPERTY_GRAPH")
                {
                    stream.advance();
                    let expr = GraphExpression::CurrentGraph(start_pos..stream.position());
                    return (
                        Some(GraphInitializer {
                            expression: expr,
                            span: start_pos..stream.position(),
                        }),
                        vec![],
                    );
                }
                let name = name.clone();
                let span = token.span.clone();
                stream.advance();
                let expr = GraphExpression::VariableReference(name, span.clone());
                return (
                    Some(GraphInitializer {
                        expression: expr,
                        span: start_pos..stream.position(),
                    }),
                    vec![],
                );
            }
            _ => {}
        }
    }

    // Fall back to general expression parsing
    match parse_expression_at(stream) {
        Ok(expression) => {
            let init = GraphInitializer {
                expression: GraphExpression::Expression(Box::new(expression)),
                span: start_pos..stream.position(),
            };
            (Some(init), vec![])
        }
        Err(diag) => (None, vec![*diag]),
    }
}

/// Parse a binding table variable definition.
///
/// Grammar: `bindingTableVariableDefinition: BINDING? TABLE bindingVariable optTypedBindingTableInitializer?`
pub fn parse_binding_table_variable_definition(
    stream: &mut TokenStream,
) -> ParseResult<BindingTableVariableDefinition> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Check for optional BINDING keyword
    let is_binding = stream.consume(&TokenKind::Binding);

    // Expect TABLE keyword
    if let Err(diag) = stream.expect(TokenKind::Table) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse binding variable
    let (variable, var_diags) = parse_binding_variable(stream);
    diags.extend(var_diags);

    let variable = match variable {
        Some(v) => v,
        None => {
            return (None, diags);
        }
    };

    // Parse optional type annotation and required initializer.
    let mut type_annotation = None;

    if !stream.check(&TokenKind::Eq) {
        let had_typed_marker = consume_typed_marker(stream);
        let type_start = stream.position();
        let type_end = find_type_annotation_end(stream);
        let tokens = stream.tokens();

        if type_end > type_start {
            let type_tokens = &tokens[type_start..type_end];
            match parse_binding_table_reference_value_type(type_tokens) {
                Ok(ty) => {
                    type_annotation = Some(ty);
                    stream.set_position(type_end);
                }
                Err(diag) => diags.push(*diag),
            }
        } else if had_typed_marker {
            diags.push(
                Diag::error("Expected binding table type after typed marker").with_primary_label(
                    stream.current().span.clone(),
                    "expected binding table type here",
                ),
            );
            return (None, diags);
        }
    }

    if !stream.consume(&TokenKind::Eq) {
        diags.push(
            Diag::error("Expected '=' in binding table variable definition").with_primary_label(
                stream.current().span.clone(),
                "expected '=' here",
            ),
        );
        return (None, diags);
    }

    let (init_opt, init_diags) = parse_binding_table_initializer(stream);
    diags.extend(init_diags);
    let Some(init) = init_opt else {
        return (None, diags);
    };
    let initializer = Some(init);

    let def = BindingTableVariableDefinition {
        is_binding,
        variable,
        type_annotation,
        initializer,
        span: start_pos..stream.position(),
    };
    (Some(def), diags)
}

/// Parse a binding table initializer.
fn parse_binding_table_initializer(
    stream: &mut TokenStream,
) -> ParseResult<BindingTableInitializer> {
    let start_pos = stream.position();

    // For now, we parse simple binding table expressions
    if !stream.check(&TokenKind::Eof) {
        let token = stream.current();
        if let TokenKind::Identifier(name) = &token.kind {
            let name = name.clone();
            let span = token.span.clone();
            stream.advance();
            let expr = BindingTableExpression::VariableReference(name, span);
            return (
                Some(BindingTableInitializer {
                    expression: expr,
                    span: start_pos..stream.position(),
                }),
                vec![],
            );
        }
    }

    // Fall back to general expression parsing
    match parse_expression_at(stream) {
        Ok(expression) => {
            let init = BindingTableInitializer {
                expression: BindingTableExpression::Expression(Box::new(expression)),
                span: start_pos..stream.position(),
            };
            (Some(init), vec![])
        }
        Err(diag) => (None, vec![*diag]),
    }
}

/// Parse a value variable definition.
///
/// Grammar: `valueVariableDefinition: VALUE bindingVariable optTypedValueInitializer?`
pub fn parse_value_variable_definition(
    stream: &mut TokenStream,
) -> ParseResult<ValueVariableDefinition> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect VALUE keyword
    if let Err(diag) = stream.expect(TokenKind::Value) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse binding variable
    let (variable, var_diags) = parse_binding_variable(stream);
    diags.extend(var_diags);

    let variable = match variable {
        Some(v) => v,
        None => {
            return (None, diags);
        }
    };

    // Parse optional type annotation and required initializer.
    let mut type_annotation = None;

    if !stream.check(&TokenKind::Eq) {
        let had_typed_marker = consume_typed_marker(stream);
        let type_start = stream.position();
        let type_end = find_type_annotation_end(stream);
        let tokens = stream.tokens();

        if type_end > type_start {
            let type_tokens = &tokens[type_start..type_end];
            match parse_value_type(type_tokens) {
                Ok(ty) => {
                    type_annotation = Some(ty);
                    stream.set_position(type_end);
                }
                Err(diag) => diags.push(*diag),
            }
        } else if had_typed_marker {
            diags.push(
                Diag::error("Expected value type after typed marker").with_primary_label(
                    stream.current().span.clone(),
                    "expected value type here",
                ),
            );
            return (None, diags);
        }
    }

    if !stream.consume(&TokenKind::Eq) {
        diags.push(
            Diag::error("Expected '=' in value variable definition").with_primary_label(
                stream.current().span.clone(),
                "expected '=' here",
            ),
        );
        return (None, diags);
    }

    let (init_opt, init_diags) = parse_value_initializer(stream);
    diags.extend(init_diags);
    let Some(init) = init_opt else {
        return (None, diags);
    };
    let initializer = Some(init);

    let def = ValueVariableDefinition {
        variable,
        type_annotation,
        initializer,
        span: start_pos..stream.position(),
    };
    (Some(def), diags)
}

/// Parse a value initializer.
fn parse_value_initializer(stream: &mut TokenStream) -> ParseResult<ValueInitializer> {
    let start_pos = stream.position();

    // Parse expression
    match parse_expression_at(stream) {
        Ok(expression) => {
            let init = ValueInitializer {
                expression,
                span: start_pos..stream.position(),
            };
            (Some(init), vec![])
        }
        Err(diag) => (None, vec![*diag]),
    }
}

// ============================================================================
// Statement Blocks and NEXT Chaining (Task 14)
// ============================================================================

/// Parse a statement block.
///
/// Grammar: `statementBlock: statement nextStatement*`
pub fn parse_statement_block(stream: &mut TokenStream) -> ParseResult<StatementBlock> {
    let start_pos = stream.position();
    let mut statements = vec![];
    let mut next_statements = vec![];
    let mut diags = vec![];

    // Parse required initial statement.
    let (first_opt, first_diags) = parse_statement(stream);
    diags.extend(first_diags);
    let Some(first) = first_opt else {
        diags.push(
            Diag::error("Expected statement in procedure body").with_primary_label(
                stream.current().span.clone(),
                "expected statement here",
            ),
        );
        return (None, diags);
    };
    statements.push(first);

    // Parse NEXT statements
    while stream.check(&TokenKind::Next) {
        let (next_opt, next_diags) = parse_next_statement(stream);
        diags.extend(next_diags);

        if let Some(next) = next_opt {
            statements.push((*next.statement).clone());
            next_statements.push(next);
        } else {
            break;
        }
    }

    let end = next_statements
        .last()
        .map(|next| next.span.end)
        .unwrap_or_else(|| statements.last().map(|s| s.span().end).unwrap_or(start_pos));

    let block = StatementBlock {
        statements,
        next_statements,
        span: start_pos..end,
    };
    (Some(block), diags)
}

fn parse_statement(stream: &mut TokenStream) -> ParseResult<ProcedureStatement> {
    let start = stream.position();
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    let mut candidates: Vec<(usize, usize, ProcedureStatement, Vec<Diag>)> = Vec::new();
    let start_kind = &stream.current().kind;
    let is_optional_call = matches!(start_kind, TokenKind::Optional)
        && stream.peek().map(|t| &t.kind) == Some(&TokenKind::Call);

    // Query candidate - use bridge pattern
    if !matches!(start_kind, TokenKind::Create | TokenKind::Drop) {
        let tokens = stream.tokens();
        let mut query_pos = start;
        let (query_opt, query_diags) = parse_query_legacy(tokens, &mut query_pos);
        if let Some(query) = query_opt {
            candidates.push((
                query_pos,
                query_diags.len(),
                ProcedureStatement::CompositeQuery(Box::new(query)),
                query_diags,
            ));
        }
    }

    // Mutation candidate - use bridge pattern
    if matches!(
        start_kind,
        TokenKind::Use
            | TokenKind::Insert
            | TokenKind::Set
            | TokenKind::Remove
            | TokenKind::Delete
            | TokenKind::Detach
            | TokenKind::Nodetach
            | TokenKind::Call
            | TokenKind::Optional
    ) {
        let tokens = stream.tokens();
        let mut mutation_pos = start;
        let (mutation_opt, mutation_diags) =
            parse_linear_data_modifying_statement(tokens, &mut mutation_pos);
        if let Some(mutation) = mutation_opt {
            candidates.push((
                mutation_pos,
                mutation_diags.len(),
                ProcedureStatement::LinearDataModifying(Box::new(mutation)),
                mutation_diags,
            ));
        }
    }

    // Catalog candidate
    if matches!(
        start_kind,
        TokenKind::Create | TokenKind::Drop | TokenKind::Call
    ) || is_optional_call
    {
        let (catalog_opt, catalog_diags, catalog_pos) = parse_catalog_statement_at(stream);
        if let Some(catalog) = catalog_opt {
            candidates.push((
                catalog_pos,
                catalog_diags.len(),
                ProcedureStatement::LinearCatalogModifying(Box::new(catalog)),
                catalog_diags,
            ));
        }
    }

    if candidates.is_empty() {
        return (None, vec![]);
    }

    let mut best_idx = 0usize;
    for idx in 1..candidates.len() {
        let (best_pos, best_diag_count, _, _) = &candidates[best_idx];
        let (pos_candidate, diag_candidate, _, _) = &candidates[idx];
        if pos_candidate > best_pos
            || (pos_candidate == best_pos && diag_candidate < best_diag_count)
        {
            best_idx = idx;
        }
    }

    let (best_pos, _, statement, diags) = candidates.remove(best_idx);
    stream.set_position(best_pos);
    (Some(statement), diags)
}

fn parse_catalog_statement_at(
    stream: &TokenStream,
) -> (Option<LinearCatalogModifyingStatement>, Vec<Diag>, usize) {
    let start = stream.position();
    let end = find_statement_boundary(stream);
    if end <= start {
        return (None, vec![], start);
    }

    let tokens = stream.tokens();
    let statement_tokens = &tokens[start..end];
    match parse_catalog_statement_kind(statement_tokens) {
        Ok(kind) => (Some(kind), vec![], end),
        Err(diag) => (None, vec![*diag], start),
    }
}

fn find_statement_boundary(stream: &TokenStream) -> usize {
    let tokens = stream.tokens();
    let start = stream.position();
    let mut cursor = start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    while cursor < tokens.len() {
        let kind = &tokens[cursor].kind;
        let at_top_level = paren_depth == 0 && bracket_depth == 0 && brace_depth == 0;
        if at_top_level
            && matches!(
                kind,
                TokenKind::Next | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::Eof
            )
        {
            break;
        }

        match kind {
            TokenKind::LParen => paren_depth += 1,
            TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LBracket => bracket_depth += 1,
            TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::LBrace => brace_depth += 1,
            TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }

        cursor += 1;
    }

    cursor
}

/// Parse a NEXT statement.
///
/// Grammar: `nextStatement: NEXT yieldClause? statement`
pub fn parse_next_statement(stream: &mut TokenStream) -> ParseResult<NextStatement> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect NEXT keyword
    if let Err(diag) = stream.expect(TokenKind::Next) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse optional YIELD clause
    let (yield_clause, yield_diags) = if stream.check(&TokenKind::Yield) {
        parse_yield_clause(stream)
    } else {
        (None, vec![])
    };
    diags.extend(yield_diags);

    let (statement_opt, statement_diags) = parse_statement(stream);
    diags.extend(statement_diags);
    let Some(statement) = statement_opt else {
        diags.push(
            Diag::error("Expected statement after NEXT").with_primary_label(
                stream.current().span.clone(),
                "expected statement here",
            ),
        );
        return (None, diags);
    };

    let next_stmt = NextStatement {
        yield_clause,
        span: start_pos..statement.span().end,
        statement: Box::new(statement),
    };
    (Some(next_stmt), diags)
}

// ============================================================================
// AT Schema and USE Graph Clauses (Task 15)
// ============================================================================

/// Parse an AT schema clause.
///
/// Grammar: `atSchemaClause: AT schemaReference`
pub fn parse_at_schema_clause(stream: &mut TokenStream) -> ParseResult<AtSchemaClause> {
    let start_pos = stream.position();
    let mut diags = vec![];

    // Expect AT keyword
    if let Err(diag) = stream.expect(TokenKind::At) {
        diags.push(*diag);
        return (None, diags);
    }

    // Parse the longest schema reference that ends on a known clause boundary.
    let schema_ref_start = stream.position();
    let mut parsed_schema = None;
    let mut schema_ref_end = schema_ref_start;

    let tokens = stream.tokens();
    for end in (schema_ref_start + 1)..=tokens.len() {
        let candidate_tokens = &tokens[schema_ref_start..end];
        if let Ok(schema) = parse_schema_reference(candidate_tokens) {
            let boundary = tokens
                .get(end)
                .map(|token| is_at_schema_follow_boundary(&token.kind))
                .unwrap_or(true);
            if boundary {
                parsed_schema = Some(schema);
                schema_ref_end = end;
            }
        }
    }

    let Some(schema) = parsed_schema else {
        diags.push(
            Diag::error("Expected schema reference after AT keyword").with_primary_label(
                stream.current().span.clone(),
                "expected schema reference here",
            ),
        );
        return (None, diags);
    };
    stream.set_position(schema_ref_end);

    let end_span = schema.span().end;
    let clause = AtSchemaClause {
        schema,
        span: start_pos..end_span,
    };
    (Some(clause), diags)
}

// Note: UseGraphClause is already implemented in src/parser/query.rs (from Sprint 7)
// and will be reused for procedure contexts.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_call_statement_simple() {
        let source = "CALL my_procedure()";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut pos = 0;

        let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
        assert!(stmt_opt.is_some(), "Failed to parse simple CALL statement");
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let stmt = stmt_opt.unwrap();
        assert!(!stmt.optional);
        assert!(matches!(stmt.call, ProcedureCall::Named(_)));
    }

    #[test]
    fn test_parse_optional_call() {
        let source = "OPTIONAL CALL my_procedure()";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut pos = 0;

        let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
        assert!(
            stmt_opt.is_some(),
            "Failed to parse OPTIONAL CALL statement"
        );
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let stmt = stmt_opt.unwrap();
        assert!(stmt.optional);
    }

    #[test]
    fn test_parse_variable_scope_empty() {
        let source = "()";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (scope_opt, diags) = parse_variable_scope_clause(&mut stream);
        assert!(scope_opt.is_some(), "Failed to parse empty variable scope");
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let scope = scope_opt.unwrap();
        assert!(scope.variables.is_empty());
    }

    #[test]
    fn test_parse_variable_scope_with_variables() {
        let source = "(x, y, z)";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (scope_opt, diags) = parse_variable_scope_clause(&mut stream);
        assert!(
            scope_opt.is_some(),
            "Failed to parse variable scope with variables"
        );
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let scope = scope_opt.unwrap();
        assert_eq!(scope.variables.len(), 3);
    }

    #[test]
    fn test_parse_yield_clause() {
        let source = "YIELD result1, result2 AS alias2";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (yield_opt, diags) = parse_yield_clause(&mut stream);
        assert!(yield_opt.is_some(), "Failed to parse YIELD clause");
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let yield_clause = yield_opt.unwrap();
        assert_eq!(yield_clause.items.items.len(), 2);
        assert!(yield_clause.items.items[1].alias.is_some());
    }

    #[test]
    fn test_parse_procedure_argument_list_empty() {
        let source = "()";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (args_opt, diags) = parse_procedure_argument_list(&mut stream);
        assert!(args_opt.is_some(), "Failed to parse empty argument list");
        assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

        let args = args_opt.unwrap();
        assert!(args.arguments.is_empty());
    }

    #[test]
    fn test_parse_procedure_argument_list_with_args() {
        let source = "(1, 'test', x + 5)";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (args_opt, _diags) = parse_procedure_argument_list(&mut stream);
        assert!(
            args_opt.is_some(),
            "Failed to parse argument list with args"
        );

        let args = args_opt.unwrap();
        assert_eq!(args.arguments.len(), 3);
    }

    #[test]
    fn test_parse_value_variable_definition() {
        let source = "VALUE counter";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, diags) = parse_value_variable_definition(&mut stream);
        assert!(def_opt.is_none(), "Expected missing initializer to fail");
        assert!(!diags.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn test_parse_value_variable_definition_with_initializer() {
        let source = "VALUE counter = 0";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, _diags) = parse_value_variable_definition(&mut stream);
        assert!(
            def_opt.is_some(),
            "Failed to parse value variable definition with initializer"
        );

        let def = def_opt.unwrap();
        assert_eq!(def.variable.name.as_str(), "counter");
        assert!(def.initializer.is_some());
    }

    #[test]
    fn test_parse_graph_variable_definition() {
        let source = "GRAPH g";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, diags) = parse_graph_variable_definition(&mut stream);
        assert!(def_opt.is_none(), "Expected missing initializer to fail");
        assert!(!diags.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn test_parse_property_graph_variable_definition() {
        let source = "PROPERTY GRAPH g";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, diags) = parse_graph_variable_definition(&mut stream);
        assert!(def_opt.is_none(), "Expected missing initializer to fail");
        assert!(!diags.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn test_parse_binding_table_variable_definition() {
        let source = "TABLE t";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, diags) = parse_binding_table_variable_definition(&mut stream);
        assert!(def_opt.is_none(), "Expected missing initializer to fail");
        assert!(!diags.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn test_parse_binding_binding_table_variable_definition() {
        let source = "BINDING TABLE t";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (def_opt, diags) = parse_binding_table_variable_definition(&mut stream);
        assert!(def_opt.is_none(), "Expected missing initializer to fail");
        assert!(!diags.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn test_parse_at_schema_clause() {
        let source = "AT my_schema";
        let tokens = Lexer::new(source).tokenize().tokens;
        let mut stream = TokenStream::new(&tokens);

        let (clause_opt, _diags) = parse_at_schema_clause(&mut stream);
        assert!(clause_opt.is_some(), "Failed to parse AT schema clause");

        let _clause = clause_opt.unwrap();
    }
}
