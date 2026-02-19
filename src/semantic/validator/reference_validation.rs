//! Pass 8: Reference Validation
//!
//! Validates that references to catalog entities (e.g., graphs) exist.

use crate::ast::{Program, Query, Statement};
use crate::diag::Diag;

/// Pass 8: Reference Validation - Validates that references to catalog entities exist.
pub(super) fn run_reference_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - Graph names referenced in USE GRAPH clauses exist in catalog
    // - Other catalog references are valid
    // Note: This pass is skipped if no catalog is available

    // Only perform validation if catalog is provided
    let Some(catalog) = validator.catalog else {
        // Catalog not available, skip validation
        return;
    };

    for statement in &program.statements {
        match statement {
            Statement::Catalog(_catalog_stmt) => {
                // Validate catalog statement references
                // e.g., CREATE GRAPH SCHEMA myschema ...
                // This would check if references in the catalog statement are valid
                // Placeholder for future catalog-level validation
            }
            Statement::Query(query_stmt) => {
                // Validate references in queries (e.g., USE GRAPH)
                validate_query_references(&query_stmt.query, catalog, diagnostics);
            }
            Statement::Mutation(mutation_stmt) => {
                // Validate references in mutations (e.g., USE GRAPH in focused mutations)
                validate_mutation_references(&mutation_stmt.statement, catalog, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validates catalog references in a query.
fn validate_query_references(
    query: &Query,
    catalog: &dyn crate::semantic::catalog::Catalog,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            // Check for USE GRAPH clause
            if let crate::ast::query::LinearQuery::Focused(focused) = linear_query {
                // Extract graph name from USE GRAPH expression (if it's a simple reference)
                if let crate::ast::expression::Expression::VariableReference(name, span) =
                    &focused.use_graph.graph
                    && catalog.validate_graph(name).is_err()
                {
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag = SemanticDiagBuilder::unknown_reference("graph", name, span.clone())
                        .with_note("Graph not found in catalog")
                        .build();
                    diagnostics.push(diag);
                }
                // Note: Complex USE GRAPH expressions (functions, computations)
                // cannot be validated statically - skip them
            }
        }
        Query::Composite(composite) => {
            validate_query_references(&composite.left, catalog, diagnostics);
            validate_query_references(&composite.right, catalog, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_references(query, catalog, diagnostics);
        }
    }
}

/// Validates catalog references in a mutation.
fn validate_mutation_references(
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    catalog: &dyn crate::semantic::catalog::Catalog,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::LinearDataModifyingStatement;

    // Check for USE GRAPH clause in focused mutations
    if let LinearDataModifyingStatement::Focused(focused) = mutation {
        // Extract graph name from USE GRAPH expression (if it's a simple reference)
        if let crate::ast::expression::Expression::VariableReference(name, span) =
            &focused.use_graph_clause.graph
            && catalog.validate_graph(name).is_err()
        {
            use crate::semantic::diag::SemanticDiagBuilder;
            let diag = SemanticDiagBuilder::unknown_reference("graph", name, span.clone())
                .with_note("Graph not found in catalog")
                .build();
            diagnostics.push(diag);
        }
    }
}
