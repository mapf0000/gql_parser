//! Semantic validation and analysis tests
//!
//! This module contains tests for the semantic validator,
//! including type checking, scope analysis, and catalog integration.

mod validator;
mod scoping_and_aggregation;
mod procedure_definitions;
mod schema_integration;
mod aggregate_validation;
mod aggregation_groupby_validation;
mod callable_validation;
mod type_inference;
mod mutation_validation;
mod procedure_validation;
mod path_pattern_validation;
mod label_expression_validation;
mod subquery_exists_validation;
mod type_system_validation;
mod set_operations_validation;
mod case_expression_validation;
mod predicate_validation;
mod catalog_session_validation;
mod edge_case_regression_validation;
