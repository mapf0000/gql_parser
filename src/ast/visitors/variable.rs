//! Variable collection visitor.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use smol_str::SmolStr;

use crate::ast::Expression;
use crate::ast::query::{
    EdgePattern, ForOrdinalityOrOffset, ForStatement, LetVariableDefinition, NodePattern,
    PathPattern,
};
use crate::ast::visit::{
    Visit, walk_edge_pattern, walk_expression, walk_for_statement, walk_let_binding,
    walk_node_pattern, walk_path_pattern,
};

/// Collects variable definitions and references from query ASTs.
#[derive(Debug, Clone, Default)]
pub struct VariableCollector {
    references: BTreeSet<SmolStr>,
    definitions: BTreeSet<SmolStr>,
}

impl VariableCollector {
    /// Creates a new empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Collects variable references from a single expression.
    pub fn collect_references_from_expression(expression: &Expression) -> BTreeSet<SmolStr> {
        let mut collector = Self::new();
        let _ = collector.visit_expression(expression);
        collector.references
    }

    /// Returns collected variable references.
    pub fn references(&self) -> &BTreeSet<SmolStr> {
        &self.references
    }

    /// Returns collected variable definitions.
    pub fn definitions(&self) -> &BTreeSet<SmolStr> {
        &self.definitions
    }

    /// Returns collected variable references and consumes this collector.
    pub fn into_references(self) -> BTreeSet<SmolStr> {
        self.references
    }

    fn define(&mut self, name: &SmolStr) {
        self.definitions.insert(name.clone());
    }

    fn reference(&mut self, name: &SmolStr) {
        self.references.insert(name.clone());
    }
}

impl Visit for VariableCollector {
    type Break = ();

    fn visit_expression(&mut self, expression: &Expression) -> ControlFlow<Self::Break> {
        if let Expression::VariableReference(name, _) = expression {
            self.reference(name);
        }

        walk_expression(self, expression)
    }

    fn visit_let_binding(&mut self, binding: &LetVariableDefinition) -> ControlFlow<Self::Break> {
        self.define(&binding.variable.name);
        walk_let_binding(self, binding)
    }

    fn visit_for_statement(&mut self, statement: &ForStatement) -> ControlFlow<Self::Break> {
        self.define(&statement.item.binding_variable.name);

        if let Some(for_clause) = &statement.ordinality_or_offset {
            match for_clause {
                ForOrdinalityOrOffset::Ordinality { variable }
                | ForOrdinalityOrOffset::Offset { variable } => {
                    self.define(&variable.name);
                }
            }
        }

        walk_for_statement(self, statement)
    }

    fn visit_path_pattern(&mut self, pattern: &PathPattern) -> ControlFlow<Self::Break> {
        if let Some(path_variable) = &pattern.variable_declaration {
            self.definitions.insert(path_variable.variable.clone());
        }

        walk_path_pattern(self, pattern)
    }

    fn visit_node_pattern(&mut self, pattern: &NodePattern) -> ControlFlow<Self::Break> {
        if let Some(variable) = &pattern.variable {
            self.definitions.insert(variable.variable.clone());
        }

        walk_node_pattern(self, pattern)
    }

    fn visit_edge_pattern(&mut self, pattern: &EdgePattern) -> ControlFlow<Self::Break> {
        if let EdgePattern::Full(full) = pattern
            && let Some(variable) = &full.filler.variable
        {
            self.definitions.insert(variable.variable.clone());
        }

        walk_edge_pattern(self, pattern)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::visit::Visit;
    use crate::ast::visitors::variable::VariableCollector;
    use crate::parse;

    #[test]
    fn variable_collector_collects_expression_references() {
        let parse_result = parse("MATCH (n) RETURN n + b");
        let program = parse_result.ast.expect("expected AST");

        let mut collector = VariableCollector::new();
        let _ = collector.visit_program(&program);

        assert!(collector.references().contains("n"));
        assert!(collector.references().contains("b"));
    }

    #[test]
    fn variable_collector_collects_query_definitions() {
        let parse_result = parse("MATCH (n)-[r]->(m) LET x = n RETURN x, m");
        let program = parse_result.ast.expect("expected AST");

        let mut collector = VariableCollector::new();
        let _ = collector.visit_program(&program);

        assert!(collector.definitions().contains("n"));
        assert!(collector.definitions().contains("r"));
        assert!(collector.definitions().contains("m"));
        assert!(collector.definitions().contains("x"));
    }
}
