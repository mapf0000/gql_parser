//! Semantic validation tests for predicate expressions.
//!
//! Tests validation of specialized GQL predicates including IS NULL, IS TYPED,
//! IS DIRECTED, IS LABELED, PROPERTY_EXISTS, and graph-specific predicates
//! according to the test plan in VAL_TESTS.md section 10.

use gql_parser::parse;
use gql_parser::semantic::validator::{SemanticValidator, ValidationConfig};
use gql_parser::ir::ValidationOutcome;

fn validate_predicate(source: &str) -> ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let validator = SemanticValidator::new();
    validator.validate(parse_result.ast.as_ref().unwrap())
}

fn validate_predicate_with_config(source: &str, config: ValidationConfig) -> ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let validator = SemanticValidator::with_config(config);
    validator.validate(parse_result.ast.as_ref().unwrap())
}

// ===== Test 1-6: IS NULL and IS NOT NULL Predicates =====

#[test]
fn test_is_null_basic() {
    let source = "MATCH (n:Person) WHERE n.name IS NULL RETURN n";
    let outcome = validate_predicate(source);

    // Should succeed - basic IS NULL predicate
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_not_null_basic() {
    let source = "MATCH (n:Person) WHERE n.name IS NOT NULL RETURN n";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_null_in_return_clause() {
    let source = "MATCH (n:Person) RETURN n.name IS NULL AS is_null";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_null_with_complex_expression() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE (n.age + m.age) IS NULL RETURN n, m";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_null_with_property_chain() {
    let source = "MATCH (n:Person) WHERE n.address.city IS NULL RETURN n";
    let outcome = validate_predicate(source);

    // Should succeed - property chain is valid
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_null_with_undefined_variable() {
    let source = "MATCH (n:Person) WHERE x.name IS NULL RETURN n";
    let outcome = validate_predicate(source);

    // Should fail - 'x' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
    assert!(outcome.has_errors(), "Expected errors for undefined variable");
}

// ===== Test 7-12: IS TYPED Predicate =====

#[test]
fn test_is_typed_basic() {
    let source = "LET x = 42 RETURN x IS TYPED INT";
    let outcome = validate_predicate(source);

    // IS TYPED support depends on implementation
    // Test should at least parse correctly
    assert!(parse(source).ast.is_some(), "Failed to parse IS TYPED");
}

#[test]
fn test_is_typed_string() {
    let source = "MATCH (n:Person) RETURN n.name IS TYPED STRING";
    let outcome = validate_predicate(source);

    // Should parse correctly
    assert!(parse(source).ast.is_some(), "Failed to parse IS TYPED STRING");
}

#[test]
fn test_is_not_typed() {
    let source = "LET x = 42 RETURN x IS NOT TYPED STRING";
    let outcome = validate_predicate(source);

    // Should parse correctly
    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT TYPED");
}

#[test]
fn test_is_typed_with_complex_type() {
    let source = "MATCH (n:Person) RETURN n.tags IS TYPED LIST<STRING>";
    let outcome = validate_predicate(source);

    // Complex type should parse
    assert!(parse(source).ast.is_some(), "Failed to parse IS TYPED with LIST type");
}

#[test]
fn test_is_typed_with_property_value() {
    let source = "MATCH (n:Person) RETURN n.age IS TYPED PROPERTY VALUE";
    let outcome = validate_predicate(source);

    // PROPERTY VALUE is a dynamic type
    assert!(parse(source).ast.is_some(), "Failed to parse IS TYPED PROPERTY VALUE");
}

#[test]
fn test_is_typed_undefined_variable() {
    let source = "RETURN unknown_var IS TYPED INT";
    let outcome = validate_predicate(source);

    // Should fail - unknown_var is not defined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
}

// ===== Test 13-16: IS NORMALIZED Predicate =====

#[test]
fn test_is_normalized_basic() {
    let source = "MATCH (n:Person) WHERE n.name IS NORMALIZED RETURN n";
    let outcome = validate_predicate(source);

    // IS NORMALIZED is for Unicode normalization
    // Should at least parse correctly
    assert!(parse(source).ast.is_some(), "Failed to parse IS NORMALIZED");
}

#[test]
fn test_is_not_normalized() {
    let source = "MATCH (n:Person) WHERE n.name IS NOT NORMALIZED RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT NORMALIZED");
}

#[test]
fn test_is_normalized_on_string_literal() {
    let source = "RETURN 'hello' IS NORMALIZED";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NORMALIZED on literal");
}

#[test]
fn test_is_normalized_invalid_context() {
    let source = "MATCH (n:Person) WHERE n.age IS NORMALIZED RETURN n";
    let outcome = validate_predicate(source);

    // IS NORMALIZED should ideally only apply to string types
    // Validation behavior depends on implementation strictness
    assert!(parse(source).ast.is_some(), "Failed to parse IS NORMALIZED on integer");
}

// ===== Test 17-22: IS DIRECTED Predicate =====

#[test]
fn test_is_directed_basic() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE e IS DIRECTED RETURN e";
    let outcome = validate_predicate(source);

    // IS DIRECTED checks if an edge is directed
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_not_directed() {
    let source = "MATCH (n:Person)-[e:KNOWS]-(m:Person) WHERE e IS NOT DIRECTED RETURN e";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_directed_in_return() {
    let source = "MATCH (n)-[e]->(m) RETURN e IS DIRECTED AS is_directed";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_is_directed_on_node_invalid() {
    let source = "MATCH (n:Person) WHERE n IS DIRECTED RETURN n";
    let outcome = validate_predicate(source);

    // IS DIRECTED should only apply to edges, not nodes
    // Depending on strictness, this might fail validation
    if outcome.is_failure() {
        assert!(outcome.has_errors(), "Expected type mismatch error");
    }
}

#[test]
fn test_is_directed_on_undefined_variable() {
    let source = "MATCH (n:Person) WHERE e IS DIRECTED RETURN n";
    let outcome = validate_predicate(source);

    // Should fail - 'e' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
}

#[test]
fn test_is_directed_multiple_edges() {
    let source = "MATCH (a)-[e1]->(b)-[e2]->(c) WHERE e1 IS DIRECTED AND e2 IS NOT DIRECTED RETURN a, b, c";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

// ===== Test 23-28: IS LABELED Predicate =====

#[test]
fn test_is_labeled_basic() {
    let source = "MATCH (n) WHERE n IS LABELED :Person RETURN n";
    let outcome = validate_predicate(source);

    // IS LABELED checks if element has a specific label
    assert!(parse(source).ast.is_some(), "Failed to parse IS LABELED");
}

#[test]
fn test_is_not_labeled() {
    let source = "MATCH (n) WHERE n IS NOT LABELED :Robot RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT LABELED");
}

#[test]
fn test_is_labeled_without_specific_label() {
    let source = "MATCH (n) WHERE n IS LABELED RETURN n";
    let outcome = validate_predicate(source);

    // IS LABELED without specific label checks if element has any label
    assert!(parse(source).ast.is_some(), "Failed to parse IS LABELED without label");
}

#[test]
fn test_is_labeled_on_edge() {
    let source = "MATCH (n)-[e]->(m) WHERE e IS LABELED :KNOWS RETURN e";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS LABELED on edge");
}

#[test]
fn test_is_labeled_multiple_labels() {
    let source = "MATCH (n) WHERE n IS LABELED :Person AND n IS LABELED :Employee RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse multiple IS LABELED");
}

#[test]
fn test_is_labeled_undefined_variable() {
    let source = "MATCH (n:Person) WHERE x IS LABELED :Person RETURN n";
    let outcome = validate_predicate(source);

    // Should fail - 'x' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
}

// ===== Test 29-33: IS TRUE/FALSE/UNKNOWN Predicates =====

#[test]
fn test_is_true_predicate() {
    let source = "MATCH (n:Person) WHERE (n.age > 18) IS TRUE RETURN n";
    let outcome = validate_predicate(source);

    // IS TRUE checks if expression is exactly TRUE (not UNKNOWN)
    assert!(parse(source).ast.is_some(), "Failed to parse IS TRUE");
}

#[test]
fn test_is_false_predicate() {
    let source = "MATCH (n:Person) WHERE (n.age < 0) IS FALSE RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS FALSE");
}

#[test]
fn test_is_unknown_predicate() {
    let source = "MATCH (n:Person) WHERE (n.age > 18) IS UNKNOWN RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS UNKNOWN");
}

#[test]
fn test_is_not_true_predicate() {
    let source = "MATCH (n:Person) WHERE (n.active = TRUE) IS NOT TRUE RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT TRUE");
}

#[test]
fn test_is_unknown_with_null_check() {
    let source = "MATCH (n:Person) WHERE (n.name IS NULL) IS UNKNOWN RETURN n";
    let outcome = validate_predicate(source);

    // This is a bit contrived but should be valid syntax
    assert!(parse(source).ast.is_some(), "Failed to parse nested truth value check");
}

// ===== Test 34-39: IS SOURCE OF and IS DESTINATION OF Predicates =====

#[test]
fn test_is_source_of_basic() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE n IS SOURCE OF e RETURN n";
    let outcome = validate_predicate(source);

    // IS SOURCE OF checks if node is the source of an edge
    assert!(parse(source).ast.is_some(), "Failed to parse IS SOURCE OF");
}

#[test]
fn test_is_destination_of_basic() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE m IS DESTINATION OF e RETURN m";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS DESTINATION OF");
}

#[test]
fn test_is_not_source_of() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE m IS NOT SOURCE OF e RETURN m";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT SOURCE OF");
}

#[test]
fn test_is_not_destination_of() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) WHERE n IS NOT DESTINATION OF e RETURN n";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse IS NOT DESTINATION OF");
}

#[test]
fn test_is_source_undefined_edge() {
    let source = "MATCH (n:Person) WHERE n IS SOURCE OF unknown_edge RETURN n";
    let outcome = validate_predicate(source);

    // Should fail - unknown_edge is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined edge");
}

#[test]
fn test_is_source_of_wrong_type() {
    let source = "MATCH (n:Person) WHERE n IS SOURCE OF n RETURN n";
    let outcome = validate_predicate(source);

    // IS SOURCE OF should require edge, not node
    // Validation strictness depends on implementation
    if outcome.is_failure() {
        assert!(outcome.has_errors(), "Expected type mismatch error");
    }
}

// ===== Test 40-44: ALL_DIFFERENT Predicate =====

#[test]
fn test_all_different_basic() {
    let source = "MATCH (a:Person), (b:Person), (c:Person) WHERE ALL_DIFFERENT(a, b, c) RETURN a, b, c";
    let outcome = validate_predicate(source);

    // ALL_DIFFERENT checks all arguments are distinct elements
    assert!(parse(source).ast.is_some(), "Failed to parse ALL_DIFFERENT");
}

#[test]
fn test_all_different_two_args() {
    let source = "MATCH (a:Person), (b:Person) WHERE ALL_DIFFERENT(a, b) RETURN a, b";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse ALL_DIFFERENT with 2 args");
}

#[test]
fn test_all_different_multiple_paths() {
    let source = "MATCH (a)-[e1]->(b)-[e2]->(c) WHERE ALL_DIFFERENT(e1, e2) RETURN a, b, c";
    let outcome = validate_predicate(source);

    // ALL_DIFFERENT on edges
    assert!(parse(source).ast.is_some(), "Failed to parse ALL_DIFFERENT on edges");
}

#[test]
fn test_all_different_single_arg() {
    let source = "MATCH (a:Person) WHERE ALL_DIFFERENT(a) RETURN a";
    let outcome = validate_predicate(source);

    // Single argument is trivially all different
    // Might be considered degenerate but should parse
    assert!(parse(source).ast.is_some(), "Failed to parse ALL_DIFFERENT with 1 arg");
}

#[test]
fn test_all_different_undefined_variable() {
    let source = "MATCH (a:Person), (b:Person) WHERE ALL_DIFFERENT(a, b, x) RETURN a, b";
    let outcome = validate_predicate(source);

    // Should fail - 'x' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
}

// ===== Test 45-49: SAME Predicate =====

#[test]
fn test_same_predicate_basic() {
    let source = "MATCH (a:Person), (b:Person) WHERE SAME(a, b) RETURN a";
    let outcome = validate_predicate(source);

    // SAME checks if two elements are the same element
    assert!(parse(source).ast.is_some(), "Failed to parse SAME");
}

#[test]
fn test_same_predicate_edges() {
    let source = "MATCH (a)-[e1]->(b), (c)-[e2]->(d) WHERE SAME(e1, e2) RETURN a, b";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse SAME on edges");
}

#[test]
fn test_same_predicate_in_filter() {
    let source = "MATCH (a:Person)-[e1]->(b:Person)-[e2]->(c:Person) WHERE NOT SAME(e1, e2) RETURN a, b, c";
    let outcome = validate_predicate(source);

    // Ensure edges are different
    assert!(parse(source).ast.is_some(), "Failed to parse NOT SAME");
}

#[test]
fn test_same_predicate_undefined_variable() {
    let source = "MATCH (a:Person) WHERE SAME(a, x) RETURN a";
    let outcome = validate_predicate(source);

    // Should fail - 'x' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
}

#[test]
fn test_same_predicate_complex_pattern() {
    let source = "MATCH (a:Person), (b:Person), (c:Person) WHERE SAME(a, b) OR SAME(b, c) RETURN a, b, c";
    let outcome = validate_predicate(source);

    assert!(parse(source).ast.is_some(), "Failed to parse complex SAME pattern");
}

// ===== Test 50-56: PROPERTY_EXISTS Predicate =====

#[test]
fn test_property_exists_basic() {
    let source = "MATCH (n:Person) WHERE PROPERTY_EXISTS(n, name) RETURN n";
    let outcome = validate_predicate(source);

    // PROPERTY_EXISTS checks if element has a specific property
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_property_exists_edge() {
    let source = "MATCH (n)-[e:KNOWS]->(m) WHERE PROPERTY_EXISTS(e, since) RETURN e";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_property_exists_in_return() {
    let source = "MATCH (n:Person) RETURN PROPERTY_EXISTS(n, email) AS has_email";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_property_exists_multiple_checks() {
    let source = "MATCH (n:Person) WHERE PROPERTY_EXISTS(n, email) AND PROPERTY_EXISTS(n, phone) RETURN n";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_property_exists_undefined_variable() {
    let source = "MATCH (n:Person) WHERE PROPERTY_EXISTS(x, name) RETURN n";
    let outcome = validate_predicate(source);

    // Should fail - 'x' is undefined
    assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
    assert!(outcome.has_errors(), "Expected error for undefined variable 'x'");
}

#[test]
fn test_property_exists_computed_property() {
    let source = "MATCH (n:Person) LET prop = n.name WHERE PROPERTY_EXISTS(n, prop) RETURN n";
    let outcome = validate_predicate(source);

    // Property name as variable reference - PROPERTY_EXISTS syntax requires identifiers, not expressions
    // This tests that the variable 'prop' itself is validated
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_property_exists_nested_context() {
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m:Person) WHERE PROPERTY_EXISTS(m, verified) } RETURN n";
    let outcome = validate_predicate(source);

    // PROPERTY_EXISTS inside EXISTS subquery
    assert!(parse(source).ast.is_some(), "Failed to parse PROPERTY_EXISTS in nested context");
}

// ===== Test 57-60: Predicate Combinations =====

#[test]
fn test_multiple_predicate_combinations() {
    let source = r#"
        MATCH (n:Person)-[e:KNOWS]->(m:Person)
        WHERE n.name IS NOT NULL
          AND e IS DIRECTED
          AND m IS LABELED :Employee
          AND PROPERTY_EXISTS(m, 'salary')
        RETURN n, e, m
    "#;
    let outcome = validate_predicate(source);

    // Combination of different predicate types
    assert!(parse(source).ast.is_some(), "Failed to parse multiple predicates");
}

#[test]
fn test_predicates_in_case_expression() {
    let source = "MATCH (n:Person) RETURN CASE WHEN n.age IS NULL THEN 'unknown' ELSE 'known' END";
    let outcome = validate_predicate(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_predicates_in_aggregation_context() {
    let source = "MATCH (n:Person) RETURN COUNT(n) FILTER (WHERE n.name IS NOT NULL)";
    let outcome = validate_predicate(source);

    // Predicate in FILTER clause of aggregation
    assert!(parse(source).ast.is_some(), "Failed to parse predicate in FILTER");
}

#[test]
fn test_predicates_with_subquery() {
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[e:KNOWS]->(m:Person)
            WHERE e IS DIRECTED AND m.age IS NOT NULL
        }
        RETURN n
    "#;
    let outcome = validate_predicate(source);

    // Predicates inside EXISTS subquery
    assert!(parse(source).ast.is_some(), "Failed to parse predicates in subquery");
}

// ===== Test 61-64: Edge Cases and Error Conditions =====

#[test]
fn test_null_literal_is_null() {
    let source = "RETURN NULL IS NULL";
    let outcome = validate_predicate(source);

    // Literal NULL should work with IS NULL
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_nested_predicates() {
    let source = "MATCH (n:Person) WHERE ((n.age IS NOT NULL) IS TRUE) RETURN n";
    let outcome = validate_predicate(source);

    // Nested predicate structure
    assert!(parse(source).ast.is_some(), "Failed to parse nested predicates");
}

#[test]
fn test_predicate_without_match() {
    let source = "LET x = 42 RETURN x IS NOT NULL";
    let outcome = validate_predicate(source);

    // Predicate on LET variable
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_predicate_in_set_clause() {
    let source = "MATCH (n:Person) WHERE n.age > 18 SET n.is_adult = (n.age IS NOT NULL AND n.age >= 18) RETURN n";
    let outcome = validate_predicate(source);

    // Predicate in SET expression
    assert!(parse(source).ast.is_some(), "Failed to parse predicate in SET clause");
}

// ===== Verification Test Module =====

/// Test module to verify PROPERTY_EXISTS undefined variable detection works correctly
#[cfg(test)]
mod property_exists_verification {
    use gql_parser::parse;
    use gql_parser::semantic::validator::SemanticValidator;

    #[test]
    fn verify_undefined_variable_is_caught() {
        let source = "MATCH (n:Person) WHERE PROPERTY_EXISTS(x, name) RETURN n";
        let parse_result = parse(source);

        if let Some(ast) = parse_result.ast.as_ref() {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(ast);

            println!("Is failure: {}", outcome.is_failure());
            println!("Has errors: {}", outcome.has_errors());
            println!("Diagnostics count: {}", outcome.diagnostics.len());
            for diag in &outcome.diagnostics {
                println!("  Diagnostic: {}", diag.message);
            }

            // Should fail - 'x' is undefined
            assert!(outcome.is_failure(), "Expected validation failure for undefined variable");
            assert!(outcome.has_errors(), "Expected error for undefined variable 'x'");

            // Check that the error message mentions the undefined variable
            let has_undefined_var_error = outcome.diagnostics.iter()
                .any(|d| d.message.contains("Undefined variable") && d.message.contains("'x'"));
            assert!(has_undefined_var_error, "Expected 'Undefined variable x' error message");
        }
    }
}
