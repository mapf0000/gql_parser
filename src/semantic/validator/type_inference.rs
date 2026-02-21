//! Type inference pass - infers types for expressions and builds the type table.

use crate::ast::expression::{BinaryOperator, FunctionName, Literal, UnaryOperator};
use crate::ast::program::{Program, Statement};
use crate::ast::query::{LinearQuery, PrimitiveQueryStatement, Query};
use crate::diag::Diag;
use crate::ir::type_table::Type;
use crate::ir::{SymbolTable, TypeTable};
use crate::semantic::schema_catalog::TypeRef;

/// Pass 2: Type Inference - Infers types for all expressions in the program.
///
/// This pass walks the AST and infers types for expressions, building a type table
/// that can be queried by subsequent validation passes.
///
/// # Milestone 5 Enhancements
///
/// When type metadata catalog is available, this pass will:
/// - Query property types from the catalog instead of falling back to Type::Any
/// - Query callable return types for better function inference
/// - Apply inference policies to control fallback behavior
/// - Preserve integer types in arithmetic when appropriate
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
    let primitive_statements = &linear_query.primitive_statements;

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
    

    let statements = &mutation.statements;

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
///
/// # Milestone 5 Integration
///
/// This function now utilizes the type metadata catalog when available to:
/// - Query property types instead of defaulting to Type::Any
/// - Query callable return types for functions
/// - Respect inference policy for fallback behavior
fn infer_expression_type(
    validator: &super::SemanticValidator,
    expr: &crate::ast::expression::Expression,
    type_table: &mut TypeTable,
) -> Type {
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
                // Infer element types recursively and find common type
                if exprs.is_empty() {
                    Type::List(Box::new(Type::Any))
                } else {
                    let elem_types: Vec<Type> = exprs
                        .iter()
                        .map(|e| infer_expression_type(validator, e, type_table))
                        .collect();

                    // Find common type
                    let common_type = infer_common_type(&elem_types);
                    Type::List(Box::new(common_type))
                }
            }
            Literal::Record(_) => {
                // For now, use Record with empty fields
                Type::Record(vec![])
            }
        },

        // Unary operations
        crate::ast::expression::Expression::Unary(op, operand, _) => {
            let operand_type = infer_expression_type(validator, operand, type_table);
            match op {
                UnaryOperator::Plus | UnaryOperator::Minus => {
                    // Preserve the numeric type: +5 is Int, +5.0 is Float
                    if operand_type.is_numeric() {
                        operand_type
                    } else {
                        Type::Float // Fallback for non-numeric
                    }
                }
                UnaryOperator::Not => Type::Boolean, // NOT produces boolean
            }
        }

        // Binary operations
        crate::ast::expression::Expression::Binary(op, left, right, _) => {
            let left_type = infer_expression_type(validator, left, type_table);
            let right_type = infer_expression_type(validator, right, type_table);

            match op {
                BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    // Preserve Int if both operands are Int, otherwise Float
                    match (&left_type, &right_type) {
                        (Type::Int, Type::Int) if *op != BinaryOperator::Divide => Type::Int,
                        (Type::Int, Type::Float) | (Type::Float, Type::Int) | (Type::Float, Type::Float) => Type::Float,
                        _ if left_type.is_numeric() && right_type.is_numeric() => Type::Float,
                        _ => Type::Float, // Fallback for Any or unknown types
                    }
                }
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
            return infer_expression_type(validator, inner, type_table);
        }

        // Property reference - query from metadata provider
        crate::ast::expression::Expression::PropertyReference(object, prop_name, _) => {
            let object_type = infer_expression_type(validator, object, type_table);

            // Try to query property type from metadata provider
            if let Some(metadata) = validator.metadata_provider {
                if let Some(owner_ref) = type_to_type_ref(&object_type) {
                    if let Some(prop_type) = metadata.get_property_metadata(&owner_ref, prop_name.as_str()) {
                        return map_value_type_to_type(&prop_type);
                    }
                }
            }

            // Fallback based on policy
            fallback_type(validator)
        }

        // Variable reference - type should be looked up in symbol table
        crate::ast::expression::Expression::VariableReference(_, _) => {
            // TODO: Lookup in symbol table once integrated
            fallback_type(validator)
        }

        // Parameter reference
        crate::ast::expression::Expression::ParameterReference(_, _) => {
            // Parameters can be any type
            fallback_type(validator)
        }

        // Function calls - query from metadata provider
        crate::ast::expression::Expression::FunctionCall(func_call) => {
            // Infer argument types
            for arg in &func_call.arguments {
                infer_expression_type(validator, arg, type_table);
            }

            // Try to query return type from metadata provider
            if let Some(metadata) = validator.metadata_provider {
                let name_str = function_name_to_string(&func_call.name);
                if let Some(return_type) = metadata.get_callable_return_type_metadata(name_str) {
                    return map_value_type_to_type(&return_type);
                }
            }

            // Fallback based on policy
            fallback_type(validator)
        }

        // Case expressions - type is union of all THEN clause types
        crate::ast::expression::Expression::Case(case_expr) => {
            let mut result_types = Vec::new();

            // Handle both Simple and Searched CASE expressions
            match case_expr {
                crate::ast::expression::CaseExpression::Searched(searched) => {
                    // Collect types from all THEN clauses
                    for when_clause in &searched.when_clauses {
                        let then_type = infer_expression_type(validator, &when_clause.then_result, type_table);
                        result_types.push(then_type);
                    }

                    // ELSE clause if present
                    if let Some(else_expr) = &searched.else_clause {
                        let else_type = infer_expression_type(validator, else_expr, type_table);
                        result_types.push(else_type);
                    }
                }
                crate::ast::expression::CaseExpression::Simple(simple) => {
                    // Infer operand type
                    infer_expression_type(validator, &simple.operand, type_table);

                    // Collect types from all THEN clauses
                    for when_clause in &simple.when_clauses {
                        let then_type = infer_expression_type(validator, &when_clause.then_result, type_table);
                        result_types.push(then_type);
                    }

                    // ELSE clause if present
                    if let Some(else_expr) = &simple.else_clause {
                        let else_type = infer_expression_type(validator, else_expr, type_table);
                        result_types.push(else_type);
                    }
                }
            }

            // Find common type
            if result_types.is_empty() {
                fallback_type(validator)
            } else {
                infer_common_type(&result_types)
            }
        }

        // Cast expression - type is the target type
        crate::ast::expression::Expression::Cast(cast) => {
            infer_expression_type(validator, &cast.operand, type_table);
            // Map ValueType to Type
            map_value_type_to_type(&cast.target_type)
        }

        // Aggregate functions
        crate::ast::expression::Expression::AggregateFunction(agg) => {
            use crate::ast::expression::{AggregateFunction, GeneralSetFunctionType};
            match &**agg {
                AggregateFunction::CountStar { .. } => Type::Int,
                AggregateFunction::GeneralSetFunction(gsf) => {
                    let expr_type = infer_expression_type(validator, &gsf.expression, type_table);
                    match gsf.function_type {
                        GeneralSetFunctionType::Count => Type::Int,
                        GeneralSetFunctionType::Avg => Type::Float,
                        GeneralSetFunctionType::Sum => {
                            // SUM preserves type: SUM(int) = Int, SUM(float) = Float
                            if expr_type == Type::Int {
                                Type::Int
                            } else {
                                Type::Float
                            }
                        }
                        GeneralSetFunctionType::Max | GeneralSetFunctionType::Min => {
                            // MAX/MIN preserve input type
                            expr_type
                        }
                        GeneralSetFunctionType::CollectList => Type::List(Box::new(expr_type)),
                        _ => fallback_type(validator), // Other aggregate functions
                    }
                }
                AggregateFunction::BinarySetFunction(_) => fallback_type(validator),
            }
        }

        // Type annotation - use the annotated type
        crate::ast::expression::Expression::TypeAnnotation(inner, annotation, _) => {
            infer_expression_type(validator, inner, type_table);
            map_value_type_to_type(&annotation.type_ref)
        }

        // List constructor
        crate::ast::expression::Expression::ListConstructor(elements, _) => {
            if elements.is_empty() {
                Type::List(Box::new(Type::Any))
            } else {
                let elem_types: Vec<Type> = elements
                    .iter()
                    .map(|e| infer_expression_type(validator, e, type_table))
                    .collect();
                let common_type = infer_common_type(&elem_types);
                Type::List(Box::new(common_type))
            }
        }

        // Record constructor
        crate::ast::expression::Expression::RecordConstructor(fields, _) => {
            let field_types: Vec<(String, Type)> = fields
                .iter()
                .map(|field| {
                    let ty = infer_expression_type(validator, &field.value, type_table);
                    (field.name.to_string(), ty)
                })
                .collect();
            Type::Record(field_types)
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
            infer_expression_type(validator, inner, type_table)
        }

        // Binding table expressions
        crate::ast::expression::Expression::BindingTableExpression(inner, _) => {
            infer_expression_type(validator, inner, type_table)
        }

        // Subquery expressions
        crate::ast::expression::Expression::SubqueryExpression(_, _) => fallback_type(validator),
    };

    // Persist the inferred type to the type table using span-based lookup
    type_table.set_type_by_span(&expr.span(), inferred_type.clone());
    inferred_type
}

/// Returns the appropriate fallback type.
fn fallback_type(_validator: &super::SemanticValidator) -> Type {
    // For now, always return Type::Any as the fallback
    // In the future, this could be controlled by validation config
    Type::Any
}

/// Converts a Type to a TypeRef for catalog lookups.
fn type_to_type_ref(ty: &Type) -> Option<TypeRef> {
    match ty {
        Type::Node(Some(labels)) if !labels.is_empty() => {
            Some(TypeRef::NodeType(labels[0].as_str().into()))
        }
        Type::Node(None) => None, // Can't query properties without knowing the label
        Type::Edge(Some(labels)) if !labels.is_empty() => {
            Some(TypeRef::EdgeType(labels[0].as_str().into()))
        }
        Type::Edge(None) => None,
        _ => None,
    }
}

/// Infers a common type from a list of types.
///
/// Returns the most specific type that all input types can be converted to:
/// - If all types are the same, return that type
/// - If all types are numeric, return Float (widest numeric type)
/// - If types include Any, return Any
/// - Otherwise, return a Union type or Any
fn infer_common_type(types: &[Type]) -> Type {
    if types.is_empty() {
        return Type::Any;
    }

    // If all types are the same, return that type
    let first = &types[0];
    if types.iter().all(|t| t == first) {
        return first.clone();
    }

    // If any type is Any, return Any
    if types.iter().any(|t| matches!(t, Type::Any)) {
        return Type::Any;
    }

    // If all types are numeric, return Float (widest)
    if types.iter().all(|t| t.is_numeric()) {
        return Type::Float;
    }

    // If all types are compatible, return the first one
    if types.iter().all(|t| first.is_compatible_with(t)) {
        return first.clone();
    }

    // Otherwise, return a union type (or Any as fallback)
    Type::Union(types.to_vec())
}

/// Maps a ValueType from the AST to a Type.
fn map_value_type_to_type(value_type: &crate::ast::types::ValueType) -> Type {
    use crate::ast::types::{PredefinedType, ValueType};

    match value_type {
        ValueType::Predefined(ptype, _) => match ptype {
            PredefinedType::Boolean(_) => Type::Boolean,
            PredefinedType::CharacterString(_) => Type::String,
            PredefinedType::ByteString(_) => Type::String,
            PredefinedType::Numeric(ntype) => {
                use crate::ast::types::NumericType;
                match ntype {
                    NumericType::Exact(_) => Type::Int,
                    NumericType::Approximate(_) => Type::Float,
                }
            }
            PredefinedType::Temporal(ttype) => {
                use crate::ast::types::{TemporalInstantType, TemporalType};
                match ttype {
                    TemporalType::Instant(itype) => match itype {
                        TemporalInstantType::Date => Type::Date,
                        TemporalInstantType::LocalTime | TemporalInstantType::ZonedTime => {
                            Type::Time
                        }
                        TemporalInstantType::LocalDatetime | TemporalInstantType::ZonedDatetime => {
                            Type::Timestamp
                        }
                    },
                    TemporalType::Duration(_) => Type::Duration,
                }
            }
            PredefinedType::ReferenceValue(rtype) => {
                use crate::ast::types::ReferenceValueType;
                match rtype {
                    ReferenceValueType::Node(_) => Type::Node(None),
                    ReferenceValueType::Edge(_) => Type::Edge(None),
                    ReferenceValueType::Graph(_) => Type::Any, // No direct graph type
                    ReferenceValueType::BindingTable(_) => Type::Any,
                }
            }
            PredefinedType::Immaterial(_) => Type::Any,
        },
        ValueType::Path(_) => Type::Path,
        ValueType::List(list_type) => {
            Type::List(Box::new(map_value_type_to_type(&list_type.element_type)))
        }
        ValueType::Record(_) => Type::Any,
    }
}

/// Helper function to convert FunctionName enum to string.
fn function_name_to_string(name: &FunctionName) -> &str {
    match name {
        // Numeric functions
        FunctionName::Abs => "abs",
        FunctionName::Mod => "mod",
        FunctionName::Floor => "floor",
        FunctionName::Ceil => "ceil",
        FunctionName::Sqrt => "sqrt",
        FunctionName::Power => "power",
        FunctionName::Exp => "exp",
        FunctionName::Ln => "ln",
        FunctionName::Log => "log",
        FunctionName::Log10 => "log10",

        // Trigonometric functions
        FunctionName::Sin => "sin",
        FunctionName::Cos => "cos",
        FunctionName::Tan => "tan",
        FunctionName::Cot => "cot",
        FunctionName::Sinh => "sinh",
        FunctionName::Cosh => "cosh",
        FunctionName::Tanh => "tanh",
        FunctionName::Asin => "asin",
        FunctionName::Acos => "acos",
        FunctionName::Atan => "atan",
        FunctionName::Atan2 => "atan2",
        FunctionName::Degrees => "degrees",
        FunctionName::Radians => "radians",

        // String functions
        FunctionName::Upper => "upper",
        FunctionName::Lower => "lower",
        FunctionName::Trim(_) => "trim",
        FunctionName::BTrim => "btrim",
        FunctionName::LTrim => "ltrim",
        FunctionName::RTrim => "rtrim",
        FunctionName::Left => "left",
        FunctionName::Right => "right",
        FunctionName::Normalize => "normalize",
        FunctionName::CharLength => "char_length",
        FunctionName::ByteLength => "byte_length",
        FunctionName::Substring => "substring",

        // Temporal functions
        FunctionName::CurrentDate => "current_date",
        FunctionName::CurrentTime => "current_time",
        FunctionName::CurrentTimestamp => "current_timestamp",
        FunctionName::Date => "date",
        FunctionName::Time => "time",
        FunctionName::Datetime => "datetime",
        FunctionName::ZonedTime => "zoned_time",
        FunctionName::ZonedDatetime => "zoned_datetime",
        FunctionName::LocalTime => "local_time",
        FunctionName::LocalDatetime => "local_datetime",
        FunctionName::Duration => "duration",
        FunctionName::DurationBetween => "duration_between",

        // List functions
        FunctionName::TrimList => "trim_list",
        FunctionName::Elements => "elements",

        // Cardinality functions
        FunctionName::Cardinality => "cardinality",
        FunctionName::Size => "size",
        FunctionName::PathLength => "path_length",

        // Graph functions
        FunctionName::ElementId => "element_id",

        // Conditional functions
        FunctionName::Coalesce => "coalesce",
        FunctionName::NullIf => "nullif",

        // Custom function
        FunctionName::Custom(name) => name,
    }
}
