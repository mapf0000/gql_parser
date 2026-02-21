//! Callable validation pass (Milestone 4).
//!
//! This module validates function and procedure calls against their signatures,
//! including arity checking and parameter validation.

use crate::ast::expression::{
    AggregateFunction, Expression, FunctionCall, FunctionName, GeneralSetFunctionType,
};
use crate::ast::procedure::{NamedProcedureCall, ProcedureCall};
use crate::ast::program::Program;
use crate::ast::query::PrimitiveQueryStatement;
use crate::ast::visit::{
    Visit, VisitResult, walk_expression, walk_linear_query, walk_primitive_query_statement,
    walk_program,
};
use crate::diag::Diag;

use super::SemanticValidator;

/// Runs callable validation on the AST.
///
/// This pass validates:
/// - Function arity (argument count)
/// - Aggregate function arity
/// - Parameter types (if metadata provider provides signatures)
pub(super) fn run_callable_validation(
    validator: &SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    eprintln!("DEBUG: run_callable_validation called");
    eprintln!(
        "DEBUG: metadata_provider is_some: {}",
        validator.metadata_provider.is_some()
    );

    let mut visitor = CallableValidationVisitor {
        validator,
        diagnostics,
    };

    let _ = walk_program(&mut visitor, program);
    eprintln!("DEBUG: walk_program completed");
}

/// Visitor for callable validation.
struct CallableValidationVisitor<'v, 'm> {
    validator: &'v SemanticValidator<'m>,
    diagnostics: &'v mut Vec<Diag>,
}

impl<'v, 'm> CallableValidationVisitor<'v, 'm> {
    /// Validates a procedure call against the metadata provider.
    fn validate_procedure_call(&mut self, call: &NamedProcedureCall) {
        eprintln!("DEBUG: validate_procedure_call called");

        // Get procedure name - extract from ProcedureReference
        use crate::ast::references::ProcedureReference;
        let name = match &call.procedure {
            ProcedureReference::CatalogQualified { name, .. } => &name.name,
            ProcedureReference::ReferenceParameter { name, .. } => name,
        };

        eprintln!("DEBUG: Looking up callable: {}", name);

        // Check built-ins first (direct function call - zero cost)
        use crate::semantic::callable::{CallableKind, lookup_builtin_callable};
        let signature = lookup_builtin_callable(name, CallableKind::Procedure)
            .or_else(|| lookup_builtin_callable(name, CallableKind::Function))
            // Then check metadata provider for UDFs (if configured)
            .or_else(|| {
                self.validator
                    .metadata_provider
                    .and_then(|m| m.lookup_callable(name))
            });

        let Some(signature) = signature else {
            eprintln!("DEBUG: Callable not found, reporting error");
            // Only report error if metadata validation is enabled
            if self.validator.config.metadata_validation {
                self.diagnostics.push(
                    crate::diag::Diag::new(
                        crate::diag::DiagSeverity::Error,
                        format!("Unknown procedure or function '{}'", name),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "not found in catalog",
                    )),
                );
            }
            return;
        };

        eprintln!("DEBUG: Callable found: {:?}", signature.name);

        // Validate arity if arguments provided
        if let Some(arguments) = &call.arguments {
            let args: Vec<&Expression> =
                arguments.arguments.iter().map(|a| &a.expression).collect();

            // Use metadata provider for validation if available, otherwise do basic arity check
            let validation_result = if let Some(metadata) = self.validator.metadata_provider {
                metadata.validate_callable_invocation(&signature, &args)
            } else {
                // Basic arity check
                let min_arity = signature.min_arity();
                if args.len() < min_arity {
                    Err(format!(
                        "Function '{}' requires at least {} arguments, got {}",
                        signature.name,
                        min_arity,
                        args.len()
                    ))
                } else if let Some(max_arity) = signature.max_arity() {
                    if args.len() > max_arity {
                        Err(format!(
                            "Function '{}' accepts at most {} arguments, got {}",
                            signature.name,
                            max_arity,
                            args.len()
                        ))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            };

            if let Err(e) = validation_result {
                self.diagnostics.push(
                    crate::diag::Diag::new(crate::diag::DiagSeverity::Error, e.to_string())
                        .with_label(crate::diag::DiagLabel::primary(
                            call.span.clone(),
                            "invalid call",
                        )),
                );
            }
        }

        // Validate YIELD clause if present
        if let Some(yield_clause) = &call.yield_clause {
            // YIELD validation: check that yielded fields match procedure output
            // The return_type in CallableSignature represents the output field name(s)
            if let Some(return_type) = &signature.return_type {
                // Check each yielded field
                for yield_item in &yield_clause.items.items {
                    // Extract the field name from the expression
                    // In typical cases, it should be a variable reference
                    use crate::ast::expression::Expression;
                    let field_name = match &yield_item.expression {
                        Expression::VariableReference(name, _span) => name,
                        _ => continue, // Skip validation for non-variable expressions
                    };

                    // Simple validation: check if field name matches return type
                    // In a more sophisticated implementation, return_type could be
                    // a comma-separated list or a structured type
                    // Note: Use case-insensitive comparison because keywords used as identifiers
                    // (like 'node', 'result', etc.) are normalized to uppercase by the parser
                    if !field_name.eq_ignore_ascii_case(return_type.as_str()) {
                        self.diagnostics.push(
                            crate::diag::Diag::new(
                                crate::diag::DiagSeverity::Error,
                                format!(
                                    "YIELD field '{}' does not exist in procedure output. Expected '{}'",
                                    field_name, return_type
                                ),
                            )
                            .with_label(crate::diag::DiagLabel::primary(
                                yield_item.span.clone(),
                                "invalid field",
                            )),
                        );
                    }
                }
            }
        }
    }

    /// Validates a function call against the metadata provider.
    fn validate_function_call(&mut self, call: &FunctionCall) {
        // Build callable name
        let name = function_name_to_string(&call.name);

        // Check built-ins first (direct function call - zero cost)
        use crate::semantic::callable::{CallableKind, lookup_builtin_callable};
        let signature = lookup_builtin_callable(name, CallableKind::Function)
            .or_else(|| lookup_builtin_callable(name, CallableKind::AggregateFunction))
            // Then check metadata provider for UDFs (if configured)
            .or_else(|| {
                self.validator
                    .metadata_provider
                    .and_then(|m| m.lookup_callable(name))
            });

        let Some(signature) = signature else {
            // Only report error if metadata validation is enabled
            if self.validator.config.metadata_validation {
                self.diagnostics.push(
                    crate::diag::Diag::new(
                        crate::diag::DiagSeverity::Error,
                        format!("Unknown function '{}'", name),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "undefined function",
                    )),
                );
            }
            return;
        };

        // Validate the call
        let args: Vec<&Expression> = call.arguments.iter().collect();

        // Use metadata provider for validation if available, otherwise do basic arity check
        let validation_result = if let Some(metadata) = self.validator.metadata_provider {
            metadata.validate_callable_invocation(&signature, &args)
        } else {
            // Basic arity check
            let min_arity = signature.min_arity();
            if args.len() < min_arity {
                Err(format!(
                    "Function '{}' requires at least {} arguments, got {}",
                    signature.name,
                    min_arity,
                    args.len()
                ))
            } else if let Some(max_arity) = signature.max_arity() {
                if args.len() > max_arity {
                    Err(format!(
                        "Function '{}' accepts at most {} arguments, got {}",
                        signature.name,
                        max_arity,
                        args.len()
                    ))
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        };

        if let Err(e) = validation_result {
            self.diagnostics.push(
                crate::diag::Diag::new(crate::diag::DiagSeverity::Error, e.to_string())
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "invalid call",
                    )),
            );
        }
    }

    /// Validates an aggregate function call against the metadata provider.
    fn validate_aggregate_function(&mut self, agg: &AggregateFunction) {
        use crate::semantic::callable::{CallableKind, lookup_builtin_callable};

        match agg {
            AggregateFunction::CountStar { span } => {
                // COUNT(*) - special case with 0 arguments
                // Check built-ins first, then UDFs
                let signature = lookup_builtin_callable("count", CallableKind::AggregateFunction)
                    .or_else(|| {
                        self.validator
                            .metadata_provider
                            .and_then(|m| m.lookup_callable("count"))
                    });

                if let Some(signature) = signature {
                    let args: Vec<&Expression> = vec![];

                    // Use metadata provider for validation if available, otherwise do basic arity check
                    let validation_result = if let Some(metadata) = self.validator.metadata_provider
                    {
                        metadata.validate_callable_invocation(&signature, &args)
                    } else {
                        // Basic arity check
                        if signature.matches_arity(0) {
                            Ok(())
                        } else {
                            Err(format!(
                                "Function '{}' does not accept {} arguments",
                                signature.name, 0
                            ))
                        }
                    };

                    if let Err(e) = validation_result {
                        self.diagnostics.push(
                            crate::diag::Diag::new(
                                crate::diag::DiagSeverity::Error,
                                e.to_string(),
                            )
                            .with_label(
                                crate::diag::DiagLabel::primary(span.clone(), "invalid aggregate"),
                            ),
                        );
                    }
                }
            }
            AggregateFunction::GeneralSetFunction(general_func) => {
                // Aggregate functions like COUNT, SUM, AVG, etc.
                let name = match general_func.function_type {
                    GeneralSetFunctionType::Avg => "avg",
                    GeneralSetFunctionType::Count => "count",
                    GeneralSetFunctionType::Max => "max",
                    GeneralSetFunctionType::Min => "min",
                    GeneralSetFunctionType::Sum => "sum",
                    GeneralSetFunctionType::CollectList => "collect_list",
                    GeneralSetFunctionType::StddevSamp => "stddev_samp",
                    GeneralSetFunctionType::StddevPop => "stddev_pop",
                };

                // Check built-ins first, then UDFs
                let signature = lookup_builtin_callable(name, CallableKind::AggregateFunction)
                    .or_else(|| {
                        self.validator
                            .metadata_provider
                            .and_then(|m| m.lookup_callable(name))
                    });

                if let Some(signature) = signature {
                    let args: Vec<&Expression> = vec![general_func.expression.as_ref()];

                    // Use metadata provider for validation if available, otherwise do basic arity check
                    let validation_result = if let Some(metadata) = self.validator.metadata_provider
                    {
                        metadata.validate_callable_invocation(&signature, &args)
                    } else {
                        // Basic arity check
                        if signature.matches_arity(1) {
                            Ok(())
                        } else {
                            Err(format!(
                                "Function '{}' does not accept {} arguments",
                                signature.name, 1
                            ))
                        }
                    };

                    if let Err(e) = validation_result {
                        self.diagnostics.push(
                            crate::diag::Diag::new(
                                crate::diag::DiagSeverity::Error,
                                e.to_string(),
                            )
                            .with_label(
                                crate::diag::DiagLabel::primary(
                                    general_func.span.clone(),
                                    "invalid aggregate",
                                ),
                            ),
                        );
                    }
                }
            }
            AggregateFunction::BinarySetFunction(binary_func) => {
                // Binary set functions like PERCENTILE_CONT, PERCENTILE_DISC
                // For now, we skip validation for these as they require special handling
                let _ = binary_func; // Suppress unused warning
            }
        }
    }
}

impl<'v, 'm> Visit for CallableValidationVisitor<'v, 'm> {
    type Break = ();

    fn visit_linear_query(&mut self, query: &crate::ast::query::LinearQuery) -> VisitResult<()> {
        eprintln!("DEBUG: visit_linear_query called");
        eprintln!(
            "DEBUG: Query type: {}, primitive_statements count: {}",
            if query.use_graph.is_some() {
                "Focused"
            } else {
                "Ambient"
            },
            query.primitive_statements.len()
        );
        walk_linear_query(self, query)
    }

    fn visit_primitive_query_statement(
        &mut self,
        statement: &PrimitiveQueryStatement,
    ) -> VisitResult<()> {
        eprintln!("DEBUG: visit_primitive_query_statement called");

        // Check if this is a CALL statement
        if let PrimitiveQueryStatement::Call(call_stmt) = statement {
            eprintln!("DEBUG: Found CALL statement");
            // Validate procedure call
            if let ProcedureCall::Named(named_call) = &call_stmt.call {
                eprintln!("DEBUG: Named procedure call");
                self.validate_procedure_call(named_call);
            }
        }

        // Continue walking the statement to visit expressions
        walk_primitive_query_statement(self, statement)
    }

    fn visit_expression(&mut self, expr: &Expression) -> VisitResult<()> {
        match expr {
            Expression::FunctionCall(call) => {
                // Validate function call
                self.validate_function_call(call);
                walk_expression(self, expr)
            }
            Expression::AggregateFunction(agg) => {
                // Validate aggregate function
                self.validate_aggregate_function(agg);
                walk_expression(self, expr)
            }
            _ => walk_expression(self, expr),
        }
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
        FunctionName::BTrim => "trim",
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
