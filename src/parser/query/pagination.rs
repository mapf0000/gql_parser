//! Pagination and grouping statement parsing.
//!
//! This module handles parsing of:
//! - ORDER BY clauses with sort specifications
//! - LIMIT clauses
//! - OFFSET/SKIP clauses
//! - GROUP BY clauses with grouping elements

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use crate::parser::base::TokenStream;

use super::{ParseResult, parse_expression_with_diags};

// ============================================================================
// Ordering and Pagination (Task 21)
// ============================================================================

/// Parses ORDER BY and pagination statement.
pub(super) fn parse_order_by_and_page_statement(
    stream: &mut TokenStream,
) -> ParseResult<OrderByAndPageStatement> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

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

    if order_by.is_none() && offset.is_none() && limit.is_none() {
        return (None, diags);
    }

    let end = stream.previous_span().end;

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
pub(super) fn parse_order_by_clause(stream: &mut TokenStream) -> ParseResult<OrderByClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Order) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Expect BY
    if stream.check(&TokenKind::By) {
        stream.advance();
    } else {
        diags.push(
            Diag::error("Expected BY after ORDER")
                .with_primary_label(stream.current().span.clone(), "expected 'BY' here"),
        );
        return (None, diags);
    }

    // Parse sort specifications (comma-separated)
    let mut sort_specifications = Vec::new();

    loop {
        let (spec_opt, mut spec_diags) = parse_sort_specification(stream);
        diags.append(&mut spec_diags);

        match spec_opt {
            Some(spec) => sort_specifications.push(spec),
            None => break,
        }

        // Check for comma
        if stream.check(&TokenKind::Comma) {
            stream.advance();
        } else {
            break;
        }
    }

    if sort_specifications.is_empty() {
        diags.push(
            Diag::error("Expected sort specification after ORDER BY")
                .with_primary_label(stream.current().span.clone(), "expected expression here"),
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
fn parse_sort_specification(stream: &mut TokenStream) -> ParseResult<SortSpecification> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Parse sort key expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (key_opt, mut key_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut key_diags);

    let key = match key_opt {
        Some(k) => k,
        None => return (None, diags),
    };

    // Parse optional ordering (ASC/DESC)
    let ordering = match stream.current().kind {
        TokenKind::Asc | TokenKind::Ascending => {
            stream.advance();
            Some(OrderingSpecification::Ascending)
        }
        TokenKind::Desc | TokenKind::Descending => {
            stream.advance();
            Some(OrderingSpecification::Descending)
        }
        _ => None,
    };

    // Parse optional NULLS FIRST/LAST
    let null_ordering = if stream.check(&TokenKind::Nulls) {
        stream.advance();

        match stream.current().kind {
            TokenKind::First => {
                stream.advance();
                Some(NullOrdering::NullsFirst)
            }
            TokenKind::Last => {
                stream.advance();
                Some(NullOrdering::NullsLast)
            }
            _ => None,
        }
    } else {
        None
    };

    let end = stream.previous_span().end;

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
pub(super) fn parse_limit_clause(stream: &mut TokenStream) -> ParseResult<LimitClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Limit) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse count expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (count_opt, mut count_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut count_diags);

    let count = match count_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected count expression after LIMIT")
                    .with_primary_label(stream.current().span.clone(), "expected expression here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(LimitClause {
            count,
            span: start..end,
        }),
        diags,
    )
}

/// Parses OFFSET/SKIP clause.
pub(super) fn parse_offset_clause(stream: &mut TokenStream) -> ParseResult<OffsetClause> {
    let mut diags = Vec::new();

    let use_skip_keyword = stream.check(&TokenKind::Skip);

    if !stream.check(&TokenKind::Offset) && !use_skip_keyword {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Parse count expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (count_opt, mut count_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
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
                Diag::error(msg)
                    .with_primary_label(stream.current().span.clone(), "expected expression here"),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

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
pub(super) fn parse_group_by_clause(stream: &mut TokenStream) -> ParseResult<GroupByClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Group) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Expect BY
    if stream.check(&TokenKind::By) {
        stream.advance();
    } else {
        diags.push(
            Diag::error("Expected BY after GROUP")
                .with_primary_label(stream.current().span.clone(), "expected 'BY' here"),
        );
        return (None, diags);
    }

    // Parse grouping elements (comma-separated)
    let mut elements = Vec::new();

    loop {
        let (elem_opt, mut elem_diags) = parse_grouping_element(stream);
        diags.append(&mut elem_diags);

        match elem_opt {
            Some(elem) => elements.push(elem),
            None => break,
        }

        // Check for comma
        if stream.check(&TokenKind::Comma) {
            stream.advance();
        } else {
            break;
        }
    }

    if elements.is_empty() {
        diags.push(
            Diag::error("Expected grouping element after GROUP BY")
                .with_primary_label(stream.current().span.clone(), "expected expression here"),
        );
        return (None, diags);
    }

    let end = stream.previous_span().end;

    (
        Some(GroupByClause {
            elements,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a grouping element (expression or empty set).
fn parse_grouping_element(stream: &mut TokenStream) -> ParseResult<GroupingElement> {
    // Check for empty grouping set ()
    if stream.check(&TokenKind::LParen) {
        let next_pos = stream.position() + 1;
        if next_pos < stream.tokens().len()
            && matches!(stream.tokens()[next_pos].kind, TokenKind::RParen)
        {
            stream.advance(); // consume (
            stream.advance(); // consume )
            return (Some(GroupingElement::EmptyGroupingSet), vec![]);
        }
    }

    // Parse expression - need to use legacy interface temporarily
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (expr_opt, diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    (expr_opt.map(GroupingElement::Expression), diags)
}
