//! Graph pattern and path pattern parsing for GQL (Sprint 8).

use crate::ast::expression::{Expression, Literal};
use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::expression::parse_expression;
use smol_str::SmolStr;

/// Parse result with optional value and diagnostics.
type ParseResult<T> = (Option<T>, Vec<Diag>);

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

    fn parse_path_pattern(&mut self) -> Option<PathPattern> {
        if self.pos >= self.tokens.len()
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
        let name_token = self.tokens.get(self.pos)?;
        let Some(Token {
            kind: TokenKind::Eq,
            ..
        }) = self.tokens.get(self.pos + 1)
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
            self.pos += 2;
            return None;
        };

        let start = name_token.span.start;
        self.pos += 2;
        let end = self.last_consumed_end(start);
        Some(PathVariableDeclaration {
            variable: name,
            span: start..end,
        })
    }

    fn parse_path_pattern_prefix(&mut self) -> Option<PathPatternPrefix> {
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

    fn parse_path_mode(&mut self) -> Option<PathMode> {
        let mode = match self.current_kind() {
            Some(TokenKind::Walk) => PathMode::Walk,
            Some(TokenKind::Trail) => PathMode::Trail,
            Some(TokenKind::Simple) => PathMode::Simple,
            Some(TokenKind::Acyclic) => PathMode::Acyclic,
            _ => return None,
        };
        self.pos += 1;
        Some(mode)
    }

    fn parse_path_search_prefix(&mut self) -> Option<PathSearch> {
        let start = self.current_start()?;

        match self.current_kind() {
            Some(TokenKind::All) => {
                self.pos += 1;
                if matches!(self.current_kind(), Some(TokenKind::Shortest)) {
                    self.pos += 1;
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
                self.pos += 1;
                let _count = self.parse_non_negative_integer_expression();

                if matches!(self.current_kind(), Some(TokenKind::Shortest)) {
                    self.pos += 1;
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
                self.pos += 1;
                let count = self.parse_non_negative_integer_expression();
                let mode = self.parse_path_mode();
                let use_paths_keyword = self.consume_path_or_paths();

                if matches!(
                    self.current_kind(),
                    Some(TokenKind::Group | TokenKind::Groups)
                ) {
                    self.pos += 1;
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
        let token = self.tokens.get(self.pos)?;

        let TokenKind::IntegerLiteral(value) = &token.kind else {
            return None;
        };

        self.pos += 1;
        Some(Expression::Literal(
            Literal::Integer(value.clone()),
            token.span.clone(),
        ))
    }

    fn parse_path_pattern_expression(&mut self) -> Option<PathPatternExpression> {
        let first_term = self.parse_path_term()?;

        if self.is_multiset_alternation_operator() {
            let mut alternatives = vec![first_term];

            while self.is_multiset_alternation_operator() {
                self.pos += 3;
                let Some(term) = self.parse_path_term() else {
                    self.diags.push(
                        Diag::error("Expected path term after '|+|'").with_primary_label(
                            self.current_span_or(self.pos),
                            "expected term here",
                        ),
                    );
                    break;
                };
                alternatives.push(term);
            }

            if alternatives.len() == 1 {
                return Some(PathPatternExpression::Term(alternatives.remove(0)));
            }

            let start = alternatives.first().map(|t| t.span.start).unwrap_or(0);
            let end = alternatives.last().map(|t| t.span.end).unwrap_or(start);
            return Some(PathPatternExpression::Alternation {
                alternatives,
                span: start..end,
            });
        }

        if matches!(self.current_kind(), Some(TokenKind::Pipe)) {
            let mut expr = PathPatternExpression::Term(first_term);

            while matches!(self.current_kind(), Some(TokenKind::Pipe)) {
                self.pos += 1;
                let Some(right_term) = self.parse_path_term() else {
                    self.diags.push(
                        Diag::error("Expected path term after '|'").with_primary_label(
                            self.current_span_or(self.pos),
                            "expected term here",
                        ),
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

            return Some(expr);
        }

        Some(PathPatternExpression::Term(first_term))
    }

    fn parse_path_term(&mut self) -> Option<PathTerm> {
        let mut factors = Vec::new();

        loop {
            let checkpoint = self.checkpoint();
            let Some(factor) = self.parse_path_factor() else {
                self.restore(checkpoint);
                break;
            };
            factors.push(factor);
        }

        if factors.is_empty() {
            return None;
        }

        let start = factors.first().map(|f| f.span.start).unwrap_or(0);
        let end = factors.last().map(|f| f.span.end).unwrap_or(start);
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
        self.pos += 1;

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
            self.pos += 1;
            let expr_start = self.pos;
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

        self.pos += 1;
        Some(expression)
    }

    fn parse_subpath_variable_declaration(&mut self) {
        let Some(token) = self.tokens.get(self.pos) else {
            return;
        };
        if !matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
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
        self.pos += 2;
    }

    fn parse_node_pattern(&mut self) -> Option<NodePattern> {
        if !matches!(self.current_kind(), Some(TokenKind::LParen)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.pos += 1;

        let filler = self.parse_element_pattern_filler(FillerTerminator::RParen, start);

        if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
            self.diags.push(
                Diag::error("Expected ')' to close node pattern")
                    .with_primary_label(self.current_span_or(start), "expected ')' here"),
            );
            self.skip_to_token(|kind| matches!(kind, TokenKind::RParen));
            if matches!(self.current_kind(), Some(TokenKind::RParen)) {
                self.pos += 1;
            }
        } else {
            self.pos += 1;
        }

        let end = self.last_consumed_end(start);
        Some(NodePattern {
            variable: filler.variable,
            label_expression: filler.label_expression,
            properties: filler.properties,
            where_clause: filler.where_clause,
            span: start..end,
        })
    }

    fn parse_edge_pattern(&mut self) -> Option<EdgePattern> {
        let start = self.current_start()?;

        match self.current_kind() {
            Some(TokenKind::LeftArrow) => {
                self.pos += 1;
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.pos += 1;
                    let filler =
                        self.parse_element_pattern_filler(FillerTerminator::RBracket, start);
                    if !matches!(self.current_kind(), Some(TokenKind::RBracket)) {
                        self.diags.push(
                            Diag::error("Expected ']' in edge pattern").with_primary_label(
                                self.current_span_or(start),
                                "expected ']' here",
                            ),
                        );
                        return None;
                    }
                    self.pos += 1;

                    let direction = match self.current_kind() {
                        Some(TokenKind::Minus) => EdgeDirection::PointingLeft,
                        Some(TokenKind::Arrow) => EdgeDirection::AnyDirected,
                        _ => {
                            self.diags.push(
                                Diag::error("Expected '-' or '->' after edge filler")
                                    .with_primary_label(
                                        self.current_span_or(start),
                                        "expected edge direction terminator here",
                                    ),
                            );
                            return None;
                        }
                    };
                    self.pos += 1;

                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Full(Box::new(FullEdgePattern {
                        direction,
                        filler: FullEdgePointingFiller {
                            variable: filler.variable,
                            label_expression: filler.label_expression,
                            properties: filler.properties,
                            where_clause: filler.where_clause,
                            span: filler.span,
                        },
                        span: start..end,
                    })))
                } else {
                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Abbreviated(
                        AbbreviatedEdgePattern::LeftArrow { span: start..end },
                    ))
                }
            }
            Some(TokenKind::Minus) => {
                self.pos += 1;
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.pos += 1;
                    let filler =
                        self.parse_element_pattern_filler(FillerTerminator::RBracket, start);
                    if !matches!(self.current_kind(), Some(TokenKind::RBracket)) {
                        self.diags.push(
                            Diag::error("Expected ']' in edge pattern").with_primary_label(
                                self.current_span_or(start),
                                "expected ']' here",
                            ),
                        );
                        return None;
                    }
                    self.pos += 1;

                    let direction = match self.current_kind() {
                        Some(TokenKind::Arrow) => EdgeDirection::PointingRight,
                        Some(TokenKind::Minus) => EdgeDirection::AnyDirection,
                        _ => {
                            self.diags.push(
                                Diag::error("Expected '-' or '->' after edge filler")
                                    .with_primary_label(
                                        self.current_span_or(start),
                                        "expected edge direction terminator here",
                                    ),
                            );
                            return None;
                        }
                    };
                    self.pos += 1;

                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Full(Box::new(FullEdgePattern {
                        direction,
                        filler: FullEdgePointingFiller {
                            variable: filler.variable,
                            label_expression: filler.label_expression,
                            properties: filler.properties,
                            where_clause: filler.where_clause,
                            span: filler.span,
                        },
                        span: start..end,
                    })))
                } else {
                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Abbreviated(
                        AbbreviatedEdgePattern::AnyDirection { span: start..end },
                    ))
                }
            }
            Some(TokenKind::Tilde) => {
                self.pos += 1;
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.pos += 1;
                    let filler =
                        self.parse_element_pattern_filler(FillerTerminator::RBracket, start);
                    if !matches!(self.current_kind(), Some(TokenKind::RBracket)) {
                        self.diags.push(
                            Diag::error("Expected ']' in edge pattern").with_primary_label(
                                self.current_span_or(start),
                                "expected ']' here",
                            ),
                        );
                        return None;
                    }
                    self.pos += 1;

                    let direction = match self.current_kind() {
                        Some(TokenKind::Tilde) => EdgeDirection::Undirected,
                        Some(TokenKind::RightTilde) => EdgeDirection::RightOrUndirected,
                        _ => {
                            self.diags.push(
                                Diag::error("Expected '~' or '~>' after edge filler")
                                    .with_primary_label(
                                        self.current_span_or(start),
                                        "expected edge direction terminator here",
                                    ),
                            );
                            return None;
                        }
                    };
                    self.pos += 1;

                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Full(Box::new(FullEdgePattern {
                        direction,
                        filler: FullEdgePointingFiller {
                            variable: filler.variable,
                            label_expression: filler.label_expression,
                            properties: filler.properties,
                            where_clause: filler.where_clause,
                            span: filler.span,
                        },
                        span: start..end,
                    })))
                } else {
                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Abbreviated(
                        AbbreviatedEdgePattern::Undirected { span: start..end },
                    ))
                }
            }
            Some(TokenKind::LeftTilde) => {
                self.pos += 1;
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.pos += 1;
                    let filler =
                        self.parse_element_pattern_filler(FillerTerminator::RBracket, start);
                    if !matches!(self.current_kind(), Some(TokenKind::RBracket)) {
                        self.diags.push(
                            Diag::error("Expected ']' in edge pattern").with_primary_label(
                                self.current_span_or(start),
                                "expected ']' here",
                            ),
                        );
                        return None;
                    }
                    self.pos += 1;

                    if !matches!(self.current_kind(), Some(TokenKind::Tilde)) {
                        self.diags.push(
                            Diag::error("Expected '~' after <~[ ... ]").with_primary_label(
                                self.current_span_or(start),
                                "expected '~' here",
                            ),
                        );
                        return None;
                    }
                    self.pos += 1;

                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Full(Box::new(FullEdgePattern {
                        direction: EdgeDirection::LeftOrUndirected,
                        filler: FullEdgePointingFiller {
                            variable: filler.variable,
                            label_expression: filler.label_expression,
                            properties: filler.properties,
                            where_clause: filler.where_clause,
                            span: filler.span,
                        },
                        span: start..end,
                    })))
                } else {
                    let end = self.last_consumed_end(start);
                    Some(EdgePattern::Full(Box::new(FullEdgePattern {
                        direction: EdgeDirection::LeftOrUndirected,
                        filler: FullEdgePointingFiller {
                            variable: None,
                            label_expression: None,
                            properties: None,
                            where_clause: None,
                            span: start..end,
                        },
                        span: start..end,
                    })))
                }
            }
            Some(TokenKind::Arrow) => {
                self.pos += 1;
                let end = self.last_consumed_end(start);
                Some(EdgePattern::Abbreviated(
                    AbbreviatedEdgePattern::RightArrow { span: start..end },
                ))
            }
            Some(TokenKind::RightTilde) => {
                self.pos += 1;
                let end = self.last_consumed_end(start);
                Some(EdgePattern::Full(Box::new(FullEdgePattern {
                    direction: EdgeDirection::RightOrUndirected,
                    filler: FullEdgePointingFiller {
                        variable: None,
                        label_expression: None,
                        properties: None,
                        where_clause: None,
                        span: start..end,
                    },
                    span: start..end,
                })))
            }
            _ => None,
        }
    }

    fn parse_element_pattern_filler(
        &mut self,
        terminator: FillerTerminator,
        fallback_start: usize,
    ) -> ParsedElementFiller {
        let start = self.current_start().unwrap_or(fallback_start);

        let variable = match self.current_kind() {
            Some(kind) if regular_identifier_from_kind(kind).is_some() => {
                let token = self.tokens[self.pos].clone();
                self.pos += 1;
                regular_identifier_from_kind(&token.kind).map(|name| ElementVariableDeclaration {
                    variable: name,
                    span: token.span,
                })
            }
            Some(TokenKind::DelimitedIdentifier(_)) => {
                let token = self.tokens[self.pos].clone();
                self.diags.push(
                    Diag::error("Element variable declaration requires a regular identifier")
                        .with_primary_label(
                            token.span.clone(),
                            "delimited identifiers are not allowed here",
                        ),
                );
                self.pos += 1;
                None
            }
            _ => None,
        };

        let label_expression =
            if matches!(self.current_kind(), Some(TokenKind::Is | TokenKind::Colon)) {
                self.parse_is_label_expression()
            } else {
                None
            };

        let mut properties = None;
        let mut where_clause = None;
        let mut parsed_predicate_kind: Option<TokenKind> = None;

        if matches!(self.current_kind(), Some(TokenKind::LBrace)) {
            properties = self.parse_element_property_specification();
            parsed_predicate_kind = Some(TokenKind::LBrace);
        } else if matches!(self.current_kind(), Some(TokenKind::Where)) {
            where_clause = self.parse_element_pattern_where_clause(terminator);
            parsed_predicate_kind = Some(TokenKind::Where);
        }

        // ISO grammar permits at most one elementPatternPredicate.
        if matches!(
            self.current_kind(),
            Some(TokenKind::LBrace | TokenKind::Where)
        ) {
            let next_is_property = matches!(self.current_kind(), Some(TokenKind::LBrace));
            let already_parsed_property = matches!(parsed_predicate_kind, Some(TokenKind::LBrace));
            let already_parsed_where = matches!(parsed_predicate_kind, Some(TokenKind::Where));
            if (next_is_property && already_parsed_where)
                || (!next_is_property && already_parsed_property)
            {
                self.diags.push(
                    Diag::error("Element pattern can have either property specification or WHERE predicate, not both")
                        .with_primary_label(
                            self.current_span_or(start),
                            "remove this second element predicate",
                        ),
                );
                if next_is_property {
                    let _ = self.parse_element_property_specification();
                } else {
                    let _ = self.parse_element_pattern_where_clause(terminator);
                }
            }
        }

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        ParsedElementFiller {
            variable,
            label_expression,
            properties,
            where_clause,
            span: start..end,
        }
    }

    fn parse_is_label_expression(&mut self) -> Option<LabelExpression> {
        if !matches!(self.current_kind(), Some(TokenKind::Is | TokenKind::Colon)) {
            return None;
        }
        self.pos += 1;

        let Some(expr) = self.parse_label_expression() else {
            self.diags
                .push(Diag::error("Expected label expression").with_primary_label(
                    self.current_span_or(self.pos),
                    "expected label expression here",
                ));
            return None;
        };

        Some(expr)
    }

    fn parse_label_expression(&mut self) -> Option<LabelExpression> {
        self.parse_label_disjunction()
    }

    fn parse_label_disjunction(&mut self) -> Option<LabelExpression> {
        let mut expr = self.parse_label_conjunction()?;

        while matches!(self.current_kind(), Some(TokenKind::Pipe)) {
            self.pos += 1;
            let Some(right) = self.parse_label_conjunction() else {
                self.diags.push(
                    Diag::error("Expected label expression after '|'")
                        .with_primary_label(self.current_span_or(self.pos), "expected label here"),
                );
                break;
            };

            let span = expr.span().start..right.span().end;
            expr = LabelExpression::Disjunction {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Some(expr)
    }

    fn parse_label_conjunction(&mut self) -> Option<LabelExpression> {
        let mut expr = self.parse_label_unary()?;

        while matches!(self.current_kind(), Some(TokenKind::Ampersand)) {
            self.pos += 1;
            let Some(right) = self.parse_label_unary() else {
                self.diags.push(
                    Diag::error("Expected label expression after '&'")
                        .with_primary_label(self.current_span_or(self.pos), "expected label here"),
                );
                break;
            };

            let span = expr.span().start..right.span().end;
            expr = LabelExpression::Conjunction {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Some(expr)
    }

    fn parse_label_unary(&mut self) -> Option<LabelExpression> {
        if matches!(self.current_kind(), Some(TokenKind::Bang | TokenKind::Not)) {
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
            let operand = self.parse_label_unary()?;
            let span = start..operand.span().end;
            return Some(LabelExpression::Negation {
                operand: Box::new(operand),
                span,
            });
        }

        self.parse_label_primary()
    }

    fn parse_label_primary(&mut self) -> Option<LabelExpression> {
        match self.current_kind() {
            Some(TokenKind::Percent) => {
                let span = self.current_span_or(self.pos);
                self.pos += 1;
                Some(LabelExpression::Wildcard { span })
            }
            Some(TokenKind::LParen) => {
                let start = self.current_start().unwrap_or(self.pos);
                self.pos += 1;
                let inner = self.parse_label_expression()?;
                if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
                    self.diags.push(
                        Diag::error("Expected ')' to close label expression")
                            .with_primary_label(self.current_span_or(start), "expected ')' here"),
                    );
                } else {
                    self.pos += 1;
                }
                let end = self.last_consumed_end(start);
                Some(LabelExpression::Parenthesized {
                    expression: Box::new(inner),
                    span: start..end,
                })
            }
            Some(TokenKind::Label | TokenKind::Labels) => {
                let phrase_start = self.current_start().unwrap_or(self.pos);
                self.pos += 1;

                let Some((name, span)) = self.parse_label_name() else {
                    self.diags.push(
                        Diag::error("Expected label name after LABEL/LABELS").with_primary_label(
                            self.current_span_or(phrase_start),
                            "expected label name here",
                        ),
                    );
                    return None;
                };

                let mut expr = LabelExpression::LabelName { name, span };
                while matches!(self.current_kind(), Some(TokenKind::Ampersand)) {
                    self.pos += 1;
                    let Some((next_name, next_span)) = self.parse_label_name() else {
                        self.diags.push(
                            Diag::error("Expected label name after '&'").with_primary_label(
                                self.current_span_or(phrase_start),
                                "expected label name here",
                            ),
                        );
                        break;
                    };
                    let right = LabelExpression::LabelName {
                        name: next_name,
                        span: next_span,
                    };
                    let span = expr.span().start..right.span().end;
                    expr = LabelExpression::Conjunction {
                        left: Box::new(expr),
                        right: Box::new(right),
                        span,
                    };
                }

                Some(expr)
            }
            _ => {
                let (name, span) = self.parse_label_name()?;
                Some(LabelExpression::LabelName { name, span })
            }
        }
    }

    fn parse_label_name(&mut self) -> Option<(SmolStr, std::ops::Range<usize>)> {
        let token = self.tokens.get(self.pos)?;
        let name = identifier_from_kind(&token.kind)?;
        self.pos += 1;
        Some((name, token.span.clone()))
    }

    fn parse_element_property_specification(&mut self) -> Option<ElementPropertySpecification> {
        if !matches!(self.current_kind(), Some(TokenKind::LBrace)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.pos += 1;

        let mut properties = Vec::new();

        while !matches!(
            self.current_kind(),
            Some(TokenKind::RBrace | TokenKind::Eof)
        ) {
            let Some(pair) = self.parse_property_key_value_pair() else {
                self.skip_to_token(|kind| matches!(kind, TokenKind::Comma | TokenKind::RBrace));
                if matches!(self.current_kind(), Some(TokenKind::Comma)) {
                    self.pos += 1;
                    continue;
                }
                break;
            };
            properties.push(pair);

            if matches!(self.current_kind(), Some(TokenKind::Comma)) {
                self.pos += 1;
                continue;
            }
            break;
        }

        if !matches!(self.current_kind(), Some(TokenKind::RBrace)) {
            self.diags.push(
                Diag::error("Expected '}' to close property specification")
                    .with_primary_label(self.current_span_or(start), "expected '}' here"),
            );
        } else {
            self.pos += 1;
        }

        let end = self.last_consumed_end(start);
        Some(ElementPropertySpecification {
            properties,
            span: start..end,
        })
    }

    fn parse_property_key_value_pair(&mut self) -> Option<PropertyKeyValuePair> {
        let key_start = self.current_start().unwrap_or(self.pos);
        let key = match self.parse_property_name() {
            Some(key) => key,
            None => {
                self.diags.push(
                    Diag::error("Expected property name in property specification")
                        .with_primary_label(
                            self.current_span_or(key_start),
                            "expected property name here",
                        ),
                );
                return None;
            }
        };

        if !matches!(self.current_kind(), Some(TokenKind::Colon)) {
            self.diags.push(
                Diag::error("Expected ':' after property name")
                    .with_primary_label(self.current_span_or(key_start), "expected ':' here"),
            );
            return None;
        }
        self.pos += 1;

        let expr_start = self.pos;
        let expr_end = self.find_expression_end(expr_start, |kind| {
            matches!(kind, TokenKind::Comma | TokenKind::RBrace)
        });

        let value = self.parse_expression_range(expr_start, expr_end, "property value")?;
        let span = key_start..value.span().end;

        Some(PropertyKeyValuePair { key, value, span })
    }

    fn parse_property_name(&mut self) -> Option<SmolStr> {
        let token = self.tokens.get(self.pos)?;
        let key = match &token.kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::StringLiteral(name) => name.clone(),
            kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
            _ => return None,
        };

        self.pos += 1;
        Some(key)
    }

    fn parse_element_pattern_where_clause(
        &mut self,
        terminator: FillerTerminator,
    ) -> Option<ElementPatternPredicate> {
        if !matches!(self.current_kind(), Some(TokenKind::Where)) {
            return None;
        }

        let start = self.current_start().unwrap_or(self.pos);
        self.pos += 1;

        let expr_start = self.pos;
        let expr_end = self.find_expression_end(expr_start, |kind| terminator.matches(kind));
        let condition =
            self.parse_expression_range(expr_start, expr_end, "condition after WHERE")?;
        let end = condition.span().end;

        Some(ElementPatternPredicate {
            condition,
            span: start..end,
        })
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
        self.pos += 1;

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
        match (
            self.current_kind(),
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
        ) {
            (Some(TokenKind::LeftArrow), Some(TokenKind::Slash)) => {
                self.pos += 2;
                Some(SimplifiedOpening::LeftArrow)
            }
            (Some(TokenKind::LeftTilde), Some(TokenKind::Slash)) => {
                self.pos += 2;
                Some(SimplifiedOpening::LeftOrUndirected)
            }
            (Some(TokenKind::Tilde), Some(TokenKind::Slash)) => {
                self.pos += 2;
                Some(SimplifiedOpening::Undirected)
            }
            (Some(TokenKind::Minus), Some(TokenKind::Slash)) => {
                self.pos += 2;
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
        self.pos += 1;
        Some(closing)
    }

    fn parse_simplified_contents(&mut self) -> Option<SimplifiedPathPatternExpression> {
        let first = self.parse_simplified_term()?;

        if self.is_multiset_alternation_operator() {
            let mut alternatives = vec![first];

            while self.is_multiset_alternation_operator() {
                self.pos += 3;
                let Some(term) = self.parse_simplified_term() else {
                    self.diags.push(
                        Diag::error("Expected simplified term after '|+|'").with_primary_label(
                            self.current_span_or(self.pos),
                            "expected term here",
                        ),
                    );
                    break;
                };
                alternatives.push(term);
            }

            let start = simplified_expression_span(alternatives.first()?).start;
            let end = simplified_expression_span(alternatives.last()?).end;
            return Some(SimplifiedPathPatternExpression::MultisetAlternation(
                SimplifiedMultisetAlternation {
                    alternatives,
                    span: start..end,
                },
            ));
        }

        if matches!(self.current_kind(), Some(TokenKind::Pipe)) {
            let mut expr = first;

            while matches!(self.current_kind(), Some(TokenKind::Pipe)) {
                self.pos += 1;
                let Some(right) = self.parse_simplified_term() else {
                    self.diags.push(
                        Diag::error("Expected simplified term after '|'").with_primary_label(
                            self.current_span_or(self.pos),
                            "expected term here",
                        ),
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

            return Some(expr);
        }

        Some(first)
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

            let checkpoint = self.checkpoint();
            let Some(part) = self.parse_simplified_factor_low() else {
                self.restore(checkpoint);
                break;
            };
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
            self.pos += 1;
            let Some(right) = self.parse_simplified_factor_high() else {
                self.diags.push(
                    Diag::error("Expected simplified factor after '&'")
                        .with_primary_label(self.current_span_or(self.pos), "expected factor here"),
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
            self.pos += 1;
            let end = self.last_consumed_end(start);
            return Some(SimplifiedPathPatternExpression::Questioned(
                SimplifiedQuestioned {
                    pattern: Box::new(tertiary),
                    span: start..end,
                },
            ));
        }

        if let Some(quantifier) = self.parse_graph_pattern_quantifier() {
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
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
            let inner = self.parse_simplified_secondary()?;
            let direction = if matches!(self.current_kind(), Some(TokenKind::Gt)) {
                self.pos += 1;
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
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
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

        if matches!(self.current_kind(), Some(TokenKind::Tilde))
            && !matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Slash)
            )
        {
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
            let inner = self.parse_simplified_secondary()?;
            let direction = if matches!(self.current_kind(), Some(TokenKind::Gt)) {
                self.pos += 1;
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
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Slash)
            )
        {
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
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
            self.pos += 1;
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
            let start = self.current_start().unwrap_or(self.pos);
            self.pos += 1;
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
            self.pos += 1;
            let inner = self.parse_simplified_contents()?;
            if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
                self.diags.push(
                    Diag::error("Expected ')' to close simplified subexpression")
                        .with_primary_label(self.current_span_or(self.pos), "expected ')' here"),
                );
            } else {
                self.pos += 1;
            }
            return Some(inner);
        }

        let token = self.tokens.get(self.pos)?;
        let label = match &token.kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => name.clone(),
            kind if kind.is_keyword() => SmolStr::new(kind.to_string()),
            _ => return None,
        };
        let span = token.span.clone();
        self.pos += 1;

        Some(SimplifiedPathPatternExpression::Contents(
            SimplifiedContents {
                labels: vec![label],
                span,
            },
        ))
    }

    fn parse_graph_pattern_quantifier(&mut self) -> Option<GraphPatternQuantifier> {
        match self.current_kind() {
            Some(TokenKind::Star) => {
                let span = self.current_span_or(self.pos);
                self.pos += 1;
                Some(GraphPatternQuantifier::Star { span })
            }
            Some(TokenKind::Plus) => {
                let span = self.current_span_or(self.pos);
                self.pos += 1;
                Some(GraphPatternQuantifier::Plus { span })
            }
            Some(TokenKind::Question) => {
                let span = self.current_span_or(self.pos);
                self.pos += 1;
                Some(GraphPatternQuantifier::QuestionMark { span })
            }
            Some(TokenKind::LBrace) => self.parse_brace_quantifier(),
            _ => None,
        }
    }

    fn parse_brace_quantifier(&mut self) -> Option<GraphPatternQuantifier> {
        let start = self.current_start().unwrap_or(self.pos);
        self.pos += 1;

        let lower = self.parse_u32_bound();

        if matches!(self.current_kind(), Some(TokenKind::RBrace)) {
            self.pos += 1;
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
                self.pos += 1;
            }
            return None;
        }

        self.pos += 1;
        let upper = self.parse_u32_bound();

        if !matches!(self.current_kind(), Some(TokenKind::RBrace)) {
            self.diags.push(
                Diag::error("Expected '}' to close quantifier")
                    .with_primary_label(self.current_span_or(start), "expected '}' here"),
            );
            self.skip_to_token(|kind| matches!(kind, TokenKind::RBrace));
            if matches!(self.current_kind(), Some(TokenKind::RBrace)) {
                self.pos += 1;
            }
            return None;
        }

        self.pos += 1;
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
        let token = self.tokens.get(self.pos)?;

        let TokenKind::IntegerLiteral(raw) = &token.kind else {
            return None;
        };

        self.pos += 1;
        match parse_integer_literal_to_u32(raw) {
            Some(value) => Some(value),
            None => {
                self.diags.push(
                    Diag::error("Expected non-negative integer bound")
                        .with_primary_label(token.span.clone(), "invalid integer literal"),
                );
                None
            }
        }
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

    fn looks_like_simplified_opening(&self) -> bool {
        matches!(
            (
                self.current_kind(),
                self.tokens.get(self.pos + 1).map(|t| &t.kind)
            ),
            (Some(TokenKind::LeftArrow), Some(TokenKind::Slash))
                | (Some(TokenKind::LeftTilde), Some(TokenKind::Slash))
                | (Some(TokenKind::Tilde), Some(TokenKind::Slash))
                | (Some(TokenKind::Minus), Some(TokenKind::Slash))
        )
    }

    fn is_multiset_alternation_operator(&self) -> bool {
        matches!(self.current_kind(), Some(TokenKind::Pipe))
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Plus)
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::Pipe)
            )
    }

    fn skip_to_where_or_statement_boundary(&mut self) {
        self.skip_to_token(|kind| matches!(kind, TokenKind::Where) || is_query_boundary(kind));
    }

    fn skip_to_statement_boundary(&mut self) {
        self.skip_to_token(is_query_boundary);
    }

    fn skip_to_path_pattern_boundary(&mut self) {
        self.skip_to_token(is_path_pattern_delimiter);
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

fn regular_identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}

fn identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}

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

fn simplified_expression_span(expr: &SimplifiedPathPatternExpression) -> std::ops::Range<usize> {
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

fn edge_pattern_span(edge: &EdgePattern) -> std::ops::Range<usize> {
    match edge {
        EdgePattern::Full(full) => full.span.clone(),
        EdgePattern::Abbreviated(abbrev) => abbrev.span().clone(),
    }
}

fn opening_closing_direction(
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

fn is_path_pattern_delimiter(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comma | TokenKind::Keep | TokenKind::Where | TokenKind::Yield
    ) || is_query_boundary(kind)
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
        || matches!(kind, TokenKind::Identifier(name) if name.eq_ignore_ascii_case("ELEMENTS"))
}

fn is_bindings_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Binding)
        || matches!(kind, TokenKind::Identifier(name) if name.eq_ignore_ascii_case("BINDINGS"))
}

fn is_edge_keyword(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Edge | TokenKind::Relationship)
        || matches!(kind, TokenKind::Identifier(name) if name.eq_ignore_ascii_case("EDGE") || name.eq_ignore_ascii_case("EDGES"))
}
