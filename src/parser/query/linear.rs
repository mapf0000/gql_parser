//! Linear query parsing.
//!
//! This module handles parsing of linear queries, which consist of a sequence of
//! primitive query statements optionally followed by a result statement.

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use crate::parser::InternalParseResult;
use crate::parser::base::TokenStream;

use super::primitive::parse_primitive_query_statement;
use super::primitive::parse_use_graph_clause;
use super::result::parse_return_statement;

/// Parse result with optional value and diagnostics.
pub(super) type ParseResult<T> = InternalParseResult<T>;

/// Parses a linear query and wraps it in Query enum.
pub(super) fn parse_linear_query_as_query(stream: &mut TokenStream) -> ParseResult<Query> {
    // Check for parenthesized query
    if stream.check(&TokenKind::LParen) {
        let start = stream.current().span.start;
        stream.advance();

        let (query_opt, mut diags) = super::parse_composite_query(stream);

        if stream.check(&TokenKind::RParen) {
            let end = stream.current().span.end;
            stream.advance();

            if let Some(query) = query_opt {
                return (
                    Some(Query::Parenthesized(Box::new(query), start..end)),
                    diags,
                );
            }
        } else {
            diags.push(
                Diag::error("Expected ')' to close parenthesized query").with_primary_label(
                    stream.current().span.clone(),
                    "expected ')' here",
                ),
            );
        }

        return (None, diags);
    }

    let (linear_opt, diags) = parse_linear_query(stream);
    (linear_opt.map(Query::Linear), diags)
}

/// Parses a linear query statement.
fn parse_linear_query(stream: &mut TokenStream) -> ParseResult<LinearQuery> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Check for optional USE clause
    let use_graph = if stream.check(&TokenKind::Use) {
        let (use_graph_opt, mut use_diags) = parse_use_graph_clause(stream);
        diags.append(&mut use_diags);
        use_graph_opt
    } else {
        None
    };

    // Parse primitive statements and result statement
    let (primitive_statements, result_statement, has_query_body, mut stmt_diags, end) =
        parse_query_statements(stream, start);
    diags.append(&mut stmt_diags);

    if !has_query_body {
        let error_msg = if use_graph.is_some() {
            "Expected query statement after USE clause"
        } else {
            "Expected query statement"
        };
        diags.push(
            Diag::error(error_msg).with_primary_label(
                stream.current().span.clone(),
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
    stream: &mut TokenStream,
    _start: usize,
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
        if stream.check(&TokenKind::Eof) {
            break;
        }

        // Check for result statement (RETURN or FINISH)
        if stream.check(&TokenKind::Return) {
            let (return_opt, mut return_diags) = parse_return_statement(stream);
            diags.append(&mut return_diags);

            if let Some(ret) = return_opt {
                result_statement = Some(Box::new(PrimitiveResultStatement::Return(ret)));
            }
            break;
        }

        if stream.check(&TokenKind::Finish) {
            let span = stream.current().span.clone();
            stream.advance();
            result_statement = Some(Box::new(PrimitiveResultStatement::Finish(span)));
            break;
        }

        // Try to parse primitive statement
        let (stmt_opt, mut stmt_diags) = parse_primitive_query_statement(stream);
        diags.append(&mut stmt_diags);

        match stmt_opt {
            Some(stmt) => primitive_statements.push(stmt),
            None => break, // No more statements
        }
    }

    let end = stream.current().span.start;
    let has_query_body = !primitive_statements.is_empty() || result_statement.is_some();
    (
        primitive_statements,
        result_statement,
        has_query_body,
        diags,
        end,
    )
}
