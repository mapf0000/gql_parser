//! Callable validation pass (Milestone 4).
//!
//! This module validates function and procedure calls against their signatures,
//! including arity checking and parameter validation.

use crate::ast::expression::{AggregateFunction, Expression, FunctionCall, FunctionName, GeneralSetFunctionType};
use crate::ast::program::Program;
use crate::ast::visitor::{walk_expression, walk_program, AstVisitor, VisitResult};
use crate::diag::Diag;
use crate::semantic::callable::{CallSite, CallableKind, CallableLookupContext};

use super::SemanticValidator;

/// Runs callable validation on the AST.
///
/// This pass validates:
/// - Function arity (argument count)
/// - Aggregate function arity
/// - Parameter types (if catalog provides signatures)
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
struct CallableValidationVisitor<'v, 's, 'c> {
    validator: &'v SemanticValidator<'s, 'c>,
    diagnostics: &'v mut Vec<Diag>,
}

impl<'v, 's, 'c> CallableValidationVisitor<'v, 's, 'c> {
    /// Validates a function call against the callable catalog.
    fn validate_function_call(&mut self, call: &FunctionCall) {
        // Only validate if we have a callable catalog and validator
        let (catalog, validator) = match (
            &self.validator.callable_catalog,
            &self.validator.callable_validator,
        ) {
            (Some(c), Some(v)) => (c, v),
            _ => return, // No catalog configured, skip validation
        };

        // Get function name - map FunctionName enum to string
        let name = match &call.name {
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

            // Custom functions - resolve by name
            FunctionName::Custom(name) => name.as_str(),
        };

        // Create call site
        let call_site = CallSite {
            name,
            kind: CallableKind::Function,
            arg_count: call.arguments.len(),
            span: call.span.clone(),
        };

        // Resolve signatures
        let ctx = CallableLookupContext::new();
        let signatures = match catalog.resolve(name, CallableKind::Function, &ctx) {
            Ok(sigs) => sigs,
            Err(e) => {
                // Catalog error - report as diagnostic
                self.diagnostics.push(
                    crate::diag::Diag::new(
                        crate::diag::DiagSeverity::Error,
                        format!("Failed to resolve function '{}': {}", name, e),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "catalog error",
                    )),
                );
                return;
            }
        };

        // Validate the call
        let mut call_diagnostics = validator.validate_call(&call_site, &signatures);
        self.diagnostics.append(&mut call_diagnostics);
    }

    /// Validates an aggregate function call against the callable catalog.
    fn validate_aggregate_function(&mut self, agg: &AggregateFunction) {
        // Only validate if we have a callable catalog and validator
        let (catalog, validator) = match (
            &self.validator.callable_catalog,
            &self.validator.callable_validator,
        ) {
            (Some(c), Some(v)) => (c, v),
            _ => return, // No catalog configured, skip validation
        };

        match agg {
            AggregateFunction::CountStar { span } => {
                // COUNT(*) is a special case with 0 arguments
                let call_site = CallSite {
                    name: "count",
                    kind: CallableKind::AggregateFunction,
                    arg_count: 0,
                    span: span.clone(),
                };

                let ctx = CallableLookupContext::new();
                if let Ok(signatures) = catalog.resolve("count", CallableKind::AggregateFunction, &ctx) {
                    // COUNT(*) is always valid, but we still validate it exists
                    if signatures.is_empty() {
                        let mut call_diagnostics = validator.validate_call(&call_site, &signatures);
                        self.diagnostics.append(&mut call_diagnostics);
                    }
                }
            }
            AggregateFunction::GeneralSetFunction(func) => {
                // Map aggregate function type to name
                let name = match func.function_type {
                    GeneralSetFunctionType::Avg => "avg",
                    GeneralSetFunctionType::Count => "count",
                    GeneralSetFunctionType::Max => "max",
                    GeneralSetFunctionType::Min => "min",
                    GeneralSetFunctionType::Sum => "sum",
                    GeneralSetFunctionType::CollectList => "collect",
                    GeneralSetFunctionType::StddevSamp => "stddev_samp",
                    GeneralSetFunctionType::StddevPop => "stddev_pop",
                };

                // Create call site (aggregate functions take 1 argument)
                let call_site = CallSite {
                    name,
                    kind: CallableKind::AggregateFunction,
                    arg_count: 1,
                    span: func.span.clone(),
                };

                // Resolve and validate
                let ctx = CallableLookupContext::new();
                if let Ok(signatures) = catalog.resolve(name, CallableKind::AggregateFunction, &ctx) {
                    let mut call_diagnostics = validator.validate_call(&call_site, &signatures);
                    self.diagnostics.append(&mut call_diagnostics);
                }
            }
            AggregateFunction::BinarySetFunction(func) => {
                // Binary set functions (PERCENTILE_CONT, PERCENTILE_DISC) take 2 arguments
                // These are not in the built-in catalog yet, so we skip validation
                // Future: Add these to the catalog
                let _ = func;
            }
        }
    }
}

impl<'v, 's, 'c> AstVisitor for CallableValidationVisitor<'v, 's, 'c> {
    type Break = ();

    fn visit_expression(&mut self, expr: &Expression) -> VisitResult<Self::Break> {
        match expr {
            Expression::FunctionCall(call) => {
                self.validate_function_call(call);
                // Continue walking to validate nested expressions
                for arg in &call.arguments {
                    let _ = walk_expression(self, arg);
                }
            }
            Expression::AggregateFunction(agg) => {
                self.validate_aggregate_function(agg);
                // Walk nested expressions in aggregate
                match agg.as_ref() {
                    AggregateFunction::CountStar { .. } => {}
                    AggregateFunction::GeneralSetFunction(func) => {
                        let _ = walk_expression(self, &func.expression);
                    }
                    AggregateFunction::BinarySetFunction(func) => {
                        let _ = walk_expression(self, &func.inverse_distribution_argument);
                        let _ = walk_expression(self, &func.expression);
                    }
                }
            }
            _ => {
                // For other expressions, continue walking
                let _ = walk_expression(self, expr);
            }
        }
        VisitResult::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::program::Program;
    use crate::semantic::callable::{
        BuiltinCallableCatalog, CallableCatalog, DefaultCallableValidator,
    };
    use crate::semantic::validator::SemanticValidator;

    #[test]
    fn test_callable_validation_infrastructure() {
        let program = Program {
            statements: vec![],
            span: 0..0,
        };

        let catalog = BuiltinCallableCatalog::new();
        let validator_impl = DefaultCallableValidator::new();

        let validator = SemanticValidator::new()
            .with_callable_catalog(&catalog)
            .with_callable_validator(&validator_impl);

        let mut diagnostics = Vec::new();
        run_callable_validation(&validator, &program, &mut diagnostics);

        // Should have no errors for empty program
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_callable_catalog_integration() {
        let catalog = BuiltinCallableCatalog::new();
        let ctx = crate::semantic::callable::CallableLookupContext::new();

        // Test that built-in functions are registered
        let sigs = catalog
            .resolve("abs", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].min_arity(), 1);
        assert_eq!(sigs[0].max_arity(), Some(1));
    }
}
