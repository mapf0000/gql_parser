//! Pattern validation pass for semantic validation.
//!
//! This module handles pattern validation, checking:
//! - Graph patterns are connected
//! - Path patterns are valid
//! - Quantified patterns maintain connectivity
//!
//! This pass validates that patterns in MATCH and INSERT statements maintain
//! proper connectivity between variables to avoid disconnected graph patterns.

use std::collections::{HashMap, HashSet};

use crate::ast::program::{Program, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, LinearQuery, MatchStatement, PathPattern, PathPatternExpression,
    PathPrimary, PathTerm, PrimitiveQueryStatement, Query,
};
use crate::diag::Diag;
use crate::semantic::diag::SemanticDiagBuilder;

/// Main entry point for pattern validation pass.
///
/// Validates patterns in all queries and mutations for connectivity.
pub(super) fn run_pattern_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - Graph patterns are connected
    // - Path patterns are valid
    // - Quantified patterns maintain connectivity

    for statement in &program.statements {
        match statement {
            Statement::Query(query_stmt) => {
                validate_query_patterns(validator, &query_stmt.query, diagnostics);
            }
            Statement::Mutation(mutation_stmt) => {
                // Validate mutation patterns (INSERT patterns should be connected)
                validate_mutation_patterns(validator, &mutation_stmt.statement, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validates patterns in a query for connectivity.
fn validate_query_patterns(
    validator: &super::SemanticValidator,
    query: &Query,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            validate_linear_query_patterns(validator, linear_query, diagnostics);
        }
        Query::Composite(composite) => {
            validate_query_patterns(validator, &composite.left, diagnostics);
            validate_query_patterns(validator, &composite.right, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_patterns(validator, query, diagnostics);
        }
    }
}

/// Validates patterns in a linear query.
fn validate_linear_query_patterns(
    validator: &super::SemanticValidator,
    linear_query: &LinearQuery,
    diagnostics: &mut Vec<Diag>,
) {
    let primitive_statements = match linear_query {
        LinearQuery::Focused(focused) => &focused.primitive_statements,
        LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
    };

    for statement in primitive_statements {
        if let PrimitiveQueryStatement::Match(match_stmt) = statement {
            validate_match_pattern_connectivity(validator, match_stmt, diagnostics);
        }
    }
}

/// Validates connectivity of a MATCH pattern.
fn validate_match_pattern_connectivity(
    validator: &super::SemanticValidator,
    match_stmt: &MatchStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::{GraphPattern, OptionalOperand};

    let validate_pattern = |pattern: &GraphPattern, diagnostics: &mut Vec<Diag>| {
        if !validator.config.warn_on_disconnected_patterns {
            return;
        }

        // Build connectivity graph
        // Each variable is a node, and edges connect variables that appear together
        let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
        let mut all_variables = HashSet::new();

        for path_pattern in &pattern.paths.patterns {
            extract_connectivity_from_path_pattern(
                validator,
                path_pattern,
                &mut adjacency,
                &mut all_variables,
            );
        }

        // If no variables or only one variable, skip connectivity check
        if all_variables.len() <= 1 {
            return;
        }

        // Perform DFS to check connectivity
        let mut visited = HashSet::new();
        let start_var = all_variables.iter().next().unwrap();
        dfs_connectivity(start_var, &adjacency, &mut visited);

        // Check if all variables were reached
        for var in &all_variables {
            if !visited.contains(var) {
                let diag = SemanticDiagBuilder::disconnected_pattern(pattern.span.clone())
                    .with_note(format!("Variable '{}' is not connected to the rest of the pattern. Consider adding an edge connecting it, or use a separate MATCH clause.", var))
                    .build();
                diagnostics.push(diag);
            }
        }
    };

    match match_stmt {
        MatchStatement::Simple(simple) => {
            validate_pattern(&simple.pattern, diagnostics);
        }
        MatchStatement::Optional(optional) => match &optional.operand {
            OptionalOperand::Match { pattern } => {
                validate_pattern(pattern, diagnostics);
            }
            OptionalOperand::Block { statements }
            | OptionalOperand::ParenthesizedBlock { statements } => {
                for stmt in statements {
                    validate_match_pattern_connectivity(validator, stmt, diagnostics);
                }
            }
        },
    }
}

/// Extracts connectivity information from a path pattern.
fn extract_connectivity_from_path_pattern(
    validator: &super::SemanticValidator,
    path_pattern: &PathPattern,
    adjacency: &mut HashMap<String, HashSet<String>>,
    all_variables: &mut HashSet<String>,
) {
    // If there's a path-level variable, add it
    if let Some(var_decl) = &path_pattern.variable_declaration {
        all_variables.insert(var_decl.variable.to_string());
    }

    extract_connectivity_from_expression(
        validator,
        &path_pattern.expression,
        adjacency,
        all_variables,
    );
}

/// Extracts connectivity from a path pattern expression.
fn extract_connectivity_from_expression(
    validator: &super::SemanticValidator,
    expr: &PathPatternExpression,
    adjacency: &mut HashMap<String, HashSet<String>>,
    all_variables: &mut HashSet<String>,
) {
    match expr {
        PathPatternExpression::Term(term) => {
            extract_connectivity_from_term(validator, term, adjacency, all_variables);
        }
        PathPatternExpression::Union { left, right, .. } => {
            extract_connectivity_from_expression(validator, left, adjacency, all_variables);
            extract_connectivity_from_expression(validator, right, adjacency, all_variables);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for alt in alternatives {
                extract_connectivity_from_term(validator, alt, adjacency, all_variables);
            }
        }
    }
}

/// Extracts connectivity from a path term (sequence of elements).
fn extract_connectivity_from_term(
    validator: &super::SemanticValidator,
    term: &PathTerm,
    adjacency: &mut HashMap<String, HashSet<String>>,
    all_variables: &mut HashSet<String>,
) {
    let mut prev_var: Option<String> = None;

    for factor in &term.factors {
        if let PathPrimary::ElementPattern(elem) = &factor.primary {
            match elem.as_ref() {
                ElementPattern::Node(node) => {
                    if let Some(var) = &node.variable {
                        let var_name = var.variable.to_string();
                        all_variables.insert(var_name.clone());

                        // Connect to previous variable if exists
                        if let Some(prev) = &prev_var {
                            adjacency
                                .entry(prev.clone())
                                .or_default()
                                .insert(var_name.clone());
                            adjacency
                                .entry(var_name.clone())
                                .or_default()
                                .insert(prev.clone());
                        }
                        prev_var = Some(var_name);
                    }
                }
                ElementPattern::Edge(edge) => {
                    // Edges connect nodes, but also can have their own variables
                    if let Some(var_name) = get_edge_variable(edge) {
                        all_variables.insert(var_name.to_string());

                        // Connect edge variable to adjacent node if exists
                        if let Some(prev) = &prev_var {
                            adjacency
                                .entry(prev.clone())
                                .or_default()
                                .insert(var_name.to_string());
                            adjacency
                                .entry(var_name.to_string())
                                .or_default()
                                .insert(prev.clone());
                        }
                    }
                }
            }
        } else if let PathPrimary::ParenthesizedExpression(nested_expr) = &factor.primary {
            extract_connectivity_from_expression(validator, nested_expr, adjacency, all_variables);
        }
    }
}

/// Gets the variable name from an edge pattern if it exists.
fn get_edge_variable<'a>(edge: &'a EdgePattern) -> Option<&'a str> {
    match edge {
        EdgePattern::Full(full) => full.filler.variable.as_ref().map(|v| v.variable.as_str()),
        EdgePattern::Abbreviated(_) => None,
    }
}

/// DFS to check connectivity of variables in the pattern.
fn dfs_connectivity(
    var: &str,
    adjacency: &HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
) {
    if visited.contains(var) {
        return;
    }

    visited.insert(var.to_string());

    if let Some(neighbors) = adjacency.get(var) {
        for neighbor in neighbors {
            dfs_connectivity(neighbor, adjacency, visited);
        }
    }
}

/// Validates patterns in a mutation for connectivity.
fn validate_mutation_patterns(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::{
        LinearDataModifyingStatement, PrimitiveDataModifyingStatement,
        SimpleDataAccessingStatement, SimpleDataModifyingStatement,
    };

    let statements = match mutation {
        LinearDataModifyingStatement::Focused(focused) => &focused.statements,
        LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
    };

    for statement in statements {
        match statement {
            SimpleDataAccessingStatement::Modifying(
                SimpleDataModifyingStatement::Primitive(primitive),
            ) => {
                match primitive {
                    PrimitiveDataModifyingStatement::Insert(insert) => {
                        // Validate INSERT patterns are connected
                        validate_insert_pattern_connectivity(validator, &insert.pattern, diagnostics);
                    }
                    PrimitiveDataModifyingStatement::Set(_)
                    | PrimitiveDataModifyingStatement::Remove(_)
                    | PrimitiveDataModifyingStatement::Delete(_) => {
                        // These don't have graph patterns to validate
                    }
                }
            }
            SimpleDataAccessingStatement::Query(_) => {
                // Query statements within mutations don't need additional pattern validation here
            }
            SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                // Procedure calls don't have patterns to validate
            }
        }
    }
}

/// Validates that an INSERT pattern is connected.
fn validate_insert_pattern_connectivity(
    validator: &super::SemanticValidator,
    pattern: &crate::ast::mutation::InsertGraphPattern,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::InsertElementPattern;
    use crate::diag::DiagSeverity;

    // Build adjacency list for INSERT pattern
    let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_variables = HashSet::new();
    let mut prev_var: Option<String> = None;

    for path in &pattern.paths {
        for element in &path.elements {
            let var_name = match element {
                InsertElementPattern::Node(node) => node
                    .filler
                    .as_ref()
                    .and_then(|f| f.variable.as_ref())
                    .map(|v| v.variable.to_string()),
                InsertElementPattern::Edge(edge) => {
                    use crate::ast::mutation::InsertEdgePattern;
                    match edge {
                        InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                        InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                        InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                    }
                    .and_then(|f| f.variable.as_ref())
                    .map(|v| v.variable.to_string())
                }
            };

            if let Some(var) = var_name {
                all_variables.insert(var.clone());

                // Connect to previous variable in path
                if let Some(prev) = &prev_var {
                    adjacency
                        .entry(prev.clone())
                        .or_default()
                        .insert(var.clone());
                    adjacency
                        .entry(var.clone())
                        .or_default()
                        .insert(prev.clone());
                }
                prev_var = Some(var);
            }
        }
        // Reset prev_var at end of path
        prev_var = None;
    }

    // Check connectivity if we have variables
    if all_variables.len() > 1 && validator.config.warn_on_disconnected_patterns {
        let mut visited = HashSet::new();
        if let Some(start_var) = all_variables.iter().next() {
            dfs_connectivity(start_var, &adjacency, &mut visited);

            if visited.len() < all_variables.len() {
                let disconnected: Vec<_> = all_variables.difference(&visited).collect();
                diagnostics.push(
                    Diag::new(DiagSeverity::Warning, format!(
                        "Disconnected INSERT pattern: variables {:?} are not connected to the main pattern",
                        disconnected
                    ))
                );
            }
        }
    }
}
