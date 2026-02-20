//! Tests for aggregate function validation (Milestone 4 enhancement).

use gql_parser::parse;
use gql_parser::semantic::callable::{
    resolve_builtin_signatures, CallableKind,
};
use gql_parser::semantic::SemanticValidator;

#[test]
fn test_aggregate_function_validation_count_star() {
    // COUNT(*) should validate successfully
    let source = "MATCH (n:Person) RETURN COUNT(*)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

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

    // Test new trigonometric functions
    assert!(resolve_builtin_signatures("cot", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("sinh", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("cosh", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("tanh", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("degrees", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("radians", CallableKind::Function).is_some());

    // Test new string functions
    assert!(resolve_builtin_signatures("left", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("right", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("normalize", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("char_length", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("byte_length", CallableKind::Function).is_some());

    // Test temporal constructors
    assert!(resolve_builtin_signatures("date", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("time", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("datetime", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("duration", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("duration_between", CallableKind::Function).is_some());

    // Test cardinality functions
    assert!(resolve_builtin_signatures("cardinality", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("size", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("path_length", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("element_id", CallableKind::Function).is_some());
    assert!(resolve_builtin_signatures("elements", CallableKind::Function).is_some());

    // Test new aggregate functions
    assert!(resolve_builtin_signatures("stddev_samp", CallableKind::AggregateFunction).is_some());
    assert!(resolve_builtin_signatures("stddev_pop", CallableKind::AggregateFunction).is_some());
}
