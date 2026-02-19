//! Variable dependency graph construction over clause boundaries.

use std::collections::{BTreeMap, BTreeSet};

use smol_str::SmolStr;

use crate::analysis::query_info::{ClauseId, ClauseInfo, ClauseKind, QueryInfo};
use crate::ast::{Span, Statement};

/// A variable definition site in the clause pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPoint {
    /// Variable name.
    pub variable: SmolStr,
    /// Clause where the variable is defined.
    pub clause_id: ClauseId,
    /// Clause kind where definition occurs.
    pub clause_kind: ClauseKind,
    /// Clause span containing the definition site.
    pub span: Span,
}

/// A variable usage site in the clause pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsagePoint {
    /// Variable name.
    pub variable: SmolStr,
    /// Clause where the variable is used.
    pub clause_id: ClauseId,
    /// Clause kind where usage occurs.
    pub clause_kind: ClauseKind,
    /// Clause span containing the usage site.
    pub span: Span,
}

/// Cross-clause define/use edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefineUseEdge {
    /// Variable name for this dependency edge.
    pub variable: SmolStr,
    /// Index into [`VariableDependencyGraph::definition_points`].
    pub definition_index: usize,
    /// Index into [`VariableDependencyGraph::usage_points`].
    pub usage_index: usize,
}

/// Define/use dependency graph derived from clause metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariableDependencyGraph {
    /// Ordered list of discovered definition points.
    pub definition_points: Vec<DefinitionPoint>,
    /// Ordered list of discovered usage points.
    pub usage_points: Vec<UsagePoint>,
    /// Define/use dependency edges.
    pub edges: Vec<DefineUseEdge>,
    /// Usage indices with no in-scope definition.
    pub unresolved_usage_indices: Vec<usize>,
}

impl VariableDependencyGraph {
    /// Builds a dependency graph from a top-level statement.
    pub fn build(statement: &Statement) -> Self {
        let query_info = QueryInfo::from_ast(statement);

        let mut graph = Self::default();
        let mut active_definitions: BTreeMap<usize, BTreeMap<SmolStr, usize>> = BTreeMap::new();

        for clause in &query_info.clause_sequence {
            let usage_indices = graph.push_usages(clause);
            let active_for_pipeline = active_definitions
                .entry(clause.clause_id.pipeline_id)
                .or_default();

            for usage_index in usage_indices {
                let usage = &graph.usage_points[usage_index];
                if let Some(definition_index) = active_for_pipeline.get(&usage.variable) {
                    graph.edges.push(DefineUseEdge {
                        variable: usage.variable.clone(),
                        definition_index: *definition_index,
                        usage_index,
                    });
                } else {
                    graph.unresolved_usage_indices.push(usage_index);
                }
            }

            let definition_indices = graph.push_definitions(clause);
            for definition_index in definition_indices {
                let definition = &graph.definition_points[definition_index];
                active_for_pipeline.insert(definition.variable.clone(), definition_index);
            }
        }

        graph
    }

    fn push_usages(&mut self, clause: &ClauseInfo) -> Vec<usize> {
        let mut indices = Vec::new();

        for variable in &clause.uses {
            let index = self.usage_points.len();
            self.usage_points.push(UsagePoint {
                variable: variable.clone(),
                clause_id: clause.clause_id,
                clause_kind: clause.kind.clone(),
                span: clause.span.clone(),
            });
            indices.push(index);
        }

        indices
    }

    fn push_definitions(&mut self, clause: &ClauseInfo) -> Vec<usize> {
        let mut indices = Vec::new();

        for variable in &clause.definitions {
            let index = self.definition_points.len();
            self.definition_points.push(DefinitionPoint {
                variable: variable.clone(),
                clause_id: clause.clause_id,
                clause_kind: clause.kind.clone(),
                span: clause.span.clone(),
            });
            indices.push(index);
        }

        indices
    }

    /// Returns unresolved variable names in deterministic order.
    pub fn unresolved_variables(&self) -> BTreeSet<SmolStr> {
        self.unresolved_usage_indices
            .iter()
            .map(|index| self.usage_points[*index].variable.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::variable_dependency::VariableDependencyGraph;
    use crate::ast::Statement;
    use crate::parse;

    fn first_statement(source: &str) -> Statement {
        let parse_result = parse(source);
        let program = parse_result.ast.expect("expected AST");
        program.statements[0].clone()
    }

    #[test]
    fn dependency_graph_builds_cross_clause_define_use_edges() {
        let statement = first_statement("MATCH (n) LET x = n.age RETURN x");
        let graph = VariableDependencyGraph::build(&statement);

        assert!(graph.definition_points.iter().any(|point| point.variable == "n"));
        assert!(graph.definition_points.iter().any(|point| point.variable == "x"));
        assert!(graph.usage_points.iter().any(|point| point.variable == "n"));
        assert!(graph.usage_points.iter().any(|point| point.variable == "x"));

        let has_n_edge = graph
            .edges
            .iter()
            .any(|edge| edge.variable == "n" && graph.usage_points[edge.usage_index].variable == "n");
        assert!(has_n_edge, "expected n define/use edge");

        let has_x_edge = graph
            .edges
            .iter()
            .any(|edge| edge.variable == "x" && graph.usage_points[edge.usage_index].variable == "x");
        assert!(has_x_edge, "expected x define/use edge");

        assert!(graph.unresolved_usage_indices.is_empty());
    }

    #[test]
    fn dependency_graph_tracks_unresolved_usages() {
        let statement = first_statement("RETURN missing_var");
        let graph = VariableDependencyGraph::build(&statement);

        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.unresolved_usage_indices.len(), 1);
        assert!(graph.unresolved_variables().contains("missing_var"));
    }
}
