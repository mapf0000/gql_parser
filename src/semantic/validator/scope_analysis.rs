//! Scope analysis pass for semantic validation.
//!
//! This module handles the first pass of semantic validation: scope analysis.
//! It walks through the program AST and:
//! - Extracts variable definitions from MATCH, LET, FOR, and INSERT statements
//! - Builds a symbol table with proper scoping
//! - Tracks statement boundaries for variable isolation
//! - Detects variable shadowing when configured
//!
//! The scope analysis phase produces:
//! - A `SymbolTable` containing all declared variables with their scopes
//! - `ScopeMetadata` for tracking expression contexts and statement scopes

use std::collections::HashMap;

use crate::ast::program::Program;
use crate::ast::query::{
    EdgePattern, ElementPattern, ForStatement, LetStatement, LinearQuery, MatchStatement,
    PathPattern, PathPatternExpression, PathPrimary, PathTerm, PrimitiveQueryStatement, Query,
};
use crate::diag::Diag;
use crate::ir::SymbolTable;
use crate::ir::symbol_table::{ScopeId, ScopeKind, SymbolKind};

use super::ScopeMetadata;

/// Main entry point for scope analysis pass.
///
/// Walks through all statements in the program, extracting variable definitions
/// and building the symbol table with proper scoping.
///
/// Returns a tuple of:
/// - `SymbolTable`: Contains all declared variables with their scopes
/// - `ScopeMetadata`: Tracks expression contexts and statement scopes
pub(super) fn run_scope_analysis(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) -> (SymbolTable, super::ScopeMetadata) {
    let mut symbol_table = SymbolTable::new();
    let mut scope_metadata = ScopeMetadata {
        expr_contexts: HashMap::new(),
        statement_scopes: Vec::new(),
    };

    // Walk all statements in the program, tracking statement boundaries
    let mut statement_id = 0;
    for statement in &program.statements {
        match statement {
            crate::ast::program::Statement::Query(query_stmt) => {
                analyze_query(
                    validator,
                    &query_stmt.query,
                    &mut symbol_table,
                    &mut scope_metadata,
                    statement_id,
                    diagnostics,
                );
                statement_id += 1;
            }
            crate::ast::program::Statement::Mutation(mutation_stmt) => {
                analyze_mutation_with_scope(
                    validator,
                    &mutation_stmt.statement,
                    &mut symbol_table,
                    &mut scope_metadata,
                    statement_id,
                    diagnostics,
                );
                statement_id += 1;
            }
            crate::ast::program::Statement::Session(_)
            | crate::ast::program::Statement::Transaction(_)
            | crate::ast::program::Statement::Catalog(_)
            | crate::ast::program::Statement::Empty(_) => {
                // These don't introduce variables or scopes
            }
        }
    }

    (symbol_table, scope_metadata)
}

/// Analyzes a query and extracts variables, tracking statement context.
fn analyze_query(
    validator: &super::SemanticValidator,
    query: &Query,
    symbol_table: &mut SymbolTable,
    scope_metadata: &mut ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            analyze_linear_query(
                validator,
                linear_query,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Query::Composite(composite_query) => {
            // Composite queries: each side gets its own isolated scope
            // Left query uses current statement_id
            analyze_query(
                validator,
                &composite_query.left,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );

            // Right query gets a new statement_id for isolation
            let right_statement_id = statement_id + 1000; // Use high offset to avoid collision
            analyze_query(
                validator,
                &composite_query.right,
                symbol_table,
                scope_metadata,
                right_statement_id,
                diagnostics,
            );
        }
        Query::Parenthesized(query, _) => {
            analyze_query(
                validator,
                query,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
    }
}

/// Wrapper for analyze_mutation that tracks scope metadata.
fn analyze_mutation_with_scope(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    symbol_table: &mut SymbolTable,
    scope_metadata: &mut ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    let statement_scope_id = analyze_mutation(validator, mutation, symbol_table, diagnostics);
    if statement_id >= scope_metadata.statement_scopes.len() {
        scope_metadata
            .statement_scopes
            .resize(statement_id + 1, ScopeId::new(0));
    }
    scope_metadata.statement_scopes[statement_id] = statement_scope_id;
}

/// Analyzes a linear query and extracts variables from clauses.
fn analyze_linear_query(
    validator: &super::SemanticValidator,
    linear_query: &LinearQuery,
    symbol_table: &mut SymbolTable,
    scope_metadata: &mut ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    // Push a statement-local scope.
    symbol_table.push_scope(ScopeKind::Query);
    let statement_scope_id = symbol_table.current_scope();

    // Track this statement's scope
    if statement_id >= scope_metadata.statement_scopes.len() {
        scope_metadata
            .statement_scopes
            .resize(statement_id + 1, ScopeId::new(0));
    }
    scope_metadata.statement_scopes[statement_id] = statement_scope_id;

    let primitive_statements = match linear_query {
        LinearQuery::Focused(focused) => &focused.primitive_statements,
        LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
    };

    // Walk through primitive statements in order, using the old analyze method
    // We'll track expression contexts during variable validation instead
    for statement in primitive_statements {
        analyze_primitive_statement(validator, statement, symbol_table, diagnostics);
    }

    // Restore parent scope so sibling statements/branches remain isolated.
    symbol_table.pop_scope();
}

/// Analyzes a primitive query statement.
fn analyze_primitive_statement(
    validator: &super::SemanticValidator,
    statement: &PrimitiveQueryStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    match statement {
        PrimitiveQueryStatement::Match(match_stmt) => {
            analyze_match_statement(validator, match_stmt, symbol_table, diagnostics);
        }
        PrimitiveQueryStatement::Let(let_stmt) => {
            analyze_let_statement(validator, let_stmt, symbol_table, diagnostics);
        }
        PrimitiveQueryStatement::For(for_stmt) => {
            analyze_for_statement(validator, for_stmt, symbol_table, diagnostics);
        }
        PrimitiveQueryStatement::Call(_) => {
            // CALL statements are handled separately - they may reference variables
            // but don't introduce new binding variables in the scope analysis phase
        }
        PrimitiveQueryStatement::Filter(_) => {
            // FILTER statements reference existing variables in their condition
            // but don't introduce new binding variables
        }
        PrimitiveQueryStatement::OrderByAndPage(_) => {
            // ORDER BY and pagination statements reference existing variables
            // but don't introduce new binding variables
        }
        PrimitiveQueryStatement::Select(_) => {
            // SELECT statements reference existing variables in their expressions
            // but don't introduce new binding variables (unless aliased, handled elsewhere)
        }
    }
}

/// Analyzes a MATCH statement and extracts binding variables from patterns.
fn analyze_match_statement(
    validator: &super::SemanticValidator,
    match_stmt: &MatchStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::{GraphPattern, OptionalOperand};

    let mut process_pattern = |pattern: &GraphPattern, symbol_table: &mut SymbolTable| {
        extract_pattern_variables(
            validator,
            &pattern.paths.patterns,
            symbol_table,
            diagnostics,
        );
    };

    match match_stmt {
        MatchStatement::Simple(simple) => {
            process_pattern(&simple.pattern, symbol_table);
        }
        MatchStatement::Optional(optional) => match &optional.operand {
            OptionalOperand::Match { pattern } => {
                process_pattern(pattern, symbol_table);
            }
            OptionalOperand::Block { statements }
            | OptionalOperand::ParenthesizedBlock { statements } => {
                for stmt in statements {
                    analyze_match_statement(validator, stmt, symbol_table, diagnostics);
                }
            }
        },
    };
}

/// Extracts binding variables from path patterns.
fn extract_pattern_variables(
    validator: &super::SemanticValidator,
    path_patterns: &[PathPattern],
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    for path_pattern in path_patterns {
        // Extract path-level variable (e.g., p = (a)-[e]->(b))
        if let Some(path_var_decl) = &path_pattern.variable_declaration {
            let var_name = path_var_decl.variable.to_string();
            let span = path_var_decl.span.clone();

            if validator.config.warn_on_shadowing
                && let Some(existing) = symbol_table.lookup(&var_name)
            {
                // Emit variable shadowing warning
                use crate::semantic::diag::SemanticDiagBuilder;
                let diag = SemanticDiagBuilder::variable_shadowing(
                    &var_name,
                    span.clone(),
                    existing.declared_at.clone(),
                )
                .build();
                diagnostics.push(diag);
            }

            symbol_table.define(var_name, SymbolKind::BindingVariable, span);
        }

        // Extract element variables from the path expression
        extract_expression_variables(
            validator,
            &path_pattern.expression,
            symbol_table,
            diagnostics,
        );
    }
}

/// Extracts variables from a path pattern expression.
fn extract_expression_variables(
    validator: &super::SemanticValidator,
    expression: &PathPatternExpression,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    match expression {
        PathPatternExpression::Term(term) => {
            extract_term_variables(validator, term, symbol_table, diagnostics);
        }
        PathPatternExpression::Union { left, right, .. } => {
            extract_expression_variables(validator, left, symbol_table, diagnostics);
            extract_expression_variables(validator, right, symbol_table, diagnostics);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for term in alternatives {
                extract_term_variables(validator, term, symbol_table, diagnostics);
            }
        }
    }
}

/// Extracts variables from a path term.
fn extract_term_variables(
    validator: &super::SemanticValidator,
    term: &PathTerm,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    for factor in &term.factors {
        extract_primary_variables(validator, &factor.primary, symbol_table, diagnostics);
    }
}

/// Extracts variables from a path primary.
fn extract_primary_variables(
    validator: &super::SemanticValidator,
    primary: &PathPrimary,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    match primary {
        PathPrimary::ElementPattern(element) => {
            extract_element_variables(validator, element, symbol_table, diagnostics);
        }
        PathPrimary::ParenthesizedExpression(expr) => {
            extract_expression_variables(validator, expr, symbol_table, diagnostics);
        }
        PathPrimary::SimplifiedExpression(_) => {
            // Simplified expressions don't have explicit variables
        }
    }
}

/// Extracts variables from an element pattern (node or edge).
fn extract_element_variables(
    validator: &super::SemanticValidator,
    element: &ElementPattern,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    match element {
        ElementPattern::Node(node_pattern) => {
            if let Some(var_decl) = &node_pattern.variable {
                let var_name = var_decl.variable.to_string();
                let span = var_decl.span.clone();

                if validator.config.warn_on_shadowing
                    && let Some(existing) = symbol_table.lookup(&var_name)
                {
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag = SemanticDiagBuilder::variable_shadowing(
                        &var_name,
                        span.clone(),
                        existing.declared_at.clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }

                symbol_table.define(var_name, SymbolKind::BindingVariable, span);
            }
        }
        ElementPattern::Edge(edge_pattern) => {
            if let EdgePattern::Full(full_edge) = edge_pattern
                && let Some(var_decl) = &full_edge.filler.variable
            {
                let var_name = var_decl.variable.to_string();
                let span = var_decl.span.clone();

                if validator.config.warn_on_shadowing
                    && let Some(existing) = symbol_table.lookup(&var_name)
                {
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag = SemanticDiagBuilder::variable_shadowing(
                        &var_name,
                        span.clone(),
                        existing.declared_at.clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }

                symbol_table.define(var_name, SymbolKind::BindingVariable, span);
            }
        }
    }
}

/// Analyzes a LET statement and extracts variable definitions.
fn analyze_let_statement(
    validator: &super::SemanticValidator,
    let_stmt: &LetStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::semantic::diag::SemanticDiagBuilder;

    for binding in &let_stmt.bindings {
        let var_name = binding.variable.name.to_string();
        let span = binding.variable.span.clone();

        if validator.config.warn_on_shadowing
            && let Some(existing) = symbol_table.lookup(&var_name)
        {
            // Emit variable shadowing warning
            let diag = SemanticDiagBuilder::variable_shadowing(
                &var_name,
                span.clone(),
                existing.declared_at.clone(),
            )
            .build();
            diagnostics.push(diag);
        }

        symbol_table.define(var_name, SymbolKind::LetVariable, span);
    }
}

/// Analyzes a FOR statement and extracts loop variable.
fn analyze_for_statement(
    validator: &super::SemanticValidator,
    for_stmt: &ForStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::ForOrdinalityOrOffset;
    use crate::semantic::diag::SemanticDiagBuilder;

    let var_name = for_stmt.item.binding_variable.name.to_string();
    let span = for_stmt.item.binding_variable.span.clone();

    if validator.config.warn_on_shadowing
        && let Some(existing) = symbol_table.lookup(&var_name)
    {
        // Emit variable shadowing warning
        let diag = SemanticDiagBuilder::variable_shadowing(
            &var_name,
            span.clone(),
            existing.declared_at.clone(),
        )
        .build();
        diagnostics.push(diag);
    }

    symbol_table.define(var_name, SymbolKind::ForVariable, span);

    // Also handle ordinality/offset variable if present
    if let Some(ordinality_or_offset) = &for_stmt.ordinality_or_offset {
        let ord_var = match ordinality_or_offset {
            ForOrdinalityOrOffset::Ordinality { variable } => variable,
            ForOrdinalityOrOffset::Offset { variable } => variable,
        };
        let ord_var_name = ord_var.name.to_string();
        let ord_span = ord_var.span.clone();

        if validator.config.warn_on_shadowing
            && let Some(existing) = symbol_table.lookup(&ord_var_name)
        {
            // Emit variable shadowing warning for ordinality/offset variable
            let diag = SemanticDiagBuilder::variable_shadowing(
                &ord_var_name,
                ord_span.clone(),
                existing.declared_at.clone(),
            )
            .build();
            diagnostics.push(diag);
        }

        symbol_table.define(ord_var_name, SymbolKind::ForVariable, ord_span);
    }
}

/// Analyzes a mutation statement and extracts variable definitions.
fn analyze_mutation(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) -> ScopeId {
    use crate::ast::mutation::LinearDataModifyingStatement;

    // Push a statement-local mutation scope.
    let statement_scope_id = symbol_table.push_scope(ScopeKind::Query);

    match mutation {
        LinearDataModifyingStatement::Focused(focused) => {
            // Analyze all data accessing statements
            for stmt in &focused.statements {
                analyze_data_accessing_statement(validator, stmt, symbol_table, diagnostics);
            }
        }
        LinearDataModifyingStatement::Ambient(ambient) => {
            // Analyze all data accessing statements
            for stmt in &ambient.statements {
                analyze_data_accessing_statement(validator, stmt, symbol_table, diagnostics);
            }
        }
    }

    // Restore parent scope so sibling statements remain isolated.
    symbol_table.pop_scope();
    statement_scope_id
}

/// Analyzes a data accessing statement (query or mutation).
fn analyze_data_accessing_statement(
    validator: &super::SemanticValidator,
    stmt: &crate::ast::mutation::SimpleDataAccessingStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::SimpleDataAccessingStatement;

    match stmt {
        SimpleDataAccessingStatement::Query(query_stmt) => {
            // Analyze query statement
            analyze_primitive_statement(validator, query_stmt, symbol_table, diagnostics);
        }
        SimpleDataAccessingStatement::Modifying(modifying_stmt) => {
            analyze_modifying_statement(validator, modifying_stmt, symbol_table, diagnostics);
        }
    }
}

/// Analyzes a data modifying statement.
fn analyze_modifying_statement(
    validator: &super::SemanticValidator,
    stmt: &crate::ast::mutation::SimpleDataModifyingStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::SimpleDataModifyingStatement;

    match stmt {
        SimpleDataModifyingStatement::Primitive(primitive) => {
            analyze_primitive_modifying_statement(validator, primitive, symbol_table, diagnostics);
        }
        SimpleDataModifyingStatement::Call(_call_stmt) => {
            // CALL statements may yield variables - would need to analyze YIELD clause
            // For now, this is a placeholder for future implementation
        }
    }
}

/// Analyzes a primitive data modifying statement (INSERT/SET/REMOVE/DELETE).
fn analyze_primitive_modifying_statement(
    validator: &super::SemanticValidator,
    stmt: &crate::ast::mutation::PrimitiveDataModifyingStatement,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::PrimitiveDataModifyingStatement;

    match stmt {
        PrimitiveDataModifyingStatement::Insert(insert_stmt) => {
            // Extract variables from INSERT patterns
            analyze_insert_pattern(validator, &insert_stmt.pattern, symbol_table, diagnostics);
        }
        PrimitiveDataModifyingStatement::Set(_set_stmt) => {
            // SET statements reference existing variables but don't define new ones
        }
        PrimitiveDataModifyingStatement::Remove(_remove_stmt) => {
            // REMOVE statements reference existing variables but don't define new ones
        }
        PrimitiveDataModifyingStatement::Delete(_delete_stmt) => {
            // DELETE statements reference existing variables but don't define new ones
        }
    }
}

/// Analyzes an INSERT pattern and extracts variable definitions.
fn analyze_insert_pattern(
    validator: &super::SemanticValidator,
    pattern: &crate::ast::mutation::InsertGraphPattern,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::semantic::diag::SemanticDiagBuilder;

    // Walk all insert path patterns
    for path in &pattern.paths {
        for element in &path.elements {
            use crate::ast::mutation::InsertElementPattern;

            match element {
                InsertElementPattern::Node(node_pattern) => {
                    if let Some(filler) = &node_pattern.filler
                        && let Some(var_decl) = &filler.variable
                    {
                        let var_name = var_decl.variable.to_string();
                        let span = var_decl.span.clone();

                        if validator.config.warn_on_shadowing
                            && let Some(existing) = symbol_table.lookup(&var_name)
                        {
                            let diag = SemanticDiagBuilder::variable_shadowing(
                                &var_name,
                                span.clone(),
                                existing.declared_at.clone(),
                            )
                            .build();
                            diagnostics.push(diag);
                        }

                        symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                    }
                }
                InsertElementPattern::Edge(edge_pattern) => {
                    let filler_opt = match edge_pattern {
                        crate::ast::mutation::InsertEdgePattern::PointingLeft(edge) => &edge.filler,
                        crate::ast::mutation::InsertEdgePattern::PointingRight(edge) => {
                            &edge.filler
                        }
                        crate::ast::mutation::InsertEdgePattern::Undirected(edge) => &edge.filler,
                    };

                    if let Some(filler) = filler_opt
                        && let Some(var_decl) = &filler.variable
                    {
                        let var_name = var_decl.variable.to_string();
                        let span = var_decl.span.clone();

                        if validator.config.warn_on_shadowing
                            && let Some(existing) = symbol_table.lookup(&var_name)
                        {
                            let diag = SemanticDiagBuilder::variable_shadowing(
                                &var_name,
                                span.clone(),
                                existing.declared_at.clone(),
                            )
                            .build();
                            diagnostics.push(diag);
                        }

                        symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                    }
                }
            }
        }
    }
}
