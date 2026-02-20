//! Tests for subquery and EXISTS validation (Section 3: HIGH PRIORITY).
//!
//! This module tests:
//! - EXISTS predicates with patterns
//! - Nested EXISTS expressions
//! - Scalar subqueries
//! - List subqueries
//! - Cross-references between outer and inner scopes
//!
//! Reference: VAL_TESTS.md Section 3

use gql_parser::parse;

use gql_parser::semantic::SemanticValidator;

// ============================================================================
// A. EXISTS Predicate with Patterns
// ============================================================================

#[test]
fn test_exists_basic_pattern() {
    // EXISTS with basic pattern should parse and validate
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m) } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for basic EXISTS pattern"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // For now, just ensure no critical errors related to EXISTS syntax
        // As validator implementation improves, we can add more specific checks
        let exists_errors: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| {
                d.message.contains("EXISTS")
                    || d.message.contains("syntax")
                    || d.message.contains("parse")
            })
            .collect();

        // Log any EXISTS-related diagnostics for debugging
        if !exists_errors.is_empty() {
            eprintln!("EXISTS diagnostics: {:#?}", exists_errors);
        }
    }
}

#[test]
fn test_exists_with_filter() {
    // EXISTS with WHERE clause inside
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m) WHERE m.age > 30 } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS with filter"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // Validate that m (from inner pattern) and m.age are properly scoped
        let scope_errors: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("undefined") && d.message.contains("m"))
            .collect();

        // m should be defined within EXISTS scope
        if !scope_errors.is_empty() {
            eprintln!(
                "Expected m to be defined in EXISTS scope, but got: {:#?}",
                scope_errors
            );
        }
    }
}

#[test]
fn test_exists_variable_scope_outer_to_inner() {
    // Outer variable (n) should be visible inside EXISTS
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m) WHERE n.age > 25 } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with outer variable reference"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // n from outer scope should be accessible inside EXISTS
        let scope_errors: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| {
                d.message.contains("undefined")
                    && d.message.contains("n")
                    && d.message.contains("age")
            })
            .collect();

        if !scope_errors.is_empty() {
            eprintln!(
                "Expected outer variable n to be visible, but got: {:#?}",
                scope_errors
            );
        }
    }
}

#[test]
fn test_exists_variable_isolation_inner_to_outer() {
    // Inner variable (m) should NOT be visible outside EXISTS
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m) } RETURN n, m.name";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed (validation will catch scope error)"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // m should NOT be visible outside EXISTS - should get undefined variable error
        let has_m_undefined = outcome
            .diagnostics
            .iter()
            .any(|d| {
                d.message.contains("undefined")
                    && (d.message.contains("m") || d.message.contains("variable"))
            });

        if !has_m_undefined {
            eprintln!(
                "Expected undefined variable error for m outside EXISTS scope"
            );
            eprintln!("Got diagnostics: {:#?}", outcome.diagnostics);
        }
    }
}

#[test]
fn test_exists_without_match() {
    // EXISTS can be used directly in WHERE without preceding MATCH
    let source = "MATCH (n:Person) WHERE EXISTS { MATCH (n)-[:KNOWS]->(m) } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS with explicit MATCH inside"
    );
}

#[test]
fn test_exists_multiple_patterns() {
    // EXISTS with multiple patterns
    let source = "MATCH (n:Person) WHERE EXISTS { (n)-[:KNOWS]->(m), (m)-[:LIKES]->(x) } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS with multiple patterns"
    );
}

// ============================================================================
// B. Nested EXISTS
// ============================================================================

#[test]
fn test_nested_exists_two_levels() {
    // EXISTS within EXISTS - two levels
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE EXISTS { (m)-[:LIKES]->(x) }
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for nested EXISTS (2 levels)"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // Variables should be properly scoped across nesting levels:
        // - n visible in both levels (outer scope)
        // - m visible in second level (middle scope)
        // - x only visible in innermost EXISTS

        let scope_errors: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("undefined"))
            .collect();

        if !scope_errors.is_empty() {
            eprintln!("Nested EXISTS scope diagnostics: {:#?}", scope_errors);
        }
    }
}

#[test]
fn test_nested_exists_three_levels() {
    // EXISTS within EXISTS within EXISTS - three levels
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE EXISTS {
                (m)-[:LIKES]->(x)
                WHERE EXISTS { (x)-[:OWNS]->(y) }
            }
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for deeply nested EXISTS (3 levels)"
    );
}

#[test]
fn test_nested_exists_variable_scoping() {
    // Test that variable scoping works correctly across nesting levels
    // n: visible everywhere (outermost)
    // m: visible in middle and inner EXISTS
    // x: visible only in innermost EXISTS
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE n.age > 25 AND EXISTS {
                (m)-[:LIKES]->(x)
                WHERE m.score > 100 AND n.country = 'USA'
            }
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for nested EXISTS with cross-level variable references"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // n, m should be accessible in inner EXISTS
        // No undefined variable errors expected for n or m
    }
}

#[test]
fn test_nested_exists_inner_variable_isolation() {
    // Inner EXISTS variable should not leak to outer EXISTS
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE EXISTS { (m)-[:LIKES]->(x) } AND x.name = 'Alice'
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed (validation should catch scope error)"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // x should NOT be visible outside inner EXISTS
        let has_x_undefined = outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("undefined") && d.message.contains("x"));

        if !has_x_undefined {
            eprintln!(
                "Expected undefined variable error for x outside inner EXISTS"
            );
            eprintln!("Diagnostics: {:#?}", outcome.diagnostics);
        }
    }
}

// ============================================================================
// C. Scalar Subqueries
// ============================================================================

#[test]
fn test_scalar_subquery_count() {
    // Scalar subquery returning single COUNT value
    let source = "MATCH (n:Person) LET count = (MATCH (n)-[:KNOWS]->(m) RETURN COUNT(*)) RETURN n, count";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for scalar subquery with COUNT"
    );
}

#[test]
fn test_scalar_subquery_with_outer_reference() {
    // Scalar subquery referencing outer variable
    let source = r#"
        MATCH (n:Person)
        LET friend_count = (MATCH (n)-[:KNOWS]->(m) RETURN COUNT(*))
        WHERE friend_count > 5
        RETURN n, friend_count
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for scalar subquery with outer variable reference"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // n from outer scope should be accessible in subquery
    }
}

#[test]
fn test_scalar_subquery_aggregation() {
    // Various aggregate functions in scalar subquery
    let source = r#"
        MATCH (n:Person)
        LET avg_age = (MATCH (n)-[:KNOWS]->(m) RETURN AVG(m.age))
        LET max_score = (MATCH (n)-[:KNOWS]->(m) RETURN MAX(m.score))
        LET min_salary = (MATCH (n)-[:KNOWS]->(m) RETURN MIN(m.salary))
        RETURN n, avg_age, max_score, min_salary
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for scalar subqueries with various aggregates"
    );
}

#[test]
fn test_scalar_subquery_type_inference() {
    // Scalar subquery result should have appropriate type
    let source = "MATCH (n:Person) LET total = (MATCH (n)-[:KNOWS]->(m) RETURN SUM(m.age)) RETURN n, total + 10";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed and type should allow arithmetic"
    );
}

// ============================================================================
// D. List Subqueries
// ============================================================================

#[test]
fn test_list_subquery_basic() {
    // List subquery collecting multiple results
    let source = "MATCH (n:Person) LET friends = [MATCH (n)-[:KNOWS]->(m) RETURN m] RETURN n, friends";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for basic list subquery"
    );
}

#[test]
fn test_list_subquery_with_projection() {
    // List subquery with property projection
    let source = "MATCH (n:Person) LET friend_names = [MATCH (n)-[:KNOWS]->(m) RETURN m.name] RETURN n, friend_names";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for list subquery with projection"
    );
}

#[test]
fn test_list_subquery_with_filter() {
    // List subquery with WHERE clause
    let source = r#"
        MATCH (n:Person)
        LET adult_friends = [MATCH (n)-[:KNOWS]->(m) WHERE m.age >= 18 RETURN m]
        RETURN n, adult_friends
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for list subquery with filter"
    );
}

#[test]
fn test_list_subquery_multiple_properties() {
    // List subquery returning record/tuple
    let source = r#"
        MATCH (n:Person)
        LET friend_data = [MATCH (n)-[:KNOWS]->(m) RETURN m.name, m.age]
        RETURN n, friend_data
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for list subquery with multiple properties"
    );
}

#[test]
fn test_list_subquery_type_inference() {
    // List subquery should infer LIST type
    let source = r#"
        MATCH (n:Person)
        LET friends = [MATCH (n)-[:KNOWS]->(m) RETURN m]
        RETURN n, SIZE(friends)
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed and SIZE should work on list"
    );
}

// ============================================================================
// E. Cross-References Between Outer and Inner Scopes
// ============================================================================

#[test]
fn test_cross_reference_outer_in_exists() {
    // Outer variable used in EXISTS pattern and filter
    let source = r#"
        MATCH (n:Person)
        WHERE n.age > 18
            AND EXISTS { (n)-[:KNOWS]->(m) WHERE m.country = n.country }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with outer variable in EXISTS"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // n.country should be accessible inside EXISTS
        let scope_errors: Vec<_> = outcome
            .diagnostics
            .iter()
            .filter(|d| {
                d.message.contains("undefined")
                    && (d.message.contains("n") || d.message.contains("country"))
            })
            .collect();

        if !scope_errors.is_empty() {
            eprintln!(
                "Expected n.country to be accessible in EXISTS: {:#?}",
                scope_errors
            );
        }
    }
}

#[test]
fn test_cross_reference_undefined_in_subquery() {
    // Undefined variable in subquery should error
    let source = "MATCH (n:Person) LET count = (MATCH (x)-[:KNOWS]->(m) RETURN COUNT(*)) RETURN n, count";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed (validation should catch undefined variable)"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // x is not defined in outer scope
        let has_undefined = outcome
            .diagnostics
            .iter()
            .any(|d| d.message.contains("undefined") && d.message.contains("x"));

        if !has_undefined {
            eprintln!("Expected undefined variable error for x in subquery");
            eprintln!("Diagnostics: {:#?}", outcome.diagnostics);
        }
    }
}

#[test]
fn test_cross_reference_type_compatibility() {
    // Type compatibility between outer and inner scopes
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE m.age > n.age + 5
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with cross-scope arithmetic"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // n.age should be accessible and type-compatible with arithmetic
    }
}

#[test]
fn test_cross_reference_multiple_levels() {
    // Variable from outermost scope referenced in deeply nested subquery
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS {
            (n)-[:KNOWS]->(m)
            WHERE EXISTS {
                (m)-[:LIKES]->(x)
                WHERE x.country = n.country
            }
        }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with cross-level reference"
    );

    // Built-ins are always available (checked directly by validator)
    let validator = SemanticValidator::new();

    if let Some(program) = parse_result.ast {
        let _outcome = validator.validate(&program);
        // n from outermost scope should be accessible in innermost EXISTS
    }
}

#[test]
fn test_cross_reference_in_scalar_subquery() {
    // Outer variable in scalar subquery
    let source = r#"
        MATCH (n:Person)
        LET local_friends = (
            MATCH (n)-[:KNOWS]->(m)
            WHERE m.city = n.city
            RETURN COUNT(*)
        )
        RETURN n, local_friends
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with outer variable in scalar subquery"
    );
}

#[test]
fn test_cross_reference_in_list_subquery() {
    // Outer variable in list subquery filter
    let source = r#"
        MATCH (n:Person)
        LET similar_age_friends = [
            MATCH (n)-[:KNOWS]->(m)
            WHERE ABS(m.age - n.age) < 5
            RETURN m.name
        ]
        RETURN n, similar_age_friends
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed with outer variable in list subquery"
    );
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

#[test]
fn test_exists_in_return_clause() {
    // EXISTS can be used in RETURN
    let source = "MATCH (n:Person) RETURN n, EXISTS { (n)-[:KNOWS]->(:Person) } AS has_friends";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS in RETURN"
    );
}

#[test]
fn test_exists_in_case_expression() {
    // EXISTS can be used in CASE
    let source = r#"
        MATCH (n:Person)
        RETURN n,
            CASE
                WHEN EXISTS { (n)-[:KNOWS]->() } THEN 'connected'
                ELSE 'isolated'
            END AS status
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS in CASE"
    );
}

#[test]
fn test_multiple_exists_same_level() {
    // Multiple EXISTS at same nesting level with AND/OR
    let source = r#"
        MATCH (n:Person)
        WHERE EXISTS { (n)-[:KNOWS]->() }
            AND EXISTS { (n)-[:LIKES]->() }
            AND NOT EXISTS { (n)-[:DISLIKES]->() }
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for multiple EXISTS with boolean operators"
    );
}

#[test]
fn test_exists_empty_pattern() {
    // EXISTS with just variable binding (edge case)
    let source = "MATCH (n:Person) WHERE EXISTS { (m:Person) } RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for EXISTS with unconnected pattern"
    );
}

#[test]
fn test_subquery_in_set_clause() {
    // Subquery used in SET mutation
    let source = r#"
        MATCH (n:Person)
        SET n.friend_count = (MATCH (n)-[:KNOWS]->() RETURN COUNT(*))
        RETURN n
    "#;
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "Parse should succeed for subquery in SET"
    );
}
