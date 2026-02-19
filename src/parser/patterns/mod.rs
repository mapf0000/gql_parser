//! Graph pattern and path pattern parsing for GQL (Sprint 8).

use crate::ast::expression::Expression;
use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::LegacyParseResult;
use crate::parser::expression::parse_expression;
use smol_str::SmolStr;

mod element;
mod label;
mod path;

/// Parse result with optional value and diagnostics.
type ParseResult<T> = LegacyParseResult<T>;

/// Parses a graph pattern starting at the given position.
pub fn parse_graph_pattern(tokens: &[Token], pos: &mut usize) -> ParseResult<GraphPattern> {
    let mut parser = PatternParser::new(tokens, *pos);
    let pattern = parser.parse_graph_pattern();
    *pos = parser.pos;
    (pattern, parser.diags)
}

/// Parses a graph pattern binding table.
pub fn parse_graph_pattern_binding_table(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<GraphPatternBindingTable> {
    let mut parser = PatternParser::new(tokens, *pos);
    let table = parser.parse_graph_pattern().map(|pattern| {
        let span = pattern.span.clone();
        let yield_clause = pattern.yield_clause.clone();
        GraphPatternBindingTable {
            pattern: Box::new(pattern),
            yield_clause,
            span,
        }
    });
    *pos = parser.pos;
    (table, parser.diags)
}

#[derive(Clone, Copy)]
enum FillerTerminator {
    RParen,
    RBracket,
}

impl FillerTerminator {
    fn matches(self, kind: &TokenKind) -> bool {
        match self {
            FillerTerminator::RParen => matches!(kind, TokenKind::RParen),
            FillerTerminator::RBracket => matches!(kind, TokenKind::RBracket),
        }
    }
}

#[derive(Clone, Copy)]
enum SimplifiedOpening {
    LeftArrow,
    LeftOrUndirected,
    Undirected,
    AnyOrRight,
}

#[derive(Clone, Copy)]
enum SimplifiedClosing {
    Minus,
    Arrow,
    Tilde,
    RightTilde,
}

struct ParsedElementFiller {
    variable: Option<ElementVariableDeclaration>,
    label_expression: Option<LabelExpression>,
    properties: Option<ElementPropertySpecification>,
    where_clause: Option<ElementPatternPredicate>,
    span: std::ops::Range<usize>,
}

struct PatternParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diags: Vec<Diag>,
}

impl<'a> PatternParser<'a> {
    fn new(tokens: &'a [Token], start: usize) -> Self {
        Self {
            tokens,
            pos: start,
            diags: Vec::new(),
        }
    }

    fn parse_graph_pattern(&mut self) -> Option<GraphPattern> {
        let start_pos = self.pos;
        if self.pos >= self.tokens.len()
            || matches!(self.current_kind(), Some(kind) if is_query_boundary(kind))
        {
            let span = self.current_span_or(start_pos);
            self.diags.push(
                Diag::error("Expected graph pattern")
                    .with_primary_label(span, "expected graph pattern here"),
            );
            return None;
        }

        let match_mode = self.parse_match_mode();
        let paths = match self.parse_path_pattern_list() {
            Some(paths) => paths,
            None => {
                if self.pos == start_pos {
                    self.skip_to_statement_boundary();
                }
                return None;
            }
        };

        let keep_clause = if matches!(self.current_kind(), Some(TokenKind::Keep)) {
            self.parse_keep_clause()
        } else {
            None
        };

        let where_clause = if matches!(self.current_kind(), Some(TokenKind::Where)) {
            self.parse_graph_pattern_where_clause()
        } else {
            None
        };

        let yield_clause = if matches!(self.current_kind(), Some(TokenKind::Yield)) {
            self.parse_graph_pattern_yield_clause()
        } else {
            None
        };

        if self.pos == start_pos {
            self.diags.push(
                Diag::error("Graph pattern parser made no progress").with_primary_label(
                    self.current_span_or(start_pos),
                    "pattern parser stalled here",
                ),
            );
            return None;
        }

        let start = self
            .tokens
            .get(start_pos)
            .map(|t| t.span.start)
            .unwrap_or(paths.span.start);
        let end = self.last_consumed_end(paths.span.end);

        Some(GraphPattern {
            match_mode,
            paths,
            keep_clause,
            where_clause,
            yield_clause,
            span: start..end,
        })
    }

    fn parse_match_mode(&mut self) -> Option<MatchMode> {
        match self.current_kind() {
            Some(TokenKind::Repeatable) => {
                let repeatable_span = self.current_span_or(self.pos);
                self.pos += 1;

                if matches!(self.current_kind(), Some(kind) if is_elements_keyword(kind)) {
                    self.pos += 1;
                    if matches!(self.current_kind(), Some(kind) if is_bindings_keyword(kind)) {
                        self.pos += 1;
                    }
                } else {
                    self.diags.push(
                        Diag::error("Expected ELEMENT or ELEMENTS after REPEATABLE")
                            .with_primary_label(
                                self.current_span_or(repeatable_span.end),
                                "expected ELEMENTS here",
                            ),
                    );
                }

                Some(MatchMode::RepeatableElements)
            }
            Some(TokenKind::Different) => {
                let different_span = self.current_span_or(self.pos);
                self.pos += 1;

                if matches!(self.current_kind(), Some(kind) if is_edge_keyword(kind)) {
                    self.pos += 1;
                    if matches!(self.current_kind(), Some(kind) if is_bindings_keyword(kind)) {
                        self.pos += 1;
                    }
                } else {
                    self.diags.push(
                        Diag::error("Expected EDGE or EDGES after DIFFERENT").with_primary_label(
                            self.current_span_or(different_span.end),
                            "expected EDGES here",
                        ),
                    );
                }

                Some(MatchMode::DifferentEdges)
            }
            _ => None,
        }
    }

    fn parse_path_pattern_list(&mut self) -> Option<PathPatternList> {
        let start = self.current_start().unwrap_or(0);
        let mut patterns = Vec::new();

        let first = self.parse_path_pattern();
        if let Some(pattern) = first {
            patterns.push(pattern);
        } else {
            self.diags.push(
                Diag::error("Expected path pattern in MATCH clause")
                    .with_primary_label(self.current_span_or(start), "expected path pattern here"),
            );
            return None;
        }

        while matches!(self.current_kind(), Some(TokenKind::Comma)) {
            let comma_span = self.current_span_or(start);
            self.pos += 1;

            let Some(pattern) = self.parse_path_pattern() else {
                self.diags.push(
                    Diag::error("Expected path pattern after ','")
                        .with_primary_label(comma_span, "missing path pattern"),
                );
                break;
            };
            patterns.push(pattern);
        }

        let end = patterns.last().map(|p| p.span.end).unwrap_or(start);
        Some(PathPatternList {
            patterns,
            span: start..end,
        })
    }

    fn parse_keep_clause(&mut self) -> Option<KeepClause> {
        if !matches!(self.current_kind(), Some(TokenKind::Keep)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.pos += 1;

        let Some(prefix) = self.parse_path_pattern_prefix() else {
            self.diags.push(
                Diag::error("Expected path prefix after KEEP")
                    .with_primary_label(self.current_span_or(start), "expected prefix after KEEP"),
            );
            self.skip_to_where_or_statement_boundary();
            return None;
        };

        let end = self.last_consumed_end(start);
        Some(KeepClause {
            prefix,
            span: start..end,
        })
    }

    fn parse_graph_pattern_where_clause(&mut self) -> Option<GraphPatternWhereClause> {
        if !matches!(self.current_kind(), Some(TokenKind::Where)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.pos += 1;

        let expr_start = self.pos;
        let expr_end = self.find_expression_end(expr_start, |kind| {
            matches!(
                kind,
                TokenKind::Comma | TokenKind::Where | TokenKind::Keep | TokenKind::Yield
            ) || is_query_boundary(kind)
        });

        let condition =
            self.parse_expression_range(expr_start, expr_end, "condition after WHERE")?;
        let end = condition.span().end;

        Some(GraphPatternWhereClause {
            condition,
            span: start..end,
        })
    }

    fn parse_graph_pattern_yield_clause(&mut self) -> Option<GraphPatternYieldClause> {
        if !matches!(self.current_kind(), Some(TokenKind::Yield)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.pos += 1;

        let mut items = Vec::new();

        let Some(first_item) = self.parse_graph_pattern_yield_item() else {
            self.diags.push(
                Diag::error("Expected YIELD item after YIELD")
                    .with_primary_label(self.current_span_or(start), "expected YIELD item here"),
            );
            return None;
        };
        items.push(first_item);

        while matches!(self.current_kind(), Some(TokenKind::Comma)) {
            self.pos += 1;
            let Some(item) = self.parse_graph_pattern_yield_item() else {
                self.diags.push(
                    Diag::error("Expected YIELD item after ','").with_primary_label(
                        self.current_span_or(start),
                        "expected YIELD item here",
                    ),
                );
                break;
            };
            items.push(item);
        }

        let end = items.last().map(|item| item.span.end).unwrap_or(start);
        Some(GraphPatternYieldClause {
            items,
            span: start..end,
        })
    }

    fn parse_graph_pattern_yield_item(&mut self) -> Option<YieldItem> {
        let start_index = self.pos;
        let start_span = self.current_start().unwrap_or(self.pos);

        let item_end = self.find_expression_end(start_index, |kind| {
            matches!(kind, TokenKind::Comma) || is_query_boundary(kind)
        });

        if item_end <= start_index {
            self.diags.push(
                Diag::error("Expected YIELD item").with_primary_label(
                    self.current_span_or(start_span),
                    "expected YIELD item here",
                ),
            );
            return None;
        }

        let (expr_end, alias) = self.find_trailing_alias(start_index, item_end);
        let expression =
            self.parse_expression_range(start_index, expr_end, "YIELD item expression")?;

        self.pos = item_end;

        let end = self
            .tokens
            .get(item_end.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or_else(|| expression.span().end);

        Some(YieldItem {
            expression,
            alias,
            span: start_span..end,
        })
    }

    fn find_trailing_alias(&self, start: usize, end: usize) -> (usize, Option<SmolStr>) {
        let mut i = start;
        let mut depth = 0usize;

        while i < end {
            let kind = &self.tokens[i].kind;

            match kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                TokenKind::As if depth == 0 => {
                    if i + 2 == end
                        && let Some(name) = self
                            .tokens
                            .get(i + 1)
                            .and_then(|t| identifier_from_kind(&t.kind))
                    {
                        return (i, Some(name));
                    }
                }
                _ => {}
            }

            i += 1;
        }

        (end, None)
    }

    fn parse_expression_range(
        &mut self,
        start: usize,
        end: usize,
        context: &str,
    ) -> Option<Expression> {
        if end <= start {
            self.diags.push(
                Diag::error(format!("Expected {context}")).with_primary_label(
                    self.current_span_or(start),
                    format!("expected {context} here"),
                ),
            );
            self.pos = end;
            return None;
        }

        let result = parse_expression(&self.tokens[start..end]);
        self.pos = end;

        match result {
            Ok(expr) => Some(expr),
            Err(err) => {
                self.diags.push(*err);
                None
            }
        }
    }

    fn find_expression_end<F>(&self, start: usize, mut should_stop: F) -> usize
    where
        F: FnMut(&TokenKind) -> bool,
    {
        let mut idx = start;
        let mut depth = 0usize;

        while idx < self.tokens.len() {
            let kind = &self.tokens[idx].kind;

            if depth == 0 && should_stop(kind) {
                break;
            }

            match kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }

            idx += 1;
        }

        idx
    }

    fn consume_path_or_paths(&mut self) -> bool {
        match self.current_kind() {
            Some(TokenKind::Path) | Some(TokenKind::Paths) => {
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    fn skip_to_where_or_statement_boundary(&mut self) {
        self.skip_to_token(|kind| matches!(kind, TokenKind::Where) || is_query_boundary(kind));
    }

    fn skip_to_statement_boundary(&mut self) {
        self.skip_to_token(is_query_boundary);
    }

    fn skip_to_token<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&TokenKind) -> bool,
    {
        while self.pos < self.tokens.len() {
            if predicate(&self.tokens[self.pos].kind) {
                break;
            }
            self.pos += 1;
        }
    }

    fn checkpoint(&self) -> (usize, usize) {
        (self.pos, self.diags.len())
    }

    fn restore(&mut self, checkpoint: (usize, usize)) {
        self.pos = checkpoint.0;
        self.diags.truncate(checkpoint.1);
    }

    fn try_parse<T>(&mut self, parser: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let checkpoint = self.checkpoint();
        let result = parser(self);
        if result.is_none() {
            self.restore(checkpoint);
        }
        result
    }

    fn current_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn current_start(&self) -> Option<usize> {
        self.tokens.get(self.pos).map(|t| t.span.start)
    }

    fn current_span_or(&self, fallback: usize) -> std::ops::Range<usize> {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or(fallback..fallback)
    }

    fn last_consumed_end(&self, fallback: usize) -> usize {
        self.tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(fallback)
    }
}

fn identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}

fn is_query_boundary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Semicolon
            | TokenKind::Eof
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
            | TokenKind::Return
            | TokenKind::Select
            | TokenKind::Finish
            | TokenKind::Union
            | TokenKind::Except
            | TokenKind::Intersect
            | TokenKind::Otherwise
            | TokenKind::Group
            | TokenKind::Having
            | TokenKind::RBrace
            | TokenKind::RParen
    )
}

fn is_elements_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Elements | TokenKind::Element)
        || token_matches_word(kind, "ELEMENT")
        || token_matches_word(kind, "ELEMENTS")
}

fn is_bindings_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Binding)
        || token_matches_word(kind, "BINDING")
        || token_matches_word(kind, "BINDINGS")
}

fn is_edge_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Edge | TokenKind::Relationship)
        || token_matches_word(kind, "EDGE")
        || token_matches_word(kind, "EDGES")
}

fn token_matches_word(kind: &TokenKind, word: &str) -> bool {
    match kind {
        TokenKind::Identifier(name)
        | TokenKind::ReservedKeyword(name)
        | TokenKind::PreReservedKeyword(name)
        | TokenKind::NonReservedKeyword(name) => name.eq_ignore_ascii_case(word),
        _ => false,
    }
}
