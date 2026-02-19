//! Pagination and grouping statement parsing.
//!
//! This module handles parsing of:
//! - ORDER BY clauses with sort specifications
//! - LIMIT clauses
//! - OFFSET/SKIP clauses
//! - GROUP BY clauses with grouping elements

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};

use super::{parse_expression_with_diags, ParseResult};

// ============================================================================
// Ordering and Pagination (Task 21)
// ============================================================================

/// Parses ORDER BY and pagination statement.
pub(super) fn parse_order_by_and_page_statement(
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
pub(super) fn parse_order_by_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<OrderByClause> {
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
pub(super) fn parse_limit_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<LimitClause> {
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
pub(super) fn parse_offset_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<OffsetClause> {
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
pub(super) fn parse_group_by_clause(tokens: &[Token], pos: &mut usize) -> ParseResult<GroupByClause> {
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
