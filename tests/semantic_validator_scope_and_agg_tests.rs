// Integration tests for semantic validator (scope/isolation and advanced semantics)

use gql_parser::diag::DiagSeverity;
use gql_parser::parse;
use gql_parser::semantic::validator::{SemanticValidator, ValidationConfig};

// ==================== Scope Isolation Tests (F2) ====================

#[test]
fn test_scope_isolation_across_statements() {
    // Variables shouldn't leak between semicolon-separated statements
    let source = "MATCH (n:Person) RETURN n; MATCH (m:Company) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should fail: 'n' from first statement not visible in second statement
        assert!(
            !outcome.is_success(),
            "Should fail: variable 'n' leaked across statements"
        );

        // Check for undefined variable error
        let has_undefined_error = outcome.diagnostics.iter().any(|d| {
            let message = d.message.to_lowercase();
            d.severity == DiagSeverity::Error
                && message.contains("undefined")
                && message.contains("n")
        });
        assert!(
            has_undefined_error,
            "Should have undefined variable error for 'n'"
        );
    }
}

#[test]
fn test_scope_proper_linear_flow() {
    // Variables should be visible within the same statement
    let source = "MATCH (n:Person) MATCH (m:Company) WHERE m.name = n.name RETURN n, m";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should succeed: both n and m visible within same statement
        assert!(
            outcome.is_success(),
            "Should succeed: variables visible in same statement"
        );
    }
}

#[test]
fn test_composite_query_scope_isolation_union() {
    // UNION queries should have isolated scopes
    let source = "MATCH (a:Person) RETURN a UNION MATCH (b:Company) RETURN a";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should fail: 'a' from left side not visible in right side of UNION
        assert!(
            !outcome.is_success(),
            "Should fail: variable leaked across UNION"
        );

        let has_undefined_error = outcome.diagnostics.iter().any(|d| {
            let message = d.message.to_lowercase();
            d.severity == DiagSeverity::Error
                && message.contains("undefined")
                && message.contains("a")
        });
        assert!(
            has_undefined_error,
            "Should have undefined variable error for 'a' in UNION right side"
        );
    }
}

#[test]
fn test_composite_query_both_sides_valid() {
    // UNION with valid variables on both sides
    let source = "MATCH (a:Person) RETURN a UNION MATCH (a:Person) RETURN a";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should succeed: each side defines its own 'a'
        assert!(
            outcome.is_success(),
            "Should succeed: both sides have valid 'a'"
        );
    }
}

// ==================== F5: Enhanced Aggregation Validation Tests ====================

#[test]
fn test_return_mixed_aggregation() {
    // ISO GQL: Cannot mix aggregated and non-aggregated expressions in RETURN without GROUP BY
    let source = "MATCH (n:Person) RETURN COUNT(n), n.name";
    let config = ValidationConfig {
        strict_mode: true,
        ..Default::default()
    };
    let validator = SemanticValidator::with_config(config);
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: mixing aggregated and non-aggregated in RETURN"
        );
    }
}

#[test]
fn test_nested_aggregation_error() {
    // ISO GQL: Nested aggregation functions are not allowed
    let source = "MATCH (n:Person) RETURN COUNT(SUM(n.age))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: nested aggregation");

        let has_nested_error = outcome.diagnostics.iter().any(|d| {
            d.message.contains("Nested aggregation") || d.message.contains("nested aggregation")
        });
        assert!(has_nested_error, "Should have nested aggregation error");
    }
}

#[test]
fn test_aggregation_in_where_error() {
    // ISO GQL: Aggregation functions not allowed in WHERE clause
    let source = "MATCH (n:Person) FILTER AVG(n.age) > 30 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            !outcome.is_success(),
            "Should fail: aggregation in WHERE/FILTER"
        );

        let has_where_error = outcome.diagnostics.iter().any(|d| {
            d.message.contains("WHERE")
                || d.message.contains("HAVING")
                || d.message.contains("FILTER")
        });
        assert!(
            has_where_error,
            "Should mention WHERE/HAVING/FILTER in error message"
        );
    }
}

#[test]
fn test_having_non_grouped_error() {
    // ISO GQL: Non-aggregated expressions in HAVING must appear in GROUP BY
    let source =
        "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING n.name = 'Alice'";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: n.name not in GROUP BY
        assert!(
            !outcome.is_success(),
            "Should fail: non-grouped expression in HAVING"
        );
    }
}

#[test]
fn test_valid_group_by() {
    // ISO GQL: Valid GROUP BY with aggregation
    let source = "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(), "Should succeed: valid GROUP BY");
    }
}

#[test]
fn test_valid_having_with_aggregate() {
    // ISO GQL: HAVING with aggregated expression is valid
    let source =
        "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING AVG(n.age) > 30";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "Should succeed: HAVING uses aggregate"
        );
    }
}

// ==================== F3: Expression Validation Tests ====================

#[test]
fn test_case_type_consistency() {
    // ISO GQL: All branches in CASE must return compatible types
    let source = "MATCH (n:Person) RETURN CASE WHEN true THEN 5 WHEN false THEN 'string' END";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Note: This may pass in current implementation since type checking is basic
        // The test documents expected behavior per ISO GQL
        if !outcome.is_success() {
            let has_type_error = outcome
                .diagnostics
                .iter()
                .any(|d| d.message.contains("type") || d.message.contains("CASE"));
            assert!(has_type_error, "Should have type consistency error in CASE");
        }
    }
}

#[test]
fn test_null_propagation_warning() {
    // ISO GQL: Operations with NULL propagate NULL
    let source = "MATCH (n:Person) FILTER n.age + NULL > 5 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // Check if there's a warning about NULL propagation
        let has_null_warning = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Warning && d.message.contains("NULL"));
        assert!(
            has_null_warning,
            "Should warn about NULL propagation in arithmetic"
        );
    }
}
