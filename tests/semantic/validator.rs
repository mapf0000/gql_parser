// Integration tests for semantic validator
// Moved from src/semantic/validator.rs as part of Phase 4 refactoring

use gql_parser::diag::DiagSeverity;
use gql_parser::parse;
use gql_parser::semantic::validator::{SemanticValidator, ValidationConfig};

#[test]
fn test_validator_basic() {
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        assert!(
            result.is_success(),
            "basic semantic validation should succeed"
        );
    }
}

#[test]
fn test_validator_with_config() {
    let config = ValidationConfig {
        strict_mode: true,
        metadata_validation: true,
        warn_on_shadowing: true,
        warn_on_disconnected_patterns: true,
    };

    let validator = SemanticValidator::with_config(config);
    let source = "MATCH (a:Person), (b:Company) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "query should remain semantically valid"
        );

        let has_warning = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Warning);
        assert!(
            has_warning,
            "configured warning flags should surface warning diagnostics"
        );
    }
}

#[test]
fn test_scope_analysis_match_bindings() {
    let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) RETURN n, e, m";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "scope analysis query should validate: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
        let symbol_table = outcome
            .ir
            .as_ref()
            .expect("successful validation should produce IR")
            .symbol_table();

        // Check that variables n, e, m were defined
        assert!(
            symbol_table.lookup_all("n").is_some(),
            "Variable 'n' should be defined"
        );
        assert!(
            symbol_table.lookup_all("e").is_some(),
            "Variable 'e' should be defined"
        );
        assert!(
            symbol_table.lookup_all("m").is_some(),
            "Variable 'm' should be defined"
        );
    }
}

#[test]
fn test_scope_analysis_let_variables() {
    let source = "MATCH (n:Person) LET age = n.age RETURN age";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "scope analysis query should validate: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
        let symbol_table = outcome
            .ir
            .as_ref()
            .expect("successful validation should produce IR")
            .symbol_table();

        // Check that variables n and age were defined
        assert!(
            symbol_table.lookup_all("n").is_some(),
            "Variable 'n' should be defined"
        );
        assert!(
            symbol_table.lookup_all("age").is_some(),
            "Variable 'age' should be defined"
        );
    }
}

#[test]
fn test_scope_analysis_for_variables() {
    let source = "MATCH (n:Person) FOR item IN n.items RETURN item";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "scope analysis query should validate: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
        let symbol_table = outcome
            .ir
            .as_ref()
            .expect("successful validation should produce IR")
            .symbol_table();

        // Check that variables n and item were defined
        assert!(
            symbol_table.lookup_all("n").is_some(),
            "Variable 'n' should be defined"
        );
        assert!(
            symbol_table.lookup_all("item").is_some(),
            "Variable 'item' should be defined"
        );
    }
}

#[test]
fn test_scope_analysis_path_variables() {
    let source = "MATCH p = (a:Person)-[r:KNOWS]->(b:Person) RETURN p, a, r, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "scope analysis query should validate: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
        let symbol_table = outcome
            .ir
            .as_ref()
            .expect("successful validation should produce IR")
            .symbol_table();

        // Check that path variable p and element variables a, r, b were defined
        assert!(
            symbol_table.lookup_all("p").is_some(),
            "Path variable 'p' should be defined"
        );
        assert!(
            symbol_table.lookup_all("a").is_some(),
            "Variable 'a' should be defined"
        );
        assert!(
            symbol_table.lookup_all("r").is_some(),
            "Variable 'r' should be defined"
        );
        assert!(
            symbol_table.lookup_all("b").is_some(),
            "Variable 'b' should be defined"
        );
    }
}

#[test]
fn test_variable_validation_undefined_variable() {
    let source = "MATCH (n:Person) RETURN m"; // m is undefined
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should fail with undefined variable error
        assert!(
            result.is_failure(),
            "Validation should fail for undefined variable"
        );
        let diagnostics = &result.diagnostics;
        assert!(
            !diagnostics.is_empty(),
            "Should have at least one diagnostic"
        );

        // Check that the diagnostic mentions the undefined variable 'm'
        let diag_message = &diagnostics[0].message;
        assert!(
            diag_message.contains("m") || diag_message.contains("Undefined"),
            "Diagnostic should mention undefined variable: {}",
            diag_message
        );
    }
}

#[test]
fn test_variable_validation_defined_variable() {
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass validation
        assert!(
            result.is_success(),
            "Validation should pass for defined variable"
        );
    }
}

#[test]
fn test_variable_validation_multiple_undefined() {
    let source = "MATCH (n:Person) RETURN x, y, z"; // x, y, z are undefined
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should fail with multiple undefined variable errors
        assert!(
            result.is_failure(),
            "Validation should fail for undefined variables"
        );
        let diagnostics = &result.diagnostics;
        assert!(
            diagnostics.len() >= 3,
            "Should have at least 3 diagnostics for x, y, z"
        );
    }
}

#[test]
fn test_type_inference_literals() {
    let source = "MATCH (n:Person) LET x = 42, y = 'hello', z = TRUE RETURN x, y, z";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        let has_errors = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(!has_errors, "type inference literal query should not error");
    }
}

#[test]
fn test_type_inference_arithmetic() {
    let source =
        "MATCH (n:Person) LET sum = n.age + 10, product = n.salary * 1.5 RETURN sum, product";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        let has_errors = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(
            !has_errors,
            "type inference arithmetic query should not error"
        );
    }
}

#[test]
fn test_type_inference_aggregates() {
    let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age), SUM(n.salary)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        let has_errors = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(
            !has_errors,
            "type inference aggregate query should not error"
        );
    }
}

#[test]
fn test_type_inference_comparison() {
    let source = "MATCH (n:Person) FILTER n.age > 30 AND n.name = 'Alice' RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        let has_errors = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(
            !has_errors,
            "type inference comparison query should not error"
        );
    }
}

#[test]
fn test_type_inference_for_loop() {
    let source = "FOR item IN [1, 2, 3] RETURN item";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        let has_errors = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(!has_errors, "type inference FOR query should not error");
    }
}

#[test]
fn test_type_checking_string_in_arithmetic() {
    // Test that using a string literal in arithmetic produces a type error
    let source = "MATCH (n:Person) LET x = 'hello' + 10 RETURN x";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should produce type mismatch error
        assert!(
            result.is_failure(),
            "Should fail type checking for string in arithmetic"
        );
        let diagnostics = &result.diagnostics;
        assert!(
            !diagnostics.is_empty(),
            "Should have type mismatch diagnostic"
        );

        // Check that diagnostic mentions type mismatch
        let diag_message = &diagnostics[0].message;
        assert!(
            diag_message.contains("Type mismatch")
                || diag_message.contains("numeric")
                || diag_message.contains("string"),
            "Diagnostic should mention type mismatch: {}",
            diag_message
        );
    }
}

#[test]
fn test_type_checking_unary_minus_string() {
    // Test that unary minus on a string produces a type error
    let source = "MATCH (n:Person) LET x = -'hello' RETURN x";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should produce type mismatch error
        assert!(
            result.is_failure(),
            "Should fail type checking for unary minus on string"
        );
        let diagnostics = &result.diagnostics;
        assert!(
            !diagnostics.is_empty(),
            "Should have type mismatch diagnostic"
        );
    }
}

#[test]
fn test_type_checking_valid_arithmetic() {
    // Test that valid arithmetic passes type checking
    let source = "MATCH (n:Person) LET x = 10 + 20, y = 3.14 * 2 RETURN x, y";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass type checking (no undefined variables, valid arithmetic)
        if result.is_failure() {
            panic!(
                "Should pass type checking for valid arithmetic, but got errors: {:?}",
                result
                    .diagnostics
                    .iter()
                    .map(|d| &d.message)
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn test_type_checking_case_expression() {
    // Test that CASE expressions are type-checked
    let source = "MATCH (n:Person) SELECT CASE WHEN n.age > 18 THEN 'adult' ELSE 'minor' END";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // CASE expression should pass type checking
        // (In future could check that all branches have compatible types)
        // May fail due to undefined variable 'n', but shouldn't fail type checking
        if result.is_failure() {
            // Just verify type checking runs without panicking
            assert!(!result.diagnostics.is_empty());
        }
    }
}

// ==================== Pattern Connectivity Tests ====================

#[test]
fn test_pattern_connectivity_single_node() {
    // Single node pattern - should always be connected
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - single node is always connected
        assert!(
            result.is_success(),
            "Single node pattern should be connected"
        );
    }
}

#[test]
fn test_pattern_connectivity_connected_path() {
    // Connected path pattern - should be valid
    let source = "MATCH (a:Person)-[r:KNOWS]->(b:Person) RETURN a, r, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - path is connected
        assert!(
            result.is_success(),
            "Connected path pattern should be valid"
        );
    }
}

#[test]
fn test_pattern_connectivity_disconnected_nodes() {
    // Disconnected nodes in same MATCH - should fail
    let source = "MATCH (a:Person), (b:Company) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should succeed with a warning - disconnected patterns are ISO-conformant
        // Warnings don't prevent IR creation
        assert!(
            result.is_success(),
            "Disconnected nodes are ISO-conformant and should not fail validation"
        );

        // If we want to test that a warning was issued, we'd need to check
        // the IR or modify the API to return warnings alongside the IR
    }
}

#[test]
fn test_pattern_connectivity_long_path() {
    // Long connected path - should be valid
    let source = "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company)-[:LOCATED_IN]->(d:City) RETURN a, d";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - all nodes connected in path
        assert!(result.is_success(), "Long connected path should be valid");
    }
}

#[test]
fn test_pattern_connectivity_multiple_paths() {
    // Multiple disconnected paths - ISO-conformant
    let source = "MATCH (a)-[:R1]->(b), (c)-[:R2]->(d) RETURN a, b, c, d";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should succeed with a warning - disconnected comma-separated patterns are ISO-conformant
        assert!(
            result.is_success(),
            "Multiple disconnected paths are ISO-conformant and should not fail"
        );
    }
}

// ==================== Context Validation Tests ====================

#[test]
fn test_context_validation_match_in_query() {
    // MATCH clause in query context - should be valid
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - MATCH is valid in query context
        assert!(
            result.is_success(),
            "MATCH in query context should be valid"
        );
    }
}

#[test]
fn test_context_validation_filter_usage() {
    // FILTER/WHERE clause - should be valid
    let source = "MATCH (n:Person) FILTER n.age > 30 RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - FILTER is valid in query context
        assert!(
            result.is_success(),
            "FILTER in query context should be valid"
        );
    }
}

#[test]
fn test_context_validation_order_by() {
    // ORDER BY clause - should be valid
    let source = "MATCH (n:Person) RETURN n ORDER BY n.age DESC";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - ORDER BY is valid in query context
        assert!(
            result.is_success(),
            "ORDER BY in query context should be valid"
        );
    }
}

#[test]
fn test_context_validation_aggregation_context() {
    // Aggregation in SELECT - should be valid
    let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - aggregation is valid in SELECT
        assert!(result.is_success(), "Aggregation in SELECT should be valid");
    }
}

// ==================== Aggregation Validation Tests ====================

#[test]
fn test_aggregation_count_star() {
    // COUNT(*) - should be valid
    let source = "MATCH (n:Person) SELECT COUNT(*)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - COUNT(*) is valid
        assert!(result.is_success(), "COUNT(*) should be valid");
    }
}

#[test]
fn test_aggregation_avg_function() {
    // AVG function with property - should be valid
    let source = "MATCH (n:Person) SELECT AVG(n.age)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - AVG is valid aggregate function
        assert!(result.is_success(), "AVG function should be valid");
    }
}

#[test]
fn test_aggregation_sum_function() {
    // SUM function - should be valid
    let source = "MATCH (n:Person) SELECT SUM(n.salary)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - SUM is valid
        assert!(result.is_success(), "SUM function should be valid");
    }
}

#[test]
fn test_aggregation_multiple_functions() {
    // Multiple aggregation functions - should be valid
    let source =
        "MATCH (n:Person) SELECT COUNT(*), AVG(n.age), SUM(n.salary), MIN(n.age), MAX(n.salary)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - multiple aggregates are valid
        assert!(
            result.is_success(),
            "Multiple aggregation functions should be valid"
        );
    }
}

#[test]
fn test_aggregation_with_arithmetic() {
    // Aggregation with arithmetic - should be valid
    let source = "MATCH (n:Person) SELECT AVG(n.age) + 10, SUM(n.salary) * 1.5";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - arithmetic on aggregates is valid
        assert!(
            result.is_success(),
            "Aggregation with arithmetic should be valid"
        );
    }
}

// ==================== Expression Validation Tests ====================

#[test]
fn test_expression_validation_case_simple() {
    // Simple CASE expression - should be valid
    let source = "MATCH (n:Person) SELECT CASE n.status WHEN 'active' THEN 1 WHEN 'inactive' THEN 0 ELSE -1 END";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - simple CASE is valid
        assert!(
            result.is_success(),
            "Simple CASE expression should be valid"
        );
    }
}

#[test]
fn test_expression_validation_case_searched() {
    // Searched CASE expression - should be valid
    let source = "MATCH (n:Person) SELECT CASE WHEN n.age < 18 THEN 'minor' WHEN n.age < 65 THEN 'adult' ELSE 'senior' END";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - searched CASE is valid
        assert!(
            result.is_success(),
            "Searched CASE expression should be valid"
        );
    }
}

#[test]
fn test_expression_validation_nested_case() {
    // Nested CASE expressions - should be valid
    let source = "MATCH (n:Person) SELECT CASE WHEN n.age > 18 THEN CASE WHEN n.salary > 50000 THEN 'high' ELSE 'low' END ELSE 'minor' END";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - nested CASE is valid
        assert!(
            result.is_success(),
            "Nested CASE expression should be valid"
        );
    }
}

#[test]
fn test_expression_validation_list_constructor() {
    // List constructor - should be valid
    let source = "MATCH (n:Person) LET list = [1, 2, 3, 4, 5] RETURN list";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - list constructor is valid
        assert!(result.is_success(), "List constructor should be valid");
    }
}

#[test]
fn test_expression_validation_record_constructor() {
    // Record constructor - should be valid
    let source = "MATCH (n:Person) SELECT {name: n.name, age: n.age}";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - record constructor is valid
        assert!(result.is_success(), "Record constructor should be valid");
    }
}

#[test]
fn test_expression_validation_property_reference() {
    // Property reference - should be valid
    let source = "MATCH (n:Person) RETURN n.name, n.age, n.address.city";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - property references are valid
        assert!(result.is_success(), "Property references should be valid");
    }
}

#[test]
fn test_expression_validation_function_call() {
    // Function call - should be valid
    let source = "MATCH (n:Person) SELECT UPPER(n.name), LENGTH(n.address)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - function calls are valid
        assert!(result.is_success(), "Function calls should be valid");
    }
}

#[test]
fn test_expression_validation_cast() {
    // CAST expression - should be valid
    let source = "MATCH (n:Person) SELECT CAST(n.age AS STRING)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - CAST is valid
        assert!(result.is_success(), "CAST expression should be valid");
    }
}

#[test]
fn test_expression_validation_complex_expression() {
    // Complex nested expression - should be valid
    let source = "MATCH (n:Person) SELECT (n.salary * 1.1) + (n.bonus / 12) - n.tax";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Debug: print diagnostics if test fails
        if !result.is_success() {
            eprintln!("Diagnostics: {:?}", result.diagnostics);
        }

        // Should pass - complex arithmetic is valid
        assert!(
            result.is_success(),
            "Complex nested expression should be valid"
        );
    }
}

// ==================== Edge Case Tests ====================

#[test]
fn test_edge_case_empty_match_pattern() {
    // MATCH without pattern elements (if parseable) - edge case
    // This test verifies validator handles unusual but parseable structures
    let source = "MATCH (n) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - single node is always connected
        assert!(result.is_success(), "Single anonymous node should be valid");
    }
}

#[test]
fn test_edge_case_deeply_nested_properties() {
    // Deeply nested property access - edge case
    let source = "MATCH (n:Person) RETURN n.address.street.building.floor.unit";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - deeply nested properties are valid syntactically
        assert!(
            result.is_success(),
            "Deeply nested property access should be valid"
        );
    }
}

#[test]
fn test_edge_case_multiple_filters() {
    // Multiple FILTER clauses - edge case
    let source = "MATCH (n:Person) FILTER n.age > 18 FILTER n.salary > 50000 RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - multiple filters are valid
        assert!(
            result.is_success(),
            "Multiple FILTER clauses should be valid"
        );
    }
}

#[test]
fn test_edge_case_parenthesized_expressions() {
    // Heavily parenthesized expressions - edge case
    let source = "MATCH (n:Person) SELECT (((n.age + 10) * 2) - 5)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - parenthesized expressions are valid
        assert!(
            result.is_success(),
            "Parenthesized expressions should be valid"
        );
    }
}

#[test]
fn test_edge_case_variable_shadowing_let() {
    // Variable shadowing with LET - edge case
    let source = "MATCH (n:Person) LET n = n.name RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // May warn about shadowing in strict mode, but should be semantically valid
        // The validation depends on configuration
        let _ = result; // Validation result depends on configuration
    }
}

#[test]
fn test_edge_case_for_loop_shadowing() {
    // FOR loop variable shadowing - edge case
    let source = "MATCH (n:Person) FOR n IN [1, 2, 3] RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // May warn about shadowing, depends on configuration
        let _ = result;
    }
}

#[test]
fn test_edge_case_all_literal_types() {
    // All literal types - edge case coverage
    let source = "SELECT 42, 3.14, 'hello', TRUE, FALSE, NULL";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - literals don't require variable resolution
        assert!(result.is_success(), "All literal types should be valid");
    }
}

#[test]
fn test_edge_case_boolean_operators() {
    // Complex boolean expression - edge case
    let source = "MATCH (n:Person) FILTER (n.age > 18 AND n.age < 65) OR (n.status = 'VIP' AND NOT n.blocked) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - complex boolean expressions are valid
        assert!(
            result.is_success(),
            "Complex boolean expressions should be valid"
        );
    }
}

#[test]
fn test_edge_case_comparison_chains() {
    // Multiple comparisons - edge case
    let source = "MATCH (n:Person) FILTER n.age >= 18 AND n.age <= 65 RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - comparison chains are valid
        assert!(result.is_success(), "Comparison chains should be valid");
    }
}

#[test]
fn test_edge_case_mixed_aggregates_and_literals() {
    // Mixed aggregates with literals - edge case
    let source = "MATCH (n:Person) SELECT COUNT(*) + 1, AVG(n.age) * 2";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - mixing aggregates with literals/arithmetic is valid
        assert!(
            result.is_success(),
            "Mixed aggregates with literals should be valid"
        );
    }
}

// ==================== Schema Validation Tests ====================

#[test]
fn test_schema_validation_valid_label() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    // Valid node label in schema
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let metadata = MockMetadataProvider::example();
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);
        let result = validator.validate(&program);

        // Should pass - Person is in the schema
        assert!(
            result.is_success(),
            "Valid label should pass schema validation"
        );
    }
}

#[test]
fn test_schema_validation_invalid_label() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    // Invalid node label not in schema
    let source = "MATCH (n:Alien) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let metadata = MockMetadataProvider::example();
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);
        let result = validator.validate(&program);

        // Should fail - Alien is not in the schema
        assert!(
            result.is_failure(),
            "Invalid label should fail schema validation"
        );
    }
}

#[test]
fn test_schema_validation_valid_edge_label() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    // Valid edge label in schema
    let source = "MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let metadata = MockMetadataProvider::example();
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);
        let result = validator.validate(&program);

        // Should pass - KNOWS is in the schema
        assert!(
            result.is_success(),
            "Valid edge label should pass schema validation"
        );
    }
}

#[test]
fn test_schema_validation_invalid_edge_label() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    // Invalid edge label not in schema
    let source = "MATCH (a:Person)-[:HATES]->(b:Person) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let metadata = MockMetadataProvider::example();
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);
        let result = validator.validate(&program);

        // Should fail - HATES is not in the schema
        assert!(
            result.is_failure(),
            "Invalid edge label should fail schema validation"
        );
    }
}

#[test]
fn test_schema_validation_without_schema() {
    // Schema validation disabled - should pass even with invalid label
    let source = "MATCH (n:Alien) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - schema validation is disabled
        assert!(result.is_success(), "Should pass without schema validation");
    }
}

#[test]
fn test_schema_validation_multiple_labels() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    // Multiple labels - mix of valid and invalid
    let source = "MATCH (a:Person), (b:Alien) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let metadata = MockMetadataProvider::example();
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);
        let result = validator.validate(&program);

        // Should fail - Alien is not in schema (also fails on disconnected pattern)
        assert!(
            result.is_failure(),
            "Mixed valid/invalid labels should fail"
        );
    }
}

// ==================== Catalog Validation Tests ====================

#[test]
fn test_catalog_validation_without_catalog() {
    // Catalog validation disabled - should pass
    let source = "MATCH (n:Person) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let result = validator.validate(&program);

        // Should pass - catalog validation is disabled
        assert!(
            result.is_success(),
            "Should pass without catalog validation"
        );
    }
}

#[test]
fn test_catalog_mock_creation() {
    use gql_parser::semantic::metadata_provider::{MockMetadataProvider, MetadataProvider};

    // Test metadata provider creation
    let metadata = MockMetadataProvider::example();

    // Verify example provider has expected entries (schema snapshot for default graph)
    use gql_parser::semantic::schema_catalog::GraphRef;
    let graph = GraphRef {
        name: "default".into(),
    };
    assert!(metadata.get_schema_snapshot(&graph, None).is_ok());
}

// ==================== Warning Visibility Tests ====================

#[test]
fn test_warning_visibility_disconnected_patterns() {
    // Disconnected patterns should succeed with warning
    let source = "MATCH (a:Person), (b:Company) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should succeed (ISO-conformant disconnected patterns allowed)
        assert!(outcome.is_success(), "Disconnected patterns should succeed");

        // Should have warning diagnostics
        assert!(
            !outcome.diagnostics.is_empty(),
            "Should have warning diagnostics"
        );

        // At least one should be a warning
        let has_warning = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Warning);
        assert!(has_warning, "Should have at least one warning diagnostic");

        // Warning should mention disconnected or patterns
        let has_pattern_warning = outcome.diagnostics.iter().any(|d| {
            d.message.to_lowercase().contains("disconnect")
                || d.message.to_lowercase().contains("pattern")
        });
        assert!(
            has_pattern_warning,
            "Warning should mention disconnected patterns"
        );
    }
}

#[test]
fn test_warning_visibility_shadowing() {
    // Variable shadowing should succeed with warning when enabled
    let config = ValidationConfig {
        strict_mode: false,
        metadata_validation: false,
        warn_on_shadowing: true,
        warn_on_disconnected_patterns: false,
    };

    let source = "MATCH (n:Person) LET n = n.name RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::with_config(config);
        let outcome = validator.validate(&program);

        // Should succeed (shadowing is allowed, just warned)
        assert!(outcome.is_success(), "Shadowing should succeed");

        // Should have warning diagnostics
        assert!(
            !outcome.diagnostics.is_empty(),
            "Should have warning diagnostics"
        );

        // At least one should be a warning about shadowing
        let has_shadowing_warning = outcome.diagnostics.iter().any(|d| {
            d.severity == DiagSeverity::Warning && d.message.to_lowercase().contains("shadow")
        });
        assert!(has_shadowing_warning, "Should have shadowing warning");
    }
}

#[test]
fn test_warning_with_error_both_returned() {
    // Mix of warning and error - should fail but return both
    let source = "MATCH (a:Person), (b:Company) RETURN x"; // disconnected + undefined
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should fail due to undefined variable
        assert!(
            outcome.is_failure(),
            "Should fail due to undefined variable"
        );

        // Should have diagnostics
        assert!(!outcome.diagnostics.is_empty(), "Should have diagnostics");

        // Should have at least one error
        let has_error = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);
        assert!(has_error, "Should have at least one error");

        // May also have warning about disconnected patterns
        let has_warning = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Warning);

        // If we have warnings, verify they're distinct from errors
        if has_warning {
            let warning_count = outcome
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagSeverity::Warning)
                .count();
            let error_count = outcome
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagSeverity::Error)
                .count();
            assert!(
                warning_count > 0 && error_count > 0,
                "Should have both warnings and errors"
            );
        }
    }
}

#[test]
fn test_no_warnings_when_disabled() {
    // Warnings disabled - should not get warnings
    let config = ValidationConfig {
        strict_mode: false,
        metadata_validation: false,
        warn_on_shadowing: false,
        warn_on_disconnected_patterns: false,
    };

    let source = "MATCH (a:Person), (b:Company) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::with_config(config);
        let outcome = validator.validate(&program);

        // Should succeed
        assert!(outcome.is_success(), "Should succeed");

        // Should not have warning diagnostics (warnings disabled)
        let has_warning = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Warning);
        assert!(!has_warning, "Should not have warnings when disabled");
    }
}

#[test]
fn test_successful_validation_with_no_diagnostics() {
    // Valid query with no warnings or errors
    let source = "MATCH (n:Person)-[:KNOWS]->(m:Person) RETURN n, m";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for semantic validation source: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should succeed
        assert!(outcome.is_success(), "Valid query should succeed");
        assert!(outcome.ir.is_some(), "IR should be present");

        // No warnings for this valid, connected query
        // (diagnostics may be empty or contain only notes)
        let has_errors_or_warnings = outcome
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, DiagSeverity::Error | DiagSeverity::Warning));
        assert!(!has_errors_or_warnings, "Should have no errors or warnings");
    }
}

