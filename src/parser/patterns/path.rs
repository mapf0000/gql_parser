//! Path pattern parsing for GQL.

use crate::ast::expression::{Expression, Literal};
use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use smol_str::SmolStr;

use super::{
    PatternParser, PatternSyncContext, SimplifiedClosing, SimplifiedOpening,
    element::edge_pattern_span, is_path_pattern_delimiter,
};

impl<'a> PatternParser<'a> {
    pub(super) fn parse_path_pattern(&mut self) -> Option<PathPattern> {
        if self.stream.check(&TokenKind::Eof)
            || matches!(self.current_kind(), Some(kind) if is_path_pattern_delimiter(kind))
        {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        let variable_declaration = self.parse_path_variable_declaration();
        let prefix = self.parse_path_pattern_prefix();

        let expression = match self.parse_path_pattern_expression() {
            Some(expression) => expression,
            None => {
                if variable_declaration.is_some() || prefix.is_some() {
                    self.diags.push(
                        Diag::error("Expected path pattern expression")
                            .with_primary_label(self.current_span_or(start), "expected path here"),
                    );
                }
                return None;
            }
        };

        let end = expression.span().end;
        Some(PathPattern {
            prefix,
            expression,
            variable_declaration,
            span: start..end,
        })
    }

    fn parse_path_variable_declaration(&mut self) -> Option<PathVariableDeclaration> {
        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        let name_token = tokens.get(pos)?;
        let Some(Token {
            kind: TokenKind::Eq,
            ..
        }) = tokens.get(pos + 1)
        else {
            return None;
        };

        let Some(name) = regular_identifier_from_kind(&name_token.kind) else {
            self.diags.push(
                Diag::error("Path variable declaration requires a regular identifier")
                    .with_primary_label(
                        name_token.span.clone(),
                        "expected regular identifier here",
                    ),
            );
            self.stream.advance();
            self.stream.advance();
            return None;
        };

        let start = name_token.span.start;
        self.stream.advance();
        self.stream.advance();
        let end = self.last_consumed_end(start);
        Some(PathVariableDeclaration {
            variable: name,
            span: start..end,
        })
    }

    pub(super) fn parse_path_pattern_prefix(&mut self) -> Option<PathPatternPrefix> {
        if let Some(search) = self.try_parse(|p| p.parse_path_search_prefix()) {
            return Some(PathPatternPrefix::PathSearch(search));
        }

        self.parse_path_mode_prefix()
            .map(PathPatternPrefix::PathMode)
    }

    fn parse_path_mode_prefix(&mut self) -> Option<PathMode> {
        let mode = self.parse_path_mode()?;
        self.consume_path_or_paths();
        Some(mode)
    }

    pub(super) fn parse_path_mode(&mut self) -> Option<PathMode> {
        let mode = match self.current_kind() {
            Some(TokenKind::Walk) => PathMode::Walk,
            Some(TokenKind::Trail) => PathMode::Trail,
            Some(TokenKind::Simple) => PathMode::Simple,
            Some(TokenKind::Acyclic) => PathMode::Acyclic,
            _ => return None,
        };
        self.stream.advance();
        Some(mode)
    }

    fn parse_path_search_prefix(&mut self) -> Option<PathSearch> {
        let start = self.current_start()?;

        match self.current_kind() {
            Some(TokenKind::All) => {
                self.stream.advance();
                if matches!(self.current_kind(), Some(TokenKind::Shortest)) {
                    self.stream.advance();
                    let mode = self.parse_path_mode();
                    self.consume_path_or_paths();
                    let end = self.last_consumed_end(start);
                    Some(PathSearch::Shortest(ShortestPathSearch::AllShortest {
                        mode,
                        span: start..end,
                    }))
                } else {
                    let mode = self.parse_path_mode();
                    let use_paths_keyword = self.consume_path_or_paths();
                    let end = self.last_consumed_end(start);
                    Some(PathSearch::All(AllPathSearch {
                        mode,
                        use_paths_keyword,
                        span: start..end,
                    }))
                }
            }
            Some(TokenKind::Any) => {
                self.stream.advance();
                let _count = self.parse_non_negative_integer_expression();

                if matches!(self.current_kind(), Some(TokenKind::Shortest)) {
                    self.stream.advance();
                    let mode = self.parse_path_mode();
                    self.consume_path_or_paths();
                    let end = self.last_consumed_end(start);
                    Some(PathSearch::Shortest(ShortestPathSearch::AnyShortest {
                        mode,
                        span: start..end,
                    }))
                } else {
                    let mode = self.parse_path_mode();
                    self.consume_path_or_paths();
                    let end = self.last_consumed_end(start);
                    Some(PathSearch::Any(AnyPathSearch {
                        mode,
                        span: start..end,
                    }))
                }
            }
            Some(TokenKind::Shortest) => {
                self.stream.advance();
                let count = self.parse_non_negative_integer_expression();
                let mode = self.parse_path_mode();
                let use_paths_keyword = self.consume_path_or_paths();

                if matches!(
                    self.current_kind(),
                    Some(TokenKind::Group | TokenKind::Groups)
                ) {
                    self.stream.advance();
                    let end = self.last_consumed_end(start);
                    let count = count.unwrap_or_else(|| {
                        Expression::Literal(
                            Literal::Integer(SmolStr::new("1")),
                            start..start.saturating_add(1),
                        )
                    });
                    return Some(PathSearch::Shortest(
                        ShortestPathSearch::CountedShortestGroups {
                            count,
                            mode,
                            span: start..end,
                        },
                    ));
                }

                if let Some(count) = count {
                    let end = self.last_consumed_end(start);
                    return Some(PathSearch::Shortest(ShortestPathSearch::CountedShortest {
                        count,
                        mode,
                        use_paths_keyword,
                        span: start..end,
                    }));
                }

                let end = self.last_consumed_end(start);
                Some(PathSearch::Shortest(ShortestPathSearch::AnyShortest {
                    mode,
                    span: start..end,
                }))
            }
            _ => None,
        }
    }

    fn parse_non_negative_integer_expression(&mut self) -> Option<Expression> {
        let token = self.stream.current();

        let TokenKind::IntegerLiteral(value) = &token.kind else {
            return None;
        };

        let value = value.clone();
        let span = token.span.clone();
        self.stream.advance();
        Some(Expression::Literal(
            Literal::Integer(value),
            span,
        ))
    }

    fn parse_path_pattern_expression(&mut self) -> Option<PathPatternExpression> {
        let first = self.parse_path_union_expression()?;

        if !self.is_multiset_alternation_operator() {
            return Some(first);
        }

        let mut alternatives = vec![first];
        while self.is_multiset_alternation_operator() {
            // Skip past '|+|'
            self.stream.advance();
            self.stream.advance();
            self.stream.advance();
            let Some(next) = self.parse_path_union_expression() else {
                self.diags.push(
                    Diag::error("Expected path expression after '|+|'").with_primary_label(
                        self.current_span_or(self.stream.position()),
                        "expected expression here",
                    ),
                );
                break;
            };
            alternatives.push(next);
        }

        let start = alternatives
            .first()
            .map(|expr| expr.span().start)
            .unwrap_or(0);
        let end = alternatives
            .last()
            .map(|expr| expr.span().end)
            .unwrap_or(start);
        Some(PathPatternExpression::Alternation {
            alternatives,
            span: start..end,
        })
    }

    fn parse_path_union_expression(&mut self) -> Option<PathPatternExpression> {
        let first_term = self.parse_path_term()?;
        let mut expr = PathPatternExpression::Term(first_term);

        while matches!(self.current_kind(), Some(TokenKind::Pipe))
            && !self.is_multiset_alternation_operator()
        {
            self.stream.advance();
            let Some(right_term) = self.parse_path_term() else {
                self.diags.push(
                    Diag::error("Expected path term after '|'")
                        .with_primary_label(self.current_span_or(self.stream.position()), "expected term here"),
                );
                break;
            };

            let right = PathPatternExpression::Term(right_term);
            let span = expr.span().start..right.span().end;
            expr = PathPatternExpression::Union {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Some(expr)
    }

    fn parse_path_term(&mut self) -> Option<PathTerm> {
        // GQL Grammar: pathTerm : pathFactor+ ;
        // This means: one or more path factors (not zero or more)

        // Parse first factor (required by grammar)
        let first_factor = self.parse_path_factor()?;
        let mut factors = vec![first_factor];

        // Parse remaining factors (zero or more)
        // Defensive check: ensure each successful parse makes forward progress.
        // This prevents infinite loops if a parse function incorrectly returns Some
        // without consuming tokens (e.g., calling advance() at EOF where it's a no-op).
        loop {
            let position_before = self.stream.position();

            let Some(factor) = self.parse_path_factor() else {
                break;
            };

            // Invariant check: successful parsing must advance the token position
            let position_after = self.stream.position();
            if position_after == position_before {
                // A parse function returned Some without consuming any tokens.
                // This violates the fundamental parsing invariant and would cause an
                // infinite loop. Common causes:
                // - Calling advance() at EOF (which is a no-op)
                // - Returning Some in error recovery without ensuring progress
                self.diags.push(
                    Diag::error("Internal parser error: parse_path_factor returned Some without advancing position")
                        .with_primary_label(
                            self.current_span_or(position_before),
                            "parsing stalled here - likely at EOF or in error recovery path"
                        ),
                );
                break;
            }

            factors.push(factor);
        }

        // SAFETY: factors is guaranteed to contain at least one element (first_factor)
        let start = factors.first().expect("factors vec must have at least one element").span.start;
        let end = factors.last().expect("factors vec must have at least one element").span.end;
        Some(PathTerm {
            factors,
            span: start..end,
        })
    }

    fn parse_path_factor(&mut self) -> Option<PathFactor> {
        let primary = self.parse_path_primary()?;
        let start = match &primary {
            PathPrimary::ElementPattern(element) => match element.as_ref() {
                ElementPattern::Node(node) => node.span.start,
                ElementPattern::Edge(edge) => edge_pattern_span(edge).start,
            },
            PathPrimary::ParenthesizedExpression(expr) => expr.span().start,
            PathPrimary::SimplifiedExpression(expr) => simplified_expression_span(expr).start,
        };

        let quantifier = self.parse_graph_pattern_quantifier();
        if quantifier.is_some() {
            self.consume_chained_quantifiers(start);
        }
        let end = quantifier.as_ref().map_or_else(
            || match &primary {
                PathPrimary::ElementPattern(element) => match element.as_ref() {
                    ElementPattern::Node(node) => node.span.end,
                    ElementPattern::Edge(edge) => edge_pattern_span(edge).end,
                },
                PathPrimary::ParenthesizedExpression(expr) => expr.span().end,
                PathPrimary::SimplifiedExpression(expr) => simplified_expression_span(expr).end,
            },
            |q| q.span().end,
        );

        Some(PathFactor {
            primary,
            quantifier,
            span: start..end,
        })
    }

    fn parse_path_primary(&mut self) -> Option<PathPrimary> {
        if self.looks_like_simplified_opening()
            && let Some(expr) = self.try_parse(|p| p.parse_simplified_path_pattern_expression())
        {
            return Some(PathPrimary::SimplifiedExpression(Box::new(expr)));
        }

        if matches!(self.current_kind(), Some(TokenKind::LParen)) {
            if matches!(
                self.stream.tokens().get(self.stream.position() + 1).map(|token| &token.kind),
                Some(TokenKind::LParen)
            ) && let Some(expr) =
                self.try_parse(|p| p.parse_parenthesized_path_pattern_expression())
            {
                return Some(PathPrimary::ParenthesizedExpression(Box::new(expr)));
            }

            if let Some(node) = self.try_parse(|p| p.parse_node_pattern()) {
                return Some(PathPrimary::ElementPattern(Box::new(ElementPattern::Node(
                    Box::new(node),
                ))));
            }

            if let Some(expr) = self.try_parse(|p| p.parse_parenthesized_path_pattern_expression())
            {
                return Some(PathPrimary::ParenthesizedExpression(Box::new(expr)));
            }

            return None;
        }

        if matches!(self.current_kind(), Some(kind) if is_edge_pattern_start(kind)) {
            let edge = self.parse_edge_pattern()?;
            return Some(PathPrimary::ElementPattern(Box::new(ElementPattern::Edge(
                edge,
            ))));
        }

        None
    }

    fn parse_parenthesized_path_pattern_expression(&mut self) -> Option<PathPatternExpression> {
        if !matches!(self.current_kind(), Some(TokenKind::LParen)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.stream.advance();

        self.parse_subpath_variable_declaration();
        self.parse_path_mode_prefix();

        let Some(expression) = self.parse_path_pattern_expression() else {
            self.diags.push(
                Diag::error("Expected path pattern expression inside parentheses")
                    .with_primary_label(
                        self.current_span_or(start),
                        "expected path expression here",
                    ),
            );
            return None;
        };

        if matches!(self.current_kind(), Some(TokenKind::Where)) {
            self.stream.advance();
            let expr_start = self.stream.position();
            let expr_end =
                self.find_expression_end(expr_start, |kind| matches!(kind, TokenKind::RParen));
            let _ = self.parse_expression_range(expr_start, expr_end, "condition after WHERE");
        }

        if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
            self.diags.push(
                Diag::error("Expected ')' to close parenthesized path pattern")
                    .with_primary_label(self.current_span_or(start), "expected ')' here"),
            );
            self.skip_to_path_pattern_boundary();
            return Some(expression);
        }

        self.stream.advance();
        Some(expression)
    }

    fn parse_subpath_variable_declaration(&mut self) {
        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        let Some(token) = tokens.get(pos) else {
            return;
        };
        if !matches!(
            tokens.get(pos + 1).map(|t| &t.kind),
            Some(TokenKind::Eq)
        ) {
            return;
        }

        if regular_identifier_from_kind(&token.kind).is_none() {
            self.diags.push(
                Diag::error("Subpath variable declaration requires a regular identifier")
                    .with_primary_label(token.span.clone(), "expected regular identifier here"),
            );
        }
        self.stream.advance();
        self.stream.advance();
    }

    fn parse_simplified_path_pattern_expression(
        &mut self,
    ) -> Option<SimplifiedPathPatternExpression> {
        let start = self.current_start()?;
        let opening = self.parse_simplified_opening()?;

        let Some(contents) = self.parse_simplified_contents() else {
            self.diags.push(
                Diag::error("Expected simplified path contents").with_primary_label(
                    self.current_span_or(start),
                    "expected simplified path content here",
                ),
            );
            return None;
        };

        if !matches!(self.current_kind(), Some(TokenKind::Slash)) {
            self.diags.push(
                Diag::error("Expected '/' to close simplified path body")
                    .with_primary_label(self.current_span_or(start), "expected '/' here"),
            );
            return Some(contents);
        }
        self.stream.advance();

        let closing = self.parse_simplified_closing();
        let end = self.last_consumed_end(start);

        let Some(direction) = opening_closing_direction(opening, closing) else {
            self.diags.push(
                Diag::error("Unsupported simplified direction delimiter combination")
                    .with_primary_label(
                        self.current_span_or(start),
                        "invalid simplified direction here",
                    ),
            );
            return Some(contents);
        };

        Some(SimplifiedPathPatternExpression::DirectionOverride(
            SimplifiedDirectionOverride {
                pattern: Box::new(contents),
                direction,
                span: start..end,
            },
        ))
    }

    fn parse_simplified_opening(&mut self) -> Option<SimplifiedOpening> {
        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        match (
            self.current_kind(),
            tokens.get(pos + 1).map(|t| &t.kind),
        ) {
            (Some(TokenKind::LeftArrow), Some(TokenKind::Slash)) => {
                self.stream.advance();
                self.stream.advance();
                Some(SimplifiedOpening::LeftArrow)
            }
            (Some(TokenKind::LeftTilde), Some(TokenKind::Slash)) => {
                self.stream.advance();
                self.stream.advance();
                Some(SimplifiedOpening::LeftOrUndirected)
            }
            (Some(TokenKind::Tilde), Some(TokenKind::Slash)) => {
                self.stream.advance();
                self.stream.advance();
                Some(SimplifiedOpening::Undirected)
            }
            (Some(TokenKind::Minus), Some(TokenKind::Slash)) => {
                self.stream.advance();
                self.stream.advance();
                Some(SimplifiedOpening::AnyOrRight)
            }
            _ => None,
        }
    }

    fn parse_simplified_closing(&mut self) -> Option<SimplifiedClosing> {
        let closing = match self.current_kind() {
            Some(TokenKind::Minus) => SimplifiedClosing::Minus,
            Some(TokenKind::Arrow) => SimplifiedClosing::Arrow,
            Some(TokenKind::Tilde) => SimplifiedClosing::Tilde,
            Some(TokenKind::RightTilde) => SimplifiedClosing::RightTilde,
            _ => return None,
        };
        self.stream.advance();
        Some(closing)
    }

    fn parse_simplified_contents(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let first = self.parse_simplified_union()?;

        if !self.is_multiset_alternation_operator() {
            return Some(first);
        }

        let mut alternatives = vec![first];
        while self.is_multiset_alternation_operator() {
            // Skip past '|+|'
            self.stream.advance();
            self.stream.advance();
            self.stream.advance();
            let Some(next) = self.parse_simplified_union() else {
                self.diags.push(
                    Diag::error("Expected simplified expression after '|+|'").with_primary_label(
                        self.current_span_or(self.stream.position()),
                        "expected expression here",
                    ),
                );
                break;
            };
            alternatives.push(next);
        }

        let start = simplified_expression_span(alternatives.first()?).start;
        let end = simplified_expression_span(alternatives.last()?).end;
        Some(SimplifiedPathPatternExpression::MultisetAlternation(
            SimplifiedMultisetAlternation {
                alternatives,
                span: start..end,
            },
        ))
    }

    fn parse_simplified_union(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let first = self.parse_simplified_term()?;
        let mut expr = first;

        while matches!(self.current_kind(), Some(TokenKind::Pipe))
            && !self.is_multiset_alternation_operator()
        {
            self.stream.advance();
            let Some(right) = self.parse_simplified_term() else {
                self.diags.push(
                    Diag::error("Expected simplified term after '|'")
                        .with_primary_label(self.current_span_or(self.stream.position()), "expected term here"),
                );
                break;
            };
            let span =
                simplified_expression_span(&expr).start..simplified_expression_span(&right).end;
            expr = SimplifiedPathPatternExpression::Union(SimplifiedPathUnion {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Some(expr)
    }

    fn parse_simplified_term(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let mut parts = Vec::new();

        loop {
            if matches!(
                self.current_kind(),
                Some(TokenKind::Pipe | TokenKind::Slash | TokenKind::RParen | TokenKind::Eof)
            ) {
                break;
            }
            if self.is_multiset_alternation_operator() {
                break;
            }

            let position_before = self.stream.position();
            let checkpoint = self.checkpoint();
            let Some(part) = self.parse_simplified_factor_low() else {
                self.restore(checkpoint);
                break;
            };

            // Invariant check: successful parsing must advance the token position
            let position_after = self.stream.position();
            if position_after == position_before {
                // A parse function returned Some without consuming any tokens.
                // This would cause an infinite loop. Common causes:
                // - Calling advance() at EOF (which is a no-op)
                // - Returning Some in error recovery without ensuring progress
                self.diags.push(
                    Diag::error("Internal parser error: parse_simplified_factor_low returned Some without advancing position")
                        .with_primary_label(
                            self.current_span_or(position_before),
                            "parsing stalled here - likely at EOF or in error recovery path"
                        ),
                );
                self.restore(checkpoint);
                break;
            }

            parts.push(part);
        }

        if parts.is_empty() {
            return None;
        }

        if parts.len() == 1 {
            return parts.into_iter().next();
        }

        let start = simplified_expression_span(parts.first()?).start;
        let end = simplified_expression_span(parts.last()?).end;
        Some(SimplifiedPathPatternExpression::Concatenation(
            SimplifiedConcatenation {
                parts,
                span: start..end,
            },
        ))
    }

    fn parse_simplified_factor_low(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let mut expr = self.parse_simplified_factor_high()?;

        while matches!(self.current_kind(), Some(TokenKind::Ampersand)) {
            self.stream.advance();
            let Some(right) = self.parse_simplified_factor_high() else {
                self.diags.push(
                    Diag::error("Expected simplified factor after '&'")
                        .with_primary_label(self.current_span_or(self.stream.position()), "expected factor here"),
                );
                break;
            };

            let span =
                simplified_expression_span(&expr).start..simplified_expression_span(&right).end;
            expr = SimplifiedPathPatternExpression::Conjunction(SimplifiedConjunction {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Some(expr)
    }

    fn parse_simplified_factor_high(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let tertiary = self.parse_simplified_tertiary()?;
        let start = simplified_expression_span(&tertiary).start;

        if matches!(self.current_kind(), Some(TokenKind::Question)) {
            self.stream.advance();
            self.consume_chained_quantifiers(start);
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::Questioned(
                SimplifiedQuestioned {
                    pattern: Box::new(tertiary),
                    span: start..end,
                },
            ));
        }

        if let Some(quantifier) = self.parse_graph_pattern_quantifier() {
            self.consume_chained_quantifiers(start);
            let end = quantifier.span().end;
            return Some(SimplifiedPathPatternExpression::Quantified(
                SimplifiedQuantified {
                    pattern: Box::new(tertiary),
                    quantifier,
                    span: start..end,
                },
            ));
        }

        Some(tertiary)
    }

    fn parse_simplified_tertiary(&mut self) -> Option<SimplifiedPathPatternExpression> {
        if let Some(override_expr) = self.try_parse(|p| p.parse_simplified_direction_override()) {
            return Some(override_expr);
        }

        self.parse_simplified_secondary()
    }

    fn parse_simplified_direction_override(&mut self) -> Option<SimplifiedPathPatternExpression> {
        if matches!(self.current_kind(), Some(TokenKind::Lt)) {
            let start = self.current_start().unwrap_or(self.stream.position());
            self.stream.advance();
            let inner = self.parse_simplified_secondary()?;
            let direction = if matches!(self.current_kind(), Some(TokenKind::Gt)) {
                self.stream.advance();
                EdgeDirection::AnyDirected
            } else {
                EdgeDirection::PointingLeft
            };
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::DirectionOverride(
                SimplifiedDirectionOverride {
                    pattern: Box::new(inner),
                    direction,
                    span: start..end,
                },
            ));
        }

        if matches!(self.current_kind(), Some(TokenKind::LeftTilde)) {
            let start = self.current_start().unwrap_or(self.stream.position());
            self.stream.advance();
            let inner = self.parse_simplified_secondary()?;
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::DirectionOverride(
                SimplifiedDirectionOverride {
                    pattern: Box::new(inner),
                    direction: EdgeDirection::LeftOrUndirected,
                    span: start..end,
                },
            ));
        }

        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        if matches!(self.current_kind(), Some(TokenKind::Tilde))
            && !matches!(
                tokens.get(pos + 1).map(|t| &t.kind),
                Some(TokenKind::Slash)
            )
        {
            let start = self.current_start().unwrap_or(self.stream.position());
            self.stream.advance();
            let inner = self.parse_simplified_secondary()?;
            let direction = if matches!(self.current_kind(), Some(TokenKind::Gt)) {
                self.stream.advance();
                EdgeDirection::RightOrUndirected
            } else {
                EdgeDirection::Undirected
            };
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::DirectionOverride(
                SimplifiedDirectionOverride {
                    pattern: Box::new(inner),
                    direction,
                    span: start..end,
                },
            ));
        }

        if matches!(self.current_kind(), Some(TokenKind::Minus))
            && !matches!(
                tokens.get(pos + 1).map(|t| &t.kind),
                Some(TokenKind::Slash)
            )
        {
            let start = self.current_start().unwrap_or(self.stream.position());
            self.stream.advance();
            let inner = self.parse_simplified_secondary()?;
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::DirectionOverride(
                SimplifiedDirectionOverride {
                    pattern: Box::new(inner),
                    direction: EdgeDirection::AnyDirection,
                    span: start..end,
                },
            ));
        }

        let checkpoint = self.checkpoint();
        let inner = self.parse_simplified_secondary()?;
        if matches!(self.current_kind(), Some(TokenKind::Gt)) {
            let start = simplified_expression_span(&inner).start;
            self.stream.advance();
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::DirectionOverride(
                SimplifiedDirectionOverride {
                    pattern: Box::new(inner),
                    direction: EdgeDirection::PointingRight,
                    span: start..end,
                },
            ));
        }

        self.restore(checkpoint);
        None
    }

    fn parse_simplified_secondary(&mut self) -> Option<SimplifiedPathPatternExpression> {
        if matches!(self.current_kind(), Some(TokenKind::Bang)) {
            let start = self.current_start().unwrap_or(self.stream.position());
            self.stream.advance();
            let inner = self.parse_simplified_primary()?;
            let end = simplified_expression_span(&inner).end;
            return Some(SimplifiedPathPatternExpression::Negation(
                SimplifiedNegation {
                    pattern: Box::new(inner),
                    span: start..end,
                },
            ));
        }

        self.parse_simplified_primary()
    }

    fn parse_simplified_primary(&mut self) -> Option<SimplifiedPathPatternExpression> {
        if matches!(self.current_kind(), Some(TokenKind::LParen)) {
            self.stream.advance();
            let inner = self.parse_simplified_contents()?;
            if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
                self.diags.push(
                    Diag::error("Expected ')' to close simplified subexpression")
                        .with_primary_label(self.current_span_or(self.stream.position()), "expected ')' here"),
                );
            } else {
                self.stream.advance();
            }
            return Some(inner);
        }

        let token = self.stream.current();
        let label = match &token.kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => name.clone(),
            kind if kind.is_keyword() => SmolStr::new(kind.to_string()),
            _ => return None,
        };
        let span = token.span.clone();
        self.stream.advance();

        Some(SimplifiedPathPatternExpression::Contents(
            SimplifiedContents {
                labels: vec![label],
                span,
            },
        ))
    }

    pub(super) fn parse_graph_pattern_quantifier(&mut self) -> Option<GraphPatternQuantifier> {
        match self.current_kind() {
            Some(TokenKind::Star) => {
                let span = self.current_span_or(self.stream.position());
                self.stream.advance();
                Some(GraphPatternQuantifier::Star { span })
            }
            Some(TokenKind::Plus) => {
                let span = self.current_span_or(self.stream.position());
                self.stream.advance();
                Some(GraphPatternQuantifier::Plus { span })
            }
            Some(TokenKind::Question) => {
                let span = self.current_span_or(self.stream.position());
                self.stream.advance();
                Some(GraphPatternQuantifier::QuestionMark { span })
            }
            Some(TokenKind::LBrace) => self.parse_brace_quantifier(),
            _ => None,
        }
    }

    fn is_quantifier_start(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::Star | TokenKind::Plus | TokenKind::Question | TokenKind::LBrace)
        )
    }

    fn consume_chained_quantifiers(&mut self, fallback_start: usize) {
        let mut reported = false;
        while self.is_quantifier_start() {
            let span = self.current_span_or(fallback_start);
            let consumed = self.parse_graph_pattern_quantifier();
            if consumed.is_none() {
                self.stream.advance();
            }
            if !reported {
                self.diags.push(
                    Diag::error("Chained path quantifiers are not allowed")
                        .with_primary_label(span, "remove the extra quantifier"),
                );
                reported = true;
            }
        }
    }

    fn parse_brace_quantifier(&mut self) -> Option<GraphPatternQuantifier> {
        let start = self.current_start().unwrap_or(self.stream.position());
        self.stream.advance();

        let lower = self.parse_u32_bound();

        if matches!(self.current_kind(), Some(TokenKind::RBrace)) {
            self.stream.advance();
            let end = self.last_consumed_end(start);
            if let Some(count) = lower {
                return Some(GraphPatternQuantifier::Fixed {
                    count,
                    span: start..end,
                });
            }

            self.diags.push(
                Diag::error("Expected integer in fixed quantifier")
                    .with_primary_label(start..end, "expected integer here"),
            );
            return None;
        }

        if !matches!(self.current_kind(), Some(TokenKind::Comma)) {
            self.diags.push(
                Diag::error("Expected ',' or '}' in quantifier")
                    .with_primary_label(self.current_span_or(start), "expected ',' or '}' here"),
            );
            self.skip_to_token(|kind| matches!(kind, TokenKind::RBrace));
            if matches!(self.current_kind(), Some(TokenKind::RBrace)) {
                self.stream.advance();
            }
            return None;
        }

        self.stream.advance();
        let upper = self.parse_u32_bound();

        if !matches!(self.current_kind(), Some(TokenKind::RBrace)) {
            self.diags.push(
                Diag::error("Expected '}' to close quantifier")
                    .with_primary_label(self.current_span_or(start), "expected '}' here"),
            );
            self.skip_to_token(|kind| matches!(kind, TokenKind::RBrace));
            if matches!(self.current_kind(), Some(TokenKind::RBrace)) {
                self.stream.advance();
            }
            return None;
        }

        self.stream.advance();
        let end = self.last_consumed_end(start);

        if lower.is_none() && upper.is_none() {
            self.diags.push(
                Diag::error("General quantifier requires at least one bound")
                    .with_primary_label(start..end, "expected bound here"),
            );
            return None;
        }

        if let (Some(min), Some(max)) = (lower, upper)
            && min > max
        {
            self.diags.push(
                Diag::error("Invalid quantifier bounds: lower bound is greater than upper bound")
                    .with_primary_label(start..end, "lower bound must be <= upper bound"),
            );
        }

        Some(GraphPatternQuantifier::General {
            min: lower,
            max: upper,
            span: start..end,
        })
    }

    fn parse_u32_bound(&mut self) -> Option<u32> {
        let token = self.stream.current();

        let TokenKind::IntegerLiteral(raw) = &token.kind else {
            return None;
        };

        let span = token.span.clone();
        let raw = raw.clone();
        self.stream.advance();
        match parse_integer_literal_to_u32(&raw) {
            Some(value) => Some(value),
            None => {
                self.diags.push(
                    Diag::error("Expected non-negative integer bound")
                        .with_primary_label(span, "invalid integer literal"),
                );
                None
            }
        }
    }

    pub(super) fn looks_like_simplified_opening(&self) -> bool {
        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        matches!(
            (
                self.current_kind(),
                tokens.get(pos + 1).map(|t| &t.kind)
            ),
            (Some(TokenKind::LeftArrow), Some(TokenKind::Slash))
                | (Some(TokenKind::LeftTilde), Some(TokenKind::Slash))
                | (Some(TokenKind::Tilde), Some(TokenKind::Slash))
                | (Some(TokenKind::Minus), Some(TokenKind::Slash))
        )
    }

    pub(super) fn is_multiset_alternation_operator(&self) -> bool {
        let tokens = self.stream.tokens();
        let pos = self.stream.position();
        matches!(self.current_kind(), Some(TokenKind::Pipe))
            && matches!(
                tokens.get(pos + 1).map(|t| &t.kind),
                Some(TokenKind::Plus)
            )
            && matches!(
                tokens.get(pos + 2).map(|t| &t.kind),
                Some(TokenKind::Pipe)
            )
    }

    fn skip_to_path_pattern_boundary(&mut self) {
        self.skip_to_sync(PatternSyncContext::PathPattern);
    }
}

use crate::lexer::token::Token;

fn parse_integer_literal_to_u32(raw: &str) -> Option<u32> {
    let cleaned = raw.replace('_', "");

    if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16).ok();
    }

    if let Some(oct) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        return u32::from_str_radix(oct, 8).ok();
    }

    if let Some(bin) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        return u32::from_str_radix(bin, 2).ok();
    }

    cleaned.parse::<u32>().ok()
}

pub(super) fn simplified_expression_span(
    expr: &SimplifiedPathPatternExpression,
) -> std::ops::Range<usize> {
    match expr {
        SimplifiedPathPatternExpression::Contents(contents) => contents.span.clone(),
        SimplifiedPathPatternExpression::Union(union) => union.span.clone(),
        SimplifiedPathPatternExpression::MultisetAlternation(alt) => alt.span.clone(),
        SimplifiedPathPatternExpression::Conjunction(conj) => conj.span.clone(),
        SimplifiedPathPatternExpression::Concatenation(concat) => concat.span.clone(),
        SimplifiedPathPatternExpression::Quantified(quant) => quant.span.clone(),
        SimplifiedPathPatternExpression::Questioned(questioned) => questioned.span.clone(),
        SimplifiedPathPatternExpression::DirectionOverride(override_) => override_.span.clone(),
        SimplifiedPathPatternExpression::Negation(negation) => negation.span.clone(),
    }
}

pub(super) fn opening_closing_direction(
    opening: SimplifiedOpening,
    closing: Option<SimplifiedClosing>,
) -> Option<EdgeDirection> {
    match (opening, closing?) {
        (SimplifiedOpening::LeftArrow, SimplifiedClosing::Minus) => {
            Some(EdgeDirection::PointingLeft)
        }
        (SimplifiedOpening::LeftArrow, SimplifiedClosing::Arrow) => {
            Some(EdgeDirection::AnyDirected)
        }
        (SimplifiedOpening::LeftOrUndirected, SimplifiedClosing::Tilde) => {
            Some(EdgeDirection::LeftOrUndirected)
        }
        (SimplifiedOpening::Undirected, SimplifiedClosing::Tilde) => {
            Some(EdgeDirection::Undirected)
        }
        (SimplifiedOpening::Undirected, SimplifiedClosing::RightTilde) => {
            Some(EdgeDirection::RightOrUndirected)
        }
        (SimplifiedOpening::AnyOrRight, SimplifiedClosing::Arrow) => {
            Some(EdgeDirection::PointingRight)
        }
        (SimplifiedOpening::AnyOrRight, SimplifiedClosing::Minus) => {
            Some(EdgeDirection::AnyDirection)
        }
        _ => None,
    }
}

fn is_edge_pattern_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LeftArrow
            | TokenKind::Arrow
            | TokenKind::Minus
            | TokenKind::Tilde
            | TokenKind::LeftTilde
            | TokenKind::RightTilde
    )
}

fn regular_identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}
