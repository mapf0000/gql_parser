//! Query statement parser for GQL.
//!
//! This module implements parsing for the GQL query pipeline, including:
//! - Composite queries with set operators (UNION, EXCEPT, INTERSECT, OTHERWISE)
//! - Linear queries with sequential statement chaining
//! - Primitive query statements (MATCH, FILTER, LET, FOR, SELECT)
//! - Result statements (RETURN)
//! - Ordering, pagination, and grouping

use crate::ast::query::*;
use crate::ast::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::InternalParseResult;
use crate::parser::expression::parse_expression;

/// Parse result with optional value and diagnostics.
pub(crate) type ParseResult<T> = InternalParseResult<T>;

// Submodules
mod linear;
mod primitive;
mod result;
mod pagination;

// Re-export functions needed by other parser modules
pub(crate) use primitive::{parse_use_graph_clause, parse_primitive_query_statement};
pub(crate) use result::parse_return_statement;

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
/// Visible to submodules for parsing expressions in various contexts.
pub(super) fn parse_expression_with_diags(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<Expression> {
    match parse_expression_at(tokens, pos) {
        Ok(expr) => (Some(expr), vec![]),
        Err(diag) => (None, vec![*diag]),
    }
}

// ============================================================================
// Shared Utilities
// ============================================================================

/// Checks if a token kind represents a query clause boundary.
pub(super) fn is_query_clause_boundary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Match
            | TokenKind::Optional
            | TokenKind::Filter
            | TokenKind::Let
            | TokenKind::For
            | TokenKind::With
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

/// Skips tokens until a query clause boundary is found.
pub(super) fn skip_to_query_clause_boundary(tokens: &[Token], pos: &mut usize) {
    while *pos < tokens.len() {
        if is_query_clause_boundary(&tokens[*pos].kind) {
            break;
        }
        *pos += 1;
    }
}

/// Checks if a token kind can start a query specification.
pub(super) fn is_query_spec_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LParen
            | TokenKind::Use
            | TokenKind::Match
            | TokenKind::Optional
            | TokenKind::Filter
            | TokenKind::Let
            | TokenKind::For
            | TokenKind::With
            | TokenKind::Order
            | TokenKind::Limit
            | TokenKind::Offset
            | TokenKind::Skip
            | TokenKind::Select
            | TokenKind::Return
            | TokenKind::Finish
    )
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
    let (left_opt, mut left_diags) = linear::parse_linear_query_as_query(tokens, pos);
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
        let (right_opt, mut right_diags) = linear::parse_linear_query_as_query(tokens, pos);
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
pub(super) fn parse_set_quantifier_opt(tokens: &[Token], pos: &mut usize) -> Option<SetQuantifier> {
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
