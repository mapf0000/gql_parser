//! Result statement parsing (SELECT and RETURN).
//!
//! This module handles parsing of query result statements including:
//! - SELECT statements with FROM, WHERE, GROUP BY, HAVING, ORDER BY, OFFSET, LIMIT
//! - RETURN statements with GROUP BY

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use smol_str::SmolStr;

use super::{
    parse_expression_with_diags, parse_set_quantifier_opt, is_query_spec_start,
    skip_to_query_clause_boundary, ParseResult,
};

// Import functions from sibling modules
use super::pagination::{parse_group_by_clause, parse_order_by_clause, parse_offset_clause, parse_limit_clause};

// Import parse_query from parent module for recursive query parsing
use super::parse_query;

// Import pattern parsing
use crate::parser::patterns::parse_graph_pattern;

// ============================================================================
// Select Statements (Task 19)
// ============================================================================

/// Parses a SELECT statement.
pub(super) fn parse_select_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectStatement> {
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

/// Parses a list of MATCH patterns in FROM clause.
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
