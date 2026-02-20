//! Expression parsing for GQL.
//!
//! This module implements expression parsing with precedence handling,
//! literal/function/predicate support, and structured diagnostics.

use crate::ast::query::SetQuantifier;
use crate::ast::{
    AggregateFunction, BinaryOperator, BinarySetFunction, BinarySetFunctionType, BooleanValue,
    CaseExpression, CastExpression, ComparisonOperator, ExistsExpression, ExistsVariant,
    Expression, FunctionCall, FunctionName, GeneralSetFunction, GeneralSetFunctionType,
    GraphPatternPlaceholder, LabelExpression, Literal, LogicalOperator, Predicate, RecordField,
    SearchedCaseExpression, SearchedWhenClause, SimpleCaseExpression, SimpleWhenClause, Span,
    TrimSpecification, TruthValue, TypeAnnotation, TypeAnnotationOperator, UnaryOperator,
    ValueType,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::base::{ParseResult, TokenStream};
use crate::parser::procedure::parse_nested_query_specification;
use crate::parser::types::parse_value_type_prefix;
use smol_str::SmolStr;

/// Parser for expressions.
pub struct ExpressionParser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> ExpressionParser<'a> {
    /// Creates a new expression parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            stream: TokenStream::new(tokens),
        }
    }

    /// Parses an expression using standard precedence rules.
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_xor_expression()?;

        while self.stream.check(&TokenKind::Or) {
            self.stream.advance();
            let right = self.parse_xor_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::Or, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_xor_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.stream.check(&TokenKind::Xor) {
            self.stream.advance();
            let right = self.parse_and_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::Xor, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_not_expression()?;

        while self.stream.check(&TokenKind::And) {
            self.stream.advance();
            let right = self.parse_not_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Logical(LogicalOperator::And, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> ParseResult<Expression> {
        if self.stream.check(&TokenKind::Not) {
            let start = self.stream.current().span.start;
            self.stream.advance();
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

        while self.stream.check(&TokenKind::Is) {
            self.stream.advance();

            let negated = if self.stream.check(&TokenKind::Not) {
                self.stream.advance();
                true
            } else {
                false
            };

            expr = match &self.stream.current().kind {
                TokenKind::Null => {
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsNull(Box::new(expr), negated, span))
                }
                TokenKind::Typed => {
                    self.stream.advance();
                    let type_ref = self.parse_value_type_inline()?;
                    let span = expr.span().start..type_ref.span().end;
                    Expression::Predicate(Predicate::IsTyped(
                        Box::new(expr),
                        type_ref,
                        negated,
                        span,
                    ))
                }
                TokenKind::Normalized => {
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsNormalized(Box::new(expr), negated, span))
                }
                TokenKind::Directed => {
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsDirected(Box::new(expr), negated, span))
                }
                TokenKind::Labeled => {
                    let labeled_span = self.stream.current().span.clone();
                    self.stream.advance();
                    let label = if self.stream.check(&TokenKind::Colon) {
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
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::True,
                        negated,
                        span,
                    ))
                }
                TokenKind::False => {
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::False,
                        negated,
                        span,
                    ))
                }
                TokenKind::Unknown => {
                    let end = self.stream.current().span.end;
                    self.stream.advance();
                    let span = expr.span().start..end;
                    Expression::Predicate(Predicate::IsTruthValue(
                        Box::new(expr),
                        TruthValue::Unknown,
                        negated,
                        span,
                    ))
                }
                TokenKind::Source => {
                    self.stream.advance();
                    self.stream.expect(TokenKind::Of)?;
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
                    self.stream.advance();
                    self.stream.expect(TokenKind::Of)?;
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
                    return Err(self.stream.error_here(format!(
                        "expected IS predicate, found {}",
                        self.stream.current().kind
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
                return Err(self.stream.error_here(
                    "chained comparison operators are not allowed without parentheses",
                ));
            }
        }

        Ok(left)
    }

    fn parse_concatenation_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive_expression()?;

        while self.stream.check(&TokenKind::DoublePipe) {
            self.stream.advance();
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
            let op = match &self.stream.current().kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => break,
            };

            self.stream.advance();
            let right = self.parse_multiplicative_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Binary(op, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_unary_expression()?;

        loop {
            let op = match &self.stream.current().kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => break,
            };

            self.stream.advance();
            let right = self.parse_unary_expression()?;
            let span = left.span().start..right.span().end;
            left = Expression::Binary(op, Box::new(left), Box::new(right), span);
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> ParseResult<Expression> {
        match &self.stream.current().kind {
            TokenKind::Plus => {
                let start = self.stream.current().span.start;
                self.stream.advance();
                let operand = self.parse_unary_expression()?;
                let span = start..operand.span().end;
                Ok(Expression::Unary(
                    UnaryOperator::Plus,
                    Box::new(operand),
                    span,
                ))
            }
            TokenKind::Minus => {
                let start = self.stream.current().span.start;
                self.stream.advance();
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

        loop {
            if matches!(self.stream.current().kind, TokenKind::Dot) {
                self.stream.advance();
                let Some(name) = self.token_name_for_property_name() else {
                    return Err(self.stream.error_here(format!(
                        "expected property name, found {}",
                        self.stream.current().kind
                    )));
                };
                self.stream.advance();
                let prev_pos = self.stream.position();
                let span = expr.span().start..self.stream.tokens()[prev_pos - 1].span.end;
                expr = Expression::PropertyReference(Box::new(expr), name, span);
                continue;
            }

            if matches!(
                self.stream.current().kind,
                TokenKind::DoubleColon | TokenKind::Typed
            ) {
                let annotation_start = self.stream.current().span.start;
                let operator = if self.stream.check(&TokenKind::DoubleColon) {
                    self.stream.advance();
                    TypeAnnotationOperator::DoubleColon
                } else {
                    self.stream.advance();
                    TypeAnnotationOperator::Typed
                };
                let type_ref = Box::new(self.parse_value_type_inline()?);
                let annotation_end = type_ref.span().end;
                let annotation = TypeAnnotation {
                    operator,
                    type_ref,
                    span: annotation_start..annotation_end,
                };
                let span = expr.span().start..annotation_end;
                expr = Expression::TypeAnnotation(Box::new(expr), annotation, span);
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
        match &self.stream.current().kind {
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
                    .stream
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

            // Aggregate functions (Sprint 9)
            TokenKind::Count
            | TokenKind::Avg
            | TokenKind::Max
            | TokenKind::Min
            | TokenKind::Sum
            | TokenKind::CollectList
            | TokenKind::StddevSamp
            | TokenKind::StddevPop
            | TokenKind::PercentileCont
            | TokenKind::PercentileDisc
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::LParen) =>
            {
                let agg_func = self.parse_aggregate_function()?;
                Ok(Expression::AggregateFunction(Box::new(agg_func)))
            }

            TokenKind::Path
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::LBracket) =>
            {
                self.parse_path_constructor()
            }

            TokenKind::Record
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::LBrace) =>
            {
                self.parse_record_constructor()
            }

            TokenKind::Property
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::Graph) =>
            {
                self.parse_graph_expression(true)
            }

            TokenKind::Binding
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::Table) =>
            {
                self.parse_binding_table_expression()
            }

            TokenKind::Value => self.parse_value_subquery_expression(),

            TokenKind::LParen => {
                let start = self.stream.current().span.start;
                self.stream.advance();
                let expr = self.parse_expression()?;
                let end = self.stream.expect(TokenKind::RParen)?.end;
                let span = start..end;
                Ok(Expression::Parenthesized(Box::new(expr), span))
            }

            TokenKind::Parameter(name) => {
                let name = name.clone();
                let span = self.stream.current().span.clone();
                self.stream.advance();
                Ok(Expression::ParameterReference(name, span))
            }

            TokenKind::Identifier(_) | TokenKind::DelimitedIdentifier(_) => {
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::LParen) {
                    let func_call = self.parse_function_call()?;
                    Ok(Expression::FunctionCall(func_call))
                } else {
                    let name = match &self.stream.current().kind {
                        TokenKind::Identifier(n) | TokenKind::DelimitedIdentifier(n) => n.clone(),
                        _ => SmolStr::new(self.stream.current().kind.to_string()),
                    };
                    let span = self.stream.current().span.clone();
                    self.stream.advance();
                    Ok(Expression::VariableReference(name, span))
                }
            }

            _ if self.stream.current().kind.is_keyword() => {
                if self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::LParen) {
                    let func_call = self.parse_function_call()?;
                    Ok(Expression::FunctionCall(func_call))
                } else {
                    let name = SmolStr::new(self.stream.current().kind.to_string());
                    let span = self.stream.current().span.clone();
                    self.stream.advance();
                    Ok(Expression::VariableReference(name, span))
                }
            }

            _ => Err(self.stream.error_here(format!(
                "expected expression, found {}",
                self.stream.current().kind
            ))),
        }
    }

    fn parse_literal_expression(&mut self) -> ParseResult<Expression> {
        let (literal, span) = self.parse_literal()?;
        Ok(Expression::Literal(literal, span))
    }

    fn parse_literal(&mut self) -> ParseResult<(Literal, Span)> {
        let token = self.stream.current();
        let span = token.span.clone();

        let literal = match &token.kind {
            TokenKind::True => {
                self.stream.advance();
                Literal::Boolean(BooleanValue::True)
            }
            TokenKind::False => {
                self.stream.advance();
                Literal::Boolean(BooleanValue::False)
            }
            TokenKind::Unknown => {
                self.stream.advance();
                Literal::Boolean(BooleanValue::Unknown)
            }
            TokenKind::Null => {
                self.stream.advance();
                Literal::Null
            }
            TokenKind::IntegerLiteral(value) => {
                let value = value.clone();
                self.stream.advance();
                Literal::Integer(value)
            }
            TokenKind::FloatLiteral(value) => {
                let value = value.clone();
                self.stream.advance();
                Literal::Float(value)
            }
            TokenKind::StringLiteral(value) => {
                let value = value.clone();
                self.stream.advance();
                Literal::String(value)
            }
            TokenKind::ByteStringLiteral(value) => {
                let value = value.clone();
                self.stream.advance();
                Literal::ByteString(value)
            }
            _ => {
                return Err(self
                    .stream
                    .error_here(format!("expected literal, found {}", token.kind)));
            }
        };

        Ok((literal, span))
    }

    fn parse_temporal_literal_expression(&mut self) -> ParseResult<Expression> {
        let (literal, span) = self.parse_temporal_literal()?;
        Ok(Expression::Literal(literal, span))
    }

    fn parse_temporal_literal(&mut self) -> ParseResult<(Literal, Span)> {
        let keyword_span = self.stream.current().span.clone();
        let kind = self.stream.current().kind.clone();

        match kind {
            TokenKind::Date
            | TokenKind::Time
            | TokenKind::Timestamp
            | TokenKind::Datetime
            | TokenKind::Duration => {}
            _ => return Err(self.stream.error_here("expected temporal literal keyword")),
        }

        self.stream.advance();

        let value = match &self.stream.current().kind {
            TokenKind::StringLiteral(value) => value.clone(),
            _ => {
                return Err(self.stream.error_here(format!(
                    "expected string literal after {}, found {}",
                    kind,
                    self.stream.current().kind
                )));
            }
        };

        let value_span = self.stream.current().span.clone();
        self.stream.advance();

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
        let start = self.stream.expect(TokenKind::LBracket)?.start;
        let mut elements = Vec::new();

        while !self.stream.check(&TokenKind::RBracket) && !self.stream.check(&TokenKind::Eof) {
            elements.push(self.parse_expression()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RBracket) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RBracket)?.end;
        let span = start..end;
        Ok(Expression::Literal(Literal::List(elements), span))
    }

    fn parse_record_literal(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::LBrace)?.start;
        let mut fields = Vec::new();

        while !self.stream.check(&TokenKind::RBrace) && !self.stream.check(&TokenKind::Eof) {
            fields.push(self.parse_record_field()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RBrace) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RBrace)?.end;
        let span = start..end;
        Ok(Expression::Literal(Literal::Record(fields), span))
    }

    fn parse_record_constructor(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::Record)?.start;
        self.stream.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.stream.check(&TokenKind::RBrace) && !self.stream.check(&TokenKind::Eof) {
            fields.push(self.parse_record_field()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RBrace) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RBrace)?.end;
        let span = start..end;
        Ok(Expression::RecordConstructor(fields, span))
    }

    fn parse_record_field(&mut self) -> ParseResult<RecordField> {
        let field_start = self.stream.current().span.start;
        let name = match &self.stream.current().kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::StringLiteral(name) => {
                let field_name = name.clone();
                self.stream.advance();
                field_name
            }
            _ => {
                return Err(self.stream.error_here(format!(
                    "expected record field name, found {}",
                    self.stream.current().kind
                )));
            }
        };

        self.stream.expect(TokenKind::Colon)?;
        let value = self.parse_expression()?;
        let span = field_start..value.span().end;

        Ok(RecordField { name, value, span })
    }

    fn parse_path_constructor(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::Path)?.start;
        self.stream.expect(TokenKind::LBracket)?;

        let mut elements = Vec::new();
        while !self.stream.check(&TokenKind::RBracket) && !self.stream.check(&TokenKind::Eof) {
            elements.push(self.parse_expression()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RBracket) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RBracket)?.end;
        Ok(Expression::PathConstructor(elements, start..end))
    }

    fn parse_graph_expression(&mut self, has_property_keyword: bool) -> ParseResult<Expression> {
        let start = if has_property_keyword {
            let start = self.stream.expect(TokenKind::Property)?.start;
            self.stream.expect(TokenKind::Graph)?;
            start
        } else {
            self.stream.expect(TokenKind::Graph)?.start
        };

        let graph_expr = if self.stream.check(&TokenKind::Current)
            && self.stream.peek().map(|t| &t.kind) == Some(&TokenKind::Graph)
        {
            let current_start = self.stream.current().span.start;
            self.stream.advance();
            let graph_end = self.stream.expect(TokenKind::Graph)?.end;
            Expression::VariableReference("CURRENT_GRAPH".into(), current_start..graph_end)
        } else {
            self.parse_unary_expression()?
        };

        let span = start..graph_expr.span().end;
        Ok(Expression::GraphExpression(Box::new(graph_expr), span))
    }

    fn parse_binding_table_expression(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::Binding)?.start;
        self.stream.expect(TokenKind::Table)?;
        let table_expr = if self.stream.check(&TokenKind::LBrace) {
            let (spec, subquery_span) = self.parse_nested_query_specification_expression(
                "expected nested query specification after BINDING TABLE",
            )?;
            Expression::SubqueryExpression(Box::new(spec), subquery_span)
        } else {
            self.parse_unary_expression()?
        };
        let span = start..table_expr.span().end;
        Ok(Expression::BindingTableExpression(
            Box::new(table_expr),
            span,
        ))
    }

    fn parse_value_subquery_expression(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::Value)?.start;
        let (spec, spec_span) =
            self.parse_nested_query_specification_expression("expected nested query after VALUE")?;

        Ok(Expression::SubqueryExpression(
            Box::new(spec),
            start..spec_span.end,
        ))
    }

    fn parse_nested_query_specification_expression(
        &mut self,
        expected_message: &str,
    ) -> ParseResult<(crate::ast::NestedQuerySpecification, Span)> {
        if !self.stream.check(&TokenKind::LBrace) {
            return Err(self.stream.error_here(expected_message));
        }

        let mut pos = self.stream.position();
        let (spec_opt, diags) = parse_nested_query_specification(self.stream.tokens(), &mut pos);
        if let Some(spec) = spec_opt {
            let span = spec.span.clone();
            self.stream.set_position(pos);
            Ok((spec, span))
        } else if let Some(diag) = diags.into_iter().next() {
            Err(Box::new(diag))
        } else {
            Err(self.stream.error_here(expected_message))
        }
    }

    fn parse_function_call(&mut self) -> ParseResult<FunctionCall> {
        let start = self.stream.current().span.start;
        let name = self.parse_function_name()?;

        self.stream.expect(TokenKind::LParen)?;
        let mut arguments = Vec::new();

        while !self.stream.check(&TokenKind::RParen) && !self.stream.check(&TokenKind::Eof) {
            arguments.push(self.parse_expression()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RParen) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RParen)?.end;
        Ok(FunctionCall {
            name,
            arguments,
            span: start..end,
        })
    }

    fn parse_function_name(&mut self) -> ParseResult<FunctionName> {
        let function = match &self.stream.current().kind {
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
                    return Err(self.stream.error_here(format!(
                        "expected function name, found {}",
                        self.stream.current().kind
                    )));
                };
                self.classify_function_name(name)
            }
        };

        self.stream.advance();
        Ok(function)
    }

    fn token_name_for_function(&self) -> Option<SmolStr> {
        match &self.stream.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                Some(name.clone())
            }
            kind if kind.is_keyword() => Some(SmolStr::new(kind.to_string())),
            _ => None,
        }
    }

    fn token_name_for_property_name(&self) -> Option<SmolStr> {
        match &self.stream.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                Some(name.clone())
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                Some(SmolStr::new(kind.to_string()))
            }
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
        let start = self.stream.expect(TokenKind::Case)?.start;

        if self.stream.check(&TokenKind::When) {
            self.parse_searched_case(start)
        } else {
            self.parse_simple_case(start)
        }
    }

    fn parse_simple_case(&mut self, start: usize) -> ParseResult<CaseExpression> {
        let operand = Box::new(self.parse_expression()?);
        let mut when_clauses = Vec::new();

        while self.stream.check(&TokenKind::When) {
            when_clauses.push(self.parse_simple_when_clause()?);
        }

        let else_clause = if self.stream.check(&TokenKind::Else) {
            self.stream.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.stream.expect(TokenKind::End)?.end;

        Ok(CaseExpression::Simple(SimpleCaseExpression {
            operand,
            when_clauses,
            else_clause,
            span: start..end,
        }))
    }

    fn parse_searched_case(&mut self, start: usize) -> ParseResult<CaseExpression> {
        let mut when_clauses = Vec::new();

        while self.stream.check(&TokenKind::When) {
            when_clauses.push(self.parse_searched_when_clause()?);
        }

        let else_clause = if self.stream.check(&TokenKind::Else) {
            self.stream.advance();
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.stream.expect(TokenKind::End)?.end;

        Ok(CaseExpression::Searched(SearchedCaseExpression {
            when_clauses,
            else_clause,
            span: start..end,
        }))
    }

    fn parse_simple_when_clause(&mut self) -> ParseResult<SimpleWhenClause> {
        let start = self.stream.expect(TokenKind::When)?.start;
        let when_value = self.parse_expression()?;
        self.stream.expect(TokenKind::Then)?;
        let then_result = self.parse_expression()?;

        Ok(SimpleWhenClause {
            span: start..then_result.span().end,
            when_value,
            then_result,
        })
    }

    fn parse_searched_when_clause(&mut self) -> ParseResult<SearchedWhenClause> {
        let start = self.stream.expect(TokenKind::When)?.start;
        let condition = self.parse_expression()?;
        self.stream.expect(TokenKind::Then)?;
        let then_result = self.parse_expression()?;

        Ok(SearchedWhenClause {
            span: start..then_result.span().end,
            condition,
            then_result,
        })
    }

    fn parse_cast_expression(&mut self) -> ParseResult<CastExpression> {
        let start = self.stream.expect(TokenKind::Cast)?.start;
        self.stream.expect(TokenKind::LParen)?;
        let operand = Box::new(self.parse_expression()?);
        self.stream.expect(TokenKind::As)?;
        let target_type = self.parse_value_type_inline()?;
        let end = self.stream.expect(TokenKind::RParen)?.end;

        Ok(CastExpression {
            operand,
            target_type,
            span: start..end,
        })
    }

    fn parse_exists_expression(&mut self) -> ParseResult<ExistsExpression> {
        let start = self.stream.expect(TokenKind::Exists)?.start;

        let variant = if self.stream.check(&TokenKind::LBrace) {
            let pattern_start = self.stream.current().span.start;
            self.stream.advance();

            let mut depth = 1usize;
            let mut pattern_end = pattern_start;

            while depth > 0 {
                if self.stream.check(&TokenKind::Eof) {
                    return Err(self
                        .stream
                        .error_here("unclosed EXISTS graph pattern, expected '}'"));
                }

                let span = self.stream.current().span.clone();
                match self.stream.current().kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth = depth.saturating_sub(1),
                    _ => {}
                }
                pattern_end = span.end;
                self.stream.advance();
            }

            ExistsVariant::GraphPattern(GraphPatternPlaceholder {
                span: pattern_start..pattern_end,
            })
        } else if self.stream.check(&TokenKind::LParen) {
            self.stream.advance();
            let query_expr = self.parse_expression()?;
            self.stream.expect(TokenKind::RParen)?;
            ExistsVariant::Subquery(Box::new(query_expr))
        } else {
            return Err(self.stream.error_here("expected '{' or '(' after EXISTS"));
        };

        let span = start..self.stream.previous_span().end;
        Ok(ExistsExpression { variant, span })
    }

    fn parse_all_different_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::AllDifferent)?.start;
        self.stream.expect(TokenKind::LParen)?;

        let mut exprs = Vec::new();
        while !self.stream.check(&TokenKind::RParen) && !self.stream.check(&TokenKind::Eof) {
            exprs.push(self.parse_expression()?);

            if !self.stream.consume(&TokenKind::Comma) {
                break;
            }
            if self.stream.check(&TokenKind::RParen) {
                break;
            }
        }

        let end = self.stream.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::AllDifferent(
            exprs,
            start..end,
        )))
    }

    fn parse_same_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::Same)?.start;
        self.stream.expect(TokenKind::LParen)?;
        let expr1 = Box::new(self.parse_expression()?);
        self.stream.expect(TokenKind::Comma)?;
        let expr2 = Box::new(self.parse_expression()?);
        let end = self.stream.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::Same(
            expr1,
            expr2,
            start..end,
        )))
    }

    fn parse_property_exists_predicate(&mut self) -> ParseResult<Expression> {
        let start = self.stream.expect(TokenKind::PropertyExists)?.start;
        self.stream.expect(TokenKind::LParen)?;
        let element = Box::new(self.parse_expression()?);
        self.stream.expect(TokenKind::Comma)?;

        let property_name = match self.token_name_for_property_name() {
            Some(name) => {
                self.stream.advance();
                name
            }
            None => return Err(self.stream.error_here("expected property name")),
        };

        let end = self.stream.expect(TokenKind::RParen)?.end;
        Ok(Expression::Predicate(Predicate::PropertyExists(
            element,
            property_name,
            start..end,
        )))
    }

    fn parse_value_type_inline(&mut self) -> ParseResult<ValueType> {
        let (value_type, consumed) =
            parse_value_type_prefix(&self.stream.tokens()[self.stream.position()..])?;
        if consumed == 0 {
            return Err(self.stream.error_here("expected type"));
        }
        for _ in 0..consumed {
            self.stream.advance();
        }
        Ok(value_type)
    }

    fn parse_label_expression(&mut self) -> ParseResult<LabelExpression> {
        self.stream.expect(TokenKind::Colon)?;
        let label = match &self.stream.current().kind {
            TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => {
                let label = name.clone();
                self.stream.advance();
                label
            }
            _ => return Err(self.stream.error_here("expected label name")),
        };

        let span = self.stream.previous_span();
        Ok(LabelExpression { label, span })
    }

    fn is_comparison_operator(&self) -> bool {
        matches!(
            self.stream.current().kind,
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
        let op = match self.stream.current().kind {
            TokenKind::Eq => ComparisonOperator::Eq,
            TokenKind::NotEq | TokenKind::NotEqBang => ComparisonOperator::NotEq,
            TokenKind::Lt => ComparisonOperator::Lt,
            TokenKind::Gt => ComparisonOperator::Gt,
            TokenKind::LtEq => ComparisonOperator::LtEq,
            TokenKind::GtEq => ComparisonOperator::GtEq,
            _ => return None,
        };
        self.stream.advance();
        Some(op)
    }

    // ============================================================================
    // Aggregate Function Parsing (Sprint 9)
    // ============================================================================

    /// Parses an aggregate function expression.
    ///
    /// # Grammar
    ///
    /// ```text
    /// aggregateFunction:
    ///     COUNT '(' '*' ')'
    ///     | generalSetFunction
    ///     | binarySetFunction
    /// ```
    fn parse_aggregate_function(&mut self) -> ParseResult<AggregateFunction> {
        match &self.stream.current().kind {
            TokenKind::Count => {
                let start = self.stream.current().span.start;
                let count_pos = self.stream.position();
                self.stream.advance();
                self.stream.expect(TokenKind::LParen)?;

                // Check for COUNT(*)
                if self.stream.check(&TokenKind::Star) {
                    self.stream.advance();
                    let end = self.stream.expect(TokenKind::RParen)?.end;
                    return Ok(AggregateFunction::CountStar { span: start..end });
                }

                // Otherwise, it's COUNT(expr) which is a general set function
                // Restore parser position and parse as a general set function.
                self.stream.set_position(count_pos);
                self.parse_general_set_function()
            }
            TokenKind::PercentileCont | TokenKind::PercentileDisc => {
                self.parse_binary_set_function()
            }
            _ => self.parse_general_set_function(),
        }
    }

    /// Parses a general set function (AVG, COUNT, MAX, MIN, SUM, etc.).
    ///
    /// # Grammar
    ///
    /// ```text
    /// generalSetFunction:
    ///     generalSetFunctionType '(' [setQuantifier] expression ')'
    /// ```
    fn parse_general_set_function(&mut self) -> ParseResult<AggregateFunction> {
        let start = self.stream.current().span.start;

        let function_type = match &self.stream.current().kind {
            TokenKind::Avg => GeneralSetFunctionType::Avg,
            TokenKind::Count => GeneralSetFunctionType::Count,
            TokenKind::Max => GeneralSetFunctionType::Max,
            TokenKind::Min => GeneralSetFunctionType::Min,
            TokenKind::Sum => GeneralSetFunctionType::Sum,
            TokenKind::CollectList => GeneralSetFunctionType::CollectList,
            TokenKind::StddevSamp => GeneralSetFunctionType::StddevSamp,
            TokenKind::StddevPop => GeneralSetFunctionType::StddevPop,
            _ => {
                return Err(self.stream.error_here(format!(
                    "expected aggregate function name, found {}",
                    self.stream.current().kind
                )));
            }
        };

        self.stream.advance();
        self.stream.expect(TokenKind::LParen)?;

        // Parse optional set quantifier (DISTINCT or ALL)
        let quantifier = self.parse_set_quantifier_opt();

        // Parse the expression to aggregate
        let expression = Box::new(self.parse_expression()?);

        let end = self.stream.expect(TokenKind::RParen)?.end;

        Ok(AggregateFunction::GeneralSetFunction(GeneralSetFunction {
            function_type,
            quantifier,
            expression,
            span: start..end,
        }))
    }

    /// Parses a binary set function (PERCENTILE_CONT, PERCENTILE_DISC).
    ///
    /// # Grammar
    ///
    /// ```text
    /// binarySetFunction:
    ///     binarySetFunctionType '(' dependentValueExpression ',' independentValueExpression ')'
    /// ```
    fn parse_binary_set_function(&mut self) -> ParseResult<AggregateFunction> {
        let start = self.stream.current().span.start;

        let function_type = match &self.stream.current().kind {
            TokenKind::PercentileCont => BinarySetFunctionType::PercentileCont,
            TokenKind::PercentileDisc => BinarySetFunctionType::PercentileDisc,
            _ => {
                return Err(self.stream.error_here(format!(
                    "expected PERCENTILE_CONT or PERCENTILE_DISC, found {}",
                    self.stream.current().kind
                )));
            }
        };

        self.stream.advance();
        self.stream.expect(TokenKind::LParen)?;

        // dependentValueExpression: [setQuantifier] numericValueExpression
        let quantifier = self.parse_set_quantifier_opt();
        let inverse_distribution_argument = Box::new(self.parse_expression()?);

        self.stream.expect(TokenKind::Comma)?;
        // independentValueExpression: numericValueExpression
        let expression = Box::new(self.parse_expression()?);

        let end = self.stream.expect(TokenKind::RParen)?.end;

        Ok(AggregateFunction::BinarySetFunction(BinarySetFunction {
            function_type,
            quantifier,
            inverse_distribution_argument,
            expression,
            span: start..end,
        }))
    }

    /// Parses an optional set quantifier (DISTINCT or ALL).
    ///
    /// # Grammar
    ///
    /// ```text
    /// setQuantifier:
    ///     DISTINCT | ALL
    /// ```
    fn parse_set_quantifier_opt(&mut self) -> Option<SetQuantifier> {
        match &self.stream.current().kind {
            TokenKind::Distinct => {
                self.stream.advance();
                Some(SetQuantifier::Distinct)
            }
            TokenKind::All => {
                self.stream.advance();
                Some(SetQuantifier::All)
            }
            _ => None,
        }
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

    if !matches!(parser.stream.current().kind, TokenKind::Eof) {
        return Err(Box::new(
            Diag::error("unexpected trailing tokens after expression")
                .with_primary_label(parser.stream.current().span.clone(), "unexpected token"),
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
    fn property_reference_accepts_non_reserved_keyword_name() {
        let expr = parse_expr("n.type").unwrap();
        assert!(matches!(expr, Expression::PropertyReference(_, _, _)));
    }

    #[test]
    fn property_reference_rejects_reserved_keyword_name() {
        let err = parse_expr("n.count").unwrap_err();
        assert!(err.message.contains("expected property name"));
    }

    #[test]
    fn property_reference_accepts_delimited_reserved_keyword_name() {
        let expr = parse_expr("n.`count`").unwrap();
        assert!(matches!(expr, Expression::PropertyReference(_, _, _)));
    }

    #[test]
    fn parses_binary_set_function_with_two_arguments() {
        let source = "PERCENTILE_CONT(0.5, n.age)";
        let expr = parse_expr(source).unwrap();

        match expr {
            Expression::AggregateFunction(agg) => match *agg {
                AggregateFunction::BinarySetFunction(func) => {
                    assert_eq!(func.function_type, BinarySetFunctionType::PercentileCont);
                    assert_eq!(func.quantifier, None);
                    assert_eq!(func.span, 0..source.len());
                }
                _ => panic!("expected binary set function"),
            },
            _ => panic!("expected aggregate function"),
        }
    }

    #[test]
    fn rejects_non_standard_within_group_percentile_syntax() {
        let err =
            parse_expr("PERCENTILE_CONT(0.5, n.age) WITHIN GROUP (ORDER BY n.age)").unwrap_err();
        assert!(err.message.contains("unexpected trailing tokens"));
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
        assert!(matches!(
            parse_expr("CAST(a AS INT)").unwrap(),
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
            parse_expr("a IS TYPED INT").unwrap(),
            Expression::Predicate(Predicate::IsTyped(_, _, false, _))
        ));
        assert!(matches!(
            parse_expr("ALL_DIFFERENT(a, b)").unwrap(),
            Expression::Predicate(Predicate::AllDifferent(_, _))
        ));
    }

    #[test]
    fn parses_type_annotation_forms() {
        assert!(matches!(
            parse_expr("a::INT").unwrap(),
            Expression::TypeAnnotation(_, _, _)
        ));
        assert!(matches!(
            parse_expr("a TYPED STRING").unwrap(),
            Expression::TypeAnnotation(_, _, _)
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
            parse_expr("VALUE { RETURN 1 }").unwrap(),
            Expression::SubqueryExpression(_, _)
        ));
    }

    #[test]
    fn parses_value_nested_query_specification() {
        assert!(matches!(
            parse_expr("VALUE { RETURN 1 }").unwrap(),
            Expression::SubqueryExpression(_, _)
        ));
    }

    #[test]
    fn rejects_legacy_parenthesized_value_payload() {
        let err = parse_expr("VALUE (x)").unwrap_err();
        assert!(
            err.message.contains("nested query")
                || err.message.contains("Expected")
                || err.message.contains("expected"),
            "unexpected diagnostic: {}",
            err.message
        );
    }

    #[test]
    fn parses_graph_keyword_expression_forms() {
        assert!(matches!(
            parse_expr("PROPERTY GRAPH CURRENT GRAPH").unwrap(),
            Expression::GraphExpression(_, _)
        ));
        assert!(matches!(
            parse_expr("PROPERTY GRAPH x").unwrap(),
            Expression::GraphExpression(_, _)
        ));
    }

    #[test]
    fn parses_binding_table_nested_query_form() {
        assert!(matches!(
            parse_expr("BINDING TABLE { RETURN 1 }").unwrap(),
            Expression::BindingTableExpression(_, _)
        ));
    }
}
