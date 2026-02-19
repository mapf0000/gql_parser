//! Generic node collection visitor.

use std::ops::ControlFlow;

use crate::ast::Expression;
use crate::ast::program::{Program, QueryStatement, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, FilterStatement, ForStatement, GraphPattern, LabelExpression,
    LetStatement, LetVariableDefinition, MatchStatement, NodePattern, PathPattern,
    PrimitiveQueryStatement, PrimitiveResultStatement, Query, ReturnItem, ReturnStatement,
    SelectStatement,
};
use crate::ast::visitor::{AstVisitor, walk_edge_pattern, walk_element_pattern, walk_expression,
    walk_filter_statement, walk_for_statement, walk_graph_pattern, walk_label_expression,
    walk_let_binding, walk_let_statement, walk_match_statement, walk_node_pattern,
    walk_path_pattern, walk_primitive_query_statement, walk_primitive_result_statement,
    walk_program, walk_query, walk_query_statement, walk_return_item, walk_return_statement,
    walk_select_statement, walk_statement,
};

/// Borrowed AST node view used by [`CollectingVisitor`].
#[derive(Debug, Clone, Copy)]
pub enum AstNode<'a> {
    Program(&'a Program),
    Statement(&'a Statement),
    QueryStatement(&'a QueryStatement),
    Query(&'a Query),
    PrimitiveQueryStatement(&'a PrimitiveQueryStatement),
    PrimitiveResultStatement(&'a PrimitiveResultStatement),
    MatchStatement(&'a MatchStatement),
    GraphPattern(&'a GraphPattern),
    PathPattern(&'a PathPattern),
    ElementPattern(&'a ElementPattern),
    NodePattern(&'a NodePattern),
    EdgePattern(&'a EdgePattern),
    LabelExpression(&'a LabelExpression),
    FilterStatement(&'a FilterStatement),
    LetStatement(&'a LetStatement),
    LetBinding(&'a LetVariableDefinition),
    ForStatement(&'a ForStatement),
    SelectStatement(&'a SelectStatement),
    ReturnStatement(&'a ReturnStatement),
    ReturnItem(&'a ReturnItem),
    Expression(&'a Expression),
}

/// Visitor that collects values produced by a node-matching closure.
#[derive(Debug)]
pub struct CollectingVisitor<T, F> {
    matcher: F,
    items: Vec<T>,
}

impl<T, F> CollectingVisitor<T, F>
where
    F: for<'a> FnMut(AstNode<'a>) -> Option<T>,
{
    /// Creates a collecting visitor.
    pub fn new(matcher: F) -> Self {
        Self {
            matcher,
            items: Vec::new(),
        }
    }

    /// Returns collected values.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Returns collected values, consuming the visitor.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    fn maybe_collect<'a>(&mut self, node: AstNode<'a>) {
        if let Some(item) = (self.matcher)(node) {
            self.items.push(item);
        }
    }
}

impl<T, F> AstVisitor for CollectingVisitor<T, F>
where
    F: for<'a> FnMut(AstNode<'a>) -> Option<T>,
{
    type Break = ();

    fn visit_program(&mut self, program: &Program) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::Program(program));
        walk_program(self, program)
    }

    fn visit_statement(&mut self, statement: &Statement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::Statement(statement));
        walk_statement(self, statement)
    }

    fn visit_query_statement(&mut self, statement: &QueryStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::QueryStatement(statement));
        walk_query_statement(self, statement)
    }

    fn visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::Query(query));
        walk_query(self, query)
    }

    fn visit_primitive_query_statement(
        &mut self,
        statement: &PrimitiveQueryStatement,
    ) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::PrimitiveQueryStatement(statement));
        walk_primitive_query_statement(self, statement)
    }

    fn visit_primitive_result_statement(
        &mut self,
        statement: &PrimitiveResultStatement,
    ) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::PrimitiveResultStatement(statement));
        walk_primitive_result_statement(self, statement)
    }

    fn visit_match_statement(&mut self, statement: &MatchStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::MatchStatement(statement));
        walk_match_statement(self, statement)
    }

    fn visit_graph_pattern(&mut self, pattern: &GraphPattern) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::GraphPattern(pattern));
        walk_graph_pattern(self, pattern)
    }

    fn visit_path_pattern(&mut self, pattern: &PathPattern) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::PathPattern(pattern));
        walk_path_pattern(self, pattern)
    }

    fn visit_element_pattern(&mut self, pattern: &ElementPattern) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::ElementPattern(pattern));
        walk_element_pattern(self, pattern)
    }

    fn visit_node_pattern(&mut self, pattern: &NodePattern) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::NodePattern(pattern));
        walk_node_pattern(self, pattern)
    }

    fn visit_edge_pattern(&mut self, pattern: &EdgePattern) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::EdgePattern(pattern));
        walk_edge_pattern(self, pattern)
    }

    fn visit_label_expression(
        &mut self,
        expression: &LabelExpression,
    ) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::LabelExpression(expression));
        walk_label_expression(self, expression)
    }

    fn visit_filter_statement(&mut self, statement: &FilterStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::FilterStatement(statement));
        walk_filter_statement(self, statement)
    }

    fn visit_let_statement(&mut self, statement: &LetStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::LetStatement(statement));
        walk_let_statement(self, statement)
    }

    fn visit_let_binding(
        &mut self,
        binding: &LetVariableDefinition,
    ) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::LetBinding(binding));
        walk_let_binding(self, binding)
    }

    fn visit_for_statement(&mut self, statement: &ForStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::ForStatement(statement));
        walk_for_statement(self, statement)
    }

    fn visit_select_statement(&mut self, statement: &SelectStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::SelectStatement(statement));
        walk_select_statement(self, statement)
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::ReturnStatement(statement));
        walk_return_statement(self, statement)
    }

    fn visit_return_item(&mut self, item: &ReturnItem) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::ReturnItem(item));
        walk_return_item(self, item)
    }

    fn visit_expression(&mut self, expression: &Expression) -> ControlFlow<Self::Break> {
        self.maybe_collect(AstNode::Expression(expression));
        walk_expression(self, expression)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::visitor::AstVisitor;
    use crate::ast::visitors::collecting::{AstNode, CollectingVisitor};
    use crate::parse;

    #[test]
    fn collecting_visitor_collects_property_names() {
        let parse_result = parse("MATCH (n:Person) WHERE n.age > 21 RETURN n.name");
        let program = parse_result.ast.expect("expected AST");

        let mut visitor = CollectingVisitor::new(|node| match node {
            AstNode::Expression(crate::ast::Expression::PropertyReference(_, key, _)) => {
                Some(key.to_string())
            }
            _ => None,
        });

        let flow = visitor.visit_program(&program);
        assert!(matches!(flow, std::ops::ControlFlow::Continue(())));
        assert_eq!(visitor.items(), &["age".to_string(), "name".to_string()]);
    }
}
