use crate::ast::expression::Expression;
use crate::ast::query::{
    GroupByClause, GroupingElement, LinearQuery, PrimitiveQueryStatement, Query,
};
use crate::ast::{Program, Statement};
use crate::diag::{Diag, DiagSeverity};
use crate::semantic::diag::aggregation_error;

/// Pass 5: Context Validation - Checks clause usage in appropriate contexts.
pub(super) fn run_context_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - MATCH clauses in query contexts
    // - INSERT/DELETE clauses in mutation contexts
    // - CREATE/DROP clauses in catalog contexts
    // - Aggregation function usage

    for statement in &program.statements {
        match statement {
            Statement::Query(query_stmt) => {
                // Queries should contain query clauses (MATCH, etc.)
                validate_query_context(validator, &query_stmt.query, diagnostics);
            }
            Statement::Mutation(_mutation_stmt) => {
                // Mutations should contain mutation clauses (INSERT, DELETE, SET, REMOVE)
                // For now, we just validate that mutation operations are in mutation context
                // More detailed validation can be added as needed
            }
            Statement::Catalog(_) => {
                // Catalog statements (CREATE, DROP, etc.) are valid in catalog context
            }
            Statement::Session(_) | Statement::Transaction(_) | Statement::Empty(_) => {
                // These are valid in their respective contexts
            }
        }
    }
}

/// Validates that query clauses are used appropriately.
fn validate_query_context(
    validator: &super::SemanticValidator,
    query: &Query,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            validate_linear_query_context(validator, linear_query, diagnostics);
        }
        Query::Composite(composite) => {
            validate_query_context(validator, &composite.left, diagnostics);
            validate_query_context(validator, &composite.right, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_context(validator, query, diagnostics);
        }
    }
}

/// Validates context in a linear query.
fn validate_linear_query_context(
    validator: &super::SemanticValidator,
    linear_query: &LinearQuery,
    diagnostics: &mut Vec<Diag>,
) {
    let primitive_statements = &linear_query.primitive_statements;

    for statement in primitive_statements {
        match statement {
            PrimitiveQueryStatement::Match(_) => {
                // MATCH is valid in query context
            }
            PrimitiveQueryStatement::Let(_) => {
                // LET is valid in query context
            }
            PrimitiveQueryStatement::For(_) => {
                // FOR is valid in query context
            }
            PrimitiveQueryStatement::Filter(_) => {
                // WHERE/FILTER is valid in query context
            }
            PrimitiveQueryStatement::OrderByAndPage(_) => {
                // ORDER BY is valid in query context
            }
            PrimitiveQueryStatement::Select(select) => {
                // Check for aggregation functions in SELECT and validate GROUP BY semantics
                validate_select_aggregation(validator, select, diagnostics);
            }
            PrimitiveQueryStatement::Call(_) => {
                // CALL is valid in query context
            }
        }
    }
}

/// Validates aggregation and GROUP BY semantics in a SELECT statement.
fn validate_select_aggregation(
    validator: &super::SemanticValidator,
    select: &crate::ast::query::SelectStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::SelectItemList;

    // Check if we have aggregation in SELECT items
    let (has_aggregation, non_aggregated_expressions) = match &select.select_items {
        SelectItemList::Items { items } => {
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
        SelectItemList::Star => {
            // SELECT * is non-aggregated
            (false, Vec::new())
        }
    };

    // ISO GQL: Check that GROUP BY expressions don't contain aggregation functions
    // This check applies regardless of whether SELECT has aggregation
    if let Some(group_by) = &select.group_by {
        let group_by_expressions = collect_group_by_expressions(group_by);

        for gb_expr in &group_by_expressions {
            if expression_contains_aggregation(gb_expr) {
                diagnostics.push(aggregation_error(
                    "Aggregation functions not allowed in GROUP BY clause",
                    gb_expr.span().clone(),
                ));
            }
        }
    }

    // If we have aggregation mixed with non-aggregated expressions
    if has_aggregation && !non_aggregated_expressions.is_empty() {
        // Check if there's a GROUP BY clause
        if let Some(group_by) = &select.group_by {
            // Validate that all non-aggregated expressions appear in GROUP BY
            let group_by_expressions = collect_group_by_expressions(group_by);

            for non_agg_expr in non_aggregated_expressions {
                // Check if this expression appears in GROUP BY
                // For simplicity, we check by expression structure (not perfect but practical)
                let expr_appears_in_group_by = group_by_expressions
                    .iter()
                    .any(|gb_expr| expressions_equivalent(non_agg_expr, gb_expr));

                if !expr_appears_in_group_by {
                    if validator.config.strict_mode {
                        diagnostics.push(
                            aggregation_error(
                                "Non-aggregated expression must appear in GROUP BY clause when mixing with aggregation",
                                non_agg_expr.span()
                            )
                            
                        );
                    } else {
                        // In non-strict mode, just warn
                        diagnostics.push(
                            Diag::new(
                                DiagSeverity::Warning,
                                "Non-aggregated expression should appear in GROUP BY clause when mixing with aggregation".to_string()
                            )
                        );
                    }
                }
            }
        } else {
            // No GROUP BY but we have mixed aggregation and non-aggregation
            if validator.config.strict_mode {
                diagnostics.push(
                    aggregation_error(
                        "GROUP BY clause required when mixing aggregated and non-aggregated expressions",
                        select.span.clone()
                    )
                    
                );
            } else {
                // In non-strict mode, just warn
                diagnostics.push(
                    Diag::new(
                        DiagSeverity::Warning,
                        "GROUP BY clause recommended when mixing aggregated and non-aggregated expressions".to_string()
                    )
                );
            }
        }
    }
}

/// Collects all expressions from a GROUP BY clause.
fn collect_group_by_expressions(group_by: &GroupByClause) -> Vec<&Expression> {
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

/// Checks if two expressions are equivalent (simple structural comparison).
/// This is a simplified check; a full implementation would need semantic equivalence.
/// Checks if two expressions are semantically equivalent per ISO GQL standard.
/// Used for GROUP BY validation and expression matching.
fn expressions_equivalent(expr1: &Expression, expr2: &Expression) -> bool {
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

/// Checks if an expression contains aggregation functions.
fn expression_contains_aggregation(expr: &Expression) -> bool {
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
