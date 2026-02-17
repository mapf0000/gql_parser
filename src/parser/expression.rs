//! Expression parsing for GQL.
//!
//! This module implements expression parsing with precedence handling,
//! literal/function/predicate support, and structured diagnostics.

use crate::ast::{
    BinaryOperator, BooleanValue, CaseExpression, CastExpression, ComparisonOperator,
    ExistsExpression, ExistsVariant, Expression, FunctionCall, FunctionName,
    GraphPatternPlaceholder, LabelExpression, Literal, LogicalOperator, Predicate, RecordField,
    SearchedCaseExpression, SearchedWhenClause, SimpleCaseExpression, SimpleWhenClause, Span,
    TrimSpecification, TruthValue, TypeReference, UnaryOperator,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use smol_str::SmolStr;

type ParseError = Box<Diag>;
type ParseResult<T> = Result<T, ParseError>;

/// Parser for expressions.
pub struct ExpressionParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> ExpressionParser<'a> {
    /// Creates a new expression parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parses an expression using standard precedence rules.
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_or_expression()
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream must be non-empty"))
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len().saturating_sub(1) {
            self.pos += 1;
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }

    fn consume(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<Span> {
        if self.check(&kind) {
            let span = self.current().span.clone();
            self.advance();
            Ok(span)
        } else {
            Err(self.error_here(format!("expected {kind}, found {}", self.current().kind)))
        }
    }

    fn error_here(&self, message: impl Into<String>) -> ParseError {
        Box::new(
            Diag::error(message.into()).with_primary_label(self.current().span.clone(), "here"),
        )
    }

    fn parse_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_xor_expression()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_xor_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::Or, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_xor_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.check(&TokenKind::Xor) {
            self.advance();
            let right = self.parse_and_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::Xor, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_not_expression()?;

        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_not_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::And, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> ParseResult<Expression> {
        if self.check(&TokenKind::Not) {
            let start = self.current().span.start;
            self.advance();
            let operand = self.parse_not_expression()?;
            let span = start..operand.span().end;
            Ok(Expression::Unary(
                UnaryOperator::Not,
                Box::new(operand),
                span,
            ))
        } else {
            self.parse_is_expression()
        }
    }

    fn parse_is_expression(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_comparison_expression()?;

        while self.check(&TokenKind::Is) {
            self.advance();

            let negated = if self.check(&TokenKind::Not) {
                self.advance();
                true
            } else {
                false
            };

            expr = match &self.current().kind {
                TokenKind::Null => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsNull(Box::new(expr), negated, span))
                }
                TokenKind::Typed => {
                    self.advance();
                    let type_ref = self.parse_type_reference()?;
                    let span = expr.span().start..type_ref.span.end;
                    Expression::Predicate(Predicate::IsTyped(
                        Box::new(expr),
                        type_ref,
                        negated,
                        span,
                    ))
                }
                TokenKind::Normalized => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsNormalized(Box::new(expr), negated, span))
                }
                TokenKind::Directed => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsDirected(Box::new(expr), negated, span))
                }
                TokenKind::Labeled => {
                    let labeled_span = self.current().span.clone();
                    self.advance();
                    let label = if self.check(&TokenKind::Colon) {
                        Some(self.parse_label_expression()?)
                    } else {
                        None
                    };
                    let end = label
                        .as_ref()
                        .map_or(labeled_span.end, |label_expr| label_expr.span.end);
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsLabeled(
                        Box::new(expr),
                        label,
                        negated,
                        span,
                    ))
                }
                TokenKind::True => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::True,
                        negated,
                        span,
                    ))
                }
                TokenKind::False => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::False,
                        negated,
                        span,
                    ))
                }
                TokenKind::Unknown => {
                    let end = self.current().span.end;
                    self.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::Unknown,
                        negated,
                        span,
                    ))
                }
                TokenKind::Source => {
                    self.advance();
                    self.expect(TokenKind::Of)?;
                    let edge_expr = self.parse_comparison_expression()?;
                    let span = expr.span().start..edge_expr.span().end;
                    Expression::Predicate(Predicate::IsSource(
                        Box::new(expr),
                        Box::new(edge_expr),
                        negated,
                        span,
                    ))
                }
                TokenKind::Destination => {
                    self.advance();
                    self.expect(TokenKind::Of)?;
                    let edge_expr = self.parse_comparison_expression()?;
                    let span = expr.span().start..edge_expr.span().end;
                    Expression::Predicate(Predicate::IsDestination(
                        Box::new(expr),
                        Box::new(edge_expr),
                        negated,
                        span,
                    ))
                }
                _ => {
                    return Err(self.error_here(format!(
                        "expected IS predicate, found {}",
                        self.current().kind
                    )));
                }
            };
        }

        Ok(expr)
    }

    fn parse_comparison_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_concatenation_expression()?;

        if let Some(op) = self.consume_comparison_operator() {
            let right = self.parse_concatenation_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Comparison(op, Box::new(left), Box::new(right), span);

            if self.is_comparison_operator() {
                return Err(self.error_here(
                    "chained comparison operators are not allowed without parentheses",
                ));
            }
        }

        Ok(left)
    }

    fn parse_concatenation_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive_expression()?;

        while self.check(&TokenKind::DoublePipe) {
            self.advance();
            let right = self.parse_additive_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Binary(
                BinaryOperator::Concatenate,
                Box::new(left),
                Box::new(right),
                span,
            );
        }

        Ok(left)
    }

    fn parse_additive_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_multiplicative_expression()?;

        loop {
            let op = match &self.current().kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Binary(op, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_unary_expression()?;

        loop {
            let op = match &self.current().kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Binary(op, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> ParseResult<Expression> {
        match &self.current().kind {
            TokenKind::Plus => {
                let start = self.current().span.start;
                self.advance();
                let operand = self.parse_unary_expression()?;
                let span = start..operand.span().end;
                Ok(Expression::Unary(
                    UnaryOperator::Plus,
                    Box::new(operand),
                    span,
                ))
            }
            TokenKind::Minus => {
                let start = self.current().span.start;
                self.advance();
                let operand = self.parse_unary_expression()?;
                let span = start..operand.span().end;
                Ok(Expression::Unary(
                    UnaryOperator::Minus,
                    Box::new(operand),
                    span,
                ))
            }
            _ => self.parse_postfix_expression(),
        }
    }

    fn parse_postfix_expression(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary_expression()?;

        while matches!(self.current().kind, TokenKind::Dot) {
            self.advance();
            let name = match &self.current().kind {
                TokenKind::Identifier(n) | TokenKind::DelimitedIdentifier(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => {
                    return Err(self.error_here(format!(
                        "expected property name, found {}",
                        self.current().kind
                    )));
                }
            };
            let span = expr.span().start..self.tokens[self.pos - 1].span.end;
            expr = Expression::PropertyReference(Box::new(expr), name, span);
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
        match &self.current().kind {
            TokenKind::True
            | TokenKind::False
            | TokenKind::Unknown
            | TokenKind::Null
            | TokenKind::StringLiteral(_)
            | TokenKind::ByteStringLiteral(_)
            | TokenKind::IntegerLiteral(_)
            | TokenKind::FloatLiteral(_) => self.parse_literal_expression(),

            TokenKind::Date
            | TokenKind::Time
            | TokenKind::Timestamp
            | TokenKind::Datetime
            | TokenKind::Duration
                if self
                    .peek()
                    .map(|token| &token.kind)
                    .is_some_and(|kind| matches!(kind, TokenKind::StringLiteral(_))) =>
            {
                self.parse_temporal_literal_expression()
            }

            TokenKind::LBracket => self.parse_list_literal(),
            TokenKind::LBrace => self.parse_record_literal(),

            TokenKind::Case => {
                let case_expr = self.parse_case_expression()?;
                Ok(Expression::Case(case_expr))
            }

            TokenKind::Cast => {
                let cast_expr = self.parse_cast_expression()?;
                Ok(Expression::Cast(cast_expr))
            }

            TokenKind::Exists => {
                let exists_expr = self.parse_exists_expression()?;
                Ok(Expression::Exists(exists_expr))
            }

            TokenKind::AllDifferent => self.parse_all_different_predicate(),
            TokenKind::Same => self.parse_same_predicate(),
            TokenKind::PropertyExists => self.parse_property_exists_predicate(),

            TokenKind::Path if self.peek().map(|t| &t.kind) == Some(&TokenKind::LBracket) => {
                self.parse_path_constructor()
            }

            TokenKind::Record if self.peek().map(|t| &t.kind) == Some(&TokenKind::LBrace) => {
                self.parse_record_constructor()
            }

            TokenKind::Property if self.peek().map(|t| &t.kind) == Some(&TokenKind::Graph) => {
                self.parse_property_graph_expression()
            }

            TokenKind::Binding if self.peek().map(|t| &t.kind) == Some(&TokenKind::Table) => {
                self.parse_binding_table_expression()
            }

            TokenKind::Value => self.parse_value_subquery_expression(),

            TokenKind::LParen => {
                let start = self.current().span.start;
                self.advance();
                let expr = self.parse_expression()?;
                let end = self.expect(TokenKind::RParen)?.end;
                let span = start..end;
                Ok(Expression::Parenthesized(Box::new(expr), span))
            }

            TokenKind::Parameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(Expression::ParameterReference(name, span))
            }

            TokenKind::Identifier(_) | TokenKind::DelimitedIdentifier(_) => {
                if self.peek().map(|t| &t.kind) == Some(&TokenKind::LParen) {
                    let func_call = self.parse_function_call()?;
                    Ok(Expression::FunctionCall(func_call))
                } else {
                    let name = match &self.current().kind {
                        TokenKind::Identifier(n) | TokenKind::DelimitedIdentifier(n) => n.clone(),
                        _ => SmolStr::new(self.current().kind.to_string()),
                    };
                    let span = self.current().span.clone();
                    self.advance();
                    Ok(Expression::VariableReference(name, span))
                }
            }

            _ if self.current().kind.is_keyword() => {
                if self.peek().map(|t| &t.kind) == Some(&TokenKind::LParen) {
                    let func_call = self.parse_function_call()?;
                    Ok(Expression::FunctionCall(func_call))
                } else {
                    let name = SmolStr::new(self.current().kind.to_string());
                    let span = self.current().span.clone();
                    self.advance();
                    Ok(Expression::VariableReference(name, span))
                }
            }

            _ => Err(self.error_here(format!(
                "expected expression, found {}",
                self.current().kind
            ))),
        }
    }

    fn parse_literal_expression(&mut self) -> ParseResult<Expression> {
        let (literal, span) = self.parse_literal()?;
        Ok(Expression::Literal(literal, span))
    }

    fn parse_literal(&mut self) -> ParseResult<(Literal, Span)> {
        let token = self.current();
        let span = token.span.clone();

        let literal = match &token.kind {
            TokenKind::True => {
                self.advance();
                Literal::Boolean(BooleanValue::True)
            }
            TokenKind::False => {
                self.advance();
                Literal::Boolean(BooleanValue::False)
            }
            TokenKind::Unknown => {
                self.advance();
                Literal::Boolean(BooleanValue::Unknown)
            }
            TokenKind::Null => {
                self.advance();
                Literal::Null
            }
            TokenKind::IntegerLiteral(value) => {
                let value = value.clone();
                self.advance();
                Literal::Integer(value)
            }
            TokenKind::FloatLiteral(value) => {
                let value = value.clone();
                self.advance();
                Literal::Float(value)
            }
            TokenKind::StringLiteral(value) => {
                let value = value.clone();
                self.advance();
                Literal::String(value)
            }
            TokenKind::ByteStringLiteral(value) => {
                let value = value.clone();
                self.advance();
                Literal::ByteString(value)
            }
            _ => {
                return Err(self.error_here(format!("expected literal, found {}", token.kind)));
            }
        };

        Ok((literal, span))
    }

    fn parse_temporal_literal_expression(&mut self) -> ParseResult<Expression> {
        let (literal, span) = self.parse_temporal_literal()?;
        Ok(Expression::Literal(literal, span))
    }

    fn parse_temporal_literal(&mut self) -> ParseResult<(Literal, Span)> {
        let keyword_span = self.current().span.clone();
        let kind = self.current().kind.clone();

        match kind {
            TokenKind::Date
            | TokenKind::Time
            | TokenKind::Timestamp
            | TokenKind::Datetime
            | TokenKind::Duration => {}
            _ => return Err(self.error_here("expected temporal literal keyword")),
        }

        self.advance();

        let value = match &self.current().kind {
            TokenKind::StringLiteral(value) => value.clone(),
            _ => {
                return Err(self.error_here(format!(
                    "expected string literal after {}, found {}",
                    kind,
                    self.current().kind
                )));
            }
        };

        let value_span = self.current().span.clone();
        self.advance();

        let literal = match kind {
            TokenKind::Date => Literal::Date(value),
            TokenKind::Time => Literal::Time(value),
            TokenKind::Timestamp | TokenKind::Datetime => Literal::Datetime(value),
            TokenKind::Duration => Literal::Duration(value),
            _ => unreachable!(),
        };

        Ok((literal, keyword_span.start..value_span.end))
    }

    fn parse_list_literal(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::LBracket)?.start;
        let mut elements = Vec::new();

        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            elements.push(self.parse_expression()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RBracket) {
                break;
            }
        }

        let end = self.expect(TokenKind::RBracket)?.end;
        let span = start..end;
        Ok(Expression::Literal(Literal::List(elements), span))
    }

    fn parse_record_literal(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::LBrace)?.start;
        let mut fields = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            fields.push(self.parse_record_field()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RBrace) {
                break;
            }
        }

        let end = self.expect(TokenKind::RBrace)?.end;
        let span = start..end;
        Ok(Expression::Literal(Literal::Record(fields), span))
    }

    fn parse_record_constructor(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Record)?.start;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            fields.push(self.parse_record_field()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RBrace) {
                break;
            }
        }

        let end = self.expect(TokenKind::RBrace)?.end;
        let span = start..end;
        Ok(Expression::RecordConstructor(fields, span))
    }

    fn parse_record_field(&mut self) -> ParseResult<RecordField> {
        let field_start = self.current().span.start;
        let name = match &self.current().kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::StringLiteral(name) => {
                let field_name = name.clone();
                self.advance();
                field_name
            }
            _ => {
                return Err(self.error_here(format!(
                    "expected record field name, found {}",
                    self.current().kind
                )));
            }
        };

        self.expect(TokenKind::Colon)?;
        let value = self.parse_expression()?;
        let span = field_start..value.span().end;

        Ok(RecordField { name, value, span })
    }

    fn parse_path_constructor(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Path)?.start;
        self.expect(TokenKind::LBracket)?;

        let mut elements = Vec::new();
        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            elements.push(self.parse_expression()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RBracket) {
                break;
            }
        }

        let end = self.expect(TokenKind::RBracket)?.end;
        Ok(Expression::PathConstructor(elements, start..end))
    }

    fn parse_property_graph_expression(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Property)?.start;
        self.expect(TokenKind::Graph)?;
        let graph_expr = self.parse_unary_expression()?;
        let span = start..graph_expr.span().end;
        Ok(Expression::GraphExpression(Box::new(graph_expr), span))
    }

    fn parse_binding_table_expression(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Binding)?.start;
        self.expect(TokenKind::Table)?;
        let table_expr = self.parse_unary_expression()?;
        let span = start..table_expr.span().end;
        Ok(Expression::BindingTableExpression(
            Box::new(table_expr),
            span,
        ))
    }

    fn parse_value_subquery_expression(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Value)?.start;
        let inner = if self.check(&TokenKind::LParen) {
            self.advance();
            let nested = self.parse_expression()?;
            self.expect(TokenKind::RParen)?;
            nested
        } else {
            self.parse_primary_expression()?
        };

        let span = start..inner.span().end;
        Ok(Expression::SubqueryExpression(Box::new(inner), span))
    }

    fn parse_function_call(&mut self) -> ParseResult<FunctionCall> {
        let start = self.current().span.start;
        let name = self.parse_function_name()?;

        self.expect(TokenKind::LParen)?;
        let mut arguments = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            arguments.push(self.parse_expression()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RParen) {
                break;
            }
        }

        let end = self.expect(TokenKind::RParen)?.end;
        Ok(FunctionCall {
            name,
            arguments,
            span: start..end,
        })
    }

    fn parse_function_name(&mut self) -> ParseResult<FunctionName> {
        let function = match &self.current().kind {
            TokenKind::Abs => FunctionName::Abs,
            TokenKind::Mod => FunctionName::Mod,
            TokenKind::Floor => FunctionName::Floor,
            TokenKind::Ceil => FunctionName::Ceil,
            TokenKind::Sqrt => FunctionName::Sqrt,
            TokenKind::Power => FunctionName::Power,
            TokenKind::Exp => FunctionName::Exp,
            TokenKind::Ln => FunctionName::Ln,
            TokenKind::Log => FunctionName::Log,

            TokenKind::Sin => FunctionName::Sin,
            TokenKind::Cos => FunctionName::Cos,
            TokenKind::Tan => FunctionName::Tan,
            TokenKind::Asin => FunctionName::Asin,
            TokenKind::Acos => FunctionName::Acos,
            TokenKind::Atan => FunctionName::Atan,

            TokenKind::Upper => FunctionName::Upper,
            TokenKind::Lower => FunctionName::Lower,
            TokenKind::Trim => FunctionName::Trim(TrimSpecification::Both),
            TokenKind::Substring => FunctionName::Substring,
            TokenKind::Normalize => FunctionName::Normalize,

            TokenKind::Date => FunctionName::Date,
            TokenKind::Time => FunctionName::Time,
            TokenKind::Datetime | TokenKind::Timestamp => FunctionName::Datetime,
            TokenKind::Duration => FunctionName::Duration,

            TokenKind::Coalesce => FunctionName::Coalesce,
            TokenKind::Nullif => FunctionName::NullIf,

            TokenKind::Cardinality => FunctionName::Cardinality,
            TokenKind::Size => FunctionName::Size,
            TokenKind::Elements => FunctionName::Elements,

            _ => {
                let Some(name) = self.token_name_for_function() else {
                    return Err(self.error_here(format!(
                        "expected function name, found {}",
                        self.current().kind
                    )));
                };
                self.classify_function_name(name)
            }
        };

        self.advance();
        Ok(function)
    }

    fn token_name_for_function(&self) -> Option<SmolStr> {
        match &self.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                Some(name.clone())
            }
            kind if kind.is_keyword() => Some(SmolStr::new(kind.to_string())),
            _ => None,
        }
    }

    fn classify_function_name(&self, raw_name: SmolStr) -> FunctionName {
        let upper = raw_name.to_ascii_uppercase();
        match upper.as_str() {
            "ABS" => FunctionName::Abs,
            "MOD" => FunctionName::Mod,
            "FLOOR" => FunctionName::Floor,
            "CEIL" => FunctionName::Ceil,
            "SQRT" => FunctionName::Sqrt,
            "POWER" => FunctionName::Power,
            "EXP" => FunctionName::Exp,
            "LN" => FunctionName::Ln,
            "LOG" => FunctionName::Log,
            "LOG10" => FunctionName::Log10,

            "SIN" => FunctionName::Sin,
            "COS" => FunctionName::Cos,
            "TAN" => FunctionName::Tan,
            "COT" => FunctionName::Cot,
            "SINH" => FunctionName::Sinh,
            "COSH" => FunctionName::Cosh,
            "TANH" => FunctionName::Tanh,
            "ASIN" => FunctionName::Asin,
            "ACOS" => FunctionName::Acos,
            "ATAN" => FunctionName::Atan,
            "ATAN2" => FunctionName::Atan2,
            "DEGREES" => FunctionName::Degrees,
            "RADIANS" => FunctionName::Radians,

            "UPPER" => FunctionName::Upper,
            "LOWER" => FunctionName::Lower,
            "TRIM" => FunctionName::Trim(TrimSpecification::Both),
            "BTRIM" => FunctionName::BTrim,
            "LTRIM" => FunctionName::LTrim,
            "RTRIM" => FunctionName::RTrim,
            "LEFT" => FunctionName::Left,
            "RIGHT" => FunctionName::Right,
            "NORMALIZE" => FunctionName::Normalize,
            "CHAR_LENGTH" => FunctionName::CharLength,
            "BYTE_LENGTH" => FunctionName::ByteLength,
            "SUBSTRING" => FunctionName::Substring,

            "CURRENT_DATE" => FunctionName::CurrentDate,
            "CURRENT_TIME" => FunctionName::CurrentTime,
            "CURRENT_TIMESTAMP" => FunctionName::CurrentTimestamp,
            "DATE" => FunctionName::Date,
            "TIME" => FunctionName::Time,
            "DATETIME" | "TIMESTAMP" => FunctionName::Datetime,
            "ZONED_TIME" => FunctionName::ZonedTime,
            "ZONED_DATETIME" => FunctionName::ZonedDatetime,
            "LOCAL_TIME" => FunctionName::LocalTime,
            "LOCAL_DATETIME" => FunctionName::LocalDatetime,
            "DURATION" => FunctionName::Duration,
            "DURATION_BETWEEN" => FunctionName::DurationBetween,

            "ELEMENTS" => FunctionName::Elements,
            "CARDINALITY" => FunctionName::Cardinality,
            "SIZE" => FunctionName::Size,
            "PATH_LENGTH" => FunctionName::PathLength,
            "ELEMENT_ID" => FunctionName::ElementId,

            "COALESCE" => FunctionName::Coalesce,
            "NULLIF" => FunctionName::NullIf,

            _ => FunctionName::Custom(raw_name),
        }
    }

    fn parse_case_expression(&mut self) -> ParseResult<CaseExpression> {
        let start = self.expect(TokenKind::Case)?.start;

        if self.check(&TokenKind::When) {
            self.parse_searched_case(start)
        } else {
            self.parse_simple_case(start)
        }
    }

    fn parse_simple_case(&mut self, start: usize) -> ParseResult<CaseExpression> {
        let operand = Box::new(self.parse_expression()?);
        let mut when_clauses = Vec::new();

        while self.check(&TokenKind::When) {
            when_clauses.push(self.parse_simple_when_clause()?);
        }

        let else_clause = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.expect(TokenKind::End)?.end;

        Ok(CaseExpression::Simple(SimpleCaseExpression {
            operand,
            when_clauses,
            else_clause,
            span: start..end,
        }))
    }

    fn parse_searched_case(&mut self, start: usize) -> ParseResult<CaseExpression> {
        let mut when_clauses = Vec::new();

        while self.check(&TokenKind::When) {
            when_clauses.push(self.parse_searched_when_clause()?);
        }

        let else_clause = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.expect(TokenKind::End)?.end;

        Ok(CaseExpression::Searched(SearchedCaseExpression {
            when_clauses,
            else_clause,
            span: start..end,
        }))
    }

    fn parse_simple_when_clause(&mut self) -> ParseResult<SimpleWhenClause> {
        let start = self.expect(TokenKind::When)?.start;
        let when_value = self.parse_expression()?;
        self.expect(TokenKind::Then)?;
        let then_result = self.parse_expression()?;

        Ok(SimpleWhenClause {
            span: start..then_result.span().end,
            when_value,
            then_result,
        })
    }

    fn parse_searched_when_clause(&mut self) -> ParseResult<SearchedWhenClause> {
        let start = self.expect(TokenKind::When)?.start;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Then)?;
        let then_result = self.parse_expression()?;

        Ok(SearchedWhenClause {
            span: start..then_result.span().end,
            condition,
            then_result,
        })
    }

    fn parse_cast_expression(&mut self) -> ParseResult<CastExpression> {
        let start = self.expect(TokenKind::Cast)?.start;
        self.expect(TokenKind::LParen)?;
        let operand = Box::new(self.parse_expression()?);
        self.expect(TokenKind::As)?;
        let target_type = self.parse_type_reference()?;
        let end = self.expect(TokenKind::RParen)?.end;

        Ok(CastExpression {
            operand,
            target_type,
            span: start..end,
        })
    }

    fn parse_exists_expression(&mut self) -> ParseResult<ExistsExpression> {
        let start = self.expect(TokenKind::Exists)?.start;

        let variant = if self.check(&TokenKind::LBrace) {
            let pattern_start = self.current().span.start;
            self.advance();

            let mut depth = 1usize;
            let mut pattern_end = pattern_start;

            while depth > 0 {
                if self.check(&TokenKind::Eof) {
                    return Err(self.error_here("unclosed EXISTS graph pattern, expected '}'"));
                }

                let span = self.current().span.clone();
                match self.current().kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth = depth.saturating_sub(1),
                    _ => {}
                }
                pattern_end = span.end;
                self.advance();
            }

            ExistsVariant::GraphPattern(GraphPatternPlaceholder {
                span: pattern_start..pattern_end,
            })
        } else if self.check(&TokenKind::LParen) {
            self.advance();
            let query_expr = self.parse_expression()?;
            self.expect(TokenKind::RParen)?;
            ExistsVariant::Subquery(Box::new(query_expr))
        } else {
            return Err(self.error_here("expected '{' or '(' after EXISTS"));
        };

        let span = start..self.tokens[self.pos - 1].span.end;
        Ok(ExistsExpression { variant, span })
    }

    fn parse_all_different_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::AllDifferent)?.start;
        self.expect(TokenKind::LParen)?;

        let mut exprs = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            exprs.push(self.parse_expression()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.check(&TokenKind::RParen) {
                break;
            }
        }

        let end = self.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::AllDifferent(
            exprs,
            start..end,
        )))
    }

    fn parse_same_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::Same)?.start;
        self.expect(TokenKind::LParen)?;
        let expr1 = Box::new(self.parse_expression()?);
        self.expect(TokenKind::Comma)?;
        let expr2 = Box::new(self.parse_expression()?);
        let end = self.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::Same(
            expr1,
            expr2,
            start..end,
        )))
    }

    fn parse_property_exists_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.expect(TokenKind::PropertyExists)?.start;
        self.expect(TokenKind::LParen)?;
        let element = Box::new(self.parse_expression()?);
        self.expect(TokenKind::Comma)?;

        let property_name = match &self.current().kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::StringLiteral(name) => {
                let prop = name.clone();
                self.advance();
                prop
            }
            _ => return Err(self.error_here("expected property name")),
        };

        let end = self.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::PropertyExists(
            element,
            property_name,
            start..end,
        )))
    }

    fn parse_type_reference(&mut self) -> ParseResult<TypeReference> {
        let type_name = match &self.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => name.clone(),
            TokenKind::String => SmolStr::new("STRING"),
            TokenKind::Integer => SmolStr::new("INTEGER"),
            TokenKind::Float => SmolStr::new("FLOAT"),
            TokenKind::Boolean => SmolStr::new("BOOLEAN"),
            TokenKind::List => SmolStr::new("LIST"),
            TokenKind::Record => SmolStr::new("RECORD"),
            TokenKind::Date => SmolStr::new("DATE"),
            TokenKind::Time => SmolStr::new("TIME"),
            TokenKind::Timestamp => SmolStr::new("TIMESTAMP"),
            TokenKind::Datetime => SmolStr::new("DATETIME"),
            TokenKind::Duration => SmolStr::new("DURATION"),
            _ => {
                return Err(
                    self.error_here(format!("expected type name, found {}", self.current().kind))
                );
            }
        };
        let span = self.current().span.clone();
        self.advance();

        Ok(TypeReference { type_name, span })
    }

    fn parse_label_expression(&mut self) -> ParseResult<LabelExpression> {
        self.expect(TokenKind::Colon)?;
        let label = match &self.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                let label = name.clone();
                self.advance();
                label
            }
            _ => return Err(self.error_here("expected label name")),
        };

        let span = self.tokens[self.pos - 1].span.clone();
        Ok(LabelExpression { label, span })
    }

    fn is_comparison_operator(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Eq
                | TokenKind::NotEq
                | TokenKind::NotEqBang
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::LtEq
                | TokenKind::GtEq
        )
    }

    fn consume_comparison_operator(&mut self) -> Option<ComparisonOperator> {
        let op = match self.current().kind {
            TokenKind::Eq => ComparisonOperator::Eq,
            TokenKind::NotEq | TokenKind::NotEqBang => ComparisonOperator::NotEq,
            TokenKind::Lt => ComparisonOperator::Lt,
            TokenKind::Gt => ComparisonOperator::Gt,
            TokenKind::LtEq => ComparisonOperator::LtEq,
            TokenKind::GtEq => ComparisonOperator::GtEq,
            _ => return None,
        };
        self.advance();
        Some(op)
    }
}

/// Parses an expression from a token stream.
pub fn parse_expression(tokens: &[Token]) -> ParseResult<Expression> {
    let mut normalized = tokens.to_vec();
    if normalized.is_empty() {
        normalized.push(Token::new(TokenKind::Eof, 0..0));
    } else if !matches!(normalized.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
        let eof_pos = normalized.last().map_or(0, |token| token.span.end);
        normalized.push(Token::new(TokenKind::Eof, eof_pos..eof_pos));
    }

    let mut parser = ExpressionParser::new(&normalized);
    let expr = parser.parse_expression()?;

    if !matches!(parser.current().kind, TokenKind::Eof) {
        return Err(Box::new(
            Diag::error("unexpected trailing tokens after expression")
                .with_primary_label(parser.current().span.clone(), "unexpected token"),
        ));
    }

    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expr(source: &str) -> ParseResult<Expression> {
        let result = Lexer::new(source).tokenize();
        if !result.diagnostics.is_empty() {
            return Err(Box::new(Diag::error(
                "lexer diagnostics present in expression test",
            )));
        }
        parse_expression(&result.tokens)
    }

    #[test]
    fn parses_basic_literals() {
        assert!(matches!(
            parse_expr("TRUE").unwrap(),
            Expression::Literal(Literal::Boolean(BooleanValue::True), _)
        ));
        assert!(matches!(
            parse_expr("NULL").unwrap(),
            Expression::Literal(Literal::Null, _)
        ));
        assert!(matches!(
            parse_expr("0xFF").unwrap(),
            Expression::Literal(Literal::Integer(_), _)
        ));
        assert!(matches!(
            parse_expr("1.2e3").unwrap(),
            Expression::Literal(Literal::Float(_), _)
        ));
        assert!(matches!(
            parse_expr("X'0aFF'").unwrap(),
            Expression::Literal(Literal::ByteString(_), _)
        ));
    }

    #[test]
    fn parses_temporal_literals() {
        assert!(matches!(
            parse_expr("DATE '2024-01-01'").unwrap(),
            Expression::Literal(Literal::Date(_), _)
        ));
        assert!(matches!(
            parse_expr("DATETIME '2024-01-01T10:00:00'").unwrap(),
            Expression::Literal(Literal::Datetime(_), _)
        ));
        assert!(matches!(
            parse_expr("TIMESTAMP '2024-01-01T10:00:00'").unwrap(),
            Expression::Literal(Literal::Datetime(_), _)
        ));
    }

    #[test]
    fn parses_temporal_functions() {
        assert!(matches!(
            parse_expr("DATE('2024-01-01')").unwrap(),
            Expression::FunctionCall(FunctionCall {
                name: FunctionName::Date,
                ..
            })
        ));
        assert!(matches!(
            parse_expr("CURRENT_DATE()").unwrap(),
            Expression::FunctionCall(FunctionCall {
                name: FunctionName::CurrentDate,
                ..
            })
        ));
    }

    #[test]
    fn enforces_trailing_token_rejection() {
        let err = parse_expr("1 foo").unwrap_err();
        assert!(err.message.contains("unexpected trailing tokens"));
    }

    #[test]
    fn concatenation_has_lower_precedence_than_additive() {
        let expr = parse_expr("1 || 2 + 3").unwrap();
        let Expression::Binary(BinaryOperator::Concatenate, _, right, _) = expr else {
            panic!("expected concatenation at root");
        };
        assert!(matches!(
            *right,
            Expression::Binary(BinaryOperator::Add, _, _, _)
        ));
    }

    #[test]
    fn rejects_chained_comparisons() {
        let err = parse_expr("a < b < c").unwrap_err();
        assert!(err.message.contains("chained comparison"));
    }

    #[test]
    fn parses_property_reference_chain() {
        let expr = parse_expr("a.b.c").unwrap();
        match expr {
            Expression::PropertyReference(inner, _, _) => {
                assert!(matches!(*inner, Expression::PropertyReference(_, _, _)));
            }
            _ => panic!("expected property reference"),
        }
    }

    #[test]
    fn parses_case_and_cast() {
        assert!(matches!(
            parse_expr("CASE WHEN a THEN b ELSE c END").unwrap(),
            Expression::Case(_)
        ));
        assert!(matches!(
            parse_expr("CAST(a AS STRING)").unwrap(),
            Expression::Cast(_)
        ));
    }

    #[test]
    fn parses_predicates() {
        assert!(matches!(
            parse_expr("a IS NOT NULL").unwrap(),
            Expression::Predicate(Predicate::IsNull(_, true, _))
        ));
        assert!(matches!(
            parse_expr("ALL_DIFFERENT(a, b)").unwrap(),
            Expression::Predicate(Predicate::AllDifferent(_, _))
        ));
    }

    #[test]
    fn parses_exists_variants_and_reports_unclosed_graph_pattern() {
        assert!(matches!(
            parse_expr("EXISTS (a)").unwrap(),
            Expression::Exists(ExistsExpression {
                variant: ExistsVariant::Subquery(_),
                ..
            })
        ));

        let err = parse_expr("EXISTS { a ").unwrap_err();
        assert!(err.message.contains("unclosed EXISTS graph pattern"));
    }

    #[test]
    fn parses_collection_constructors_and_forms() {
        assert!(matches!(
            parse_expr("[1, 2, 3]").unwrap(),
            Expression::Literal(Literal::List(_), _)
        ));
        assert!(matches!(
            parse_expr("{a: 1, b: 2}").unwrap(),
            Expression::Literal(Literal::Record(_), _)
        ));
        assert!(matches!(
            parse_expr("RECORD {a: 1}").unwrap(),
            Expression::RecordConstructor(_, _)
        ));
        assert!(matches!(
            parse_expr("PATH[a, b]").unwrap(),
            Expression::PathConstructor(_, _)
        ));
    }

    #[test]
    fn parses_graph_binding_and_value_expressions() {
        assert!(matches!(
            parse_expr("PROPERTY GRAPH x").unwrap(),
            Expression::GraphExpression(_, _)
        ));
        assert!(matches!(
            parse_expr("BINDING TABLE x").unwrap(),
            Expression::BindingTableExpression(_, _)
        ));
        assert!(matches!(
            parse_expr("VALUE (x)").unwrap(),
            Expression::SubqueryExpression(_, _)
        ));
    }
}
