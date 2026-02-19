//! Type checking pass for semantic validation.
//!
//! This module implements Pass 6: Type Checking, which validates type compatibility
//! in operations throughout the GQL program.

use crate::ast::program::{Program, Statement};
use crate::ast::query::{LinearQuery, PrimitiveQueryStatement, Query};
use crate::diag::Diag;
use crate::ir::TypeTable;
use crate::semantic::diag::SemanticDiagBuilder;

/// Pass 6: Type Checking - Checks type compatibility in operations.
pub(super) fn run_type_checking(
    _validator: &super::SemanticValidator,
    program: &Program,
    _type_table: &TypeTable,
    diagnostics: &mut Vec<Diag>,
) {
    // Walk all statements and check type compatibility
    for statement in &program.statements {
        match statement {
            Statement::Query(query_stmt) => {
                check_query_types(&query_stmt.query, diagnostics);
            }
            Statement::Mutation(mutation_stmt) => {
                // Check types in mutation statement
                check_mutation_types(&mutation_stmt.statement, diagnostics);
            }
            _ => {}
        }
    }
}

/// Checks types in a query.
fn check_query_types(query: &Query, diagnostics: &mut Vec<Diag>) {
    match query {
        Query::Linear(linear_query) => {
            check_linear_query_types(linear_query, diagnostics);
        }
        Query::Composite(composite) => {
            check_query_types(&composite.left, diagnostics);
            check_query_types(&composite.right, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            check_query_types(query, diagnostics);
        }
    }
}

/// Checks types in a linear query.
fn check_linear_query_types(linear_query: &LinearQuery, diagnostics: &mut Vec<Diag>) {
    let primitive_statements = match linear_query {
        LinearQuery::Focused(focused) => &focused.primitive_statements,
        LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
    };

    // Check types in each statement
    for statement in primitive_statements {
        match statement {
            PrimitiveQueryStatement::Let(let_stmt) => {
                for binding in &let_stmt.bindings {
                    check_expression_types(&binding.value, diagnostics);
                }
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                check_expression_types(&for_stmt.item.collection, diagnostics);
            }
            PrimitiveQueryStatement::Filter(filter) => {
                // Filter condition should be boolean
                check_expression_types(&filter.condition, diagnostics);

                // Check that the condition is likely boolean
                if is_definitely_non_boolean(&filter.condition) {
                    diagnostics.push(
                        SemanticDiagBuilder::type_mismatch(
                            "boolean",
                            "non-boolean",
                            filter.condition.span(),
                        )
                        .build(),
                    );
                }
            }
            PrimitiveQueryStatement::Select(select) => match &select.select_items {
                crate::ast::query::SelectItemList::Items { items } => {
                    for item in items {
                        check_expression_types(&item.expression, diagnostics);
                    }
                }
                crate::ast::query::SelectItemList::Star => {}
            },
            _ => {}
        }
    }
}

/// Checks type compatibility in an expression.
fn check_expression_types(
    expr: &crate::ast::expression::Expression,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::{BinaryOperator, Expression};
    use crate::semantic::diag::SemanticDiagBuilder;

    match expr {
        // Binary arithmetic operations require numeric operands
        Expression::Binary(op, left, right, _span) => {
            // Recursively check nested expressions
            check_expression_types(left, diagnostics);
            check_expression_types(right, diagnostics);

            // Check type compatibility for the operation
            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    // F3: Check for NULL in arithmetic (ISO GQL null propagation)
                    use crate::ast::expression::Literal;
                    let left_is_null = matches!(left.as_ref(), Expression::Literal(Literal::Null, _));
                    let right_is_null = matches!(right.as_ref(), Expression::Literal(Literal::Null, _));

                    if left_is_null || right_is_null {
                        diagnostics.push(
                            Diag::warning("Arithmetic operation with NULL will always return NULL")
                                .with_primary_label(_span.clone(), "NULL propagation")
                        );
                    }

                    // Simple literal type checking
                    if is_definitely_string(left) {
                        diagnostics.push(
                            SemanticDiagBuilder::type_mismatch(
                                "numeric",
                                "string",
                                left.span(),
                            )
                            .build(),
                        );
                    }
                    if is_definitely_string(right) {
                        diagnostics.push(
                            SemanticDiagBuilder::type_mismatch(
                                "numeric",
                                "string",
                                right.span(),
                            )
                            .build(),
                        );
                    }
                }
                BinaryOperator::Concatenate => {
                    // String concatenation is generally permissive
                }
            }
        }

        // Comparison operations
        Expression::Comparison(_op, left, right, _span) => {
            check_expression_types(left, diagnostics);
            check_expression_types(right, diagnostics);
        }

        // Logical operations require boolean operands
        Expression::Logical(_op, left, right, _span) => {
            check_expression_types(left, diagnostics);
            check_expression_types(right, diagnostics);
        }

        // Unary operations
        Expression::Unary(op, operand, _span) => {
            check_expression_types(operand, diagnostics);

            match op {
                crate::ast::expression::UnaryOperator::Plus
                | crate::ast::expression::UnaryOperator::Minus => {
                    // Unary +/- require numeric type
                    if is_definitely_string(operand) {
                        diagnostics.push(
                            SemanticDiagBuilder::type_mismatch(
                                "numeric",
                                "string",
                                operand.span(),
                            )
                            .build(),
                        );
                    }
                }
                crate::ast::expression::UnaryOperator::Not => {
                    // NOT requires boolean type
                }
            }
        }

        // Property reference
        Expression::PropertyReference(object, _prop, _span) => {
            check_expression_types(object, diagnostics);
        }

        // Function call
        Expression::FunctionCall(fc) => {
            for arg in &fc.arguments {
                check_expression_types(arg, diagnostics);
            }
        }

        // Case expression
        Expression::Case(case) => {
            use crate::ast::expression::CaseExpression;
            match case {
                CaseExpression::Simple(simple) => {
                    check_expression_types(&simple.operand, diagnostics);
                    for when_clause in &simple.when_clauses {
                        check_expression_types(&when_clause.when_value, diagnostics);
                        check_expression_types(&when_clause.then_result, diagnostics);
                    }
                    if let Some(else_expr) = &simple.else_clause {
                        check_expression_types(else_expr, diagnostics);
                    }
                }
                CaseExpression::Searched(searched) => {
                    for when_clause in &searched.when_clauses {
                        check_expression_types(&when_clause.condition, diagnostics);
                        check_expression_types(&when_clause.then_result, diagnostics);
                    }
                    if let Some(else_expr) = &searched.else_clause {
                        check_expression_types(else_expr, diagnostics);
                    }
                }
            }
        }

        // Cast expression
        Expression::Cast(cast) => {
            check_expression_types(&cast.operand, diagnostics);
        }

        // Aggregate function
        Expression::AggregateFunction(agg) => {
            use crate::ast::expression::AggregateFunction;
            match &**agg {
                AggregateFunction::GeneralSetFunction(gsf) => {
                    check_expression_types(&gsf.expression, diagnostics);
                }
                AggregateFunction::BinarySetFunction(bsf) => {
                    check_expression_types(&bsf.expression, diagnostics);
                    check_expression_types(
                        &bsf.inverse_distribution_argument,
                        diagnostics,
                    );
                }
                AggregateFunction::CountStar { .. } => {}
            }
        }

        // List constructor
        Expression::ListConstructor(elements, _span) => {
            for elem in elements {
                check_expression_types(elem, diagnostics);
            }
        }

        // Record constructor
        Expression::RecordConstructor(fields, _span) => {
            for field in fields {
                check_expression_types(&field.value, diagnostics);
            }
        }

        // Path constructor
        Expression::PathConstructor(elements, _span) => {
            for elem in elements {
                check_expression_types(elem, diagnostics);
            }
        }

        // Predicate
        Expression::Predicate(pred) => {
            use crate::ast::expression::Predicate;
            match pred {
                Predicate::IsNull(operand, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsTyped(operand, _, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsNormalized(operand, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsDirected(operand, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsLabeled(operand, _, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsTruthValue(operand, _, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::IsSource(operand, of, _, _)
                | Predicate::IsDestination(operand, of, _, _) => {
                    check_expression_types(operand, diagnostics);
                    check_expression_types(of, diagnostics);
                }
                Predicate::Same(left, right, _) => {
                    check_expression_types(left, diagnostics);
                    check_expression_types(right, diagnostics);
                }
                Predicate::PropertyExists(operand, _, _) => {
                    check_expression_types(operand, diagnostics);
                }
                Predicate::AllDifferent(operands, _) => {
                    for operand in operands {
                        check_expression_types(operand, diagnostics);
                    }
                }
            }
        }

        // Type annotation
        Expression::TypeAnnotation(inner, _annotation, _span) => {
            check_expression_types(inner, diagnostics);
        }

        // Graph/binding table/subquery expressions
        Expression::GraphExpression(inner, _)
        | Expression::BindingTableExpression(inner, _)
        | Expression::SubqueryExpression(inner, _) => {
            check_expression_types(inner, diagnostics);
        }

        // Parenthesized
        Expression::Parenthesized(inner, _) => {
            check_expression_types(inner, diagnostics);
        }

        // EXISTS predicate - contains complex structure
        Expression::Exists(_) => {
            // Would need to validate nested query structure
        }

        // Literals, variables, and parameters don't need type checking
        Expression::Literal(_, _)
        | Expression::VariableReference(_, _)
        | Expression::ParameterReference(_, _) => {}
    }
}

/// Helper: Check if an expression is definitely a string literal.
fn is_definitely_string(expr: &crate::ast::expression::Expression) -> bool {
    use crate::ast::expression::{Expression, Literal};
    matches!(expr, Expression::Literal(Literal::String(_), _))
}

/// Helper: Check if an expression is definitely not boolean.
fn is_definitely_non_boolean(expr: &crate::ast::expression::Expression) -> bool {
    use crate::ast::expression::{Expression, Literal};
    matches!(
        expr,
        Expression::Literal(
            Literal::String(_) | Literal::Integer(_) | Literal::Float(_),
            _
        )
    )
}


/// Checks types in a mutation statement.
fn check_mutation_types(
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::{
        LinearDataModifyingStatement, SimpleDataAccessingStatement,
        SimpleDataModifyingStatement,
    };

    let statements = match mutation {
        LinearDataModifyingStatement::Focused(focused) => &focused.statements,
        LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
    };

    for statement in statements {
        match statement {
            SimpleDataAccessingStatement::Query(query_stmt) => {
                // Check types in query statements within mutation
                match query_stmt.as_ref() {
                    PrimitiveQueryStatement::Filter(filter) => {
                        check_expression_types(&filter.condition, diagnostics);
                    }
                    PrimitiveQueryStatement::Let(let_stmt) => {
                        for binding in &let_stmt.bindings {
                            check_expression_types(&binding.value, diagnostics);
                        }
                    }
                    PrimitiveQueryStatement::For(for_stmt) => {
                        check_expression_types(&for_stmt.item.collection, diagnostics);
                    }
                    PrimitiveQueryStatement::Select(select) => {
                        check_select_types(select, diagnostics);
                    }
                    PrimitiveQueryStatement::OrderByAndPage(order_page) => {
                        if let Some(order_by) = &order_page.order_by {
                            for key in &order_by.sort_specifications {
                                check_expression_types(&key.key, diagnostics);
                            }
                        }
                        if let Some(offset) = &order_page.offset {
                            check_expression_types(&offset.count, diagnostics);
                        }
                        if let Some(limit) = &order_page.limit {
                            check_expression_types(&limit.count, diagnostics);
                        }
                    }
                    _ => {}
                }
            }
            SimpleDataAccessingStatement::Modifying(
                SimpleDataModifyingStatement::Primitive(primitive),
            ) => {
                use crate::ast::mutation::PrimitiveDataModifyingStatement;
                match primitive {
                    PrimitiveDataModifyingStatement::Insert(insert) => {
                        // Check types in INSERT property specifications
                        check_insert_types(&insert.pattern, diagnostics);
                    }
                    PrimitiveDataModifyingStatement::Set(set) => {
                        // Check types in SET operations
                        for item in &set.items.items {
                            use crate::ast::mutation::SetItem;
                            match item {
                                SetItem::Property(prop) => {
                                    check_expression_types(&prop.value, diagnostics);
                                }
                                SetItem::AllProperties(all_props) => {
                                    for pair in &all_props.properties.properties {
                                        check_expression_types(&pair.value, diagnostics);
                                    }
                                }
                                SetItem::Label(_) => {
                                    // Labels don't have expressions to check
                                }
                            }
                        }
                    }
                    PrimitiveDataModifyingStatement::Remove(remove) => {
                        // REMOVE operations don't have expressions to type check
                        let _ = remove;
                    }
                    PrimitiveDataModifyingStatement::Delete(_delete) => {
                        // DELETE operations reference variables but don't have expressions to type check
                    }
                }
            }
            SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                // Procedure calls would need procedure signature checking
            }
        }
    }
}

/// Checks types in INSERT pattern property specifications.
fn check_insert_types(
    pattern: &crate::ast::mutation::InsertGraphPattern,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::mutation::InsertElementPattern;

    for path in &pattern.paths {
        for element in &path.elements {
            match element {
                InsertElementPattern::Node(node) => {
                    if let Some(filler) = &node.filler
                        && let Some(props) = &filler.properties
                    {
                        check_property_specification_types(props, diagnostics);
                    }
                }
                InsertElementPattern::Edge(edge) => {
                    use crate::ast::mutation::InsertEdgePattern;
                    let filler = match edge {
                        InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                        InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                        InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                    };
                    if let Some(filler) = filler
                        && let Some(props) = &filler.properties
                    {
                        check_property_specification_types(props, diagnostics);
                    }
                }
            }
        }
    }
}

/// Checks types in property specifications.
fn check_property_specification_types(
    props: &crate::ast::query::ElementPropertySpecification,
    diagnostics: &mut Vec<Diag>,
) {
    for pair in &props.properties {
        check_expression_types(&pair.value, diagnostics);
    }
}

/// Checks types in SELECT statement.
fn check_select_types(
    select: &crate::ast::query::SelectStatement,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::query::SelectItemList;
    match &select.select_items {
        SelectItemList::Star => {}
        SelectItemList::Items { items } => {
            for item in items {
                check_expression_types(&item.expression, diagnostics);
            }
        }
    }
}
