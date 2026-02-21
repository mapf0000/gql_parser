//! Primitive query statement parsing.
//!
//! This module handles parsing of primitive query statements including:
//! - USE clauses
//! - MATCH statements (simple and optional)
//! - FILTER statements
//! - LET statements
//! - FOR statements

use crate::ast::query::*;
use crate::ast::references::BindingVariable;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use crate::parser::base::TokenStream;
use crate::parser::patterns::parse_graph_pattern;
use crate::parser::procedure::parse_call_procedure_statement;
use crate::parser::types::parse_value_type_prefix;
use smol_str::SmolStr;

use super::{
    ParseResult, is_query_clause_boundary, parse_expression_with_diags,
    skip_to_query_clause_boundary,
};

// Import functions from sibling modules that are needed by parse_primitive_query_statement
use super::pagination::parse_order_by_and_page_statement;
use super::result::parse_select_statement;

/// Parses a primitive query statement.
pub(crate) fn parse_primitive_query_statement(
    stream: &mut TokenStream,
) -> ParseResult<PrimitiveQueryStatement> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    let result = match &stream.current().kind {
        TokenKind::Match => {
            let (match_opt, diags) = parse_match_statement(stream);
            (match_opt.map(PrimitiveQueryStatement::Match), diags)
        }
        TokenKind::Call => {
            // Call procedure still uses legacy interface
            let tokens = stream.tokens();
            let mut pos = stream.position();
            let (call_opt, diags) = parse_call_procedure_statement(tokens, &mut pos);
            stream.set_position(pos);
            (call_opt.map(PrimitiveQueryStatement::Call), diags)
        }
        TokenKind::Optional => {
            if matches!(stream.peek().map(|t| &t.kind), Some(TokenKind::Call)) {
                // Call procedure still uses legacy interface
                let tokens = stream.tokens();
                let mut pos = stream.position();
                let (call_opt, diags) = parse_call_procedure_statement(tokens, &mut pos);
                stream.set_position(pos);
                (call_opt.map(PrimitiveQueryStatement::Call), diags)
            } else {
                let (match_opt, diags) = parse_match_statement(stream);
                (match_opt.map(PrimitiveQueryStatement::Match), diags)
            }
        }
        TokenKind::Filter => {
            let (filter_opt, diags) = parse_filter_statement(stream);
            (filter_opt.map(PrimitiveQueryStatement::Filter), diags)
        }
        TokenKind::Let => {
            let (let_opt, diags) = parse_let_statement(stream);
            (let_opt.map(PrimitiveQueryStatement::Let), diags)
        }
        TokenKind::For => {
            let (for_opt, diags) = parse_for_statement(stream);
            (for_opt.map(PrimitiveQueryStatement::For), diags)
        }
        TokenKind::Order => {
            let (order_opt, diags) = parse_order_by_and_page_statement(stream);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Limit | TokenKind::Offset | TokenKind::Skip => {
            let (order_opt, diags) = parse_order_by_and_page_statement(stream);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Select => {
            let (select_opt, diags) = parse_select_statement(stream);
            (
                select_opt.map(|select| PrimitiveQueryStatement::Select(Box::new(select))),
                diags,
            )
        }
        TokenKind::With => {
            let (select_opt, diags) = parse_select_statement(stream);
            (
                select_opt.map(|select| PrimitiveQueryStatement::Select(Box::new(select))),
                diags,
            )
        }
        _ => (None, vec![]),
    };

    result
}

// ============================================================================
// USE Clause (Task 23)
// ============================================================================

/// Parses a USE clause.
pub(crate) fn parse_use_graph_clause(stream: &mut TokenStream) -> ParseResult<UseGraphClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Use) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse graph expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (graph_opt, mut expr_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut expr_diags);

    let graph = match graph_opt {
        Some(g) => g,
        None => {
            diags.push(
                Diag::error("Expected graph expression after USE")
                    .with_primary_label(stream.current().span.clone(), "expected expression here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(UseGraphClause {
            graph,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Match Statements (Task 15)
// ============================================================================

/// Parses a MATCH statement (simple or optional).
fn parse_match_statement(stream: &mut TokenStream) -> ParseResult<MatchStatement> {
    if stream.check(&TokenKind::Optional) {
        let (opt_match, diags) = parse_optional_match_statement(stream);
        (opt_match.map(MatchStatement::Optional), diags)
    } else if stream.check(&TokenKind::Match) {
        let (simple_match, diags) = parse_simple_match_statement(stream);
        (
            simple_match.map(|simple| MatchStatement::Simple(Box::new(simple))),
            diags,
        )
    } else {
        (None, vec![])
    }
}

/// Parses a simple MATCH statement.
fn parse_simple_match_statement(stream: &mut TokenStream) -> ParseResult<SimpleMatchStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Match) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse graph pattern with integration-level progress guarantees.
    let (pattern_opt, mut pattern_diags) = parse_graph_pattern_checked(stream);
    diags.append(&mut pattern_diags);

    let pattern = match pattern_opt {
        Some(p) => p,
        None => {
            diags.push(
                Diag::error("Expected graph pattern after MATCH")
                    .with_primary_label(stream.current().span.clone(), "expected pattern here"),
            );
            return (None, diags);
        }
    };

    let end = pattern.span.end;

    (
        Some(SimpleMatchStatement {
            pattern,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an OPTIONAL MATCH statement.
fn parse_optional_match_statement(stream: &mut TokenStream) -> ParseResult<OptionalMatchStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Optional) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse optional operand
    let (operand_opt, mut operand_diags) = parse_optional_operand(stream);
    diags.append(&mut operand_diags);

    let operand = match operand_opt {
        Some(op) => op,
        None => {
            diags.push(
                Diag::error("Expected MATCH, '{', or '(' after OPTIONAL")
                    .with_primary_label(stream.current().span.clone(), "expected operand here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(OptionalMatchStatement {
            operand,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an optional operand (MATCH, block, or parenthesized block).
fn parse_optional_operand(stream: &mut TokenStream) -> ParseResult<OptionalOperand> {
    match &stream.current().kind {
        TokenKind::Match => {
            stream.advance();
            let (pattern_opt, diags) = parse_graph_pattern_checked(stream);
            (
                pattern_opt.map(|pattern| OptionalOperand::Match {
                    pattern: Box::new(pattern),
                }),
                diags,
            )
        }
        TokenKind::LBrace => {
            stream.advance();
            let (statements, mut diags) = parse_match_statement_block(stream);

            if stream.check(&TokenKind::RBrace) {
                stream.advance();
            } else {
                diags.push(
                    Diag::error("Expected '}' to close OPTIONAL block")
                        .with_primary_label(stream.current().span.clone(), "expected '}' here"),
                );
            }

            (Some(OptionalOperand::Block { statements }), diags)
        }
        TokenKind::LParen => {
            stream.advance();
            let (statements, mut diags) = parse_match_statement_block(stream);

            if stream.check(&TokenKind::RParen) {
                stream.advance();
            } else {
                diags.push(
                    Diag::error("Expected ')' to close OPTIONAL parenthesized block")
                        .with_primary_label(stream.current().span.clone(), "expected ')' here"),
                );
            }

            (
                Some(OptionalOperand::ParenthesizedBlock { statements }),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

/// Parses a match statement block (multiple MATCH statements).
fn parse_match_statement_block(stream: &mut TokenStream) -> (Vec<MatchStatement>, Vec<Diag>) {
    let mut statements = Vec::new();
    let mut diags = Vec::new();

    while !stream.check(&TokenKind::RBrace)
        && !stream.check(&TokenKind::RParen)
        && !stream.check(&TokenKind::Eof)
    {
        let (stmt_opt, mut stmt_diags) = parse_match_statement(stream);
        diags.append(&mut stmt_diags);

        match stmt_opt {
            Some(stmt) => statements.push(stmt),
            None => break,
        }
    }

    (statements, diags)
}

/// Wrapper for graph-pattern parsing with progress and diagnostic guarantees.
fn parse_graph_pattern_checked(stream: &mut TokenStream) -> ParseResult<GraphPattern> {
    let start = stream.position();

    // Need to use legacy interface for parse_graph_pattern
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (pattern_opt, mut diags) = parse_graph_pattern(tokens, &mut pos);
    stream.set_position(pos);

    if pattern_opt.is_some() && stream.position() == start {
        diags.push(
            Diag::error("Graph pattern parser succeeded without consuming input")
                .with_primary_label(stream.current().span.clone(), "pattern parser stalled here"),
        );

        // Skip to boundary using legacy interface
        let tokens = stream.tokens();
        let mut pos = stream.position();
        skip_to_query_clause_boundary(tokens, &mut pos);
        stream.set_position(pos);
        return (None, diags);
    }

    if pattern_opt.is_none() && stream.position() == start && diags.is_empty() {
        diags.push(
            Diag::error("Expected graph pattern")
                .with_primary_label(stream.current().span.clone(), "expected pattern here"),
        );
    }

    (pattern_opt, diags)
}

// ============================================================================
// Filter Statements (Task 16)
// ============================================================================

/// Parses a FILTER statement.
fn parse_filter_statement(stream: &mut TokenStream) -> ParseResult<FilterStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Filter) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Check for optional WHERE keyword
    let where_optional = if stream.check(&TokenKind::Where) {
        stream.advance();
        true
    } else {
        false
    };

    // Parse condition expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (condition_opt, mut cond_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut cond_diags);

    let condition = match condition_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected search condition after FILTER")
                    .with_primary_label(stream.current().span.clone(), "expected condition here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(FilterStatement {
            where_optional,
            condition,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Let Statements (Task 17)
// ============================================================================

/// Parses a LET statement.
fn parse_let_statement(stream: &mut TokenStream) -> ParseResult<LetStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Let) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse variable definitions (comma-separated)
    let mut bindings = Vec::new();

    loop {
        let (binding_opt, mut binding_diags) = parse_let_variable_definition(stream);
        diags.append(&mut binding_diags);

        match binding_opt {
            Some(binding) => bindings.push(binding),
            None => break,
        }

        // Check for comma (more bindings)
        if stream.check(&TokenKind::Comma) {
            stream.advance();
        } else {
            break;
        }
    }

    if bindings.is_empty() {
        diags.push(
            Diag::error("Expected variable definition after LET").with_primary_label(
                stream.current().span.clone(),
                "expected variable definition here",
            ),
        );
        return (None, diags);
    }

    let end = bindings.last().map(|b| b.span.end).unwrap_or(start);

    (
        Some(LetStatement {
            bindings,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a LET variable definition.
fn parse_let_variable_definition(stream: &mut TokenStream) -> ParseResult<LetVariableDefinition> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Parse variable name
    let variable = match &stream.current().kind {
        TokenKind::Identifier(name) => {
            let var = BindingVariable {
                name: name.clone(),
                span: stream.current().span.clone(),
            };
            stream.advance();
            var
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let var = BindingVariable {
                name: SmolStr::new(kind.to_string()),
                span: stream.current().span.clone(),
            };
            stream.advance();
            var
        }
        _ => {
            diags.push(
                Diag::error("Expected variable name in LET definition")
                    .with_primary_label(stream.current().span.clone(), "expected identifier here"),
            );
            return (None, diags);
        }
    };

    // Parse optional type annotation
    let type_annotation = if stream.check(&TokenKind::DoubleColon) {
        stream.advance();

        // Need to use legacy interface for type parsing
        let tokens = stream.tokens();
        let pos = stream.position();
        match parse_value_type_prefix(&tokens[pos..]) {
            Ok((value_type, consumed)) => {
                stream.set_position(pos + consumed);
                Some(value_type)
            }
            Err(err) => {
                diags.push(*err);
                None
            }
        }
    } else {
        None
    };

    // Expect '='
    if stream.check(&TokenKind::Eq) {
        stream.advance();
    } else {
        diags.push(
            Diag::error("Expected '=' in LET variable definition")
                .with_primary_label(stream.current().span.clone(), "expected '=' here"),
        );
        return (None, diags);
    }

    // Parse value expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut value_diags);

    let value = match value_opt {
        Some(v) => v,
        None => {
            diags.push(
                Diag::error("Expected value expression in LET definition")
                    .with_primary_label(stream.current().span.clone(), "expected expression here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(LetVariableDefinition {
            variable,
            type_annotation,
            value,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// For Statements (Task 18)
// ============================================================================

/// Parses a FOR statement.
fn parse_for_statement(stream: &mut TokenStream) -> ParseResult<ForStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::For) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse FOR item
    let (item_opt, mut item_diags) = parse_for_item(stream);
    diags.append(&mut item_diags);

    let item = match item_opt {
        Some(i) => i,
        None => {
            diags.push(
                Diag::error("Expected FOR item specification").with_primary_label(
                    stream.current().span.clone(),
                    "expected 'variable IN expression' here",
                ),
            );
            return (None, diags);
        }
    };

    // Parse optional WITH ORDINALITY/OFFSET
    let ordinality_or_offset = if stream.check(&TokenKind::With) {
        stream.advance();
        let (ordinality_opt, mut ordinality_diags) = parse_for_ordinality_or_offset(stream);
        diags.append(&mut ordinality_diags);
        ordinality_opt
    } else {
        None
    };

    let end = stream.previous_span().end;

    (
        Some(ForStatement {
            item,
            ordinality_or_offset,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a FOR item (variable IN collection).
fn parse_for_item(stream: &mut TokenStream) -> ParseResult<ForItem> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Parse binding variable
    let binding_variable = match &stream.current().kind {
        TokenKind::Identifier(name) => {
            let var = BindingVariable {
                name: name.clone(),
                span: stream.current().span.clone(),
            };
            stream.advance();
            var
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let var = BindingVariable {
                name: SmolStr::new(kind.to_string()),
                span: stream.current().span.clone(),
            };
            stream.advance();
            var
        }
        _ => {
            diags.push(
                Diag::error("Expected variable name in FOR statement")
                    .with_primary_label(stream.current().span.clone(), "expected identifier here"),
            );
            return (None, diags);
        }
    };

    // Expect IN keyword
    if stream.check(&TokenKind::In) {
        stream.advance();
    } else {
        diags.push(
            Diag::error("Expected IN keyword in FOR statement")
                .with_primary_label(stream.current().span.clone(), "expected 'IN' here"),
        );
        return (None, diags);
    }

    // Parse collection expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (collection_opt, mut coll_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut coll_diags);

    let collection = match collection_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected collection expression in FOR statement")
                    .with_primary_label(stream.current().span.clone(), "expected expression here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(ForItem {
            binding_variable,
            collection,
            span: start..end,
        }),
        diags,
    )
}

/// Parses WITH ORDINALITY or WITH OFFSET clause.
fn parse_for_ordinality_or_offset(stream: &mut TokenStream) -> ParseResult<ForOrdinalityOrOffset> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let expects_ordinality = match &stream.current().kind {
        TokenKind::Ordinality => {
            stream.advance();
            true
        }
        TokenKind::Offset => {
            stream.advance();
            false
        }
        _ => {
            diags.push(
                Diag::error("Expected ORDINALITY or OFFSET after WITH").with_primary_label(
                    stream.current().span.clone(),
                    "expected ORDINALITY or OFFSET here",
                ),
            );

            // Skip to boundary using legacy interface
            let tokens = stream.tokens();
            let mut pos = stream.position();
            skip_to_query_clause_boundary(tokens, &mut pos);
            stream.set_position(pos);
            return (None, diags);
        }
    };

    // AS is optional in this clause.
    if stream.check(&TokenKind::As) {
        stream.advance();
    }

    let variable = match &stream.current().kind {
        TokenKind::Identifier(name) => {
            let variable = BindingVariable {
                name: name.clone(),
                span: stream.current().span.clone(),
            };
            stream.advance();
            variable
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let variable = BindingVariable {
                name: SmolStr::new(kind.to_string()),
                span: stream.current().span.clone(),
            };
            stream.advance();
            variable
        }
        _ => {
            let keyword = if expects_ordinality {
                "ORDINALITY"
            } else {
                "OFFSET"
            };
            diags.push(
                Diag::error(format!("Expected variable name after WITH {keyword}"))
                    .with_primary_label(stream.current().span.clone(), "expected identifier here"),
            );
            if !is_query_clause_boundary(&stream.current().kind) {
                // Skip to boundary using legacy interface
                let tokens = stream.tokens();
                let mut pos = stream.position();
                skip_to_query_clause_boundary(tokens, &mut pos);
                stream.set_position(pos);
            }
            return (None, diags);
        }
    };

    if expects_ordinality {
        (Some(ForOrdinalityOrOffset::Ordinality { variable }), diags)
    } else {
        (Some(ForOrdinalityOrOffset::Offset { variable }), diags)
    }
}
