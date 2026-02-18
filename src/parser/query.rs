//! Query statement parser for GQL.
//!
//! This module implements parsing for the GQL query pipeline, including:
//! - Composite queries with set operators (UNION, EXCEPT, INTERSECT, OTHERWISE)
//! - Linear queries with sequential statement chaining
//! - Primitive query statements (MATCH, FILTER, LET, FOR, SELECT)
//! - Result statements (RETURN)
//! - Ordering, pagination, and grouping

use crate::ast::query::*;
use crate::ast::references::BindingVariable;
use crate::ast::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::expression::parse_expression;
use crate::parser::patterns::parse_graph_pattern;
use crate::parser::procedure::parse_call_procedure_statement;
use crate::parser::types::parse_value_type_prefix;
use smol_str::SmolStr;

/// Parse result with optional value and diagnostics.
pub(crate) type ParseResult<T> = (Option<T>, Vec<Diag>);

// ============================================================================
// Expression Parser Adapter
// ============================================================================

/// Helper to find the boundary of an expression in a token stream.
/// Returns the number of tokens that should be consumed for the expression.
fn find_expression_boundary(tokens: &[Token], start_pos: usize) -> usize {
    let mut pos = start_pos;
    let mut depth = 0; // Track nesting depth for parentheses, brackets, braces

    while pos < tokens.len() {
        let token = &tokens[pos];

        // Track nesting depth
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
                    // Closing delimiter at depth 0 means we've hit a boundary
                    break;
                }
            }
            _ => {}
        }

        // At depth 0, check for statement keywords that terminate expressions
        if depth == 0 {
            match &token.kind {
                    // Statement keywords
                    TokenKind::Match | TokenKind::Filter | TokenKind::Let | TokenKind::For |
                    TokenKind::Order | TokenKind::Limit | TokenKind::Offset | TokenKind::Skip |
                    TokenKind::Return | TokenKind::Select | TokenKind::Finish |
                    // Mutation keywords (for USE graph in data-modifying statements)
                    TokenKind::Insert | TokenKind::Set | TokenKind::Remove | TokenKind::Delete |
                    TokenKind::Detach | TokenKind::Nodetach | TokenKind::Call |
                    // Set operators
                    TokenKind::Union | TokenKind::Except | TokenKind::Intersect | TokenKind::Otherwise |
                    // Clause keywords
                TokenKind::From | TokenKind::Where | TokenKind::Group | TokenKind::Having |
                TokenKind::By | TokenKind::With | TokenKind::As |
                TokenKind::Asc | TokenKind::Ascending | TokenKind::Desc | TokenKind::Descending |
                TokenKind::Nulls |
                // Terminators
                TokenKind::Semicolon | TokenKind::Eof => {
                    break;
                }
                // Comma at depth 0 ends the expression (for list contexts)
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

    // Return the count of tokens to consume
    pos - start_pos
}

/// Parses an expression starting at the current position.
/// Returns the parsed expression and updates the position.
fn parse_expression_at(tokens: &[Token], pos: &mut usize) -> Result<Expression, Box<Diag>> {
    let start_pos = *pos;

    // Find the boundary of the expression
    let count = find_expression_boundary(tokens, start_pos);

    if count == 0 {
        return Err(Box::new(
            Diag::error("Expected expression").with_primary_label(
                tokens
                    .get(start_pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start_pos..start_pos),
                "expected expression here",
            ),
        ));
    }

    // Slice tokens for the expression
    let expr_tokens = &tokens[start_pos..start_pos + count];

    // Parse the expression
    let result = parse_expression(expr_tokens)?;

    // Update position
    *pos = start_pos + count;

    Ok(result)
}

/// Helper to wrap expression parsing errors into diagnostics.
pub(crate) fn parse_expression_with_diags(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<Expression> {
    match parse_expression_at(tokens, pos) {
        Ok(expr) => (Some(expr), vec![]),
        Err(diag) => (None, vec![*diag]),
    }
}

// ============================================================================
// Query Entry Point
// ============================================================================

/// Parses a query statement (top-level entry point).
///
/// This handles composite queries, linear queries, and parenthesized queries.
pub fn parse_query(tokens: &[Token], pos: &mut usize) -> ParseResult<Query> {
    let start_pos = *pos;
    let (query_opt, mut diags) = parse_composite_query(tokens, pos);

    // Parser contract: a successful parse must always consume input.
    if *pos == start_pos {
        if diags.is_empty() {
            diags.push(
                Diag::error("Expected query statement").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start_pos..start_pos),
                    "expected query statement here",
                ),
            );
        }
        return (None, diags);
    }

    (query_opt, diags)
}

// ============================================================================
// Composite Queries (Task 13)
// ============================================================================

/// Parses a composite query with set operators.
///
/// Grammar:
/// ```text
/// compositeQueryStatement:
///     linearQueryStatement (setOperator linearQueryStatement)*
/// ```
fn parse_composite_query(tokens: &[Token], pos: &mut usize) -> ParseResult<Query> {
    let mut diags = Vec::new();

    // Parse first linear query
    let (left_opt, mut left_diags) = parse_linear_query_as_query(tokens, pos);
    diags.append(&mut left_diags);

    let mut left = match left_opt {
        Some(q) => q,
        None => return (None, diags),
    };

    // Parse set operators and additional queries (left-associative)
    while *pos < tokens.len() {
        // Check for set operator
        let op_pos = *pos;
        let operator_opt = match &tokens[*pos].kind {
            TokenKind::Union => {
                *pos += 1;
                let quantifier = parse_set_quantifier_opt(tokens, pos).unwrap_or_default();
                Some(SetOperator::Union { quantifier })
            }
            TokenKind::Except => {
                *pos += 1;
                let quantifier = parse_set_quantifier_opt(tokens, pos).unwrap_or_default();
                Some(SetOperator::Except { quantifier })
            }
            TokenKind::Intersect => {
                *pos += 1;
                let quantifier = parse_set_quantifier_opt(tokens, pos).unwrap_or_default();
                Some(SetOperator::Intersect { quantifier })
            }
            TokenKind::Otherwise => {
                *pos += 1;
                Some(SetOperator::Otherwise)
            }
            _ => None,
        };

        let operator = match operator_opt {
            Some(op) => op,
            None => break, // No more set operators
        };

        // Parse right operand
        let (right_opt, mut right_diags) = parse_linear_query_as_query(tokens, pos);
        diags.append(&mut right_diags);

        let right = match right_opt {
            Some(q) => q,
            None => {
                diags.push(
                    Diag::error(format!("Expected query after {:?} operator", operator))
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map(|t| t.span.clone())
                                .unwrap_or(op_pos..op_pos),
                            "expected query here",
                        ),
                );
                break;
            }
        };

        let span = left.span().start..right.span().end;
        left = Query::Composite(CompositeQuery {
            left: Box::new(left),
            operator,
            right: Box::new(right),
            span,
        });
    }

    (Some(left), diags)
}

/// Parses optional set quantifier (ALL or DISTINCT).
/// Returns Some(quantifier) if ALL or DISTINCT keyword is present, None otherwise.
fn parse_set_quantifier_opt(tokens: &[Token], pos: &mut usize) -> Option<SetQuantifier> {
    if *pos < tokens.len() {
        match &tokens[*pos].kind {
            TokenKind::All => {
                *pos += 1;
                Some(SetQuantifier::All)
            }
            TokenKind::Distinct => {
                *pos += 1;
                Some(SetQuantifier::Distinct)
            }
            _ => None,
        }
    } else {
        None
    }
}

// ============================================================================
// Linear Queries (Task 14)
// ============================================================================

/// Parses a linear query and wraps it in Query enum.
fn parse_linear_query_as_query(tokens: &[Token], pos: &mut usize) -> ParseResult<Query> {
    // Check for parenthesized query
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::LParen) {
        let start = tokens[*pos].span.start;
        *pos += 1;

        let (query_opt, mut diags) = parse_composite_query(tokens, pos);

        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RParen) {
            let end = tokens[*pos].span.end;
            *pos += 1;

            if let Some(query) = query_opt {
                return (
                    Some(Query::Parenthesized(Box::new(query), start..end)),
                    diags,
                );
            }
        } else {
            diags.push(
                Diag::error("Expected ')' to close parenthesized query").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected ')' here",
                ),
            );
        }

        return (None, diags);
    }

    let (linear_opt, diags) = parse_linear_query(tokens, pos);
    (linear_opt.map(Query::Linear), diags)
}

/// Parses a linear query statement.
fn parse_linear_query(tokens: &[Token], pos: &mut usize) -> ParseResult<LinearQuery> {
    let mut diags = Vec::new();

    // Check for USE clause (focused query)
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Use) {
        let (focused_opt, mut focused_diags) = parse_focused_linear_query(tokens, pos);
        diags.append(&mut focused_diags);
        return (focused_opt.map(LinearQuery::Focused), diags);
    }

    // Otherwise, ambient query
    let (ambient_opt, mut ambient_diags) = parse_ambient_linear_query(tokens, pos);
    diags.append(&mut ambient_diags);
    (ambient_opt.map(LinearQuery::Ambient), diags)
}

/// Parses a focused linear query (with USE clause).
fn parse_focused_linear_query(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<FocusedLinearQuery> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse USE clause
    let (use_graph_opt, mut use_diags) = parse_use_graph_clause(tokens, pos);
    diags.append(&mut use_diags);

    let use_graph = match use_graph_opt {
        Some(ug) => ug,
        None => return (None, diags),
    };

    // Parse primitive statements and result statement
    let (primitive_statements, result_statement, has_query_body, mut stmt_diags, end) =
        parse_query_statements(tokens, pos, start);
    diags.append(&mut stmt_diags);

    if !has_query_body {
        diags.push(
            Diag::error("Expected query statement after USE clause").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(use_graph.span.clone()),
                "expected query statement here",
            ),
        );
        return (None, diags);
    }

    (
        Some(FocusedLinearQuery {
            use_graph,
            primitive_statements,
            result_statement,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an ambient linear query (without USE clause).
fn parse_ambient_linear_query(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<AmbientLinearQuery> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse primitive statements and result statement
    let (primitive_statements, result_statement, has_query_body, mut stmt_diags, end) =
        parse_query_statements(tokens, pos, start);
    diags.append(&mut stmt_diags);

    if !has_query_body {
        diags.push(
            Diag::error("Expected query statement").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected query statement here",
            ),
        );
        return (None, diags);
    }

    (
        Some(AmbientLinearQuery {
            primitive_statements,
            result_statement,
            span: start..end,
        }),
        diags,
    )
}

/// Helper to parse query statements (primitive + optional result).
fn parse_query_statements(
    tokens: &[Token],
    pos: &mut usize,
    start: usize,
) -> (
    Vec<PrimitiveQueryStatement>,
    Option<Box<PrimitiveResultStatement>>,
    bool,
    Vec<Diag>,
    usize,
) {
    let mut diags = Vec::new();
    let mut primitive_statements = Vec::new();
    let mut result_statement = None;

    loop {
        if *pos >= tokens.len() {
            break;
        }

        // Check for result statement (RETURN or FINISH)
        if matches!(tokens[*pos].kind, TokenKind::Return) {
            let (return_opt, mut return_diags) = parse_return_statement(tokens, pos);
            diags.append(&mut return_diags);

            if let Some(ret) = return_opt {
                result_statement = Some(Box::new(PrimitiveResultStatement::Return(ret)));
            }
            break;
        }

        if matches!(tokens[*pos].kind, TokenKind::Finish) {
            let span = tokens[*pos].span.clone();
            *pos += 1;
            result_statement = Some(Box::new(PrimitiveResultStatement::Finish(span)));
            break;
        }

        // Try to parse primitive statement
        let (stmt_opt, mut stmt_diags) = parse_primitive_query_statement(tokens, pos);
        diags.append(&mut stmt_diags);

        match stmt_opt {
            Some(stmt) => primitive_statements.push(stmt),
            None => break, // No more statements
        }
    }

    let end = tokens.get(*pos).map(|t| t.span.start).unwrap_or(start);
    let has_query_body = !primitive_statements.is_empty() || result_statement.is_some();
    (
        primitive_statements,
        result_statement,
        has_query_body,
        diags,
        end,
    )
}

/// Parses a primitive query statement.
pub(crate) fn parse_primitive_query_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<PrimitiveQueryStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match &tokens[*pos].kind {
        TokenKind::Match => {
            let (match_opt, diags) = parse_match_statement(tokens, pos);
            (match_opt.map(PrimitiveQueryStatement::Match), diags)
        }
        TokenKind::Call => {
            let (call_opt, diags) = parse_call_procedure_statement(tokens, pos);
            (call_opt.map(PrimitiveQueryStatement::Call), diags)
        }
        TokenKind::Optional => {
            if matches!(tokens.get(*pos + 1).map(|t| &t.kind), Some(TokenKind::Call)) {
                let (call_opt, diags) = parse_call_procedure_statement(tokens, pos);
                (call_opt.map(PrimitiveQueryStatement::Call), diags)
            } else {
                let (match_opt, diags) = parse_match_statement(tokens, pos);
                (match_opt.map(PrimitiveQueryStatement::Match), diags)
            }
        }
        TokenKind::Filter => {
            let (filter_opt, diags) = parse_filter_statement(tokens, pos);
            (filter_opt.map(PrimitiveQueryStatement::Filter), diags)
        }
        TokenKind::Let => {
            let (let_opt, diags) = parse_let_statement(tokens, pos);
            (let_opt.map(PrimitiveQueryStatement::Let), diags)
        }
        TokenKind::For => {
            let (for_opt, diags) = parse_for_statement(tokens, pos);
            (for_opt.map(PrimitiveQueryStatement::For), diags)
        }
        TokenKind::Order => {
            let (order_opt, diags) = parse_order_by_and_page_statement(tokens, pos);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Limit | TokenKind::Offset | TokenKind::Skip => {
            let (order_opt, diags) = parse_order_by_and_page_statement(tokens, pos);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Select => {
            let (select_opt, diags) = parse_select_statement(tokens, pos);
            (
                select_opt.map(|select| PrimitiveQueryStatement::Select(Box::new(select))),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

// ============================================================================
// USE Clause (Task 23)
// ============================================================================

/// Parses a USE clause.
pub(crate) fn parse_use_graph_clause(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<UseGraphClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Use) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse graph expression
    let (graph_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let graph = match graph_opt {
        Some(g) => g,
        None => {
            diags.push(
                Diag::error("Expected graph expression after USE").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

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
fn parse_match_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<MatchStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    if matches!(tokens[*pos].kind, TokenKind::Optional) {
        let (opt_match, diags) = parse_optional_match_statement(tokens, pos);
        (opt_match.map(MatchStatement::Optional), diags)
    } else if matches!(tokens[*pos].kind, TokenKind::Match) {
        let (simple_match, diags) = parse_simple_match_statement(tokens, pos);
        (
            simple_match.map(|simple| MatchStatement::Simple(Box::new(simple))),
            diags,
        )
    } else {
        (None, vec![])
    }
}

/// Parses a simple MATCH statement.
fn parse_simple_match_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<SimpleMatchStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Match) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse graph pattern with integration-level progress guarantees.
    let (pattern_opt, mut pattern_diags) = parse_graph_pattern_checked(tokens, pos);
    diags.append(&mut pattern_diags);

    let pattern = match pattern_opt {
        Some(p) => p,
        None => {
            diags.push(
                Diag::error("Expected graph pattern after MATCH").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected pattern here",
                ),
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
fn parse_optional_match_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<OptionalMatchStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Optional) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse optional operand
    let (operand_opt, mut operand_diags) = parse_optional_operand(tokens, pos);
    diags.append(&mut operand_diags);

    let operand = match operand_opt {
        Some(op) => op,
        None => {
            diags.push(
                Diag::error("Expected MATCH, '{', or '(' after OPTIONAL").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected operand here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(OptionalMatchStatement {
            operand,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an optional operand (MATCH, block, or parenthesized block).
fn parse_optional_operand(tokens: &[Token], pos: &mut usize) -> ParseResult<OptionalOperand> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match &tokens[*pos].kind {
        TokenKind::Match => {
            *pos += 1;
            let (pattern_opt, diags) = parse_graph_pattern_checked(tokens, pos);
            (
                pattern_opt.map(|pattern| OptionalOperand::Match {
                    pattern: Box::new(pattern),
                }),
                diags,
            )
        }
        TokenKind::LBrace => {
            *pos += 1;
            let (statements, mut diags) = parse_match_statement_block(tokens, pos);

            if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RBrace) {
                *pos += 1;
            } else {
                diags.push(
                    Diag::error("Expected '}' to close OPTIONAL block").with_primary_label(
                        tokens.get(*pos).map(|t| t.span.clone()).unwrap_or(0..0),
                        "expected '}' here",
                    ),
                );
            }

            (Some(OptionalOperand::Block { statements }), diags)
        }
        TokenKind::LParen => {
            *pos += 1;
            let (statements, mut diags) = parse_match_statement_block(tokens, pos);

            if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RParen) {
                *pos += 1;
            } else {
                diags.push(
                    Diag::error("Expected ')' to close OPTIONAL parenthesized block")
                        .with_primary_label(
                            tokens.get(*pos).map(|t| t.span.clone()).unwrap_or(0..0),
                            "expected ')' here",
                        ),
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
fn parse_match_statement_block(
    tokens: &[Token],
    pos: &mut usize,
) -> (Vec<MatchStatement>, Vec<Diag>) {
    let mut statements = Vec::new();
    let mut diags = Vec::new();

    while *pos < tokens.len() {
        if matches!(tokens[*pos].kind, TokenKind::RBrace | TokenKind::RParen) {
            break;
        }

        let (stmt_opt, mut stmt_diags) = parse_match_statement(tokens, pos);
        diags.append(&mut stmt_diags);

        match stmt_opt {
            Some(stmt) => statements.push(stmt),
            None => break,
        }
    }

    (statements, diags)
}

/// Wrapper for graph-pattern parsing with progress and diagnostic guarantees.
fn parse_graph_pattern_checked(tokens: &[Token], pos: &mut usize) -> ParseResult<GraphPattern> {
    let start = *pos;
    let (pattern_opt, mut diags) = parse_graph_pattern(tokens, pos);

    if pattern_opt.is_some() && *pos == start {
        diags.push(
            Diag::error("Graph pattern parser succeeded without consuming input")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "pattern parser stalled here",
                ),
        );
        skip_to_query_clause_boundary(tokens, pos);
        return (None, diags);
    }

    if pattern_opt.is_none() && *pos == start && diags.is_empty() {
        diags.push(
            Diag::error("Expected graph pattern").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected pattern here",
            ),
        );
    }

    (pattern_opt, diags)
}

fn is_query_clause_boundary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Match
            | TokenKind::Optional
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
            | TokenKind::Union
            | TokenKind::Except
            | TokenKind::Intersect
            | TokenKind::Otherwise
            | TokenKind::Where
            | TokenKind::Group
            | TokenKind::Having
            | TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::RBrace
            | TokenKind::RParen
    )
}

fn skip_to_query_clause_boundary(tokens: &[Token], pos: &mut usize) {
    while *pos < tokens.len() {
        if is_query_clause_boundary(&tokens[*pos].kind) {
            break;
        }
        *pos += 1;
    }
}

// ============================================================================
// Filter Statements (Task 16)
// ============================================================================

/// Parses a FILTER statement.
fn parse_filter_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<FilterStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Filter) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Check for optional WHERE keyword
    let where_optional = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Where) {
        *pos += 1;
        true
    } else {
        false
    };

    // Parse condition expression
    let (condition_opt, mut cond_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut cond_diags);

    let condition = match condition_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected search condition after FILTER").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected condition here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

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
fn parse_let_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<LetStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Let) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse variable definitions (comma-separated)
    let mut bindings = Vec::new();

    loop {
        let (binding_opt, mut binding_diags) = parse_let_variable_definition(tokens, pos);
        diags.append(&mut binding_diags);

        match binding_opt {
            Some(binding) => bindings.push(binding),
            None => break,
        }

        // Check for comma (more bindings)
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if bindings.is_empty() {
        diags.push(
            Diag::error("Expected variable definition after LET").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
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
fn parse_let_variable_definition(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<LetVariableDefinition> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse variable name
    let variable = match tokens.get(*pos) {
        Some(token) => match &token.kind {
            TokenKind::Identifier(name) => {
                *pos += 1;
                BindingVariable {
                    name: name.clone(),
                    span: token.span.clone(),
                }
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                *pos += 1;
                BindingVariable {
                    name: SmolStr::new(kind.to_string()),
                    span: token.span.clone(),
                }
            }
            _ => {
                diags.push(
                    Diag::error("Expected variable name in LET definition")
                        .with_primary_label(token.span.clone(), "expected identifier here"),
                );
                return (None, diags);
            }
        },
        None => return (None, diags),
    };

    // Parse optional type annotation
    let type_annotation =
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::DoubleColon) {
            *pos += 1;
            match parse_value_type_prefix(&tokens[*pos..]) {
                Ok((value_type, consumed)) => {
                    *pos += consumed;
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
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Eq) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected '=' in LET variable definition").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected '=' here",
            ),
        );
        return (None, diags);
    }

    // Parse value expression
    let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut value_diags);

    let value = match value_opt {
        Some(v) => v,
        None => {
            diags.push(
                Diag::error("Expected value expression in LET definition").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

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
fn parse_for_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<ForStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::For) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse FOR item
    let (item_opt, mut item_diags) = parse_for_item(tokens, pos);
    diags.append(&mut item_diags);

    let item = match item_opt {
        Some(i) => i,
        None => {
            diags.push(
                Diag::error("Expected FOR item specification").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected 'variable IN expression' here",
                ),
            );
            return (None, diags);
        }
    };

    // Parse optional WITH ORDINALITY/OFFSET
    let ordinality_or_offset = if *pos < tokens.len()
        && matches!(tokens[*pos].kind, TokenKind::With)
    {
        *pos += 1;
        let (ordinality_opt, mut ordinality_diags) = parse_for_ordinality_or_offset(tokens, pos);
        diags.append(&mut ordinality_diags);
        ordinality_opt
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

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
fn parse_for_item(tokens: &[Token], pos: &mut usize) -> ParseResult<ForItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse binding variable
    let binding_variable = match tokens.get(*pos) {
        Some(token) => match &token.kind {
            TokenKind::Identifier(name) => {
                *pos += 1;
                BindingVariable {
                    name: name.clone(),
                    span: token.span.clone(),
                }
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                *pos += 1;
                BindingVariable {
                    name: SmolStr::new(kind.to_string()),
                    span: token.span.clone(),
                }
            }
            _ => {
                diags.push(
                    Diag::error("Expected variable name in FOR statement")
                        .with_primary_label(token.span.clone(), "expected identifier here"),
                );
                return (None, diags);
            }
        },
        None => return (None, diags),
    };

    // Expect IN keyword
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::In) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected IN keyword in FOR statement").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected 'IN' here",
            ),
        );
        return (None, diags);
    }

    // Parse collection expression
    let (collection_opt, mut coll_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut coll_diags);

    let collection = match collection_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected collection expression in FOR statement").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

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
fn parse_for_ordinality_or_offset(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<ForOrdinalityOrOffset> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    if *pos >= tokens.len() {
        diags.push(
            Diag::error("Expected ORDINALITY or OFFSET after WITH")
                .with_primary_label(start..start, "expected ORDINALITY or OFFSET here"),
        );
        return (None, diags);
    }

    let expects_ordinality = match &tokens[*pos].kind {
        TokenKind::Ordinality => {
            *pos += 1;
            true
        }
        TokenKind::Offset => {
            *pos += 1;
            false
        }
        _ => {
            diags.push(
                Diag::error("Expected ORDINALITY or OFFSET after WITH").with_primary_label(
                    tokens[*pos].span.clone(),
                    "expected ORDINALITY or OFFSET here",
                ),
            );
            skip_to_query_clause_boundary(tokens, pos);
            return (None, diags);
        }
    };

    // AS is optional in this clause.
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::As) {
        *pos += 1;
    }

    let Some(token) = tokens.get(*pos) else {
        let keyword = if expects_ordinality {
            "ORDINALITY"
        } else {
            "OFFSET"
        };
        diags.push(
            Diag::error(format!("Expected variable name after WITH {keyword}"))
                .with_primary_label(start..start, "expected identifier here"),
        );
        return (None, diags);
    };

    let variable = match &token.kind {
        TokenKind::Identifier(name) => {
            let variable = BindingVariable {
                name: name.clone(),
                span: token.span.clone(),
            };
            *pos += 1;
            variable
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let variable = BindingVariable {
                name: SmolStr::new(kind.to_string()),
                span: token.span.clone(),
            };
            *pos += 1;
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
                    .with_primary_label(token.span.clone(), "expected identifier here"),
            );
            if !is_query_clause_boundary(&token.kind) {
                skip_to_query_clause_boundary(tokens, pos);
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

fn is_query_spec_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LParen
            | TokenKind::Use
            | TokenKind::Match
            | TokenKind::Optional
            | TokenKind::Filter
            | TokenKind::Let
            | TokenKind::For
            | TokenKind::Order
            | TokenKind::Limit
            | TokenKind::Offset
            | TokenKind::Skip
            | TokenKind::Select
            | TokenKind::Return
            | TokenKind::Finish
    )
}

fn parse_from_graph_match_list(
    tokens: &[Token],
    pos: &mut usize,
) -> (Vec<GraphPattern>, Vec<Diag>) {
    let mut matches = Vec::new();
    let mut diags = Vec::new();

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Match) {
        let match_span = tokens[*pos].span.clone();
        *pos += 1;

        let pattern_start = *pos;
        let (pattern_opt, mut pattern_diags) = parse_graph_pattern_checked(tokens, pos);
        diags.append(&mut pattern_diags);

        match pattern_opt {
            Some(pattern) => matches.push(pattern),
            None => {
                diags.push(
                    Diag::error("Expected graph pattern after MATCH in FROM clause")
                        .with_primary_label(match_span, "expected graph pattern here"),
                );
                if *pos == pattern_start {
                    skip_to_query_clause_boundary(tokens, pos);
                }
                break;
            }
        }

        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            let comma_span = tokens[*pos].span.clone();
            *pos += 1;
            if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Match) {
                diags.push(
                    Diag::error("Expected MATCH after ',' in FROM clause")
                        .with_primary_label(comma_span, "expected MATCH here"),
                );
                skip_to_query_clause_boundary(tokens, pos);
                break;
            }
            continue;
        }

        break;
    }

    (matches, diags)
}

// ============================================================================
// Select Statements (Task 19)
// ============================================================================

/// Parses a SELECT statement.
fn parse_select_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Select) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse optional quantifier
    let quantifier = parse_set_quantifier_opt(tokens, pos);

    // Parse select items
    let (items_opt, mut items_diags) = parse_select_items(tokens, pos);
    diags.append(&mut items_diags);

    let select_items = match items_opt {
        Some(items) => items,
        None => {
            diags.push(
                Diag::error("Expected select items after SELECT").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected items here",
                ),
            );
            return (None, diags);
        }
    };

    // Parse optional clauses
    let from_clause = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::From) {
        *pos += 1;
        let (from_opt, mut from_diags) = parse_select_from_clause(tokens, pos);
        diags.append(&mut from_diags);
        from_opt
    } else {
        None
    };

    let where_clause = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Where) {
        *pos += 1;
        let (where_opt, mut where_diags) = parse_where_clause(tokens, pos);
        diags.append(&mut where_diags);
        where_opt
    } else {
        None
    };

    let group_by = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Group) {
        let (group_opt, mut group_diags) = parse_group_by_clause(tokens, pos);
        diags.append(&mut group_diags);
        group_opt
    } else {
        None
    };

    let having = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Having) {
        *pos += 1;
        let (having_opt, mut having_diags) = parse_having_clause(tokens, pos);
        diags.append(&mut having_diags);
        having_opt
    } else {
        None
    };

    let order_by = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Order) {
        let (order_opt, mut order_diags) = parse_order_by_clause(tokens, pos);
        diags.append(&mut order_diags);
        order_opt
    } else {
        None
    };

    let offset = if *pos < tokens.len()
        && matches!(tokens[*pos].kind, TokenKind::Offset | TokenKind::Skip)
    {
        let (offset_opt, mut offset_diags) = parse_offset_clause(tokens, pos);
        diags.append(&mut offset_diags);
        offset_opt
    } else {
        None
    };

    let limit = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Limit) {
        let (limit_opt, mut limit_diags) = parse_limit_clause(tokens, pos);
        diags.append(&mut limit_diags);
        limit_opt
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(SelectStatement {
            quantifier,
            select_items,
            from_clause,
            where_clause,
            group_by,
            having,
            order_by,
            offset,
            limit,
            span: start..end,
        }),
        diags,
    )
}

/// Parses select items (SELECT * or item list).
fn parse_select_items(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectItemList> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    // Check for SELECT *
    if matches!(tokens[*pos].kind, TokenKind::Star) {
        *pos += 1;
        return (Some(SelectItemList::Star), vec![]);
    }

    // Parse item list
    let mut items = Vec::new();
    let mut diags = Vec::new();

    loop {
        let (item_opt, mut item_diags) = parse_select_item(tokens, pos);
        diags.append(&mut item_diags);

        match item_opt {
            Some(item) => items.push(item),
            None => break,
        }

        // Check for comma
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if items.is_empty() {
        (None, diags)
    } else {
        (Some(SelectItemList::Items { items }), diags)
    }
}

/// Parses a single select item.
fn parse_select_item(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse expression
    let (expression_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let expression = match expression_opt {
        Some(e) => e,
        None => return (None, diags),
    };

    // Parse optional AS alias
    let alias = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::As) {
        let as_span = tokens[*pos].span.clone();
        *pos += 1;
        if let Some(token) = tokens.get(*pos) {
            match &token.kind {
                TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                    *pos += 1;
                    Some(name.clone())
                }
                kind if kind.is_non_reserved_identifier_keyword() => {
                    *pos += 1;
                    Some(SmolStr::new(kind.to_string()))
                }
                _ => {
                    diags.push(
                        Diag::error("Expected alias after AS in SELECT item")
                            .with_primary_label(as_span, "AS must be followed by an identifier"),
                    );
                    None
                }
            }
        } else {
            diags.push(
                Diag::error("Expected alias after AS in SELECT item")
                    .with_primary_label(as_span, "AS must be followed by an identifier"),
            );
            None
        }
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(SelectItem {
            expression,
            alias,
            span: start..end,
        }),
        diags,
    )
}

/// Parses SELECT FROM clause.
fn parse_select_from_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectFromClause> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    if *pos >= tokens.len() {
        diags.push(
            Diag::error("Expected FROM clause payload").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected source after FROM",
            ),
        );
        return (None, diags);
    }

    // Variant 1: FROM MATCH <pattern> [, MATCH <pattern> ...]
    if matches!(tokens[*pos].kind, TokenKind::Match) {
        let (matches, mut match_diags) = parse_from_graph_match_list(tokens, pos);
        diags.append(&mut match_diags);

        if matches.is_empty() {
            return (None, diags);
        }

        return (Some(SelectFromClause::GraphMatchList { matches }), diags);
    }

    // Variant 2: FROM <query specification>
    if is_query_spec_start(&tokens[*pos].kind) {
        let query_start = *pos;
        let (query_opt, mut query_diags) = parse_query(tokens, pos);
        diags.append(&mut query_diags);

        if let Some(query) = query_opt {
            return (
                Some(SelectFromClause::QuerySpecification {
                    query: Box::new(query),
                }),
                diags,
            );
        }

        if *pos == query_start {
            skip_to_query_clause_boundary(tokens, pos);
        }

        if diags.is_empty() {
            diags.push(
                Diag::error("Expected query specification after FROM").with_primary_label(
                    tokens
                        .get(query_start)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected query here",
                ),
            );
        }

        return (None, diags);
    }

    // Variant 3: FROM <graph expression> <query specification>
    let (graph_opt, mut graph_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut graph_diags);

    let graph = match graph_opt {
        Some(graph) => graph,
        None => {
            if diags.is_empty() {
                diags.push(
                    Diag::error("Expected FROM clause payload").with_primary_label(
                        tokens
                            .get(*pos)
                            .map(|t| t.span.clone())
                            .unwrap_or(start..start),
                        "expected source after FROM",
                    ),
                );
            }
            return (None, diags);
        }
    };

    let query_start = *pos;
    if query_start < tokens.len() && is_query_spec_start(&tokens[query_start].kind) {
        let (query_opt, mut query_diags) = parse_query(tokens, pos);
        diags.append(&mut query_diags);

        if let Some(query) = query_opt {
            return (
                Some(SelectFromClause::GraphAndQuerySpecification {
                    graph,
                    query: Box::new(query),
                }),
                diags,
            );
        }
    }

    diags.push(
        Diag::error("Expected query specification after graph expression in FROM clause")
            .with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or_else(|| graph.span().clone()),
                "expected query specification here",
            ),
    );
    if *pos == query_start {
        skip_to_query_clause_boundary(tokens, pos);
    }

    (None, diags)
}

/// Parses WHERE clause.
fn parse_where_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<WhereClause> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    let (condition_opt, mut cond_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut cond_diags);

    let condition = match condition_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected condition after WHERE").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(WhereClause {
            condition,
            span: start..end,
        }),
        diags,
    )
}

/// Parses HAVING clause.
fn parse_having_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<HavingClause> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    let (condition_opt, mut cond_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut cond_diags);

    let condition = match condition_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected condition after HAVING").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(HavingClause {
            condition,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Return Statements (Task 20)
// ============================================================================

/// Parses a RETURN statement.
pub(crate) fn parse_return_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<ReturnStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Return) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse optional quantifier
    let quantifier = parse_set_quantifier_opt(tokens, pos);

    // Parse return items
    let (items_opt, mut items_diags) = parse_return_items(tokens, pos);
    diags.append(&mut items_diags);

    let items = match items_opt {
        Some(i) => i,
        None => {
            diags.push(
                Diag::error("Expected return items after RETURN").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected items here",
                ),
            );
            return (None, diags);
        }
    };

    // Parse optional GROUP BY
    let group_by = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Group) {
        let (group_opt, mut group_diags) = parse_group_by_clause(tokens, pos);
        diags.append(&mut group_diags);
        group_opt
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(ReturnStatement {
            quantifier,
            items,
            group_by,
            span: start..end,
        }),
        diags,
    )
}

/// Parses return items (RETURN * or item list).
fn parse_return_items(tokens: &[Token], pos: &mut usize) -> ParseResult<ReturnItemList> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    // Check for RETURN *
    if matches!(tokens[*pos].kind, TokenKind::Star) {
        *pos += 1;
        return (Some(ReturnItemList::Star), vec![]);
    }

    // Parse item list
    let mut items = Vec::new();
    let mut diags = Vec::new();

    loop {
        let (item_opt, mut item_diags) = parse_return_item(tokens, pos);
        diags.append(&mut item_diags);

        match item_opt {
            Some(item) => items.push(item),
            None => break,
        }

        // Check for comma
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if items.is_empty() {
        (None, diags)
    } else {
        (Some(ReturnItemList::Items { items }), diags)
    }
}

/// Parses a single return item.
fn parse_return_item(tokens: &[Token], pos: &mut usize) -> ParseResult<ReturnItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse expression
    let (expression_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let expression = match expression_opt {
        Some(e) => e,
        None => return (None, diags),
    };

    // Parse optional AS alias
    let alias = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::As) {
        let as_span = tokens[*pos].span.clone();
        *pos += 1;
        if let Some(token) = tokens.get(*pos) {
            match &token.kind {
                TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                    *pos += 1;
                    Some(name.clone())
                }
                kind if kind.is_non_reserved_identifier_keyword() => {
                    *pos += 1;
                    Some(SmolStr::new(kind.to_string()))
                }
                _ => {
                    diags.push(
                        Diag::error("Expected alias after AS in RETURN item")
                            .with_primary_label(as_span, "AS must be followed by an identifier"),
                    );
                    None
                }
            }
        } else {
            diags.push(
                Diag::error("Expected alias after AS in RETURN item")
                    .with_primary_label(as_span, "AS must be followed by an identifier"),
            );
            None
        }
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(ReturnItem {
            expression,
            alias,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Ordering and Pagination (Task 21)
// ============================================================================

/// Parses ORDER BY and pagination statement.
fn parse_order_by_and_page_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<OrderByAndPageStatement> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    let order_by = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Order) {
        let (order_opt, mut order_diags) = parse_order_by_clause(tokens, pos);
        diags.append(&mut order_diags);
        order_opt
    } else {
        None
    };

    let offset = if *pos < tokens.len()
        && matches!(tokens[*pos].kind, TokenKind::Offset | TokenKind::Skip)
    {
        let (offset_opt, mut offset_diags) = parse_offset_clause(tokens, pos);
        diags.append(&mut offset_diags);
        offset_opt
    } else {
        None
    };

    let limit = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Limit) {
        let (limit_opt, mut limit_diags) = parse_limit_clause(tokens, pos);
        diags.append(&mut limit_diags);
        limit_opt
    } else {
        None
    };

    if order_by.is_none() && offset.is_none() && limit.is_none() {
        return (None, diags);
    }

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(OrderByAndPageStatement {
            order_by,
            offset,
            limit,
            span: start..end,
        }),
        diags,
    )
}

/// Parses ORDER BY clause.
fn parse_order_by_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<OrderByClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Order) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Expect BY
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::By) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected BY after ORDER").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected 'BY' here",
            ),
        );
        return (None, diags);
    }

    // Parse sort specifications (comma-separated)
    let mut sort_specifications = Vec::new();

    loop {
        let (spec_opt, mut spec_diags) = parse_sort_specification(tokens, pos);
        diags.append(&mut spec_diags);

        match spec_opt {
            Some(spec) => sort_specifications.push(spec),
            None => break,
        }

        // Check for comma
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if sort_specifications.is_empty() {
        diags.push(
            Diag::error("Expected sort specification after ORDER BY").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected expression here",
            ),
        );
        return (None, diags);
    }

    let end = sort_specifications
        .last()
        .map(|s| s.span.end)
        .unwrap_or(start);

    (
        Some(OrderByClause {
            sort_specifications,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a single sort specification.
fn parse_sort_specification(tokens: &[Token], pos: &mut usize) -> ParseResult<SortSpecification> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse sort key expression
    let (key_opt, mut key_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut key_diags);

    let key = match key_opt {
        Some(k) => k,
        None => return (None, diags),
    };

    // Parse optional ordering (ASC/DESC)
    let ordering = if *pos < tokens.len() {
        match &tokens[*pos].kind {
            TokenKind::Asc | TokenKind::Ascending => {
                *pos += 1;
                Some(OrderingSpecification::Ascending)
            }
            TokenKind::Desc | TokenKind::Descending => {
                *pos += 1;
                Some(OrderingSpecification::Descending)
            }
            _ => None,
        }
    } else {
        None
    };

    // Parse optional NULLS FIRST/LAST
    let null_ordering = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Nulls) {
        *pos += 1;

        if *pos < tokens.len() {
            match &tokens[*pos].kind {
                TokenKind::First => {
                    *pos += 1;
                    Some(NullOrdering::NullsFirst)
                }
                TokenKind::Last => {
                    *pos += 1;
                    Some(NullOrdering::NullsLast)
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(SortSpecification {
            key,
            ordering,
            null_ordering,
            span: start..end,
        }),
        diags,
    )
}

/// Parses LIMIT clause.
fn parse_limit_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<LimitClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Limit) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse count expression
    let (count_opt, mut count_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut count_diags);

    let count = match count_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected count expression after LIMIT").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(LimitClause {
            count,
            span: start..end,
        }),
        diags,
    )
}

/// Parses OFFSET/SKIP clause.
fn parse_offset_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<OffsetClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() {
        return (None, diags);
    }

    let use_skip_keyword = matches!(tokens[*pos].kind, TokenKind::Skip);

    if !matches!(tokens[*pos].kind, TokenKind::Offset | TokenKind::Skip) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse count expression
    let (count_opt, mut count_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut count_diags);

    let count = match count_opt {
        Some(c) => c,
        None => {
            let msg = if use_skip_keyword {
                "Expected count expression after SKIP"
            } else {
                "Expected count expression after OFFSET"
            };
            diags.push(
                Diag::error(msg).with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(OffsetClause {
            count,
            use_skip_keyword,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Grouping (Task 22)
// ============================================================================

/// Parses GROUP BY clause.
fn parse_group_by_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<GroupByClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Group) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Expect BY
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::By) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected BY after GROUP").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected 'BY' here",
            ),
        );
        return (None, diags);
    }

    // Parse grouping elements (comma-separated)
    let mut elements = Vec::new();

    loop {
        let (elem_opt, mut elem_diags) = parse_grouping_element(tokens, pos);
        diags.append(&mut elem_diags);

        match elem_opt {
            Some(elem) => elements.push(elem),
            None => break,
        }

        // Check for comma
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if elements.is_empty() {
        diags.push(
            Diag::error("Expected grouping element after GROUP BY").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected expression here",
            ),
        );
        return (None, diags);
    }

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(GroupByClause {
            elements,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a grouping element (expression or empty set).
fn parse_grouping_element(tokens: &[Token], pos: &mut usize) -> ParseResult<GroupingElement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    // Check for empty grouping set ()
    if matches!(tokens[*pos].kind, TokenKind::LParen) {
        let next_pos = *pos + 1;
        if next_pos < tokens.len() && matches!(tokens[next_pos].kind, TokenKind::RParen) {
            *pos += 2;
            return (Some(GroupingElement::EmptyGroupingSet), vec![]);
        }
    }

    // Parse expression
    let (expr_opt, diags) = parse_expression_with_diags(tokens, pos);
    (expr_opt.map(GroupingElement::Expression), diags)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn lex(input: &str) -> Vec<Token> {
        let lexer = Lexer::new(input);
        let result = lexer.tokenize();
        result.tokens
    }

    #[test]
    fn test_parse_simple_return() {
        let tokens = lex("RETURN *");
        let mut pos = 0;
        let (result, diags) = parse_return_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let ret = result.unwrap();
        assert!(matches!(ret.items, ReturnItemList::Star));
    }

    #[test]
    fn test_parse_return_with_expressions() {
        let tokens = lex("RETURN n.name, n.age");
        let mut pos = 0;
        let (result, diags) = parse_return_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let ret = result.unwrap();
        if let ReturnItemList::Items { items } = ret.items {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Items variant");
        }
    }

    #[test]
    fn test_parse_return_with_distinct() {
        let tokens = lex("RETURN DISTINCT n.name");
        let mut pos = 0;
        let (result, diags) = parse_return_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let ret = result.unwrap();
        assert_eq!(ret.quantifier, Some(SetQuantifier::Distinct));
    }

    #[test]
    fn test_parse_filter_statement() {
        let tokens = lex("FILTER n.age > 18");
        let mut pos = 0;
        let (result, diags) = parse_filter_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let filter = result.unwrap();
        assert!(!filter.where_optional);
    }

    #[test]
    fn test_parse_filter_with_where() {
        let tokens = lex("FILTER WHERE n.active = true");
        let mut pos = 0;
        let (result, diags) = parse_filter_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let filter = result.unwrap();
        assert!(filter.where_optional);
    }

    #[test]
    fn test_parse_let_statement() {
        let tokens = lex("LET x = 5");
        let mut pos = 0;
        let (result, diags) = parse_let_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let let_stmt = result.unwrap();
        assert_eq!(let_stmt.bindings.len(), 1);
    }

    #[test]
    fn test_parse_let_multiple_bindings() {
        let tokens = lex("LET x = 5, y = 10");
        let mut pos = 0;
        let (result, diags) = parse_let_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let let_stmt = result.unwrap();
        assert_eq!(let_stmt.bindings.len(), 2);
    }

    #[test]
    fn test_parse_for_statement() {
        let tokens = lex("FOR item IN collection");
        let mut pos = 0;
        let (result, diags) = parse_for_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let for_stmt = result.unwrap();
        assert!(for_stmt.ordinality_or_offset.is_none());
    }

    #[test]
    fn test_parse_for_with_ordinality() {
        let tokens = lex("FOR item IN collection WITH ORDINALITY AS ord");
        let mut pos = 0;
        let (result, diags) = parse_for_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let for_stmt = result.unwrap();
        assert!(matches!(
            for_stmt.ordinality_or_offset,
            Some(ForOrdinalityOrOffset::Ordinality { .. })
        ));
    }

    #[test]
    fn test_parse_for_with_missing_modifier_reports_diagnostic() {
        let tokens = lex("FOR item IN collection WITH");
        let mut pos = 0;
        let (result, diags) = parse_for_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_parse_for_with_missing_target_reports_diagnostic() {
        let tokens = lex("FOR item IN collection WITH OFFSET");
        let mut pos = 0;
        let (result, diags) = parse_for_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_parse_for_with_invalid_with_payload_recovers_to_return() {
        let tokens = lex("FOR item IN collection WITH junk RETURN item");
        let mut pos = 0;
        let (result, diags) = parse_query(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_parse_order_by_clause() {
        let tokens = lex("ORDER BY n.name ASC");
        let mut pos = 0;
        let (result, diags) = parse_order_by_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let order = result.unwrap();
        assert_eq!(order.sort_specifications.len(), 1);
    }

    #[test]
    fn test_parse_order_by_multiple_keys() {
        let tokens = lex("ORDER BY n.name ASC, n.age DESC");
        let mut pos = 0;
        let (result, diags) = parse_order_by_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let order = result.unwrap();
        assert_eq!(order.sort_specifications.len(), 2);
    }

    #[test]
    fn test_parse_order_by_with_nulls() {
        let tokens = lex("ORDER BY n.name NULLS FIRST");
        let mut pos = 0;
        let (result, diags) = parse_order_by_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let order = result.unwrap();
        assert_eq!(order.sort_specifications.len(), 1);
        assert!(matches!(
            order.sort_specifications[0].null_ordering,
            Some(NullOrdering::NullsFirst)
        ));
    }

    #[test]
    fn test_parse_limit_clause() {
        let tokens = lex("LIMIT 10");
        let mut pos = 0;
        let (result, diags) = parse_limit_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_offset_clause() {
        let tokens = lex("OFFSET 5");
        let mut pos = 0;
        let (result, diags) = parse_offset_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let offset = result.unwrap();
        assert!(!offset.use_skip_keyword);
    }

    #[test]
    fn test_parse_skip_clause() {
        let tokens = lex("SKIP 5");
        let mut pos = 0;
        let (result, diags) = parse_offset_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let offset = result.unwrap();
        assert!(offset.use_skip_keyword);
    }

    #[test]
    fn test_parse_group_by_clause() {
        let tokens = lex("GROUP BY n.category");
        let mut pos = 0;
        let (result, diags) = parse_group_by_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let group = result.unwrap();
        assert_eq!(group.elements.len(), 1);
    }

    #[test]
    fn test_parse_group_by_empty_set() {
        let tokens = lex("GROUP BY ()");
        let mut pos = 0;
        let (result, diags) = parse_group_by_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let group = result.unwrap();
        assert_eq!(group.elements.len(), 1);
        assert!(matches!(
            group.elements[0],
            GroupingElement::EmptyGroupingSet
        ));
    }

    #[test]
    fn test_parse_use_graph_clause() {
        let tokens = lex("USE myGraph");
        let mut pos = 0;
        let (result, diags) = parse_use_graph_clause(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_parse_simple_match_statement() {
        let tokens = lex("MATCH (n)");
        let mut pos = 0;
        let (result, diags) = parse_match_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        match result.unwrap() {
            MatchStatement::Simple(simple) => {
                assert!(!simple.pattern.paths.patterns.is_empty());
            }
            MatchStatement::Optional(_) => panic!("expected simple MATCH"),
        }
    }

    #[test]
    fn test_parse_set_quantifiers() {
        let tokens = lex("ALL");
        let mut pos = 0;
        let quantifier = parse_set_quantifier_opt(&tokens, &mut pos);
        assert_eq!(quantifier, Some(SetQuantifier::All));

        let tokens = lex("DISTINCT");
        let mut pos = 0;
        let quantifier = parse_set_quantifier_opt(&tokens, &mut pos);
        assert_eq!(quantifier, Some(SetQuantifier::Distinct));
    }

    #[test]
    fn test_parse_ambient_linear_query() {
        let tokens = lex("MATCH (n) RETURN n");
        let mut pos = 0;
        let (result, diags) = parse_ambient_linear_query(&tokens, &mut pos);

        assert!(result.is_some(), "Query failed to parse");
        assert!(diags.is_empty());
        let query = result.unwrap();
        assert_eq!(
            query.primitive_statements.len(),
            1,
            "Should have 1 primitive statement"
        );
        assert!(matches!(
            query.result_statement.as_deref(),
            Some(PrimitiveResultStatement::Return(_))
        ));
    }

    #[test]
    fn test_parse_query_rejects_non_query_token_without_progress() {
        let tokens = lex("foo");
        let mut pos = 0;
        let (result, diags) = parse_query(&tokens, &mut pos);

        assert!(result.is_none());
        assert!(!diags.is_empty());
        assert_eq!(pos, 0, "position must not advance on non-query input");
    }

    #[test]
    fn test_parse_query_rejects_semicolon_without_progress() {
        let tokens = lex(";");
        let mut pos = 0;
        let (result, diags) = parse_query(&tokens, &mut pos);

        assert!(result.is_none());
        assert!(!diags.is_empty());
        assert_eq!(pos, 0, "position must not advance on semicolon");
    }

    #[test]
    fn test_parse_query_requires_body_after_use_clause() {
        let tokens = lex("USE g");
        let mut pos = 0;
        let (result, diags) = parse_query(&tokens, &mut pos);

        assert!(result.is_none());
        assert!(!diags.is_empty());
        assert!(pos > 0, "USE clause should still be consumed");
    }

    #[test]
    fn test_parse_select_statement() {
        let tokens = lex("SELECT *");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let select = result.unwrap();
        assert!(matches!(select.select_items, SelectItemList::Star));
    }

    #[test]
    fn test_parse_select_with_items() {
        let tokens = lex("SELECT n.name, n.age");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let select = result.unwrap();
        if let SelectItemList::Items { items } = select.select_items {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Items variant");
        }
    }

    #[test]
    fn test_parse_select_from_match_list() {
        let tokens = lex("SELECT * FROM MATCH (n)");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let select = result.unwrap();
        assert!(matches!(
            select.from_clause,
            Some(SelectFromClause::GraphMatchList { .. })
        ));
    }

    #[test]
    fn test_parse_select_from_query_specification() {
        let tokens = lex("SELECT * FROM (MATCH (n) RETURN n)");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let select = result.unwrap();
        assert!(matches!(
            select.from_clause,
            Some(SelectFromClause::QuerySpecification { .. })
        ));
    }

    #[test]
    fn test_parse_select_from_graph_and_query_specification() {
        let tokens = lex("SELECT * FROM g MATCH (n) RETURN n");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(diags.is_empty());
        let select = result.unwrap();
        assert!(matches!(
            select.from_clause,
            Some(SelectFromClause::GraphAndQuerySpecification { .. })
        ));
    }

    #[test]
    fn test_parse_select_from_missing_payload_reports_diagnostic() {
        let tokens = lex("SELECT * FROM");
        let mut pos = 0;
        let (result, diags) = parse_select_statement(&tokens, &mut pos);

        assert!(result.is_some());
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_expression_boundary_detection() {
        // Test that expression boundary detection works correctly
        let tokens = lex("x + y MATCH");
        let boundary = find_expression_boundary(&tokens, 0);
        // Should stop at MATCH keyword
        assert!(boundary < tokens.len());
    }

    #[test]
    fn test_expression_with_nested_parens() {
        let tokens = lex("(x + (y * z)) RETURN");
        let boundary = find_expression_boundary(&tokens, 0);
        // Should consume the entire parenthesized expression
        assert!(boundary > 0);
    }
}
