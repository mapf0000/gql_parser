//! Pass 8: Reference Validation
//!
//! Validates that references to catalog entities (e.g., graphs) exist.

use crate::ast::{Program, Query, Statement};
use crate::diag::Diag;

/// Pass 8: Reference Validation - Validates that references to metadata entities exist.
pub(super) fn run_reference_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - Graph names referenced in USE GRAPH clauses exist
    // - Other metadata references are valid
    // Note: This pass is skipped if no metadata provider is available

    // Only perform validation if metadata provider is configured
    let Some(metadata) = validator.metadata_provider else {
        // Metadata provider not available, skip validation
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
                validate_query_references(&query_stmt.query, metadata, diagnostics);
            }
            Statement::Mutation(mutation_stmt) => {
                // Validate references in mutations (e.g., USE GRAPH in focused mutations)
                validate_mutation_references(&mutation_stmt.statement, metadata, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validates metadata references in a query.
fn validate_query_references(
    query: &Query,
    metadata: &dyn crate::semantic::metadata_provider::MetadataProvider,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            // Check for USE GRAPH clause
            if let Some(use_graph) = &linear_query.use_graph {
                // Extract graph name from USE GRAPH expression (if it's a simple reference)
                if let crate::ast::expression::Expression::VariableReference(name, span) =
                    &use_graph.graph
                    && metadata.validate_graph_exists(name).is_err()
                {
                    use crate::semantic::diag::unknown_reference;
                    let diag = unknown_reference("graph", name, span.clone())
                        .with_note("Graph not found in metadata");
                    diagnostics.push(diag);
                }
                // Note: Complex USE GRAPH expressions (functions, computations)
                // cannot be validated statically - skip them
            }
        }
        Query::Composite(composite) => {
            validate_query_references(&composite.left, metadata, diagnostics);
            validate_query_references(&composite.right, metadata, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_references(query, metadata, diagnostics);
        }
    }
}

/// Validates metadata references in a mutation.
fn validate_mutation_references(
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    metadata: &dyn crate::semantic::metadata_provider::MetadataProvider,
    diagnostics: &mut Vec<Diag>,
) {
    // Check for USE GRAPH clause in focused mutations
    if let Some(use_graph_clause) = &mutation.use_graph_clause {
        // Extract graph name from USE GRAPH expression (if it's a simple reference)
        if let crate::ast::expression::Expression::VariableReference(name, span) =
            &use_graph_clause.graph
            && metadata.validate_graph_exists(name.as_str()).is_err()
        {
            use crate::semantic::diag::unknown_reference;
            let diag = unknown_reference("graph", name.as_str(), span.clone())
                .with_note("Graph not found in metadata");
            diagnostics.push(diag);
        }
    }
}
