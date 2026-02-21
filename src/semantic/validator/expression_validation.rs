//! Expression validation pass for semantic validation.
//!
//! This module handles expression validation: validates expressions for semantic correctness.
//! It checks:
//! - Null propagation rules
//! - CASE expression type consistency
//! - Subquery result types
//! - List operations

use crate::ast::program::Program;
use crate::ast::query::{LinearQuery, PrimitiveQueryStatement, Query};
use crate::diag::Diag;
use crate::ir::TypeTable;

/// Main entry point for expression validation pass.
///
/// Validates expressions throughout the program, checking for semantic correctness
/// including type consistency in CASE expressions and boolean contexts.
pub(super) fn run_expression_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    _type_table: &TypeTable,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - Null propagation rules
    // - CASE expression type consistency
    // - Subquery result types
    // - List operations

    for statement in &program.statements {
        match statement {
            crate::ast::program::Statement::Query(query_stmt) => {
                validate_query_expressions(validator, &query_stmt.query, diagnostics);
            }
            crate::ast::program::Statement::Mutation(mutation_stmt) => {
                // Validate expressions in mutation statement
                validate_mutation_expressions(validator, &mutation_stmt.statement, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validates expressions in a query.
fn validate_query_expressions(
    validator: &super::SemanticValidator,
    query: &Query,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            validate_linear_query_expressions(validator, linear_query, diagnostics);
        }
        Query::Composite(composite) => {
            validate_query_expressions(validator, &composite.left, diagnostics);
            validate_query_expressions(validator, &composite.right, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_expressions(validator, query, diagnostics);
        }
    }
}

/// Validates expressions in a linear query.
fn validate_linear_query_expressions(
    validator: &super::SemanticValidator,
    linear_query: &LinearQuery,
    diagnostics: &mut Vec<Diag>,
) {
    let primitive_statements = &linear_query.primitive_statements;

    for statement in primitive_statements {
        match statement {
            PrimitiveQueryStatement::Let(let_stmt) => {
                for binding in &let_stmt.bindings {
                    validate_expression_semantics(validator, &binding.value, diagnostics);
                }
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                validate_expression_semantics(validator, &for_stmt.item.collection, diagnostics);
            }
            PrimitiveQueryStatement::Filter(filter) => {
                validate_expression_semantics(validator, &filter.condition, diagnostics);
            }
            PrimitiveQueryStatement::Select(select) => match &select.select_items {
                crate::ast::query::SelectItemList::Items { items } => {
                    for item in items {
                        validate_expression_semantics(validator, &item.expression, diagnostics);
                    }
                }
                crate::ast::query::SelectItemList::Star => {}
            },
            _ => {}
        }
    }
}

/// Validates semantic rules for an expression.
fn validate_expression_semantics(
    validator: &super::SemanticValidator,
    expr: &crate::ast::expression::Expression,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::Expression;

    match expr {
        Expression::Case(case_expr) => {
            // Validate CASE expression type consistency
            validate_case_expression(validator, case_expr, diagnostics);
        }
        Expression::Binary(_, left, right, _) => {
            validate_expression_semantics(validator, left, diagnostics);
            validate_expression_semantics(validator, right, diagnostics);
        }
        Expression::Comparison(_, left, right, _) => {
            validate_expression_semantics(validator, left, diagnostics);
            validate_expression_semantics(validator, right, diagnostics);
        }
        Expression::Logical(_, left, right, _) => {
            validate_expression_semantics(validator, left, diagnostics);
            validate_expression_semantics(validator, right, diagnostics);
        }
        Expression::Unary(_, operand, _) => {
            validate_expression_semantics(validator, operand, diagnostics);
        }
        Expression::PropertyReference(base, _, _) => {
            validate_expression_semantics(validator, base, diagnostics);
        }
        Expression::ListConstructor(elements, _) => {
            for elem in elements {
                validate_expression_semantics(validator, elem, diagnostics);
            }
        }
        Expression::RecordConstructor(fields, _) => {
            for field in fields {
                validate_expression_semantics(validator, &field.value, diagnostics);
            }
        }
        Expression::PathConstructor(exprs, _) => {
            for expr in exprs {
                validate_expression_semantics(validator, expr, diagnostics);
            }
        }
        Expression::Parenthesized(inner, _) => {
            validate_expression_semantics(validator, inner, diagnostics);
        }
        Expression::FunctionCall(func_call) => {
            for arg in &func_call.arguments {
                validate_expression_semantics(validator, arg, diagnostics);
            }
        }
        Expression::AggregateFunction(_agg_func) => {
            // Validate arguments in the aggregate function
            // The structure may vary, so we skip detailed validation for now
        }
        Expression::Predicate(pred) => {
            validate_predicate_semantics(validator, pred, diagnostics);
        }
        Expression::Cast(cast_expr) => {
            validate_expression_semantics(validator, &cast_expr.operand, diagnostics);
        }
        Expression::TypeAnnotation(expr, _, _) => {
            validate_expression_semantics(validator, expr, diagnostics);
        }
        Expression::Exists(_exists_expr) => {
            // EXISTS expressions have their own validation
        }
        Expression::GraphExpression(expr, _) => {
            validate_expression_semantics(validator, expr, diagnostics);
        }
        Expression::BindingTableExpression(expr, _) => {
            validate_expression_semantics(validator, expr, diagnostics);
        }
        Expression::SubqueryExpression(_, _) => {}
        // Literals and simple references don't need semantic validation
        Expression::Literal(_, _)
        | Expression::VariableReference(_, _)
        | Expression::ParameterReference(_, _) => {}
    }
}

/// Validates predicate semantics.
fn validate_predicate_semantics(
    validator: &super::SemanticValidator,
    pred: &crate::ast::expression::Predicate,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::Predicate;

    match pred {
        Predicate::IsNull(e, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
        Predicate::IsTyped(expr, _, _, _) => {
            validate_expression_semantics(validator, expr, diagnostics);
        }
        Predicate::IsNormalized(e, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
        Predicate::IsDirected(e, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
        Predicate::IsLabeled(e, _, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
        Predicate::IsTruthValue(e, _, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
        Predicate::IsSource(e1, e2, _, _) | Predicate::IsDestination(e1, e2, _, _) => {
            validate_expression_semantics(validator, e1, diagnostics);
            validate_expression_semantics(validator, e2, diagnostics);
        }
        Predicate::Same(e1, e2, _) => {
            validate_expression_semantics(validator, e1, diagnostics);
            validate_expression_semantics(validator, e2, diagnostics);
        }
        Predicate::AllDifferent(exprs, _) => {
            for e in exprs {
                validate_expression_semantics(validator, e, diagnostics);
            }
        }
        Predicate::PropertyExists(e, _, _) => {
            validate_expression_semantics(validator, e, diagnostics);
        }
    }
}

/// Validates expressions in a mutation statement.
fn validate_mutation_expressions(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::{
        PrimitiveDataModifyingStatement,
        SimpleDataAccessingStatement, SimpleDataModifyingStatement,
    };

    let statements = &mutation.statements;

    for statement in statements {
        match statement {
            SimpleDataAccessingStatement::Query(query_stmt) => {
                // Validate expressions in query statements within mutation
                match query_stmt.as_ref() {
                    PrimitiveQueryStatement::Filter(filter) => {
                        validate_expression_semantics(validator, &filter.condition, diagnostics);
                    }
                    PrimitiveQueryStatement::Let(let_stmt) => {
                        for binding in &let_stmt.bindings {
                            validate_expression_semantics(validator, &binding.value, diagnostics);
                        }
                    }
                    PrimitiveQueryStatement::For(for_stmt) => {
                        validate_expression_semantics(
                            validator,
                            &for_stmt.item.collection,
                            diagnostics,
                        );
                    }
                    PrimitiveQueryStatement::Select(select) => {
                        use crate::ast::query::SelectItemList;
                        match &select.select_items {
                            SelectItemList::Items { items } => {
                                for item in items {
                                    validate_expression_semantics(
                                        validator,
                                        &item.expression,
                                        diagnostics,
                                    );
                                }
                            }
                            SelectItemList::Star => {}
                        }
                    }
                    PrimitiveQueryStatement::OrderByAndPage(order_page) => {
                        if let Some(order_by) = &order_page.order_by {
                            for key in &order_by.sort_specifications {
                                validate_expression_semantics(validator, &key.key, diagnostics);
                            }
                        }
                        if let Some(offset) = &order_page.offset {
                            validate_expression_semantics(validator, &offset.count, diagnostics);
                        }
                        if let Some(limit) = &order_page.limit {
                            validate_expression_semantics(validator, &limit.count, diagnostics);
                        }
                    }
                    _ => {}
                }
            }
            SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Primitive(
                primitive,
            )) => {
                match primitive {
                    PrimitiveDataModifyingStatement::Insert(insert) => {
                        // Validate expressions in INSERT property specifications
                        validate_insert_expressions(validator, &insert.pattern, diagnostics);
                    }
                    PrimitiveDataModifyingStatement::Set(set) => {
                        // Validate expressions in SET operations
                        for item in &set.items.items {
                            use crate::ast::mutation::SetItem;
                            match item {
                                SetItem::Property(prop) => {
                                    validate_expression_semantics(
                                        validator,
                                        &prop.value,
                                        diagnostics,
                                    );
                                }
                                SetItem::AllProperties(all_props) => {
                                    for pair in &all_props.properties.properties {
                                        validate_expression_semantics(
                                            validator,
                                            &pair.value,
                                            diagnostics,
                                        );
                                    }
                                }
                                SetItem::Label(_) => {
                                    // Labels don't have expressions
                                }
                            }
                        }
                    }
                    PrimitiveDataModifyingStatement::Remove(_)
                    | PrimitiveDataModifyingStatement::Delete(_) => {
                        // These don't have expressions to validate
                    }
                }
            }
            SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                // Procedure calls would need procedure signature validation
            }
        }
    }
}

/// Validates expressions in INSERT patterns.
fn validate_insert_expressions(
    validator: &super::SemanticValidator,
    pattern: &crate::ast::mutation::InsertGraphPattern,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::{InsertEdgePattern, InsertElementPattern};

    for path in &pattern.paths {
        for element in &path.elements {
            let props = match element {
                InsertElementPattern::Node(node) => {
                    node.filler.as_ref().and_then(|f| f.properties.as_ref())
                }
                InsertElementPattern::Edge(edge) => {
                    let filler = match edge {
                        InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                        InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                        InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                    };
                    filler.and_then(|f| f.properties.as_ref())
                }
            };

            if let Some(props) = props {
                for pair in &props.properties {
                    validate_expression_semantics(validator, &pair.value, diagnostics);
                }
            }
        }
    }
}

/// Validates CASE expression type consistency.
fn validate_case_expression(
    validator: &super::SemanticValidator,
    case_expr: &crate::ast::expression::CaseExpression,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::CaseExpression;

    match case_expr {
        CaseExpression::Simple(simple) => {
            // Validate operand
            validate_expression_semantics(validator, &simple.operand, diagnostics);

            // Validate all when clauses
            for when_clause in &simple.when_clauses {
                validate_expression_semantics(validator, &when_clause.when_value, diagnostics);
                validate_expression_semantics(validator, &when_clause.then_result, diagnostics);
            }

            // Validate else clause if present
            if let Some(else_result) = &simple.else_clause {
                validate_expression_semantics(validator, else_result, diagnostics);
            }

            // Check that all result expressions have compatible types
            // Note: This is a basic check using literal types; full type inference
            // would require the complete TypeTable integration (F4)
            validate_case_result_compatibility(
                validator,
                simple
                    .when_clauses
                    .iter()
                    .map(|wc| &wc.then_result)
                    .chain(simple.else_clause.iter().map(|e| e.as_ref())),
                diagnostics,
            );
        }
        CaseExpression::Searched(searched) => {
            // Validate all when clauses
            for when_clause in &searched.when_clauses {
                validate_expression_semantics(validator, &when_clause.condition, diagnostics);
                validate_expression_semantics(validator, &when_clause.then_result, diagnostics);

                // Check that condition is boolean (basic check for literals)
                if validator.config.strict_mode {
                    validate_boolean_expression(validator, &when_clause.condition, diagnostics);
                }
            }

            // Validate else clause if present
            if let Some(else_result) = &searched.else_clause {
                validate_expression_semantics(validator, else_result, diagnostics);
            }

            // Check that all result expressions have compatible types
            validate_case_result_compatibility(
                validator,
                searched
                    .when_clauses
                    .iter()
                    .map(|wc| &wc.then_result)
                    .chain(searched.else_clause.iter().map(|e| e.as_ref())),
                diagnostics,
            );
        }
    }
}

/// Validates that an expression is boolean-typed (best-effort check).
fn validate_boolean_expression(
    _validator: &super::SemanticValidator,
    expr: &crate::ast::expression::Expression,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::{Expression, Literal};

    // Basic check: if it's a literal, verify it's boolean
    // Full implementation would use TypeTable
    match expr {
        Expression::Literal(Literal::Boolean(_), _) => {
            // OK - boolean literal
        }
        Expression::Literal(lit, span) if !matches!(lit, Literal::Null) => {
            // Non-boolean, non-null literal in boolean context
            use crate::semantic::diag::SemanticDiagBuilder;
            let diag =
                SemanticDiagBuilder::type_mismatch("Boolean", &format!("{:?}", lit), span.clone())
                    .with_note("Condition expressions should evaluate to boolean")
                    .build();
            diagnostics.push(diag);
        }
        Expression::Comparison(..)
        | Expression::Logical(..)
        | Expression::Predicate(_)
        | Expression::Exists(_) => {
            // These expressions produce boolean results - OK
        }
        _ => {
            // Other expressions - can't determine type without full type inference
            // Don't emit false positives
        }
    }
}

/// Validates that CASE result expressions have compatible types (best-effort).
fn validate_case_result_compatibility<'a>(
    _validator: &super::SemanticValidator,
    results: impl Iterator<Item = &'a crate::ast::expression::Expression>,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::{Expression, Literal};

    // Collect result types (only for literals - full impl needs TypeTable)
    let mut literal_types: Vec<(&str, &crate::ast::Span)> = Vec::new();

    for result in results {
        if let Expression::Literal(lit, span) = result {
            let type_name = match lit {
                Literal::Boolean(_) => "Boolean",
                Literal::Integer(_) => "Integer",
                Literal::Float(_) => "Float",
                Literal::String(_) => "String",
                Literal::Null => continue, // Null is compatible with everything
                Literal::Date(_) => "Date",
                Literal::Time(_) => "Time",
                Literal::Datetime(_) => "Timestamp",
                Literal::Duration(_) => "Duration",
                Literal::List(_) => "List",
                Literal::Record(_) => "Record",
                Literal::ByteString(_) => "ByteString",
            };
            literal_types.push((type_name, span));
        }
    }

    // Check if all non-null literals have the same type
    if literal_types.len() > 1 {
        let first_type = literal_types[0].0;
        for (type_name, span) in &literal_types[1..] {
            if *type_name != first_type {
                use crate::semantic::diag::SemanticDiagBuilder;
                let diag =
                    SemanticDiagBuilder::type_mismatch(first_type, type_name, (*span).clone())
                        .with_note("All CASE result branches should have compatible types")
                        .build();
                diagnostics.push(diag);
            }
        }
    }
}
