//! Callable validation pass (Milestone 4).
//!
//! This module validates function and procedure calls against their signatures,
//! including arity checking and parameter validation.

use crate::ast::expression::{AggregateFunction, Expression, FunctionCall, FunctionName, GeneralSetFunctionType};
use crate::ast::program::Program;
use crate::ast::procedure::{CallProcedureStatement, ProcedureCall, NamedProcedureCall};
use crate::ast::query::PrimitiveQueryStatement;
use crate::ast::visitor::{walk_expression, walk_program, walk_primitive_query_statement, AstVisitor, VisitResult};
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
    let mut visitor = CallableValidationVisitor {
        validator,
        diagnostics,
    };

    let _ = walk_program(&mut visitor, program);
}

/// Visitor for callable validation.
struct CallableValidationVisitor<'v, 'm> {
    validator: &'v SemanticValidator<'m>,
    diagnostics: &'v mut Vec<Diag>,
}

impl<'v, 'm> CallableValidationVisitor<'v, 'm> {
    /// Validates a procedure call against the metadata provider.
    fn validate_procedure_call(&mut self, call: &NamedProcedureCall) {
        // Only validate if we have a metadata provider
        let Some(metadata) = self.validator.metadata_provider else {
            return; // No metadata provider configured, skip validation
        };

        // Get procedure name - extract from ProcedureReference
        use crate::ast::references::ProcedureReference;
        let name = match &call.procedure {
            ProcedureReference::CatalogQualified { name, .. } => &name.name,
            ProcedureReference::ReferenceParameter { name, .. } => name,
        };

        // Lookup callable signature (procedures are callables with kind=Procedure)
        let Some(signature) = metadata.lookup_callable(name) else {
            // Unknown procedure - report error
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
            return;
        };

        // Validate arity if arguments provided
        if let Some(arguments) = &call.arguments {
            let args: Vec<&Expression> = arguments.arguments.iter().map(|a| &a.expression).collect();
            if let Err(e) = metadata.validate_callable_invocation(&signature, &args) {
                self.diagnostics.push(
                    crate::diag::Diag::new(
                        crate::diag::DiagSeverity::Error,
                        format!("{}", e),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "invalid call",
                    )),
                );
            }
        }

        // Validate YIELD clause if present
        if let Some(_yield_clause) = &call.yield_clause {
            // YIELD validation would check that yielded fields match procedure output
            // For now, we skip this as it requires more complex signature metadata
            // The MetadataProvider trait can be extended to include output field metadata
        }
    }

    /// Validates a function call against the metadata provider.
    fn validate_function_call(&mut self, call: &FunctionCall) {
        // Only validate if we have a metadata provider
        let Some(metadata) = self.validator.metadata_provider else {
            return; // No metadata provider configured, skip validation
        };

        // Build callable name
        let name = function_name_to_string(&call.name);

        // Lookup callable signature
        let Some(signature) = metadata.lookup_callable(name) else {
            // Unknown callable - report error
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
            return;
        };

        // Validate the call using metadata provider
        let args: Vec<&Expression> = call.arguments.iter().collect();
        if let Err(e) = metadata.validate_callable_invocation(&signature, &args) {
            self.diagnostics.push(
                crate::diag::Diag::new(
                    crate::diag::DiagSeverity::Error,
                    format!("{}", e),
                )
                .with_label(crate::diag::DiagLabel::primary(
                    call.span.clone(),
                    "invalid call",
                )),
            );
        }
    }

    /// Validates an aggregate function call against the metadata provider.
    fn validate_aggregate_function(&mut self, agg: &AggregateFunction) {
        // Only validate if we have a metadata provider
        let Some(metadata) = self.validator.metadata_provider else {
            return; // No metadata provider configured, skip validation
        };

        match agg {
            AggregateFunction::CountStar { span } => {
                // COUNT(*) - special case with 0 arguments
                if let Some(signature) = metadata.lookup_callable("count") {
                    let args: Vec<&Expression> = vec![];
                    if let Err(e) = metadata.validate_callable_invocation(&signature, &args) {
                        self.diagnostics.push(
                            crate::diag::Diag::new(
                                crate::diag::DiagSeverity::Error,
                                format!("{}", e),
                            )
                            .with_label(crate::diag::DiagLabel::primary(
                                span.clone(),
                                "invalid aggregate",
                            )),
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

                if let Some(signature) = metadata.lookup_callable(name) {
                    let args: Vec<&Expression> = vec![general_func.expression.as_ref()];
                    if let Err(e) = metadata.validate_callable_invocation(&signature, &args) {
                        self.diagnostics.push(
                            crate::diag::Diag::new(
                                crate::diag::DiagSeverity::Error,
                                format!("{}", e),
                            )
                            .with_label(crate::diag::DiagLabel::primary(
                                general_func.span.clone(),
                                "invalid aggregate",
                            )),
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

impl<'v, 'm> AstVisitor for CallableValidationVisitor<'v, 'm> {
    type Break = ();

    fn visit_primitive_query_statement(
        &mut self,
        statement: &PrimitiveQueryStatement,
    ) -> VisitResult<()> {
        // Check if this is a CALL statement
        if let PrimitiveQueryStatement::Call(call_stmt) = statement {
            // Validate procedure call
            if let ProcedureCall::Named(named_call) = &call_stmt.call {
                self.validate_procedure_call(named_call);
            }
        }
        // Continue walking the statement
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
