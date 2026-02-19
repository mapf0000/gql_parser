//! Span collection visitor.

use std::ops::ControlFlow;

use crate::ast::program::{Program, QueryStatement, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, FilterStatement, ForStatement, GraphPattern, LabelExpression,
    LetStatement, LetVariableDefinition, MatchStatement, NodePattern, PathPattern,
    PrimitiveQueryStatement, PrimitiveResultStatement, Query, ReturnItem, ReturnStatement,
    SelectStatement,
};
use crate::ast::visitor::{
    AstVisitor, walk_edge_pattern, walk_element_pattern, walk_expression, walk_filter_statement,
    walk_for_statement, walk_graph_pattern, walk_label_expression, walk_let_binding,
    walk_let_statement, walk_match_statement, walk_node_pattern, walk_path_pattern,
    walk_primitive_query_statement, walk_primitive_result_statement, walk_program, walk_query,
    walk_query_statement, walk_return_item, walk_return_statement, walk_select_statement,
    walk_statement,
};
use crate::ast::{Expression, Span};

/// Collects spans from query-focused AST nodes.
#[derive(Debug, Default)]
pub struct SpanCollector {
    spans: Vec<Span>,
}

impl SpanCollector {
    /// Creates a new span collector.
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Collects spans from an entire program.
    pub fn collect_program(program: &Program) -> Vec<Span> {
        let mut collector = Self::new();
        let _ = collector.visit_program(program);
        collector.spans
    }

    /// Collects spans from an expression subtree.
    pub fn collect_expression(expression: &Expression) -> Vec<Span> {
        let mut collector = Self::new();
        let _ = collector.visit_expression(expression);
        collector.spans
    }

    /// Returns collected spans.
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Returns collected spans, consuming the collector.
    pub fn into_spans(self) -> Vec<Span> {
        self.spans
    }

    fn push_span(&mut self, span: Span) {
        self.spans.push(span);
    }
}

impl AstVisitor for SpanCollector {
    type Break = ();

    fn visit_program(&mut self, program: &Program) -> ControlFlow<Self::Break> {
        self.push_span(program.span.clone());
        walk_program(self, program)
    }

    fn visit_statement(&mut self, statement: &Statement) -> ControlFlow<Self::Break> {
        let span = match statement {
            Statement::Query(statement) => statement.span.clone(),
            Statement::Mutation(statement) => statement.span.clone(),
            Statement::Session(statement) => statement.span.clone(),
            Statement::Transaction(statement) => statement.span.clone(),
            Statement::Catalog(statement) => statement.span.clone(),
            Statement::Empty(span) => span.clone(),
        };
        self.push_span(span);
        walk_statement(self, statement)
    }

    fn visit_query_statement(&mut self, statement: &QueryStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_query_statement(self, statement)
    }

    fn visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        self.push_span(query.span().clone());
        walk_query(self, query)
    }

    fn visit_primitive_query_statement(
        &mut self,
        statement: &PrimitiveQueryStatement,
    ) -> ControlFlow<Self::Break> {
        self.push_span(statement.span().clone());
        walk_primitive_query_statement(self, statement)
    }

    fn visit_primitive_result_statement(
        &mut self,
        statement: &PrimitiveResultStatement,
    ) -> ControlFlow<Self::Break> {
        self.push_span(statement.span().clone());
        walk_primitive_result_statement(self, statement)
    }

    fn visit_match_statement(&mut self, statement: &MatchStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span().clone());
        walk_match_statement(self, statement)
    }

    fn visit_graph_pattern(&mut self, pattern: &GraphPattern) -> ControlFlow<Self::Break> {
        self.push_span(pattern.span.clone());
        walk_graph_pattern(self, pattern)
    }

    fn visit_path_pattern(&mut self, pattern: &PathPattern) -> ControlFlow<Self::Break> {
        self.push_span(pattern.span.clone());
        walk_path_pattern(self, pattern)
    }

    fn visit_element_pattern(&mut self, pattern: &ElementPattern) -> ControlFlow<Self::Break> {
        let span = match pattern {
            ElementPattern::Node(node) => node.span.clone(),
            ElementPattern::Edge(EdgePattern::Full(edge)) => edge.span.clone(),
            ElementPattern::Edge(EdgePattern::Abbreviated(edge)) => edge.span().clone(),
        };
        self.push_span(span);
        walk_element_pattern(self, pattern)
    }

    fn visit_node_pattern(&mut self, pattern: &NodePattern) -> ControlFlow<Self::Break> {
        self.push_span(pattern.span.clone());
        walk_node_pattern(self, pattern)
    }

    fn visit_edge_pattern(&mut self, pattern: &EdgePattern) -> ControlFlow<Self::Break> {
        let span = match pattern {
            EdgePattern::Full(edge) => edge.span.clone(),
            EdgePattern::Abbreviated(edge) => edge.span().clone(),
        };
        self.push_span(span);
        walk_edge_pattern(self, pattern)
    }

    fn visit_label_expression(
        &mut self,
        expression: &LabelExpression,
    ) -> ControlFlow<Self::Break> {
        self.push_span(expression.span().clone());
        walk_label_expression(self, expression)
    }

    fn visit_filter_statement(&mut self, statement: &FilterStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_filter_statement(self, statement)
    }

    fn visit_let_statement(&mut self, statement: &LetStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_let_statement(self, statement)
    }

    fn visit_let_binding(
        &mut self,
        binding: &LetVariableDefinition,
    ) -> ControlFlow<Self::Break> {
        self.push_span(binding.span.clone());
        walk_let_binding(self, binding)
    }

    fn visit_for_statement(&mut self, statement: &ForStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_for_statement(self, statement)
    }

    fn visit_select_statement(&mut self, statement: &SelectStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_select_statement(self, statement)
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement) -> ControlFlow<Self::Break> {
        self.push_span(statement.span.clone());
        walk_return_statement(self, statement)
    }

    fn visit_return_item(&mut self, item: &ReturnItem) -> ControlFlow<Self::Break> {
        self.push_span(item.span.clone());
        walk_return_item(self, item)
    }

    fn visit_expression(&mut self, expression: &Expression) -> ControlFlow<Self::Break> {
        self.push_span(expression.span());
        walk_expression(self, expression)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::visitors::span::SpanCollector;
    use crate::parse;

    #[test]
    fn span_collector_collects_nested_spans() {
        let parse_result = parse("MATCH (n:Person {age: 30}) WHERE n.age > 18 RETURN n.name");
        let program = parse_result.ast.expect("expected AST");

        let spans = SpanCollector::collect_program(&program);

        assert!(!spans.is_empty());
        assert!(spans.len() > 5, "expected multiple nested spans");
    }
}
