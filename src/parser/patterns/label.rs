//! Label expression parsing for GQL.

use crate::ast::query::LabelExpression;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use smol_str::SmolStr;

use super::PatternParser;

impl<'a> PatternParser<'a> {
    pub(super) fn parse_is_label_expression(&mut self) -> Option<LabelExpression> {
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

    pub(super) fn parse_label_expression(&mut self) -> Option<LabelExpression> {
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
}

fn identifier_from_kind(kind: &TokenKind) -> Option<SmolStr> {
    match kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => Some(name.clone()),
        kind if kind.is_non_reserved_identifier_keyword() => Some(SmolStr::new(kind.to_string())),
        _ => None,
    }
}
