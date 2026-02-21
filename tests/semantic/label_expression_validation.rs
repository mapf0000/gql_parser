//! Label Expression Validation Tests
//!
//! This module contains tests for semantic validation of label expressions including:
//! - Label disjunction (OR)
//! - Label conjunction (AND)
//! - Label negation (NOT)
//! - Wildcard labels
//! - Complex label combinations
//!
//! Reference: VAL_TESTS.md Section 2 - Label Expression Validation Tests (HIGH PRIORITY)

use gql_parser::parse;
use gql_parser::semantic::validator::SemanticValidator;

// ===== A. Label Disjunction (OR) =====

#[test]
fn test_label_disjunction_two_labels() {
    // Node with Person OR Company label
    let source = "MATCH (n:Person|Company) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label disjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_disjunction_multiple_labels() {
    // Node with multiple OR labels
    let source = "MATCH (n:Person|Employee|Student) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "multiple label disjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_disjunction_on_edges() {
    // Edge with type disjunction
    let source = "MATCH (a)-[e:KNOWS|FOLLOWS]->(b) RETURN a, e, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "edge type disjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_disjunction_in_return() {
    // Using disjunction labels with variable in scope
    let source = "MATCH (n:Person|Company) WHERE n.name IS NOT NULL RETURN n.name";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label disjunction with property access should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== B. Label Conjunction (AND) =====

#[test]
fn test_label_conjunction_two_labels() {
    // Node with BOTH Person AND Employee labels
    let source = "MATCH (n:Person&Employee) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label conjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_conjunction_multiple_labels() {
    // Node with multiple AND labels
    let source = "MATCH (n:Person&Employee&Active) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "multiple label conjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_conjunction_with_property_access() {
    // Conjunction with property filter
    let source = "MATCH (n:Person&Employee) WHERE n.age > 30 RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label conjunction with property access should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== C. Label Negation (NOT) =====

#[test]
fn test_label_negation_simple() {
    // Node without Robot label
    let source = "MATCH (n:!Robot) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label negation should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_negation_with_conjunction() {
    // Person but not Employee
    let source = "MATCH (n:Person&!Employee) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label negation with conjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_negation_with_disjunction() {
    // Not Robot or Not AI
    let source = "MATCH (n:!Robot|!AI) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label negation with disjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_negation_on_edge() {
    // Edge without specific type
    let source = "MATCH (a)-[e:!BLOCKED]->(b) RETURN a, e, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "edge type negation should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_multiple_negations() {
    // Multiple negations combined
    let source = "MATCH (n:!Robot&!AI&!Bot) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "multiple label negations should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== D. Wildcard =====

#[test]
fn test_label_wildcard_node() {
    // Node with any label
    let source = "MATCH (n:%) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard label should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_wildcard_edge() {
    // Edge with any type
    let source = "MATCH (a)-[e:%]->(b) RETURN a, e, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard edge type should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_wildcard_in_pattern() {
    // Using wildcard in complex pattern
    let source = "MATCH (a:Person)-[:%]->(b:%) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard in pattern should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_wildcard_with_path_quantifier() {
    // Wildcard with quantified path
    let source = "MATCH (a:Person)-[:%]->+(b) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard with quantifier should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== E. Complex Combinations =====

#[test]
fn test_label_complex_disjunction_conjunction() {
    // Testing precedence: Person OR (Company AND Active)
    let source = "MATCH (n:Person|Company&Active) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "complex label expression with OR/AND should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_complex_parenthesized() {
    // Parenthesized expression to control precedence: (Person OR Company) AND Active
    let source = "MATCH (n:(Person|Company)&Active) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "parenthesized label expression should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_complex_negation_with_disjunction() {
    // NOT (Robot OR AI)
    let source = "MATCH (n:!(Robot|AI)) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "negation with disjunction should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_complex_all_operators() {
    // Combination of all operators: (Person OR Student) AND Active AND NOT Suspended
    let source = "MATCH (n:(Person|Student)&Active&!Suspended) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "complex label expression with all operators should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_complex_nested_expression() {
    // Deeply nested label expression
    let source = "MATCH (n:((Person|Employee)&Active)|((Company|Organization)&!Defunct)) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "deeply nested label expression should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_complex_with_wildcard() {
    // Wildcard combined with other operators
    let source = "MATCH (n:%&!Deleted) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard with negation should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_multiple_patterns_with_complex_labels() {
    // Multiple patterns with different complex label expressions
    let source = "MATCH (a:Person|Company), (b:Employee&Active), (c:!Robot) WHERE a.id = b.managerId RETURN a, b, c";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "multiple patterns with complex labels should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_in_optional_match() {
    // Complex label expressions in OPTIONAL MATCH
    let source = "MATCH (a:Person) OPTIONAL MATCH (a)-[:MANAGES]->(b:Employee&!Contractor) RETURN a, b";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "label expressions in OPTIONAL MATCH should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== Error Cases =====

#[test]
fn test_label_empty_disjunction() {
    // Empty label expression - parser may catch this
    let source = "MATCH (n:) RETURN n";
    let parse_result = parse(source);

    // Parser may reject this before semantic validation
    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        let has_error = !outcome.is_success() || !parse_result.diagnostics.is_empty();
        assert!(
            has_error,
            "empty label expression should produce error"
        );
    } else {
        // Parser rejected it, which is also acceptable
        assert!(
            !parse_result.diagnostics.is_empty(),
            "empty label should be rejected by parser"
        );
    }
}

#[test]
fn test_label_double_negation() {
    // Double negation - may or may not be supported
    let source = "MATCH (n:!!Person) RETURN n";
    let parse_result = parse(source);

    // This is expected to parse and validate - double negation should be semantically valid
    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Double negation is semantically equivalent to the positive label
        // The parser and validator should handle this
        if outcome.is_success() {
            println!("Double negation validated successfully (semantically equivalent to positive)");
        } else {
            println!(
                "Double negation validation result: {:?}",
                outcome
                    .diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn test_label_conjunction_edge_precedence() {
    // Test operator precedence on edges: KNOWS or FOLLOWS and ACTIVE
    let source = "MATCH (a)-[e:KNOWS|FOLLOWS&ACTIVE]->(b) RETURN e";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "edge type expression with precedence should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_label_wildcard_negation() {
    // Wildcard with negation - semantically means "has any label except..."
    let source = "MATCH (n:%&!Deleted&!Archived) RETURN n";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);
        assert!(
            outcome.is_success(),
            "wildcard with multiple negations should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}
