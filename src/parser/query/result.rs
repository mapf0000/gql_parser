//! Result statement parsing (SELECT and RETURN).
//!
//! This module handles parsing of query result statements including:
//! - SELECT statements with FROM, WHERE, GROUP BY, HAVING, ORDER BY, OFFSET, LIMIT
//! - RETURN statements with GROUP BY

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::base::TokenStream;
use smol_str::SmolStr;

use super::{
    ParseResult, is_query_spec_start, parse_expression_with_diags, parse_set_quantifier_opt,
    skip_to_query_clause_boundary,
};

// Import functions from sibling modules
use super::pagination::{
    parse_group_by_clause, parse_limit_clause, parse_offset_clause, parse_order_by_clause,
};

// Import parse_query from parent module for recursive query parsing
use super::parse_query;

// Import pattern parsing
use crate::parser::patterns::parse_graph_pattern;

// ============================================================================
// Select Statements (Task 19)
// ============================================================================

/// Parses a SELECT statement.
pub(super) fn parse_select_statement(stream: &mut TokenStream) -> ParseResult<SelectStatement> {
    let mut diags = Vec::new();

    let (with_clause, mut with_diags) = if stream.check(&TokenKind::With) {
        parse_with_clause(stream)
    } else {
        (None, Vec::new())
    };
    diags.append(&mut with_diags);

    if !stream.check(&TokenKind::Select) {
        if with_clause.is_none() {
            return (None, diags);
        }
        diags.push(
            Diag::error("Expected SELECT after WITH clause")
                .with_primary_label(stream.current().span.clone(), "expected SELECT here"),
        );
        return (None, diags);
    }

    let start = with_clause
        .as_ref()
        .map(|clause| clause.span.start)
        .unwrap_or_else(|| stream.current().span.start);
    stream.advance();

    // Parse optional quantifier - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let quantifier = parse_set_quantifier_opt(tokens, &mut pos);
    stream.set_position(pos);

    // Parse select items - need to use legacy interface temporarily
    let (items_opt, mut items_diags) = parse_select_items(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut items_diags);

    let select_items = match items_opt {
        Some(items) => items,
        None => {
            diags.push(
                Diag::error("Expected select items after SELECT")
                    .with_primary_label(stream.current().span.clone(), "expected items here"),
            );
            return (None, diags);
        }
    };

    // Parse optional clauses
    let from_clause = if stream.check(&TokenKind::From) {
        stream.advance();
        let (from_opt, mut from_diags) = parse_select_from_clause(stream);
        diags.append(&mut from_diags);
        from_opt
    } else {
        None
    };

    let where_clause = if stream.check(&TokenKind::Where) {
        stream.advance();
        let (where_opt, mut where_diags) = parse_where_clause(stream);
        diags.append(&mut where_diags);
        where_opt
    } else {
        None
    };

    let group_by = if stream.check(&TokenKind::Group) {
        let (group_opt, mut group_diags) = parse_group_by_clause(stream);
        diags.append(&mut group_diags);
        group_opt
    } else {
        None
    };

    let having = if stream.check(&TokenKind::Having) {
        stream.advance();
        let (having_opt, mut having_diags) = parse_having_clause(stream);
        diags.append(&mut having_diags);
        having_opt
    } else {
        None
    };

    let order_by = if stream.check(&TokenKind::Order) {
        let (order_opt, mut order_diags) = parse_order_by_clause(stream);
        diags.append(&mut order_diags);
        order_opt
    } else {
        None
    };

    let offset = if stream.check(&TokenKind::Offset) || stream.check(&TokenKind::Skip) {
        let (offset_opt, mut offset_diags) = parse_offset_clause(stream);
        diags.append(&mut offset_diags);
        offset_opt
    } else {
        None
    };

    let limit = if stream.check(&TokenKind::Limit) {
        let (limit_opt, mut limit_diags) = parse_limit_clause(stream);
        diags.append(&mut limit_diags);
        limit_opt
    } else {
        None
    };

    let end = stream.previous_span().end;

    (
        Some(SelectStatement {
            with_clause,
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

/// Parses a WITH clause with one or more CTE definitions.
fn parse_with_clause(stream: &mut TokenStream) -> ParseResult<WithClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::With) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let recursive = if token_is_word(&stream.current().kind, "RECURSIVE") {
        stream.advance();
        true
    } else {
        false
    };

    let mut items = Vec::new();
    loop {
        let item_start = stream.position();
        let (item_opt, mut item_diags) = parse_common_table_expression(stream);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            if diags.is_empty() {
                diags.push(
                    Diag::error("Expected common table expression in WITH clause")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected CTE definition here",
                        ),
                );
            }
            if stream.position() == item_start {
                // Skip to boundary using legacy interface
                let tokens = stream.tokens();
                let mut pos = stream.position();
                skip_to_query_clause_boundary(tokens, &mut pos);
                stream.set_position(pos);
            }
            break;
        };
        items.push(item);

        if stream.check(&TokenKind::Comma) {
            stream.advance();
            continue;
        }
        break;
    }

    if items.is_empty() {
        return (None, diags);
    }

    let end = items.last().map(|item| item.span.end).unwrap_or(start);
    (
        Some(WithClause {
            recursive,
            items,
            span: start..end,
        }),
        diags,
    )
}

fn parse_common_table_expression(stream: &mut TokenStream) -> ParseResult<CommonTableExpression> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Need to use legacy interface for parse_identifier_token
    let tokens = stream.tokens();
    let mut pos = stream.position();

    let (name, _) = match parse_identifier_token(tokens, &mut pos) {
        Some(value) => value,
        None => {
            diags.push(
                Diag::error("Expected CTE name in WITH clause")
                    .with_primary_label(stream.current().span.clone(), "expected identifier here"),
            );
            return (None, diags);
        }
    };
    stream.set_position(pos);

    let mut columns = Vec::new();
    if stream.check(&TokenKind::LParen) {
        stream.advance();
        while !stream.check(&TokenKind::RParen) && !stream.check(&TokenKind::Eof) {
            let tokens = stream.tokens();
            let mut pos = stream.position();
            if let Some((column, _)) = parse_identifier_token(tokens, &mut pos) {
                stream.set_position(pos);
                columns.push(column);
            } else {
                diags.push(
                    Diag::error("Expected column name in CTE column list").with_primary_label(
                        stream.current().span.clone(),
                        "expected identifier here",
                    ),
                );
                break;
            }

            if stream.check(&TokenKind::Comma) {
                stream.advance();
                continue;
            }
            break;
        }

        if stream.check(&TokenKind::RParen) {
            stream.advance();
        } else {
            diags.push(
                Diag::error("Expected ')' to close CTE column list")
                    .with_primary_label(stream.current().span.clone(), "expected ')' here"),
            );
            return (None, diags);
        }
    }

    if stream.check(&TokenKind::As) {
        stream.advance();
    } else {
        diags.push(
            Diag::error("Expected AS before CTE query payload")
                .with_primary_label(stream.current().span.clone(), "expected AS here"),
        );
        return (None, diags);
    }

    if !stream.check(&TokenKind::LParen) {
        diags.push(
            Diag::error("Expected '(' to start CTE query payload")
                .with_primary_label(stream.current().span.clone(), "expected '(' here"),
        );
        return (None, diags);
    }
    stream.advance();

    let query_start = stream.position();

    // Need to use legacy interface for parse_query
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (query_opt, mut query_diags) = parse_query(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut query_diags);

    let query = match query_opt {
        Some(query) => query,
        None => {
            if stream.position() == query_start {
                let tokens = stream.tokens();
                let mut pos = stream.position();
                skip_to_query_clause_boundary(tokens, &mut pos);
                stream.set_position(pos);
            }
            return (None, diags);
        }
    };

    if !stream.check(&TokenKind::RParen) {
        diags.push(
            Diag::error("Expected ')' to close CTE query payload")
                .with_primary_label(stream.current().span.clone(), "expected ')' here"),
        );
        return (None, diags);
    }

    let end = stream.current().span.end;
    stream.advance();

    (
        Some(CommonTableExpression {
            name,
            columns,
            query: Box::new(query),
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
fn parse_select_from_clause(stream: &mut TokenStream) -> ParseResult<SelectFromClause> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Variant 1: FROM MATCH <pattern> [, MATCH <pattern> ...]
    if stream.check(&TokenKind::Match) {
        let (matches, mut match_diags) = parse_from_graph_match_list(stream);
        diags.append(&mut match_diags);

        if matches.is_empty() {
            return (None, diags);
        }

        return (Some(SelectFromClause::GraphMatchList { matches }), diags);
    }

    // Variant 2/3/4: table/query source list.
    let mut sources = Vec::new();
    loop {
        let source_start = stream.position();
        let (source_opt, mut source_diags) = parse_select_source_item(stream);
        diags.append(&mut source_diags);

        let Some(source) = source_opt else {
            if sources.is_empty() {
                if diags.is_empty() {
                    diags.push(
                        Diag::error("Expected FROM clause payload").with_primary_label(
                            stream.current().span.clone(),
                            "expected source after FROM",
                        ),
                    );
                }
                if stream.position() == source_start {
                    // Skip to boundary using legacy interface
                    let tokens = stream.tokens();
                    let mut pos = stream.position();
                    skip_to_query_clause_boundary(tokens, &mut pos);
                    stream.set_position(pos);
                }
                return (None, diags);
            }
            break;
        };
        sources.push(source);

        if stream.check(&TokenKind::Comma) {
            stream.advance();
            continue;
        }
        break;
    }

    if sources.len() == 1 {
        match sources.remove(0) {
            SelectSourceItem::Query { query, alias, .. } => {
                return (
                    Some(SelectFromClause::QuerySpecification { query, alias }),
                    diags,
                );
            }
            SelectSourceItem::GraphAndQuery {
                graph,
                query,
                alias,
                ..
            } => {
                return (
                    Some(SelectFromClause::GraphAndQuerySpecification {
                        graph,
                        query,
                        alias,
                    }),
                    diags,
                );
            }
            source @ SelectSourceItem::Expression { .. } => {
                return (
                    Some(SelectFromClause::SourceList {
                        sources: vec![source],
                    }),
                    diags,
                );
            }
        }
    }

    (Some(SelectFromClause::SourceList { sources }), diags)
}

/// Parses a single source item in SELECT FROM clause.
fn parse_select_source_item(tokens: &[Token], pos: &mut usize) -> ParseResult<SelectSourceItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    if *pos >= tokens.len() {
        return (None, diags);
    }

    // Query source: FROM <query specification>
    if is_query_spec_start(&tokens[*pos].kind) {
        let query_start = *pos;
        let (query_opt, mut query_diags) = parse_query(tokens, pos);
        diags.append(&mut query_diags);

        if let Some(query) = query_opt {
            let (alias, mut alias_diags) = parse_optional_source_alias(tokens, pos);
            diags.append(&mut alias_diags);
            let end = tokens
                .get(pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or_else(|| query.span().end);

            return (
                Some(SelectSourceItem::Query {
                    query: Box::new(query),
                    alias,
                    span: start..end,
                }),
                diags,
            );
        }

        if *pos == query_start {
            skip_to_query_clause_boundary(tokens, pos);
        }
        return (None, diags);
    }

    // Expression source: FROM <expression> [<query specification>] [alias]
    let (expr_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let expression = match expr_opt {
        Some(expr) => expr,
        None => return (None, diags),
    };

    // Graph + query source: FROM <graph_expression> <query_specification>
    if *pos < tokens.len() && is_query_spec_start(&tokens[*pos].kind) {
        let query_start = *pos;
        let (query_opt, mut query_diags) = parse_query(tokens, pos);
        diags.append(&mut query_diags);

        if let Some(query) = query_opt {
            let (alias, mut alias_diags) = parse_optional_source_alias(tokens, pos);
            diags.append(&mut alias_diags);
            let end = tokens
                .get(pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or_else(|| query.span().end);

            return (
                Some(SelectSourceItem::GraphAndQuery {
                    graph: expression,
                    query: Box::new(query),
                    alias,
                    span: start..end,
                }),
                diags,
            );
        }

        if *pos == query_start {
            skip_to_query_clause_boundary(tokens, pos);
        }

        diags.push(
            Diag::error("Expected query specification after graph expression in FROM clause")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or_else(|| expression.span()),
                    "expected query specification here",
                ),
        );
        return (None, diags);
    }

    let (alias, mut alias_diags) = parse_optional_source_alias(tokens, pos);
    diags.append(&mut alias_diags);
    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or_else(|| expression.span().end);

    (
        Some(SelectSourceItem::Expression {
            expression,
            alias,
            span: start..end,
        }),
        diags,
    )
}

fn parse_optional_source_alias(tokens: &[Token], pos: &mut usize) -> (Option<SmolStr>, Vec<Diag>) {
    let mut diags = Vec::new();

    if *pos >= tokens.len() {
        return (None, diags);
    }

    if matches!(tokens[*pos].kind, TokenKind::As) {
        let as_span = tokens[*pos].span.clone();
        *pos += 1;
        if let Some((name, _)) = parse_identifier_token(tokens, pos) {
            return (Some(name), diags);
        }

        diags.push(
            Diag::error("Expected alias name after AS in FROM clause")
                .with_primary_label(as_span, "AS must be followed by an identifier"),
        );
        return (None, diags);
    }

    if let Some((name, _)) = parse_identifier_token(tokens, pos) {
        // Bare aliases are accepted only when they are not obvious clause boundaries.
        if !is_from_clause_boundary_keyword(name.as_str()) {
            return (Some(name), diags);
        }
        *pos = (*pos).saturating_sub(1);
    }

    (None, diags)
}

fn parse_identifier_token(
    tokens: &[Token],
    pos: &mut usize,
) -> Option<(SmolStr, std::ops::Range<usize>)> {
    if *pos >= tokens.len() {
        return None;
    }

    let token = &tokens[*pos];
    let result = match &token.kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
            Some((name.clone(), token.span.clone()))
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            Some((SmolStr::new(kind.to_string()), token.span.clone()))
        }
        _ => None,
    };

    if result.is_some() {
        *pos += 1;
    }
    result
}

fn token_is_word(kind: &TokenKind, word: &str) -> bool {
    match kind {
        TokenKind::Identifier(name)
        | TokenKind::DelimitedIdentifier(name)
        | TokenKind::ReservedKeyword(name)
        | TokenKind::PreReservedKeyword(name)
        | TokenKind::NonReservedKeyword(name) => name.eq_ignore_ascii_case(word),
        _ => false,
    }
}

fn is_from_clause_boundary_keyword(word: &str) -> bool {
    matches!(
        word.to_ascii_uppercase().as_str(),
        "WHERE"
            | "GROUP"
            | "HAVING"
            | "ORDER"
            | "OFFSET"
            | "SKIP"
            | "LIMIT"
            | "RETURN"
            | "FINISH"
            | "UNION"
            | "EXCEPT"
            | "INTERSECT"
            | "OTHERWISE"
    )
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
fn parse_where_clause(stream: &mut TokenStream) -> ParseResult<WhereClause> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

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
                Diag::error("Expected condition after WHERE").with_primary_label(
                    stream.current().span.clone(),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(WhereClause {
            condition,
            span: start..end,
        }),
        diags,
    )
}

/// Parses HAVING clause.
fn parse_having_clause(stream: &mut TokenStream) -> ParseResult<HavingClause> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

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
                Diag::error("Expected condition after HAVING").with_primary_label(
                    stream.current().span.clone(),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

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
pub(crate) fn parse_return_statement(stream: &mut TokenStream) -> ParseResult<ReturnStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Return) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse optional quantifier - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let quantifier = parse_set_quantifier_opt(tokens, &mut pos);
    stream.set_position(pos);

    // Parse return items
    let (items_opt, mut items_diags) = parse_return_items(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut items_diags);

    let items = match items_opt {
        Some(i) => i,
        None => {
            diags.push(
                Diag::error("Expected return items after RETURN")
                    .with_primary_label(stream.current().span.clone(), "expected items here"),
            );
            return (None, diags);
        }
    };

    // Parse optional GROUP BY
    let group_by = if stream.check(&TokenKind::Group) {
        let (group_opt, mut group_diags) = parse_group_by_clause(stream);
        diags.append(&mut group_diags);
        group_opt
    } else {
        None
    };

    let end = stream.previous_span().end;

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
