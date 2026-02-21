//! Variable validation pass for semantic analysis.
//!
//! This module validates variable references in queries, ensuring:
//! - Variables are defined before use
//! - Reference-site-aware scope lookups
//! - ISO GQL aggregation rules (WHERE, HAVING, RETURN)
//! - Nested aggregation detection

use crate::ast::program::{Program, Statement};
use crate::ast::query::{MatchStatement, PrimitiveQueryStatement, Query};
use crate::diag::Diag;
use crate::ir::SymbolTable;
use crate::ir::symbol_table::ScopeId;

/// Runs variable validation pass on the program.
pub(super) fn run_variable_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    diagnostics: &mut Vec<Diag>,
) {
    // Walk all statements and check variable references with statement-level scope tracking
    let mut next_statement_id = 0usize;
    for statement in &program.statements {
        match statement {
            Statement::Query(query_stmt) => {
                let statement_id = next_statement_id;
                next_statement_id += 1;
                validate_query_variables(
                    validator,
                    &query_stmt.query,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    &mut next_statement_id,
                    diagnostics,
                );
            }
            Statement::Mutation(mutation_stmt) => {
                let statement_id = next_statement_id;
                next_statement_id += 1;
                validate_mutation_variables(
                    validator,
                    &mutation_stmt.statement,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            _ => {
                // Other statements don't need variable validation at this level
            }
        }
    }
}

fn statement_scope_id(
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
) -> ScopeId {
    scope_metadata
        .statement_scopes
        .get(statement_id)
        .copied()
        .unwrap_or_else(|| symbol_table.current_scope())
}

/// Validates variable references in a query.
fn validate_query_variables(
    validator: &super::SemanticValidator,
    query: &Query,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    next_statement_id: &mut usize,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            validate_linear_query_variables(
                validator,
                linear_query,
                symbol_table,
                scope_metadata,
                statement_id,
                next_statement_id,
                diagnostics,
            );
        }
        Query::Composite(composite) => {
            validate_query_variables(
                validator,
                &composite.left,
                symbol_table,
                scope_metadata,
                statement_id,
                next_statement_id,
                diagnostics,
            );

            let right_statement_id = *next_statement_id;
            *next_statement_id += 1;
            validate_query_variables(
                validator,
                &composite.right,
                symbol_table,
                scope_metadata,
                right_statement_id,
                next_statement_id,
                diagnostics,
            );
        }
        Query::Parenthesized(query, _) => {
            validate_query_variables(
                validator,
                query,
                symbol_table,
                scope_metadata,
                statement_id,
                next_statement_id,
                diagnostics,
            );
        }
    }
}

/// Validates variable references in a mutation statement.
fn validate_mutation_variables(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    // Validate all simple data-accessing statements (query and mutation statements)
    for stmt in &mutation.statements {
        validate_simple_data_accessing_statement(
            validator,
            stmt,
            symbol_table,
            scope_metadata,
            statement_id,
            diagnostics,
        );
    }

    // Validate RETURN statement if present
    if let Some(result_stmt) = &mutation.primitive_result_statement {
        validate_result_statement_variables(
            validator,
            result_stmt,
            symbol_table,
            scope_metadata,
            statement_id,
            diagnostics,
        );
    }
}

/// Validates variable references in a simple data-accessing statement.
fn validate_simple_data_accessing_statement(
    _validator: &super::SemanticValidator,
    statement: &crate::ast::mutation::SimpleDataAccessingStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::SimpleDataAccessingStatement;

    match statement {
        SimpleDataAccessingStatement::Query(query_stmt) => {
            // Query statements in mutations (like MATCH, FILTER, etc.) define and use variables
            // within the same mutation scope. We need to validate any expressions they contain.
            // For MATCH statements, we primarily need to validate WHERE clauses.
            validate_query_statement_in_mutation(
                query_stmt,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        SimpleDataAccessingStatement::Modifying(modifying_stmt) => {
            validate_simple_data_modifying_statement(
                modifying_stmt,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
    }
}

/// Validates expressions in query statements within mutations.
fn validate_query_statement_in_mutation(
    statement: &crate::ast::query::PrimitiveQueryStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::PrimitiveQueryStatement;

    match statement {
        PrimitiveQueryStatement::Match(match_stmt) => {
            // MATCH patterns define variables, they don't reference them (except in WHERE clauses)
            // Validate WHERE clause if present
            use crate::ast::query::MatchStatement;
            match match_stmt {
                MatchStatement::Simple(simple) => {
                    if let Some(where_clause) = &simple.pattern.where_clause {
                        validate_expression_variables(
                            &where_clause.condition,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );

                        // ISO GQL: Check for illegal aggregation in WHERE clause
                        check_where_clause_aggregation(&where_clause.condition, diagnostics);
                    }
                }
                MatchStatement::Optional(_) => {
                    // Optional matches would be validated here if needed
                }
            }
        }
        PrimitiveQueryStatement::Let(let_stmt) => {
            // Validate LET value expressions
            for binding in &let_stmt.bindings {
                validate_expression_variables(
                    &binding.value,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        PrimitiveQueryStatement::For(for_stmt) => {
            // Validate FOR collection expression
            validate_expression_variables(
                &for_stmt.item.collection,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        PrimitiveQueryStatement::Filter(filter) => {
            // Validate FILTER condition
            validate_expression_variables(
                &filter.condition,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        PrimitiveQueryStatement::OrderByAndPage(order_by_page) => {
            // Validate ORDER BY expressions
            if let Some(order_by) = &order_by_page.order_by {
                for sort_spec in &order_by.sort_specifications {
                    validate_expression_variables(
                        &sort_spec.key,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            // Validate LIMIT/OFFSET expressions if present
            if let Some(limit) = &order_by_page.limit {
                validate_expression_variables(
                    &limit.count,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            if let Some(offset) = &order_by_page.offset {
                validate_expression_variables(
                    &offset.count,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        _ => {
            // Other query statements (SELECT, CALL) handled elsewhere or not applicable in mutations
        }
    }
}

/// Validates variable references in a simple data modifying statement.
fn validate_simple_data_modifying_statement(
    modifying_stmt: &crate::ast::mutation::SimpleDataModifyingStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::SimpleDataModifyingStatement;

    match modifying_stmt {
        SimpleDataModifyingStatement::Primitive(primitive) => {
            validate_primitive_data_modifying_statement(
                primitive,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        SimpleDataModifyingStatement::Call(_call_stmt) => {
            // CALL statements in mutation context - validate procedure arguments
            // Similar to query CALL validation (already handled in query validation if needed)
            // For now, skip as procedure argument validation is handled elsewhere
        }
    }
}

/// Validates variable references in primitive data modifying statements.
fn validate_primitive_data_modifying_statement(
    statement: &crate::ast::mutation::PrimitiveDataModifyingStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::PrimitiveDataModifyingStatement;
    use crate::semantic::diag::undefined_variable;

    let scope_to_check = statement_scope_id(symbol_table, scope_metadata, statement_id);

    match statement {
        PrimitiveDataModifyingStatement::Insert(insert_stmt) => {
            // INSERT statements define new variables, but they can reference existing variables
            // in property value expressions. Node variables themselves can be implicitly created.
            validate_insert_statement(
                &insert_stmt.pattern,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        PrimitiveDataModifyingStatement::Set(set_stmt) => {
            // Validate SET items - check that element variables are in scope
            for item in &set_stmt.items.items {
                use crate::ast::mutation::SetItem;
                match item {
                    SetItem::Property(prop_item) => {
                        // Check element variable is in scope
                        if symbol_table
                            .lookup_from(scope_to_check, prop_item.element.as_str())
                            .is_none()
                        {
                            let diag = undefined_variable(
                                prop_item.element.as_str(),
                                prop_item.span.clone(),
                            );
                            diagnostics.push(diag);
                        }
                        // Validate value expression
                        validate_expression_variables(
                            &prop_item.value,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    SetItem::AllProperties(all_props_item) => {
                        // Check element variable is in scope
                        if symbol_table
                            .lookup_from(scope_to_check, all_props_item.element.as_str())
                            .is_none()
                        {
                            let diag = undefined_variable(
                                all_props_item.element.as_str(),
                                all_props_item.span.clone(),
                            );
                            diagnostics.push(diag);
                        }
                        // Validate property value expressions in the property specification
                        for field in &all_props_item.properties.properties {
                            validate_expression_variables(
                                &field.value,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                    SetItem::Label(label_item) => {
                        // Check element variable is in scope
                        if symbol_table
                            .lookup_from(scope_to_check, label_item.element.as_str())
                            .is_none()
                        {
                            let diag = undefined_variable(
                                label_item.element.as_str(),
                                label_item.span.clone(),
                            );
                            diagnostics.push(diag);
                        }
                    }
                }
            }
        }
        PrimitiveDataModifyingStatement::Remove(remove_stmt) => {
            // Validate REMOVE items - check that element variables are in scope
            for item in &remove_stmt.items.items {
                use crate::ast::mutation::RemoveItem;
                match item {
                    RemoveItem::Property(prop_item) => {
                        // Check element variable is in scope
                        if symbol_table
                            .lookup_from(scope_to_check, prop_item.element.as_str())
                            .is_none()
                        {
                            let diag = undefined_variable(
                                prop_item.element.as_str(),
                                prop_item.span.clone(),
                            );
                            diagnostics.push(diag);
                        }
                    }
                    RemoveItem::Label(label_item) => {
                        // Check element variable is in scope
                        if symbol_table
                            .lookup_from(scope_to_check, label_item.element.as_str())
                            .is_none()
                        {
                            let diag = undefined_variable(
                                label_item.element.as_str(),
                                label_item.span.clone(),
                            );
                            diagnostics.push(diag);
                        }
                    }
                }
            }
        }
        PrimitiveDataModifyingStatement::Delete(delete_stmt) => {
            // Validate DELETE items - each item is an expression that should reference a variable
            for item in &delete_stmt.items.items {
                validate_expression_variables(
                    &item.expression,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
    }
}

/// Validates variable references in a linear query.
fn validate_linear_query_variables(
    validator: &super::SemanticValidator,
    linear_query: &crate::ast::query::LinearQuery,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    next_statement_id: &mut usize,
    diagnostics: &mut Vec<Diag>,
) {
    // Validate all primitive statements
    for stmt in &linear_query.primitive_statements {
        validate_primitive_statement_variables(
            validator,
            stmt,
            symbol_table,
            scope_metadata,
            statement_id,
            next_statement_id,
            diagnostics,
        );
    }

    // Validate RETURN statement variables
    if let Some(result_stmt) = &linear_query.result_statement {
        validate_result_statement_variables(
            validator,
            result_stmt.as_ref(),
            symbol_table,
            scope_metadata,
            statement_id,
            diagnostics,
        );
    }
}

/// Validates variable references in primitive statements.
fn validate_primitive_statement_variables(
    validator: &super::SemanticValidator,
    statement: &PrimitiveQueryStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    next_statement_id: &mut usize,
    diagnostics: &mut Vec<Diag>,
) {
    match statement {
        PrimitiveQueryStatement::Match(match_stmt) => {
            // MATCH patterns define variables, they don't reference them (except in WHERE clauses)
            // Validate WHERE clause if present
            match match_stmt {
                MatchStatement::Simple(simple) => {
                    if let Some(where_clause) = &simple.pattern.where_clause {
                        validate_expression_variables(
                            &where_clause.condition,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );

                        // ISO GQL: Check for illegal aggregation in WHERE clause
                        check_where_clause_aggregation(&where_clause.condition, diagnostics);
                    }
                }
                MatchStatement::Optional(optional) => match &optional.operand {
                    crate::ast::query::OptionalOperand::Match { pattern } => {
                        if let Some(where_clause) = &pattern.where_clause {
                            validate_expression_variables(
                                &where_clause.condition,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );

                            // ISO GQL: Check for illegal aggregation in WHERE clause
                            check_where_clause_aggregation(&where_clause.condition, diagnostics);
                        }
                    }
                    crate::ast::query::OptionalOperand::Block { statements }
                    | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                        for stmt in statements {
                            validate_match_statement_variables(
                                stmt,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                },
            }
        }
        PrimitiveQueryStatement::Let(let_stmt) => {
            // Validate LET value expressions
            for binding in &let_stmt.bindings {
                validate_expression_variables(
                    &binding.value,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        PrimitiveQueryStatement::For(for_stmt) => {
            // Validate FOR collection expression
            validate_expression_variables(
                &for_stmt.item.collection,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        PrimitiveQueryStatement::Filter(filter) => {
            // Validate FILTER condition
            validate_expression_variables(
                &filter.condition,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );

            // ISO GQL: Check for illegal aggregation in WHERE clause
            if expression_contains_aggregation(&filter.condition) {
                use crate::semantic::diag::aggregation_error;
                let diag = aggregation_error(
                    "Aggregation functions not allowed in WHERE clause (use HAVING instead)",
                    filter.condition.span().clone(),
                );
                diagnostics.push(diag);
            }
        }
        PrimitiveQueryStatement::OrderByAndPage(order_by_page) => {
            // Validate ORDER BY expressions
            if let Some(order_by) = &order_by_page.order_by {
                for sort_spec in &order_by.sort_specifications {
                    validate_expression_variables(
                        &sort_spec.key,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            // Validate LIMIT/OFFSET expressions if present
            if let Some(limit) = &order_by_page.limit {
                validate_expression_variables(
                    &limit.count,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            if let Some(offset) = &order_by_page.offset {
                validate_expression_variables(
                    &offset.count,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        PrimitiveQueryStatement::Select(select) => {
            if let Some(with_clause) = &select.with_clause {
                for cte in &with_clause.items {
                    validate_query_variables(
                        validator,
                        &cte.query,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        next_statement_id,
                        diagnostics,
                    );
                }
            }

            // Validate SELECT expressions
            match &select.select_items {
                crate::ast::query::SelectItemList::Star => {
                    // * doesn't reference specific expressions
                }
                crate::ast::query::SelectItemList::Items { items } => {
                    for item in items {
                        validate_expression_variables(
                            &item.expression,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }

            if let Some(from_clause) = &select.from_clause {
                match from_clause {
                    crate::ast::query::SelectFromClause::GraphMatchList { matches } => {
                        for pattern in matches {
                            if let Some(where_clause) = &pattern.where_clause {
                                validate_expression_variables(
                                    &where_clause.condition,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                    }
                    crate::ast::query::SelectFromClause::QuerySpecification { query, .. } => {
                        validate_query_variables(
                            validator,
                            query,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            next_statement_id,
                            diagnostics,
                        );
                    }
                    crate::ast::query::SelectFromClause::GraphAndQuerySpecification {
                        graph,
                        query,
                        ..
                    } => {
                        validate_expression_variables(
                            graph,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        validate_query_variables(
                            validator,
                            query,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            next_statement_id,
                            diagnostics,
                        );
                    }
                    crate::ast::query::SelectFromClause::SourceList { sources } => {
                        for source in sources {
                            match source {
                                crate::ast::query::SelectSourceItem::Query { query, .. } => {
                                    validate_query_variables(
                                        validator,
                                        query,
                                        symbol_table,
                                        scope_metadata,
                                        statement_id,
                                        next_statement_id,
                                        diagnostics,
                                    );
                                }
                                crate::ast::query::SelectSourceItem::GraphAndQuery {
                                    graph,
                                    query,
                                    ..
                                } => {
                                    validate_expression_variables(
                                        graph,
                                        symbol_table,
                                        scope_metadata,
                                        statement_id,
                                        diagnostics,
                                    );
                                    validate_query_variables(
                                        validator,
                                        query,
                                        symbol_table,
                                        scope_metadata,
                                        statement_id,
                                        next_statement_id,
                                        diagnostics,
                                    );
                                }
                                crate::ast::query::SelectSourceItem::Expression {
                                    expression,
                                    ..
                                } => {
                                    validate_expression_variables(
                                        expression,
                                        symbol_table,
                                        scope_metadata,
                                        statement_id,
                                        diagnostics,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Validate GROUP BY if present
            if let Some(group_by) = &select.group_by {
                for elem in &group_by.elements {
                    if let crate::ast::query::GroupingElement::Expression(expr) = elem {
                        validate_expression_variables(
                            expr,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }
            // Validate HAVING if present
            if let Some(having) = &select.having {
                validate_expression_variables(
                    &having.condition,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );

                // ISO GQL: Validate HAVING clause semantics
                validate_having_clause(validator, &having.condition, &select.group_by, diagnostics);
            }
        }
        PrimitiveQueryStatement::Call(call_stmt) => {
            // Validate CALL statement arguments and yields
            use crate::ast::procedure::ProcedureCall;

            match &call_stmt.call {
                ProcedureCall::Named(named_call) => {
                    // Validate arguments if present
                    if let Some(args) = &named_call.arguments {
                        for arg in &args.arguments {
                            validate_expression_variables(
                                &arg.expression,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                    // YIELD clause variables are outputs, not inputs - don't validate them here
                }
                ProcedureCall::Inline(inline_call) => {
                    // Inline calls don't have traditional arguments but may reference variables
                    // in their variable scope clause - those should already be validated
                    // The nested specification would need recursive validation (future work)
                    if let Some(var_scope) = &inline_call.variable_scope {
                        let scope_to_check =
                            statement_scope_id(symbol_table, scope_metadata, statement_id);

                        // Validate that variables in the scope clause are defined
                        for var in &var_scope.variables {
                            if symbol_table
                                .lookup_from(scope_to_check, var.name.as_ref())
                                .is_none()
                            {
                                use crate::semantic::diag::undefined_variable;
                                let diag = undefined_variable(&var.name, var.span.clone());
                                diagnostics.push(diag);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Validates variable references in a MATCH statement (helper for nested validations).
fn validate_match_statement_variables(
    match_stmt: &MatchStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    match match_stmt {
        MatchStatement::Simple(simple) => {
            if let Some(where_clause) = &simple.pattern.where_clause {
                validate_expression_variables(
                    &where_clause.condition,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );

                // ISO GQL: Check for illegal aggregation in WHERE clause
                check_where_clause_aggregation(&where_clause.condition, diagnostics);
            }
        }
        MatchStatement::Optional(optional) => match &optional.operand {
            crate::ast::query::OptionalOperand::Match { pattern } => {
                if let Some(where_clause) = &pattern.where_clause {
                    validate_expression_variables(
                        &where_clause.condition,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );

                    // ISO GQL: Check for illegal aggregation in WHERE clause
                    check_where_clause_aggregation(&where_clause.condition, diagnostics);
                }
            }
            crate::ast::query::OptionalOperand::Block { statements }
            | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                for stmt in statements {
                    validate_match_statement_variables(
                        stmt,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
        },
    }
}

/// Validates variable references in a result statement (RETURN).
fn validate_result_statement_variables(
    validator: &super::SemanticValidator,
    result_stmt: &crate::ast::query::PrimitiveResultStatement,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::{PrimitiveResultStatement, ReturnItemList};

    if let PrimitiveResultStatement::Return(return_stmt) = result_stmt {
        // Validate each return item
        match &return_stmt.items {
            ReturnItemList::Star => {
                // * doesn't reference specific variables
            }
            ReturnItemList::Items { items } => {
                for item in items {
                    validate_expression_variables(
                        &item.expression,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
        }

        // Validate aggregation rules for RETURN (ISO GQL compliance)
        validate_return_aggregation(validator, return_stmt, diagnostics);
    }
}

/// Validates aggregation rules in RETURN statements per ISO GQL standard.
/// Cannot mix aggregated and non-aggregated expressions without GROUP BY.
fn validate_return_aggregation(
    validator: &super::SemanticValidator,
    return_stmt: &crate::ast::query::ReturnStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::ReturnItemList;
    use crate::semantic::diag::aggregation_error;

    // Check if mixing aggregated and non-aggregated expressions
    let (has_aggregation, non_aggregated_expressions) = match &return_stmt.items {
        ReturnItemList::Star => (false, vec![]),
        ReturnItemList::Items { items } => {
            let mut has_agg = false;
            let mut non_agg_exprs = Vec::new();

            for item in items {
                if expression_contains_aggregation(&item.expression) {
                    has_agg = true;
                } else {
                    non_agg_exprs.push(&item.expression);
                }
            }
            (has_agg, non_agg_exprs)
        }
    };

    // In strict mode or with GROUP BY, mixing requires GROUP BY
    // RETURN doesn't have GROUP BY, so this is an error in strict mode
    if has_aggregation && !non_aggregated_expressions.is_empty() && validator.config.strict_mode {
        for expr in non_aggregated_expressions {
            let diag = aggregation_error(
                "Cannot mix aggregated and non-aggregated expressions in RETURN without GROUP BY",
                expr.span().clone(),
            );
            diagnostics.push(diag);
        }
    }

    // Check for nested aggregation
    if let ReturnItemList::Items { items } = &return_stmt.items {
        for item in items {
            check_nested_aggregation(&item.expression, false, diagnostics);
        }
    }
}

/// Validates variable references in an expression with reference-site-aware lookups.
fn validate_expression_variables(
    expression: &crate::ast::expression::Expression,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::Expression;
    use crate::semantic::diag::undefined_variable;

    match expression {
        Expression::VariableReference(var_name, span) => {
            // Use reference-site-aware lookup from the statement-local scope.
            let scope_to_check = statement_scope_id(symbol_table, scope_metadata, statement_id);

            // Perform lookup from the correct scope
            if symbol_table.lookup_from(scope_to_check, var_name).is_none() {
                // Generate undefined variable diagnostic
                let diag = undefined_variable(var_name.as_str(), span.clone());
                diagnostics.push(diag);
            }
        }
        Expression::Binary(_, left, right, _) => {
            validate_expression_variables(
                left,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
            validate_expression_variables(
                right,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::Unary(_, operand, _) => {
            validate_expression_variables(
                operand,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::Comparison(_, left, right, _) => {
            validate_expression_variables(
                left,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
            validate_expression_variables(
                right,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::Logical(_, left, right, _) => {
            validate_expression_variables(
                left,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
            validate_expression_variables(
                right,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::PropertyReference(object, _, _) => {
            validate_expression_variables(
                object,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::Parenthesized(expr, _) => {
            validate_expression_variables(
                expr,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::FunctionCall(func_call) => {
            // Validate function arguments
            for arg in &func_call.arguments {
                validate_expression_variables(
                    arg,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        Expression::Case(case_expr) => {
            // Validate CASE expression
            match case_expr {
                crate::ast::expression::CaseExpression::Searched(searched) => {
                    for when in &searched.when_clauses {
                        validate_expression_variables(
                            &when.condition,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        validate_expression_variables(
                            &when.then_result,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    if let Some(else_expr) = &searched.else_clause {
                        validate_expression_variables(
                            else_expr,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
                crate::ast::expression::CaseExpression::Simple(simple) => {
                    validate_expression_variables(
                        &simple.operand,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    for when in &simple.when_clauses {
                        validate_expression_variables(
                            &when.when_value,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        validate_expression_variables(
                            &when.then_result,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    if let Some(else_expr) = &simple.else_clause {
                        validate_expression_variables(
                            else_expr,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }
        }
        Expression::Cast(cast) => {
            // Validate cast operand
            validate_expression_variables(
                &cast.operand,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::AggregateFunction(agg_func) => {
            // Validate aggregate function arguments
            match &**agg_func {
                crate::ast::expression::AggregateFunction::CountStar { .. } => {
                    // COUNT(*) has no expression to validate
                }
                crate::ast::expression::AggregateFunction::GeneralSetFunction(gsf) => {
                    validate_expression_variables(
                        &gsf.expression,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                crate::ast::expression::AggregateFunction::BinarySetFunction(bsf) => {
                    validate_expression_variables(
                        &bsf.inverse_distribution_argument,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    validate_expression_variables(
                        &bsf.expression,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
        }
        Expression::TypeAnnotation(inner, _, _) => {
            // Validate annotated expression
            validate_expression_variables(
                inner,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::ListConstructor(elements, _) => {
            // Validate list element expressions
            for elem in elements {
                validate_expression_variables(
                    elem,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        Expression::RecordConstructor(fields, _) => {
            // Validate record field expressions
            for field in fields {
                validate_expression_variables(
                    &field.value,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        Expression::PathConstructor(elements, _) => {
            // Validate path element expressions
            for elem in elements {
                validate_expression_variables(
                    elem,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
        Expression::Exists(exists_expr) => {
            // Validate EXISTS predicate - contains a nested query/pattern
            use crate::ast::expression::ExistsVariant;
            match &exists_expr.variant {
                ExistsVariant::GraphPattern(_) => {
                    // Graph pattern validation is placeholder for Sprint 8
                    // No variable validation needed yet
                }
                ExistsVariant::Subquery(subquery_expr) => {
                    // Validate the subquery expression recursively
                    // NOTE: Subqueries should have their own isolated scope, but that requires
                    // more complex scope tracking during analysis. For now, use same statement_id.
                    validate_expression_variables(
                        subquery_expr,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
        }
        Expression::Predicate(predicate) => {
            // Validate predicate expressions
            use crate::ast::expression::Predicate;
            match predicate {
                Predicate::IsNull(operand, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsTyped(operand, _, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsNormalized(operand, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsDirected(operand, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsLabeled(operand, _, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsTruthValue(operand, _, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::IsSource(operand, of, _, _)
                | Predicate::IsDestination(operand, of, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    validate_expression_variables(
                        of.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::AllDifferent(operands, _) => {
                    for operand in operands {
                        validate_expression_variables(
                            operand,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
                Predicate::Same(left, right, _) => {
                    validate_expression_variables(
                        left.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    validate_expression_variables(
                        right.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                Predicate::PropertyExists(operand, _, _) => {
                    validate_expression_variables(
                        operand.as_ref(),
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
        }
        Expression::GraphExpression(inner, _) => {
            // Validate graph expression
            validate_expression_variables(
                inner,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::BindingTableExpression(inner, _) => {
            // Validate binding table expression
            validate_expression_variables(
                inner,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
        Expression::SubqueryExpression(_, _) => {
            // Nested query specifications are validated through statement-level scope analysis.
        }
        Expression::Literal(_, _) | Expression::ParameterReference(_, _) => {
            // Literals and parameters don't reference variables
        }
    }
}

/// Validates variable references in an INSERT statement.
/// This includes:
/// - Property value expressions in node/edge patterns (must reference existing variables)
///   Note: Node variables themselves can be new or reference existing nodes (implicit creation allowed)
fn validate_insert_statement(
    pattern: &crate::ast::mutation::InsertGraphPattern,
    symbol_table: &SymbolTable,
    scope_metadata: &super::ScopeMetadata,
    statement_id: usize,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::InsertElementPattern;

    // Validate property value expressions in INSERT patterns.
    // Property expressions must reference variables that are in scope.
    // Node variables themselves can be newly created (implicit creation is allowed in INSERT).

    for path in &pattern.paths {
        for element in &path.elements {
            match element {
                InsertElementPattern::Node(node_pattern) => {
                    // Validate property value expressions
                    if let Some(filler) = &node_pattern.filler {
                        if let Some(props) = &filler.properties {
                            for prop_pair in &props.properties {
                                validate_expression_variables(
                                    &prop_pair.value,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                    }
                }
                InsertElementPattern::Edge(edge_pattern) => {
                    // Validate property value expressions in edges
                    let filler_opt = match edge_pattern {
                        crate::ast::mutation::InsertEdgePattern::PointingLeft(edge) => &edge.filler,
                        crate::ast::mutation::InsertEdgePattern::PointingRight(edge) => {
                            &edge.filler
                        }
                        crate::ast::mutation::InsertEdgePattern::Undirected(edge) => &edge.filler,
                    };

                    if let Some(filler) = filler_opt {
                        if let Some(props) = &filler.properties {
                            for prop_pair in &props.properties {
                                validate_expression_variables(
                                    &prop_pair.value,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Checks if an expression contains aggregation functions.
fn expression_contains_aggregation(expr: &crate::ast::expression::Expression) -> bool {
    use crate::ast::expression::Expression;

    match expr {
        Expression::AggregateFunction(_) => true,
        Expression::Binary(_, left, right, _) => {
            expression_contains_aggregation(left) || expression_contains_aggregation(right)
        }
        Expression::Unary(_, operand, _) => expression_contains_aggregation(operand),
        Expression::PropertyReference(base, _, _) => expression_contains_aggregation(base),
        Expression::Parenthesized(inner, _) => expression_contains_aggregation(inner),
        Expression::Comparison(_, left, right, _) => {
            expression_contains_aggregation(left) || expression_contains_aggregation(right)
        }
        Expression::Logical(_, left, right, _) => {
            expression_contains_aggregation(left) || expression_contains_aggregation(right)
        }
        _ => false,
    }
}

/// Checks for illegal aggregation in WHERE clause per ISO GQL standard.
/// WHERE clauses cannot contain aggregation functions - use HAVING instead.
fn check_where_clause_aggregation(
    where_expr: &crate::ast::expression::Expression,
    diagnostics: &mut Vec<Diag>,
) {
    if expression_contains_aggregation(where_expr) {
        use crate::semantic::diag::aggregation_error;
        let diag = aggregation_error(
            "Aggregation functions not allowed in WHERE clause (use HAVING instead)",
            where_expr.span().clone(),
        );
        diagnostics.push(diag);
    }
}

/// Checks for illegal nested aggregation functions per ISO GQL standard.
/// Nested aggregations like COUNT(SUM(x)) are not allowed.
fn check_nested_aggregation(
    expr: &crate::ast::expression::Expression,
    in_aggregate: bool,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::{AggregateFunction, Expression};
    use crate::semantic::diag::aggregation_error;

    match expr {
        Expression::AggregateFunction(agg_func) => {
            if in_aggregate {
                // Nested aggregation detected!
                let diag = aggregation_error(
                    "Nested aggregation functions are not allowed",
                    expr.span().clone(),
                );
                diagnostics.push(diag);
                return; // Don't recurse further
            }

            // Check arguments with in_aggregate=true
            match &**agg_func {
                AggregateFunction::CountStar { .. } => {}
                AggregateFunction::GeneralSetFunction(gsf) => {
                    check_nested_aggregation(&gsf.expression, true, diagnostics);
                }
                AggregateFunction::BinarySetFunction(bsf) => {
                    check_nested_aggregation(&bsf.expression, true, diagnostics);
                    check_nested_aggregation(&bsf.inverse_distribution_argument, true, diagnostics);
                }
            }
        }
        Expression::Binary(_, left, right, _) => {
            check_nested_aggregation(left, in_aggregate, diagnostics);
            check_nested_aggregation(right, in_aggregate, diagnostics);
        }
        Expression::Unary(_, operand, _) => {
            check_nested_aggregation(operand, in_aggregate, diagnostics);
        }
        Expression::PropertyReference(base, _, _) => {
            check_nested_aggregation(base, in_aggregate, diagnostics);
        }
        Expression::Parenthesized(inner, _) => {
            check_nested_aggregation(inner, in_aggregate, diagnostics);
        }
        Expression::Comparison(_, left, right, _) => {
            check_nested_aggregation(left, in_aggregate, diagnostics);
            check_nested_aggregation(right, in_aggregate, diagnostics);
        }
        Expression::Logical(_, left, right, _) => {
            check_nested_aggregation(left, in_aggregate, diagnostics);
            check_nested_aggregation(right, in_aggregate, diagnostics);
        }
        Expression::FunctionCall(func) => {
            for arg in &func.arguments {
                check_nested_aggregation(arg, in_aggregate, diagnostics);
            }
        }
        Expression::ListConstructor(exprs, _) => {
            for e in exprs {
                check_nested_aggregation(e, in_aggregate, diagnostics);
            }
        }
        Expression::RecordConstructor(fields, _) => {
            for field in fields {
                check_nested_aggregation(&field.value, in_aggregate, diagnostics);
            }
        }
        Expression::PathConstructor(exprs, _) => {
            for e in exprs {
                check_nested_aggregation(e, in_aggregate, diagnostics);
            }
        }
        Expression::Case(case_expr) => match case_expr {
            crate::ast::expression::CaseExpression::Searched(searched) => {
                for when in &searched.when_clauses {
                    check_nested_aggregation(&when.condition, in_aggregate, diagnostics);
                    check_nested_aggregation(&when.then_result, in_aggregate, diagnostics);
                }
                if let Some(else_expr) = &searched.else_clause {
                    check_nested_aggregation(else_expr, in_aggregate, diagnostics);
                }
            }
            crate::ast::expression::CaseExpression::Simple(simple) => {
                check_nested_aggregation(&simple.operand, in_aggregate, diagnostics);
                for when in &simple.when_clauses {
                    check_nested_aggregation(&when.when_value, in_aggregate, diagnostics);
                    check_nested_aggregation(&when.then_result, in_aggregate, diagnostics);
                }
                if let Some(else_expr) = &simple.else_clause {
                    check_nested_aggregation(else_expr, in_aggregate, diagnostics);
                }
            }
        },
        Expression::Cast(cast) => {
            check_nested_aggregation(&cast.operand, in_aggregate, diagnostics);
        }
        Expression::Exists(exists) => {
            // EXISTS contains a graph pattern, not expressions that can have aggregations
            // Skip for now
            _ = exists;
        }
        Expression::Predicate(pred) => {
            use crate::ast::expression::Predicate;
            match pred {
                Predicate::IsNull(expr, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsTyped(expr, _, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsNormalized(expr, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsDirected(expr, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsLabeled(expr, _, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsTruthValue(expr, _, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
                Predicate::IsSource(expr1, expr2, _, _) => {
                    check_nested_aggregation(expr1, in_aggregate, diagnostics);
                    check_nested_aggregation(expr2, in_aggregate, diagnostics);
                }
                Predicate::IsDestination(expr1, expr2, _, _) => {
                    check_nested_aggregation(expr1, in_aggregate, diagnostics);
                    check_nested_aggregation(expr2, in_aggregate, diagnostics);
                }
                Predicate::AllDifferent(exprs, _) => {
                    for e in exprs {
                        check_nested_aggregation(e, in_aggregate, diagnostics);
                    }
                }
                Predicate::Same(expr1, expr2, _) => {
                    check_nested_aggregation(expr1, in_aggregate, diagnostics);
                    check_nested_aggregation(expr2, in_aggregate, diagnostics);
                }
                Predicate::PropertyExists(expr, _, _) => {
                    check_nested_aggregation(expr, in_aggregate, diagnostics);
                }
            }
        }
        _ => {}
    }
}

/// Validates HAVING clause per ISO GQL standard.
/// Non-aggregated expressions in HAVING must appear in GROUP BY.
fn validate_having_clause(
    validator: &super::SemanticValidator,
    condition: &crate::ast::expression::Expression,
    group_by: &Option<crate::ast::query::GroupByClause>,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::semantic::diag::aggregation_error;

    // Collect non-aggregated expressions in HAVING
    let non_agg_exprs = collect_non_aggregated_expressions(condition);

    if let Some(group_by) = group_by {
        // Check each non-aggregated expression appears in GROUP BY
        let group_by_exprs = collect_group_by_expressions(group_by);

        for expr in non_agg_exprs {
            let found_in_group_by = group_by_exprs
                .iter()
                .any(|gb_expr| expressions_equivalent(expr, gb_expr));

            if !found_in_group_by {
                let diag = aggregation_error(
                    "Non-aggregated expression in HAVING must appear in GROUP BY",
                    expr.span().clone(),
                );
                diagnostics.push(diag);
            }
        }
    } else {
        // HAVING without GROUP BY - only aggregates allowed
        if !non_agg_exprs.is_empty() && validator.config.strict_mode {
            for expr in non_agg_exprs {
                let diag = aggregation_error(
                    "HAVING clause requires GROUP BY when using non-aggregated expressions",
                    expr.span().clone(),
                );
                diagnostics.push(diag);
            }
        }
    }
}

/// Collects non-aggregated expressions from an expression tree.
fn collect_non_aggregated_expressions(
    expr: &crate::ast::expression::Expression,
) -> Vec<&crate::ast::expression::Expression> {
    let mut result = Vec::new();
    collect_non_aggregated_expressions_recursive(expr, false, &mut result);
    result
}

/// Recursively collects non-aggregated expressions.
fn collect_non_aggregated_expressions_recursive<'a>(
    expr: &'a crate::ast::expression::Expression,
    in_aggregate: bool,
    result: &mut Vec<&'a crate::ast::expression::Expression>,
) {
    use crate::ast::expression::{AggregateFunction, Expression};

    match expr {
        Expression::AggregateFunction(agg) => {
            // Inside aggregate, check arguments with in_aggregate=true
            match &**agg {
                AggregateFunction::CountStar { .. } => {}
                AggregateFunction::GeneralSetFunction(gsf) => {
                    collect_non_aggregated_expressions_recursive(&gsf.expression, true, result);
                }
                AggregateFunction::BinarySetFunction(bsf) => {
                    collect_non_aggregated_expressions_recursive(&bsf.expression, true, result);
                    collect_non_aggregated_expressions_recursive(
                        &bsf.inverse_distribution_argument,
                        true,
                        result,
                    );
                }
            }
        }
        Expression::VariableReference(_, _) | Expression::PropertyReference(_, _, _) => {
            if !in_aggregate {
                result.push(expr);
            }
        }
        Expression::Binary(_, left, right, _) => {
            collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
            collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
        }
        Expression::Unary(_, operand, _) => {
            collect_non_aggregated_expressions_recursive(operand, in_aggregate, result);
        }
        Expression::Comparison(_, left, right, _) => {
            collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
            collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
        }
        Expression::Logical(_, left, right, _) => {
            collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
            collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
        }
        Expression::Parenthesized(inner, _) => {
            collect_non_aggregated_expressions_recursive(inner, in_aggregate, result);
        }
        Expression::FunctionCall(func) => {
            for arg in &func.arguments {
                collect_non_aggregated_expressions_recursive(arg, in_aggregate, result);
            }
        }
        _ => {}
    }
}

/// Collects expressions from GROUP BY clause.
fn collect_group_by_expressions(
    group_by: &crate::ast::query::GroupByClause,
) -> Vec<&crate::ast::expression::Expression> {
    use crate::ast::query::GroupingElement;

    let mut expressions = Vec::new();
    for element in &group_by.elements {
        match element {
            GroupingElement::Expression(expr) => {
                expressions.push(expr);
            }
            GroupingElement::EmptyGroupingSet => {
                // Empty grouping set doesn't provide expressions
            }
        }
    }
    expressions
}

/// Checks if two expressions are semantically equivalent per ISO GQL standard.
/// Used for GROUP BY validation and expression matching.
fn expressions_equivalent(
    expr1: &crate::ast::expression::Expression,
    expr2: &crate::ast::expression::Expression,
) -> bool {
    use crate::ast::expression::Expression;

    match (expr1, expr2) {
        // Literals
        (Expression::Literal(l1, _), Expression::Literal(l2, _)) => l1 == l2,

        // Variables
        (Expression::VariableReference(v1, _), Expression::VariableReference(v2, _)) => v1 == v2,

        // Properties
        (
            Expression::PropertyReference(base1, prop1, _),
            Expression::PropertyReference(base2, prop2, _),
        ) => prop1 == prop2 && expressions_equivalent(base1, base2),

        // Binary operations
        (Expression::Binary(op1, l1, r1, _), Expression::Binary(op2, l2, r2, _)) => {
            op1 == op2 && expressions_equivalent(l1, l2) && expressions_equivalent(r1, r2)
        }

        // Unary operations
        (Expression::Unary(op1, e1, _), Expression::Unary(op2, e2, _)) => {
            op1 == op2 && expressions_equivalent(e1, e2)
        }

        // Function calls
        (Expression::FunctionCall(f1), Expression::FunctionCall(f2)) => {
            f1.name == f2.name
                && f1.arguments.len() == f2.arguments.len()
                && f1
                    .arguments
                    .iter()
                    .zip(&f2.arguments)
                    .all(|(a1, a2)| expressions_equivalent(a1, a2))
        }

        // Parenthesized (unwrap and compare)
        (Expression::Parenthesized(e1, _), e2) => expressions_equivalent(e1, e2),
        (e1, Expression::Parenthesized(e2, _)) => expressions_equivalent(e1, e2),

        // Type annotations (ignore annotation, compare base)
        (Expression::TypeAnnotation(e1, _, _), e2) => expressions_equivalent(e1, e2),
        (e1, Expression::TypeAnnotation(e2, _, _)) => expressions_equivalent(e1, e2),

        // Comparison operations
        (Expression::Comparison(op1, l1, r1, _), Expression::Comparison(op2, l2, r2, _)) => {
            op1 == op2 && expressions_equivalent(l1, l2) && expressions_equivalent(r1, r2)
        }

        // Logical operations
        (Expression::Logical(op1, l1, r1, _), Expression::Logical(op2, l2, r2, _)) => {
            op1 == op2 && expressions_equivalent(l1, l2) && expressions_equivalent(r1, r2)
        }

        // Default: not equivalent
        _ => false,
    }
}
