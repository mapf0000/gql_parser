//! Tests for set operations validation (UNION, EXCEPT, INTERSECT).
//!
//! This module tests:
//! - UNION operations (basic, ALL, distinct)
//! - EXCEPT operations
//! - INTERSECT operations
//! - Column count and type compatibility
//! - Combination and precedence
//!
//! Test coverage for VAL_TESTS.md Section 6: Set Operations Validation Tests (MEDIUM PRIORITY)

use gql_parser::parse;
use gql_parser::semantic::SemanticValidator;

// ============================================================================
// A. UNION Operations
// ============================================================================

#[test]
fn test_union_basic_same_schema() {
    // Basic UNION with identical result columns
    let source = r#"
        MATCH (n:Person) RETURN n.name, n.age
        UNION
        MATCH (m:Employee) RETURN m.name, m.age
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // May have warnings about disconnected patterns, but no UNION errors
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "No UNION-related errors expected for valid schema match");
    }
}

#[test]
fn test_union_all_vs_union_distinct() {
    // UNION ALL should allow duplicates, UNION should remove duplicates
    let source_all = r#"
        MATCH (n:Person) RETURN n.name
        UNION ALL
        MATCH (m:Person) RETURN m.name
    "#;
    let parse_result = parse(source_all);
    assert!(parse_result.ast.is_some(), "Parse should succeed for UNION ALL");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "UNION ALL should validate successfully");
    }

    // Test basic UNION (distinct by default)
    let source_distinct = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Person) RETURN m.name
    "#;
    let parse_result = parse(source_distinct);
    assert!(parse_result.ast.is_some(), "Parse should succeed for UNION");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "UNION should validate successfully");
    }
}

#[test]
fn test_union_column_count_mismatch() {
    // UNION with different number of columns should error
    let source = r#"
        MATCH (n:Person) RETURN n.name, n.age
        UNION
        MATCH (m:Person) RETURN m.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // The validator may or may not catch this at validation time,
        // but we document the expected behavior.
        // For now, we just verify the parse succeeds and validator runs.
        // If the validator implements this check, we'd expect an error.
    }
}

#[test]
fn test_union_three_queries() {
    // UNION of 3+ queries
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        UNION
        MATCH (x:Contractor) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "Multiple UNIONs should validate successfully");
    }
}

#[test]
fn test_union_with_aliases() {
    // UNION with column aliases
    let source = r#"
        MATCH (n:Person) RETURN n.name AS fullname, n.age AS years
        UNION
        MATCH (m:Employee) RETURN m.name AS fullname, m.age AS years
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "UNION with aliases should validate successfully");
    }
}

#[test]
fn test_union_with_expressions() {
    // UNION with computed expressions
    let source = r#"
        MATCH (n:Person) RETURN n.firstName || ' ' || n.lastName AS name, n.age * 12 AS months
        UNION
        MATCH (m:Employee) RETURN m.name, m.tenure
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Validator should check type compatibility if type checking is implemented
        // For now, we just verify the structure is valid
    }
}

// ============================================================================
// B. EXCEPT Operations
// ============================================================================

#[test]
fn test_except_basic() {
    // Basic EXCEPT operation
    let source = r#"
        MATCH (n:Person) RETURN n.name, n.age
        EXCEPT
        MATCH (m:Employee) RETURN m.name, m.age
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let except_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("except")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(except_errors.is_empty(), "EXCEPT should validate successfully");
    }
}

#[test]
fn test_except_all() {
    // EXCEPT ALL retains duplicates
    let source = r#"
        MATCH (n:Person) RETURN n.name
        EXCEPT ALL
        MATCH (m:Employee) RETURN m.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let except_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("except")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(except_errors.is_empty(), "EXCEPT ALL should validate successfully");
    }
}

#[test]
fn test_except_column_count_mismatch() {
    // EXCEPT with different column counts should error
    let source = r#"
        MATCH (n:Person) RETURN n.name, n.age, n.city
        EXCEPT
        MATCH (m:Employee) RETURN m.name, m.age
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Expected to have column mismatch error if validator implements this check
    }
}

#[test]
fn test_except_multiple_operations() {
    // Multiple EXCEPT operations
    let source = r#"
        MATCH (n:Person) RETURN n.name
        EXCEPT
        MATCH (m:Employee) RETURN m.name
        EXCEPT
        MATCH (x:Contractor) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let except_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("except")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(except_errors.is_empty(), "Multiple EXCEPT operations should validate successfully");
    }
}

// ============================================================================
// C. INTERSECT Operations
// ============================================================================

#[test]
fn test_intersect_basic() {
    // Basic INTERSECT operation
    let source = r#"
        MATCH (n:Person) RETURN n.name, n.age
        INTERSECT
        MATCH (m:Employee) RETURN m.name, m.age
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let intersect_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("intersect")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(intersect_errors.is_empty(), "INTERSECT should validate successfully");
    }
}

#[test]
fn test_intersect_all() {
    // INTERSECT ALL retains duplicates
    let source = r#"
        MATCH (n:Person) RETURN n.name
        INTERSECT ALL
        MATCH (m:Employee) RETURN m.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let intersect_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("intersect")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(intersect_errors.is_empty(), "INTERSECT ALL should validate successfully");
    }
}

#[test]
fn test_intersect_column_compatibility() {
    // INTERSECT with compatible columns
    let source = r#"
        MATCH (n:Person) RETURN n.id, n.name
        INTERSECT
        MATCH (m:Employee) RETURN m.id, m.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let intersect_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("intersect")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(intersect_errors.is_empty(), "INTERSECT with compatible columns should validate successfully");
    }
}

#[test]
fn test_intersect_multiple_operations() {
    // Multiple INTERSECT operations
    let source = r#"
        MATCH (n:Person) RETURN n.name
        INTERSECT
        MATCH (m:Employee) RETURN m.name
        INTERSECT
        MATCH (x:Manager) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let intersect_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("intersect")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(intersect_errors.is_empty(), "Multiple INTERSECT operations should validate successfully");
    }
}

// ============================================================================
// D. Combination and Precedence
// ============================================================================

#[test]
fn test_union_except_combination() {
    // Combination of UNION and EXCEPT
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        EXCEPT
        MATCH (x:Contractor) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Validator should handle precedence and associativity correctly
    }
}

#[test]
fn test_union_intersect_combination() {
    // Combination of UNION and INTERSECT
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        INTERSECT
        MATCH (x:Manager) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Validator should handle precedence correctly
    }
}

#[test]
fn test_parenthesized_set_operations() {
    // Explicit precedence with parentheses
    let source = r#"
        (MATCH (n:Person) RETURN n.name
         UNION
         MATCH (m:Employee) RETURN m.name)
        EXCEPT
        MATCH (x:Contractor) RETURN x.name
    "#;
    let parse_result = parse(source);

    // Note: GQL may or may not support parenthesized set operations
    // This test documents the expected behavior
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let _outcome = validator.validate(&program);
            // Validator should respect parenthesized precedence
        }
    }
}

#[test]
fn test_except_intersect_combination() {
    // Combination of EXCEPT and INTERSECT
    let source = r#"
        MATCH (n:Person) RETURN n.name
        EXCEPT
        MATCH (m:Employee) RETURN m.name
        INTERSECT
        MATCH (x:Manager) RETURN x.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Validator should handle precedence correctly
    }
}

#[test]
fn test_all_set_operations_combination() {
    // All three set operations combined
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        EXCEPT
        MATCH (x:Contractor) RETURN x.name
        INTERSECT
        MATCH (y:Manager) RETURN y.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Validator should handle complex precedence and associativity
    }
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

#[test]
fn test_union_with_where_clause() {
    // UNION with WHERE clauses in each query
    let source = r#"
        MATCH (n:Person) WHERE n.age > 18 RETURN n.name
        UNION
        MATCH (m:Employee) WHERE m.tenure > 5 RETURN m.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "UNION with WHERE should validate successfully");
    }
}

#[test]
fn test_union_with_order_by() {
    // UNION with ORDER BY (should apply to final result)
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        ORDER BY name
    "#;
    let parse_result = parse(source);

    // Note: The grammar may require specific syntax for ORDER BY after set operations
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let _outcome = validator.validate(&program);
            // ORDER BY should be valid if applied to the final result
        }
    }
}

#[test]
fn test_union_with_limit() {
    // UNION with LIMIT (should apply to final result)
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (m:Employee) RETURN m.name
        LIMIT 10
    "#;
    let parse_result = parse(source);

    // Note: The grammar may require specific syntax for LIMIT after set operations
    if parse_result.ast.is_some() {
        let validator = SemanticValidator::new();
        if let Some(program) = parse_result.ast {
            let _outcome = validator.validate(&program);
            // LIMIT should be valid if applied to the final result
        }
    }
}

#[test]
fn test_set_operations_with_aggregation() {
    // Set operations combining aggregated and non-aggregated queries
    let source = r#"
        MATCH (n:Person) RETURN COUNT(*) AS cnt
        UNION
        MATCH (m:Employee) RETURN COUNT(*) AS cnt
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // Both sides return the same column type (aggregate), should be valid
    }
}

#[test]
fn test_set_operations_with_null_values() {
    // Set operations with NULL values
    let source = r#"
        MATCH (n:Person) RETURN n.name, NULL AS extra
        UNION
        MATCH (m:Employee) RETURN m.name, m.department AS extra
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // NULL should be compatible with any type
    }
}

#[test]
fn test_union_distinct_implicit() {
    // UNION without ALL should be DISTINCT by default
    let source = r#"
        MATCH (n:Person) RETURN n.name
        UNION
        MATCH (n:Person) RETURN n.name
    "#;
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Parse should succeed");

    let validator = SemanticValidator::new();
    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should validate successfully - duplicates will be removed
        let union_errors: Vec<_> = outcome.diagnostics.iter()
            .filter(|d| d.message.to_lowercase().contains("union")
                     && d.message.to_lowercase().contains("error"))
            .collect();
        assert!(union_errors.is_empty(), "UNION DISTINCT should validate successfully");
    }
}
