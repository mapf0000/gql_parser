//! Node and edge pattern parsing for GQL.

use crate::ast::query::*;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use smol_str::SmolStr;

use super::{FillerTerminator, ParsedElementFiller, PatternParser};

impl<'a> PatternParser<'a> {
    pub(super) fn parse_node_pattern(&mut self) -> Option<NodePattern> {
        if !matches!(self.current_kind(), Some(TokenKind::LParen)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.stream.advance();

        let filler = self.parse_element_pattern_filler(FillerTerminator::RParen, start);

        if !matches!(self.current_kind(), Some(TokenKind::RParen)) {
            self.diags.push(
                Diag::error("Expected ')' to close node pattern")
                    .with_primary_label(self.current_span_or(start), "expected ')' here"),
            );
            self.skip_to_token(|kind| matches!(kind, TokenKind::RParen));
            if matches!(self.current_kind(), Some(TokenKind::RParen)) {
                self.stream.advance();
            }
        } else {
            self.stream.advance();
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

    pub(super) fn parse_edge_pattern(&mut self) -> Option<EdgePattern> {
        let start = self.current_start()?;

        match self.current_kind() {
            Some(TokenKind::LeftArrow) => {
                self.stream.advance();
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.stream.advance();
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
                    self.stream.advance();

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
                    self.stream.advance();

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
                self.stream.advance();
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.stream.advance();
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
                    self.stream.advance();

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
                    self.stream.advance();

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
                self.stream.advance();
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.stream.advance();
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
                    self.stream.advance();

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
                    self.stream.advance();

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
                self.stream.advance();
                if matches!(self.current_kind(), Some(TokenKind::LBracket)) {
                    self.stream.advance();
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
                    self.stream.advance();

                    if !matches!(self.current_kind(), Some(TokenKind::Tilde)) {
                        self.diags.push(
                            Diag::error("Expected '~' after <~[ ... ]").with_primary_label(
                                self.current_span_or(start),
                                "expected '~' here",
                            ),
                        );
                        return None;
                    }
                    self.stream.advance();

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
                self.stream.advance();
                let end = self.last_consumed_end(start);
                Some(EdgePattern::Abbreviated(
                    AbbreviatedEdgePattern::RightArrow { span: start..end },
                ))
            }
            Some(TokenKind::RightTilde) => {
                self.stream.advance();
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

    pub(super) fn parse_element_pattern_filler(
        &mut self,
        terminator: FillerTerminator,
        fallback_start: usize,
    ) -> ParsedElementFiller {
        let start = self.current_start().unwrap_or(fallback_start);

        let variable = match self.current_kind() {
            Some(kind) if regular_identifier_from_kind(kind).is_some() => {
                let token = self.stream.current().clone();
                self.stream.advance();
                regular_identifier_from_kind(&token.kind).map(|name| ElementVariableDeclaration {
                    variable: name,
                    span: token.span,
                })
            }
            Some(TokenKind::DelimitedIdentifier(_)) => {
                let token = self.stream.current().clone();
                self.diags.push(
                    Diag::error("Element variable declaration requires a regular identifier")
                        .with_primary_label(
                            token.span.clone(),
                            "delimited identifiers are not allowed here",
                        ),
                );
                self.stream.advance();
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
            .stream
            .tokens()
            .get(self.stream.position().saturating_sub(1))
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

    fn parse_element_property_specification(&mut self) -> Option<ElementPropertySpecification> {
        if !matches!(self.current_kind(), Some(TokenKind::LBrace)) {
            return None;
        }

        let start = self.current_start().unwrap_or(0);
        self.stream.advance();

        let mut properties = Vec::new();

        while !matches!(
            self.current_kind(),
            Some(TokenKind::RBrace | TokenKind::Eof)
        ) {
            let Some(pair) = self.parse_property_key_value_pair() else {
                self.skip_to_token(|kind| matches!(kind, TokenKind::Comma | TokenKind::RBrace));
                if matches!(self.current_kind(), Some(TokenKind::Comma)) {
                    self.stream.advance();
                    continue;
                }
                break;
            };
            properties.push(pair);

            if matches!(self.current_kind(), Some(TokenKind::Comma)) {
                self.stream.advance();
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
            self.stream.advance();
        }

        let end = self.last_consumed_end(start);
        Some(ElementPropertySpecification {
            properties,
            span: start..end,
        })
    }

    fn parse_property_key_value_pair(&mut self) -> Option<PropertyKeyValuePair> {
        let key_start = self.current_start().unwrap_or(self.stream.position());
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
        self.stream.advance();

        let expr_start = self.stream.position();
        let expr_end = self.find_expression_end(expr_start, |kind| {
            matches!(kind, TokenKind::Comma | TokenKind::RBrace)
        });

        let value = self.parse_expression_range(expr_start, expr_end, "property value")?;
        let span = key_start..value.span().end;

        Some(PropertyKeyValuePair { key, value, span })
    }

    fn parse_property_name(&mut self) -> Option<SmolStr> {
        let token = self.stream.current();
        let key = match &token.kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::StringLiteral(name) => name.clone(),
            kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
            _ => return None,
        };

        self.stream.advance();
        Some(key)
    }

    fn parse_element_pattern_where_clause(
        &mut self,
        terminator: FillerTerminator,
    ) -> Option<ElementPatternPredicate> {
        if !matches!(self.current_kind(), Some(TokenKind::Where)) {
            return None;
        }

        let start = self.current_start().unwrap_or(self.stream.position());
        self.stream.advance();

        let expr_start = self.stream.position();
        let expr_end = self.find_expression_end(expr_start, |kind| terminator.matches(kind));
        let condition =
            self.parse_expression_range(expr_start, expr_end, "condition after WHERE")?;
        let end = condition.span().end;

        Some(ElementPatternPredicate {
            condition,
            span: start..end,
        })
    }
}

pub(super) fn edge_pattern_span(edge: &EdgePattern) -> std::ops::Range<usize> {
    match edge {
        EdgePattern::Full(full) => full.span.clone(),
        EdgePattern::Abbreviated(abbrev) => abbrev.span().clone(),
    }
}

fn regular_identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}
