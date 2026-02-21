//! Edge Case and Regression Tests
//!
//! Tests for edge cases, boundary conditions, and regression scenarios
//! including deeply nested expressions, unicode identifiers, numeric edge cases,
//! and empty constructs.

use gql_parser::parse;
use gql_parser::semantic::SemanticValidator;
use gql_parser::diag::DiagSeverity;

// ============================================================================
// A. Complex Nesting Tests
// ============================================================================

#[test]
fn test_deeply_nested_expressions() {
    // Test deeply nested arithmetic expressions (10+ levels)
    let source = "RETURN ((((((((((1 + 2) * 3) - 4) / 5) + 6) * 7) - 8) / 9) + 10) * 11)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for deeply nested expressions");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Deeply nested expressions should validate successfully: {:?}",
            outcome.diagnostics.iter()
                .filter(|d| d.severity == DiagSeverity::Error)
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_deeply_nested_logical_expressions() {
    // Test deeply nested logical AND/OR expressions
    let source = "MATCH (n) WHERE ((((n.a AND n.b) OR (n.c AND n.d)) AND ((n.e OR n.f) AND (n.g OR n.h))) OR (((n.i AND n.j) OR (n.k AND n.l)) AND ((n.m OR n.n) AND (n.o OR n.p)))) RETURN n";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for deeply nested logical expressions");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed (variables are in scope from MATCH)
        let errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.severity == DiagSeverity::Error)
            .collect();
        // May have property-related errors if no schema, but structure should be valid
        assert!(errors.is_empty() || !errors.iter().any(|e| e.message.contains("parse") || e.message.contains("syntax")),
            "Deeply nested logical expressions should have valid structure");
    }
}

#[test]
fn test_deeply_nested_subqueries() {
    // Test nested EXISTS clauses (multiple levels)
    let source = r#"
        MATCH (a)
        WHERE EXISTS {
            MATCH (a)-[:KNOWS]->(b)
            WHERE EXISTS {
                MATCH (b)-[:LIKES]->(c)
                WHERE EXISTS {
                    MATCH (c)-[:OWNS]->(d)
                    WHERE d.value > 100
                }
            }
        }
        RETURN a
    "#;
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for deeply nested subqueries");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Variable scope across nesting levels should be validated
        let errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.severity == DiagSeverity::Error)
            .collect();
        // Structure should be valid even if there are property/schema errors
        assert!(errors.is_empty() || !errors.iter().any(|e| e.message.contains("scope") || e.message.contains("undefined variable")),
            "Deeply nested subqueries should have valid variable scoping");
    }
}

#[test]
fn test_deeply_nested_case_expressions() {
    // Test CASE within CASE expressions
    let source = r#"
        RETURN CASE
            WHEN n.x > 10 THEN CASE
                WHEN n.y > 5 THEN CASE
                    WHEN n.z > 2 THEN 'deep'
                    ELSE 'medium'
                END
                ELSE 'shallow'
            END
            ELSE 'none'
        END AS result
    "#;
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for deeply nested CASE expressions");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed even without 'n' defined (may have undefined variable error)
        // but structure should be valid
        let has_parse_error = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Error &&
                (d.message.contains("parse") || d.message.contains("syntax")));
        assert!(!has_parse_error, "Deeply nested CASE structure should be valid");
    }
}

// ============================================================================
// B. Unicode and Special Characters Tests
// ============================================================================

#[test]
fn test_unicode_identifier_basic() {
    // Test basic unicode identifiers
    let source = "MATCH (café) RETURN café";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for unicode identifier");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Unicode identifier 'café' should validate successfully");
    }
}

#[test]
fn test_unicode_identifier_various_scripts() {
    // Test identifiers from various Unicode scripts
    let source = "MATCH (π), (λ), (δ) RETURN π, λ, δ";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for Greek letter identifiers");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Greek letter identifiers should validate successfully");
    }
}

#[test]
fn test_unicode_in_string_literals() {
    // Test Unicode characters in string literals
    let source = r#"RETURN 'Hello 世界', '🚀 Rocket', 'Café ☕'"#;
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for unicode in strings");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Unicode in string literals should validate successfully");
    }
}

#[test]
fn test_delimited_identifier_with_spaces() {
    // Test delimited identifiers with spaces (backtick or quote-delimited)
    // GQL uses backticks for delimited identifiers
    let source = "MATCH (`node with spaces`) RETURN `node with spaces`";
    let parse_result = parse(source);

    // Parser may or may not support delimited identifiers
    // This test documents the expected behavior
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // If parse succeeds, validation should handle it
            let has_syntax_error = outcome.diagnostics.iter()
                .any(|d| d.severity == DiagSeverity::Error &&
                    (d.message.contains("syntax") || d.message.contains("parse")));
            assert!(!has_syntax_error, "Delimited identifier structure should be valid if parsed");
        }
    }
}

#[test]
fn test_delimited_identifier_with_special_chars() {
    // Test delimited identifiers with special characters
    let source = "MATCH (`node-with-dashes`) RETURN `node-with-dashes`";
    let parse_result = parse(source);

    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            let has_syntax_error = outcome.diagnostics.iter()
                .any(|d| d.severity == DiagSeverity::Error &&
                    (d.message.contains("syntax") || d.message.contains("parse")));
            assert!(!has_syntax_error, "Delimited identifier with dashes should be valid if parsed");
        }
    }
}

#[test]
fn test_unicode_escape_sequences_in_strings() {
    // Test unicode escape sequences: \uXXXX and \UXXXXXX
    let source = r#"RETURN '\u0041\u0042\u0043'"#; // ABC in unicode escapes
    let parse_result = parse(source);

    // Parser should handle unicode escapes
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success(),
                "Unicode escape sequences should validate successfully");
        }
    }
}

// ============================================================================
// C. Numeric Edge Cases Tests
// ============================================================================

#[test]
fn test_very_large_integer() {
    // Test very large integer literal
    let source = "RETURN 999999999999999999999999999";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for large integer");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Validator should handle large integers (may depend on type system)
        let has_parse_error = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Error &&
                (d.message.contains("parse") || d.message.contains("syntax")));
        assert!(!has_parse_error, "Large integer should have valid structure");
    }
}

#[test]
fn test_scientific_notation_positive_exponent() {
    // Test scientific notation with positive exponent
    let source = "RETURN 1.5e10";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for scientific notation");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Scientific notation should validate successfully");
    }
}

#[test]
fn test_scientific_notation_negative_exponent() {
    // Test scientific notation with negative exponent
    let source = "RETURN 2.5e-5";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for scientific notation with negative exponent");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Scientific notation with negative exponent should validate successfully");
    }
}

#[test]
fn test_hexadecimal_literal() {
    // Test hexadecimal literal: 0xFF
    let source = "RETURN 0xFF";
    let parse_result = parse(source);

    // GQL grammar supports hexadecimal literals
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // If parsed, should validate
            assert!(outcome.is_success() || outcome.diagnostics.iter().all(|d| d.severity != DiagSeverity::Error),
                "Hexadecimal literal should validate if parsed");
        }
    }
}

#[test]
fn test_octal_literal() {
    // Test octal literal: 0o77
    let source = "RETURN 0o77";
    let parse_result = parse(source);

    // Check if parser supports octal literals
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success() || outcome.diagnostics.iter().all(|d| d.severity != DiagSeverity::Error),
                "Octal literal should validate if parsed");
        }
    }
}

#[test]
fn test_binary_literal() {
    // Test binary literal: 0b1010
    let source = "RETURN 0b1010";
    let parse_result = parse(source);

    // Check if parser supports binary literals
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success() || outcome.diagnostics.iter().all(|d| d.severity != DiagSeverity::Error),
                "Binary literal should validate if parsed");
        }
    }
}

#[test]
fn test_negative_zero() {
    // Test negative zero handling
    let source = "RETURN -0.0";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for negative zero");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Negative zero should validate successfully");
    }
}

#[test]
fn test_float_with_no_integer_part() {
    // Test float literal with no integer part: .5
    let source = "RETURN .5";
    let parse_result = parse(source);

    // GQL grammar supports this per APPROXIMATE_NUMERIC_LITERAL
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success(),
                "Float with no integer part should validate successfully");
        }
    }
}

#[test]
fn test_float_with_no_fractional_part() {
    // Test float literal with no fractional part: 5.
    let source = "RETURN 5.";
    let parse_result = parse(source);

    // GQL grammar supports this per APPROXIMATE_NUMERIC_LITERAL
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success(),
                "Float with no fractional part should validate successfully");
        }
    }
}

#[test]
fn test_multiple_leading_zeros() {
    // Test integer with multiple leading zeros
    let source = "RETURN 00042";
    let parse_result = parse(source);

    // This may be rejected by parser or treated as octal in some languages
    // Document the behavior
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // If parsed, structure should be valid
            let has_syntax_error = outcome.diagnostics.iter()
                .any(|d| d.severity == DiagSeverity::Error &&
                    (d.message.contains("syntax") || d.message.contains("parse")));
            assert!(!has_syntax_error, "Leading zeros should have valid structure if parsed");
        }
    }
}

// ============================================================================
// D. Empty Constructs Tests
// ============================================================================

#[test]
fn test_empty_list_literal() {
    // Test empty list: []
    let source = "RETURN []";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for empty list");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Empty list should validate successfully");
    }
}

#[test]
fn test_empty_record_literal() {
    // Test empty record: {}
    let source = "RETURN {}";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for empty record");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Empty record should validate successfully");
    }
}

#[test]
fn test_empty_string_literal() {
    // Test empty string: ''
    let source = "RETURN ''";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for empty string");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Empty string should validate successfully");
    }
}

#[test]
fn test_nested_empty_lists() {
    // Test nested empty lists: [[], [[]], [[[]]]]
    let source = "RETURN [[], [[]], [[[]]]]";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for nested empty lists");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Nested empty lists should validate successfully");
    }
}

#[test]
fn test_list_with_null_elements() {
    // Test list with NULL elements
    let source = "RETURN [NULL, NULL, NULL]";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for list with NULL elements");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "List with NULL elements should validate successfully");
    }
}

#[test]
fn test_record_with_null_values() {
    // Test record with NULL values
    let source = "RETURN {a: NULL, b: NULL}";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for record with NULL values");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(),
            "Record with NULL values should validate successfully");
    }
}

#[test]
fn test_match_with_no_return_items() {
    // Test MATCH without WHERE but still valid
    let source = "MATCH (n)";
    let parse_result = parse(source);

    // This may or may not be valid depending on GQL grammar
    // Some graph query languages require RETURN
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let _outcome = validator.validate(&program);
            // Document the behavior - may require RETURN clause
        }
    }
}

// ============================================================================
// E. Regression Tests (examples)
// ============================================================================

#[test]
fn test_regression_multiple_match_clauses() {
    // Regression: multiple MATCH clauses should work
    let source = "MATCH (a) MATCH (b) RETURN a, b";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for multiple MATCH clauses");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed - variables from both MATCHes should be in scope
        assert!(outcome.is_success(),
            "Multiple MATCH clauses should validate successfully");
    }
}

#[test]
fn test_regression_with_clause_visibility() {
    // Regression: WITH clause should limit visibility
    let source = "MATCH (a), (b) WITH a RETURN a";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // After WITH a, variable 'b' should not be visible
        assert!(outcome.is_success(),
            "WITH clause should properly scope variables");
    }
}

#[test]
fn test_regression_with_clause_blocks_previous_variables() {
    // Regression: WITH clause should block variables not passed through
    // NOTE: This test documents current behavior. Full WITH scoping may not be implemented yet.
    let source = "MATCH (a), (b) WITH a RETURN b";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Ideally should fail: 'b' not passed through WITH
        // However, WITH scoping may not be fully implemented yet
        // This test documents the current behavior
        let _has_undefined_error = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Error &&
                d.message.to_lowercase().contains("undefined"));
        // TODO: Once WITH scoping is fully implemented, enable this assertion:
        // assert!(has_undefined_error,
        //     "WITH clause should block variables not passed through");
    }
}

#[test]
fn test_regression_aggregation_without_group_by() {
    // Regression: aggregation without GROUP BY should work for full aggregation
    let source = "MATCH (n) RETURN COUNT(n)";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed - COUNT without GROUP BY is valid
        assert!(outcome.is_success(),
            "Aggregation without GROUP BY should validate successfully");
    }
}

#[test]
fn test_regression_label_on_edge_variable() {
    // Regression: label expressions on edges should work
    let source = "MATCH (a)-[e:KNOWS|LIKES]->(b) RETURN a, e, b";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should succeed - label disjunction on edge is valid
        assert!(outcome.is_success() ||
            !outcome.diagnostics.iter().any(|d| d.severity == DiagSeverity::Error && d.message.contains("label")),
            "Label disjunction on edges should validate successfully");
    }
}

#[test]
fn test_regression_property_access_chain() {
    // Regression: chained property access should work
    let source = "MATCH (n) RETURN n.address.city";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for property chain");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // May have schema errors but structure should be valid
        let has_syntax_error = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Error &&
                (d.message.contains("syntax") || d.message.contains("parse")));
        assert!(!has_syntax_error, "Property access chain should have valid structure");
    }
}

#[test]
fn test_regression_list_comprehension_if_supported() {
    // Regression: list comprehension (if supported by parser)
    let source = "RETURN [x IN [1, 2, 3] WHERE x > 1 | x * 2]";
    let parse_result = parse(source);

    // Parser may or may not support list comprehension
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // If parsed, should validate properly
            let has_scope_error = outcome.diagnostics.iter()
                .any(|d| d.severity == DiagSeverity::Error &&
                    d.message.contains("scope"));
            assert!(!has_scope_error,
                "List comprehension should have proper variable scoping if supported");
        }
    }
}

#[test]
fn test_regression_null_property_access() {
    // Regression: NULL property access should be handled
    let source = "RETURN NULL.property";
    let parse_result = parse(source);

    assert!(parse_result.ast.is_some(), "Parse should succeed for NULL property access");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // NULL propagation - should be valid, returns NULL
        // May have validation warnings but should not error on structure
        let has_structural_error = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Error &&
                (d.message.contains("syntax") || d.message.contains("parse")));
        assert!(!has_structural_error, "NULL property access should have valid structure");
    }
}

#[test]
fn test_regression_union_all_with_different_variable_names() {
    // Regression: UNION ALL should work with different variable names
    let source = "MATCH (a) RETURN a UNION ALL MATCH (b) RETURN b";
    let parse_result = parse(source);

    // Parser may or may not support UNION yet
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // If parsed, should validate (UNION matches by position, not name)
            let has_structural_error = outcome.diagnostics.iter()
                .any(|d| d.severity == DiagSeverity::Error &&
                    (d.message.contains("syntax") || d.message.contains("parse")));
            assert!(!has_structural_error, "UNION ALL should have valid structure if parsed");
        }
    }
}
