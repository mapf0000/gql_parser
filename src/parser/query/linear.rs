//! Linear query parsing.
//!
//! This module handles parsing of linear queries, which consist of a sequence of
//! primitive query statements optionally followed by a result statement.

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::InternalParseResult;

use super::primitive::parse_primitive_query_statement;
use super::result::parse_return_statement;
use super::primitive::parse_use_graph_clause;

/// Parse result with optional value and diagnostics.
pub(super) type ParseResult<T> = InternalParseResult<T>;

/// Parses a linear query and wraps it in Query enum.
pub(super) fn parse_linear_query_as_query(tokens: &[Token], pos: &mut usize) -> ParseResult<Query> {
    // Check for parenthesized query
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::LParen) {
        let start = tokens[*pos].span.start;
        *pos += 1;

        let (query_opt, mut diags) = super::parse_composite_query(tokens, pos);

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
