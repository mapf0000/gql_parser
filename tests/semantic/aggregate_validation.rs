//! Tests for aggregate function validation (Milestone 4 enhancement).

use gql_parser::parse;
use gql_parser::semantic::callable::{
    BuiltinCallableCatalog, DefaultCallableValidator,
};
use gql_parser::semantic::SemanticValidator;

#[test]
fn test_aggregate_function_validation_count_star() {
    // COUNT(*) should validate successfully
    let source = "MATCH (n:Person) RETURN COUNT(*)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should have warnings about disconnected patterns but no errors about COUNT
        let count_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.contains("COUNT") || d.message.contains("count"))
            .collect();
        assert!(count_errors.is_empty(), "No COUNT-related errors expected");
    }
}

#[test]
fn test_aggregate_function_validation_sum() {
    // SUM(expr) should validate successfully
    let source = "MATCH (n:Person) RETURN SUM(n.age)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let sum_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.contains("SUM") || d.message.contains("sum"))
            .collect();
        assert!(sum_errors.is_empty(), "No SUM-related errors expected");
    }
}

#[test]
fn test_aggregate_function_validation_avg() {
    // AVG(expr) should validate successfully
    let source = "MATCH (n:Person) RETURN AVG(n.salary)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let avg_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.contains("AVG") || d.message.contains("avg"))
            .collect();
        assert!(avg_errors.is_empty(), "No AVG-related errors expected");
    }
}

#[test]
fn test_aggregate_function_validation_max() {
    // MAX(expr) should validate successfully
    let source = "MATCH (n:Person) RETURN MAX(n.height)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let max_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.contains("MAX") || d.message.contains("max"))
            .collect();
        assert!(max_errors.is_empty(), "No MAX-related errors expected");
    }
}

#[test]
fn test_aggregate_function_validation_min() {
    // MIN(expr) should validate successfully
    let source = "MATCH (n:Person) RETURN MIN(n.age)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let min_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.contains("MIN") || d.message.contains("min"))
            .collect();
        assert!(min_errors.is_empty(), "No MIN-related errors expected");
    }
}

#[test]
fn test_aggregate_function_validation_collect() {
    // COLLECT(expr) should validate successfully
    let source = "MATCH (n:Person) RETURN COLLECT(n.name)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();
    let validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Print diagnostics for debugging
        for diag in &outcome.diagnostics {
            println!("Diagnostic: {}", diag.message);
        }
        let collect_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| {
                let msg_lower = d.message.to_lowercase();
                msg_lower.contains("collect") && d.severity == gql_parser::diag::DiagSeverity::Error
            })
            .collect();
        assert!(collect_errors.is_empty(), "No COLLECT-related errors expected, got: {:?}", collect_errors.iter().map(|d| &d.message).collect::<Vec<_>>());
    }
}

#[test]
fn test_new_functions_registered() {
    // Test that newly added functions are registered
    use gql_parser::semantic::callable::{CallableCatalog, CallableKind, CallableLookupContext};

    let catalog = BuiltinCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // Test new trigonometric functions
    assert!(!catalog.resolve("cot", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("sinh", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("cosh", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("tanh", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("degrees", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("radians", CallableKind::Function, &ctx).unwrap().is_empty());

    // Test new string functions
    assert!(!catalog.resolve("left", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("right", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("normalize", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("char_length", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("byte_length", CallableKind::Function, &ctx).unwrap().is_empty());

    // Test temporal constructors
    assert!(!catalog.resolve("date", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("time", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("datetime", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("duration", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("duration_between", CallableKind::Function, &ctx).unwrap().is_empty());

    // Test cardinality functions
    assert!(!catalog.resolve("cardinality", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("size", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("path_length", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("element_id", CallableKind::Function, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("elements", CallableKind::Function, &ctx).unwrap().is_empty());

    // Test new aggregate functions
    assert!(!catalog.resolve("stddev_samp", CallableKind::AggregateFunction, &ctx).unwrap().is_empty());
    assert!(!catalog.resolve("stddev_pop", CallableKind::AggregateFunction, &ctx).unwrap().is_empty());
}
