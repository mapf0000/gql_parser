//! Linear query parsing.
//!
//! This module handles parsing of linear queries, which consist of a sequence of
//! primitive query statements optionally followed by a result statement.

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::InternalParseResult;
use crate::parser::base::TokenStream;

use super::primitive::parse_primitive_query_statement;
use super::primitive::parse_use_graph_clause;
use super::result::parse_return_statement;

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
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Check for optional USE clause
    let use_graph = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Use) {
        let mut stream = TokenStream::new(tokens);
        stream.set_position(*pos);
        let (use_graph_opt, mut use_diags) = parse_use_graph_clause(&mut stream);
        *pos = stream.position();
        diags.append(&mut use_diags);
        use_graph_opt
    } else {
        None
    };

    // Parse primitive statements and result statement
    let (primitive_statements, result_statement, has_query_body, mut stmt_diags, end) =
        parse_query_statements(tokens, pos, start);
    diags.append(&mut stmt_diags);

    if !has_query_body {
        let error_msg = if use_graph.is_some() {
            "Expected query statement after USE clause"
        } else {
            "Expected query statement"
        };
        diags.push(
            Diag::error(error_msg).with_primary_label(
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
        Some(LinearQuery {
            use_graph,
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
            let mut stream = TokenStream::new(tokens);
            stream.set_position(*pos);
            let (return_opt, mut return_diags) = parse_return_statement(&mut stream);
            *pos = stream.position();
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
        let mut stream = TokenStream::new(tokens);
        stream.set_position(*pos);
        let (stmt_opt, mut stmt_diags) = parse_primitive_query_statement(&mut stream);
        *pos = stream.position();
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
