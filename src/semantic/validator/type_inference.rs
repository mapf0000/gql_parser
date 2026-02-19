//! Type inference pass - infers types for expressions and builds the type table.

use crate::ast::program::{Program, Statement};
use crate::ast::query::{LinearQuery, PrimitiveQueryStatement, Query};
use crate::diag::Diag;
use crate::ir::{SymbolTable, TypeTable};

/// Pass 2: Type Inference - Infers types for all expressions in the program.
///
/// This pass walks the AST and infers types for expressions, building a type table
/// that can be queried by subsequent validation passes.
pub(super) fn run_type_inference(
    validator: &super::SemanticValidator,
    program: &Program,
    _symbol_table: &SymbolTable,
    _diagnostics: &mut Vec<Diag>,
) -> TypeTable {
    let mut type_table = TypeTable::new();

    // Walk all statements and infer types for expressions
    for statement in &program.statements {
        match statement {
            Statement::Query(query_stmt) => {
                infer_query_types(validator, &query_stmt.query, &mut type_table);
            }
            Statement::Mutation(mutation_stmt) => {
                infer_mutation_types(validator, &mutation_stmt.statement, &mut type_table);
            }
            _ => {}
        }
    }

    type_table
}

/// Infers types in a query.
fn infer_query_types(
    validator: &super::SemanticValidator,
    query: &Query,
    type_table: &mut TypeTable,
) {
    match query {
        Query::Linear(linear_query) => {
            infer_linear_query_types(validator, linear_query, type_table);
        }
        Query::Composite(composite) => {
            infer_query_types(validator, &composite.left, type_table);
            infer_query_types(validator, &composite.right, type_table);
        }
        Query::Parenthesized(query, _) => {
            infer_query_types(validator, query, type_table);
        }
    }
}

/// Infers types in a linear query.
fn infer_linear_query_types(
    validator: &super::SemanticValidator,
    linear_query: &LinearQuery,
    type_table: &mut TypeTable,
) {
    let primitive_statements = match linear_query {
        LinearQuery::Focused(focused) => &focused.primitive_statements,
        LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
    };

    // Walk primitive statements and infer types
    for statement in primitive_statements {
        match statement {
            PrimitiveQueryStatement::Match(_) => {
                // MATCH statements don't directly have expressions to type
                // Pattern variables would be typed as Node, Edge, Path, etc.
            }
            PrimitiveQueryStatement::Let(let_stmt) => {
                // Infer types of LET variable definitions
                for binding in &let_stmt.bindings {
                    infer_expression_type(validator, &binding.value, type_table);
                }
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                // Infer type of FOR collection expression
                infer_expression_type(validator, &for_stmt.item.collection, type_table);
            }
            PrimitiveQueryStatement::Filter(filter) => {
                // Infer type of filter condition (should be boolean)
                infer_expression_type(validator, &filter.condition, type_table);
            }
            PrimitiveQueryStatement::Select(select) => {
                // Infer types of select items
                match &select.select_items {
                    crate::ast::query::SelectItemList::Items { items } => {
                        for item in items {
                            infer_expression_type(validator, &item.expression, type_table);
                        }
                    }
                    crate::ast::query::SelectItemList::Star => {
                        // SELECT * doesn't have specific expressions to type
                    }
                }
            }
            _ => {}
        }
    }
}

/// Infers types in a mutation statement.
fn infer_mutation_types(
    validator: &super::SemanticValidator,
    mutation: &crate::ast::mutation::LinearDataModifyingStatement,
    type_table: &mut TypeTable,
) {
    use crate::ast::mutation::LinearDataModifyingStatement;

    let statements = match mutation {
        LinearDataModifyingStatement::Focused(focused) => &focused.statements,
        LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
    };

    for stmt in statements {
        use crate::ast::mutation::SimpleDataAccessingStatement;

        match stmt {
            SimpleDataAccessingStatement::Query(query_stmt) => {
                // Infer types in the query part
                infer_primitive_query_statement_types(validator, query_stmt, type_table);
            }
            SimpleDataAccessingStatement::Modifying(modifying) => {
                infer_modifying_statement_types(validator, modifying, type_table);
            }
        }
    }
}

/// Infers types in a primitive query statement (helper for mutations).
fn infer_primitive_query_statement_types(
    validator: &super::SemanticValidator,
    stmt: &PrimitiveQueryStatement,
    type_table: &mut TypeTable,
) {
    match stmt {
        PrimitiveQueryStatement::Match(_) => {
            // MATCH patterns define variables but don't have expressions to type
        }
        PrimitiveQueryStatement::Let(let_stmt) => {
            for binding in &let_stmt.bindings {
                infer_expression_type(validator, &binding.value, type_table);
            }
        }
        PrimitiveQueryStatement::For(for_stmt) => {
            infer_expression_type(validator, &for_stmt.item.collection, type_table);
        }
        PrimitiveQueryStatement::Filter(filter) => {
            infer_expression_type(validator, &filter.condition, type_table);
        }
        PrimitiveQueryStatement::Select(select) => match &select.select_items {
            crate::ast::query::SelectItemList::Items { items } => {
                for item in items {
                    infer_expression_type(validator, &item.expression, type_table);
                }
            }
            crate::ast::query::SelectItemList::Star => {}
        },
        _ => {}
    }
}

/// Infers types in a modifying statement.
fn infer_modifying_statement_types(
    validator: &super::SemanticValidator,
    stmt: &crate::ast::mutation::SimpleDataModifyingStatement,
    type_table: &mut TypeTable,
) {
    use crate::ast::mutation::{PrimitiveDataModifyingStatement, SimpleDataModifyingStatement};

    match stmt {
        SimpleDataModifyingStatement::Primitive(primitive) => match primitive {
            PrimitiveDataModifyingStatement::Insert(insert_stmt) => {
                // Infer types in INSERT property specifications
                for path in &insert_stmt.pattern.paths {
                    for element in &path.elements {
                        use crate::ast::mutation::InsertElementPattern;

                        let properties_opt = match element {
                            InsertElementPattern::Node(node) => {
                                node.filler.as_ref().and_then(|f| f.properties.as_ref())
                            }
                            InsertElementPattern::Edge(edge) => {
                                let filler = match edge {
                                    crate::ast::mutation::InsertEdgePattern::PointingLeft(e) => {
                                        &e.filler
                                    }
                                    crate::ast::mutation::InsertEdgePattern::PointingRight(e) => {
                                        &e.filler
                                    }
                                    crate::ast::mutation::InsertEdgePattern::Undirected(e) => {
                                        &e.filler
                                    }
                                };
                                filler.as_ref().and_then(|f| f.properties.as_ref())
                            }
                        };

                        if let Some(properties) = properties_opt {
                            for pair in &properties.properties {
                                infer_expression_type(validator, &pair.value, type_table);
                            }
                        }
                    }
                }
            }
            PrimitiveDataModifyingStatement::Set(set_stmt) => {
                // Infer types in SET value expressions
                for item in &set_stmt.items.items {
                    use crate::ast::mutation::SetItem;

                    match item {
                        SetItem::Property(prop) => {
                            infer_expression_type(validator, &prop.value, type_table);
                        }
                        SetItem::AllProperties(all_props) => {
                            for pair in &all_props.properties.properties {
                                infer_expression_type(validator, &pair.value, type_table);
                            }
                        }
                        SetItem::Label(_) => {
                            // Labels don't have expressions to type
                        }
                    }
                }
            }
            PrimitiveDataModifyingStatement::Remove(_) => {
                // REMOVE doesn't have expressions to type
            }
            PrimitiveDataModifyingStatement::Delete(delete_stmt) => {
                // Infer types in DELETE expressions
                for item in &delete_stmt.items.items {
                    infer_expression_type(validator, &item.expression, type_table);
                }
            }
        },
        SimpleDataModifyingStatement::Call(_) => {
            // CALL procedure - would need to analyze arguments
            // Placeholder for future implementation
        }
    }
}

/// Infers the type of an expression and records it in the type table.
///
/// This function performs type inference and persists the inferred type to the type table.
/// The type can later be retrieved for validation in subsequent passes.
#[allow(clippy::only_used_in_recursion)]
fn infer_expression_type(
    validator: &super::SemanticValidator,
    expr: &crate::ast::expression::Expression,
    type_table: &mut TypeTable,
) {
    use crate::ast::expression::{BinaryOperator, Literal, UnaryOperator};
    use crate::ir::type_table::Type;

    let inferred_type = match expr {
        // Literals have direct type mappings
        crate::ast::expression::Expression::Literal(lit, _) => match lit {
            Literal::Boolean(_) => Type::Boolean,
            Literal::Null => Type::Null,
            Literal::Integer(_) => Type::Int,
            Literal::Float(_) => Type::Float,
            Literal::String(_) => Type::String,
            Literal::ByteString(_) => Type::String, // Treat as string type
            Literal::Date(_) => Type::Date,
            Literal::Time(_) => Type::Time,
            Literal::Datetime(_) => Type::Timestamp,
            Literal::Duration(_) => Type::Duration,
            Literal::List(exprs) => {
                // Infer element types recursively
                for elem in exprs {
                    infer_expression_type(validator, elem, type_table);
                }
                // For now, use List(Any) - could infer common element type
                Type::List(Box::new(Type::Any))
            }
            Literal::Record(_) => {
                // For now, use Record with empty fields
                Type::Record(vec![])
            }
        },

        // Unary operations
        crate::ast::expression::Expression::Unary(op, operand, _) => {
            infer_expression_type(validator, operand, type_table);
            match op {
                UnaryOperator::Plus | UnaryOperator::Minus => Type::Float, // Could be Int or Float, use Float as general numeric
                UnaryOperator::Not => Type::Boolean, // NOT produces boolean
            }
        }

        // Binary operations
        crate::ast::expression::Expression::Binary(op, left, right, _) => {
            infer_expression_type(validator, left, type_table);
            infer_expression_type(validator, right, type_table);
            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => Type::Float, // Arithmetic operations - use Float as general numeric
                BinaryOperator::Concatenate => Type::String, // String concatenation produces string
            }
        }

        // Comparison operations always produce boolean
        crate::ast::expression::Expression::Comparison(_, left, right, _) => {
            infer_expression_type(validator, left, type_table);
            infer_expression_type(validator, right, type_table);
            Type::Boolean
        }

        // Logical operations produce boolean
        crate::ast::expression::Expression::Logical(_, left, right, _) => {
            infer_expression_type(validator, left, type_table);
            infer_expression_type(validator, right, type_table);
            Type::Boolean
        }

        // Parenthesized expression has same type as inner expression
        crate::ast::expression::Expression::Parenthesized(inner, _) => {
            infer_expression_type(validator, inner, type_table);
            return; // Don't set type for parenthesized wrapper
        }

        // Property reference - type depends on property
        crate::ast::expression::Expression::PropertyReference(object, _prop, _) => {
            infer_expression_type(validator, object, type_table);
            Type::Any // Without schema, we don't know property types
        }

        // Variable reference - type should be looked up in symbol table
        crate::ast::expression::Expression::VariableReference(_, _) => {
            Type::Any // Without symbol table integration, use Any
        }

        // Parameter reference
        crate::ast::expression::Expression::ParameterReference(_, _) => {
            Type::Any // Parameters can be any type
        }

        // Function calls - would need function signature database
        crate::ast::expression::Expression::FunctionCall(_) => {
            Type::Any // Unknown without function signature info
        }

        // Case expressions - type is union of all THEN clause types
        crate::ast::expression::Expression::Case(_) => {
            Type::Any // Would need to infer from THEN clauses
        }

        // Cast expression - type is the target type
        crate::ast::expression::Expression::Cast(cast) => {
            infer_expression_type(validator, &cast.operand, type_table);
            // Would need to map ValueType to Type
            Type::Any
        }

        // Aggregate functions
        crate::ast::expression::Expression::AggregateFunction(agg) => {
            use crate::ast::expression::{AggregateFunction, GeneralSetFunctionType};
            match &**agg {
                AggregateFunction::CountStar { .. } => Type::Int,
                AggregateFunction::GeneralSetFunction(gsf) => {
                    infer_expression_type(validator, &gsf.expression, type_table);
                    match gsf.function_type {
                        GeneralSetFunctionType::Count => Type::Int,
                        GeneralSetFunctionType::Avg => Type::Float,
                        GeneralSetFunctionType::Sum => Type::Float,
                        GeneralSetFunctionType::Max | GeneralSetFunctionType::Min => Type::Any,
                        GeneralSetFunctionType::CollectList => Type::List(Box::new(Type::Any)),
                        _ => Type::Any, // Other aggregate functions
                    }
                }
                AggregateFunction::BinarySetFunction(_) => Type::Any,
            }
        }

        // Type annotation - use the annotated type
        crate::ast::expression::Expression::TypeAnnotation(inner, _annotation, _) => {
            infer_expression_type(validator, inner, type_table);
            Type::Any // Would need to convert ValueType to Type
        }

        // List constructor
        crate::ast::expression::Expression::ListConstructor(elements, _) => {
            for elem in elements {
                infer_expression_type(validator, elem, type_table);
            }
            Type::List(Box::new(Type::Any))
        }

        // Record constructor
        crate::ast::expression::Expression::RecordConstructor(fields, _) => {
            for field in fields {
                infer_expression_type(validator, &field.value, type_table);
            }
            Type::Record(vec![])
        }

        // Path constructor
        crate::ast::expression::Expression::PathConstructor(elements, _) => {
            for elem in elements {
                infer_expression_type(validator, elem, type_table);
            }
            Type::Path
        }

        // EXISTS predicate produces boolean
        crate::ast::expression::Expression::Exists(_) => Type::Boolean,

        // Predicates produce boolean
        crate::ast::expression::Expression::Predicate(_) => Type::Boolean,

        // Graph expressions
        crate::ast::expression::Expression::GraphExpression(inner, _) => {
            infer_expression_type(validator, inner, type_table);
            Type::Any
        }

        // Binding table expressions
        crate::ast::expression::Expression::BindingTableExpression(inner, _) => {
            infer_expression_type(validator, inner, type_table);
            Type::Any
        }

        // Subquery expressions
        crate::ast::expression::Expression::SubqueryExpression(inner, _) => {
            infer_expression_type(validator, inner, type_table);
            Type::Any
        }
    };

    // Persist the inferred type to the type table using span-based lookup
    // This allows subsequent passes to retrieve the inferred type
    type_table.set_type_by_span(&expr.span(), inferred_type);
}
