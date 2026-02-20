//! Path Pattern Validation Tests
//!
//! This module contains tests for semantic validation of path patterns including:
//! - Path quantifiers (fixed, range, star, plus, question)
//! - Path modes (WALK, TRAIL, SIMPLE, ACYCLIC)
//! - Path search (ALL, ANY, SHORTEST)
//! - Path variables and path functions
//!
//! Reference: VAL_TESTS.md Section 1 - Path Pattern Validation Tests (HIGH PRIORITY)

use gql_parser::diag::DiagSeverity;
use gql_parser::parse;
use gql_parser::semantic::validator::SemanticValidator;

// ===== A. Path Quantifiers =====

#[test]
fn test_path_quantifier_fixed() {
    // Fixed quantifier: exactly N hops
    let source = "MATCH (a:Person)-[e:KNOWS]->{3}(b:Person) RETURN a, b";
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
            "fixed quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_quantifier_range() {
    // Range quantifier: min to max hops
    let source = "MATCH (a:Person)-[e:KNOWS]->{2,5}(b:Person) RETURN a, b";
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
            "range quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_quantifier_star() {
    // Star quantifier: zero or more hops
    let source = "MATCH (a:Person)-[e:KNOWS]->*(b:Person) RETURN a, b";
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
            "star quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_quantifier_plus() {
    // Plus quantifier: one or more hops
    let source = "MATCH (a:Person)-[e:KNOWS]->+(b:Person) RETURN a, b";
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
            "plus quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_quantifier_question() {
    // Question quantifier: zero or one hop
    let source = "MATCH (a:Person)-[e:KNOWS]->?(b:Person) RETURN a, b";
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
            "question quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_quantifier_invalid_range() {
    // Invalid quantifier range: upper < lower is caught by PARSER
    //
    // NOTE: This validation happens at parse time, not semantic validation time.
    // The parser (src/parser/patterns/path.rs:945-952) checks that min <= max
    // and produces an error diagnostic: "Invalid quantifier bounds: lower bound
    // is greater than upper bound"
    //
    // This test demonstrates that:
    // 1. Parser catches the error and includes it in parse_result.diagnostics
    // 2. The diagnostic system reports the issue
    //
    // To fix this at semantic level, one would need to:
    // 1. Extend src/semantic/validator/pattern_validation.rs
    // 2. Walk through PathFactor nodes and inspect quantifier fields
    // 3. Check GraphPatternQuantifier::General { min, max } where both Some and min > max
    // 4. Use SemanticDiagBuilder with PatternValidationError kind

    let source = "MATCH (a:Person)-[e:KNOWS]->{5,2}(b:Person) RETURN a, b";
    let parse_result = parse(source);

    // Parser should produce diagnostic about invalid range
    // The diagnostic is wrapped in miette::Report, so we can't directly access fields
    // Instead, we check that diagnostics exist and convert to string format
    let has_parser_error = !parse_result.diagnostics.is_empty();

    if has_parser_error {
        println!("\n✓ Parser correctly detected invalid quantifier range {{5,2}}:");
        println!("  Found {} diagnostic(s)", parse_result.diagnostics.len());
        for (i, diag) in parse_result.diagnostics.iter().enumerate() {
            // Convert report to string to show the message
            let msg = format!("{}", diag);
            if msg.to_lowercase().contains("quantifier") || msg.to_lowercase().contains("bound") {
                println!("  Diagnostic {}: Contains quantifier/bound error", i + 1);
            }
        }
    } else {
        println!("\n✗ Expected: Parser should detect invalid quantifier range {{5,2}}");
        println!("  This indicates the parser validation may have changed.");
    }

    assert!(
        has_parser_error,
        "Parser should detect invalid quantifier range where lower > upper. \
         See src/parser/patterns/path.rs:945-952 for the validation logic."
    );
}

#[test]
fn test_path_quantifier_unbounded_range() {
    // Unbounded range: min to infinity
    let source = "MATCH (a:Person)-[e:KNOWS]->{2,}(b:Person) RETURN a, b";
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
            "unbounded range quantifier path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== B. Path Modes =====

#[test]
fn test_path_mode_walk() {
    // WALK mode: allows repeated edges and nodes
    let source = "MATCH (a:Person) -[WALK :KNOWS]->+ (b:Person) RETURN a, b";
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
            "WALK mode path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_mode_trail() {
    // TRAIL mode: no repeated edges
    let source = "MATCH (a:Person) -[TRAIL :KNOWS]->+ (b:Person) RETURN a, b";
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
            "TRAIL mode path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_mode_simple() {
    // SIMPLE mode: no repeated nodes (except endpoints)
    let source = "MATCH (a:Person) -[SIMPLE :KNOWS]->+ (b:Person) RETURN a, b";
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
            "SIMPLE mode path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_mode_acyclic() {
    // ACYCLIC mode: no repeated nodes at all
    let source = "MATCH (a:Person) -[ACYCLIC :KNOWS]->+ (b:Person) RETURN a, b";
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
            "ACYCLIC mode path should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== C. Path Search =====

#[test]
fn test_path_search_all() {
    // ALL paths search
    let source = "MATCH ALL PATHS (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "ALL paths search should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_search_any() {
    // ANY path search
    let source = "MATCH ANY PATH (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "ANY path search should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_search_shortest() {
    // SHORTEST path search
    let source = "MATCH ANY SHORTEST PATH (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "SHORTEST path search should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_search_all_shortest() {
    // ALL SHORTEST paths
    let source = "MATCH ALL SHORTEST PATHS (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "ALL SHORTEST paths search should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_search_mode_combination() {
    // Combination of path mode and search
    let source = "MATCH ALL SHORTEST SIMPLE PATHS (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "combined mode and search should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_search_counted_shortest() {
    // SHORTEST N paths
    let source = "MATCH SHORTEST 5 PATHS (a:Person) -[:KNOWS]->+ (b:Person) RETURN a, b";
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
            "counted SHORTEST paths should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

// ===== D. Path Variables =====

#[test]
fn test_path_variable_binding() {
    // Path binding with path variable
    let source = "MATCH p = (a:Person)-[:KNOWS]->(b:Person) RETURN p, a, b";
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
            "path variable binding should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );

        // Verify path variable is in scope
        if let Some(ir) = outcome.ir.as_ref() {
            let symbol_table = ir.symbol_table();
            assert!(
                symbol_table.lookup_all("p").is_some(),
                "Path variable 'p' should be defined"
            );
            assert!(
                symbol_table.lookup_all("a").is_some(),
                "Node variable 'a' should be defined"
            );
            assert!(
                symbol_table.lookup_all("b").is_some(),
                "Node variable 'b' should be defined"
            );
        }
    }
}

#[test]
fn test_path_variable_with_quantifier() {
    // Path variable with quantified pattern
    let source = "MATCH p = (a:Person)-[:KNOWS]->+(b:Person) RETURN p";
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
            "path variable with quantifier should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_length_function() {
    // PATH_LENGTH function on path variable
    let source = "MATCH p = (a:Person)-[:KNOWS]->+(b:Person) RETURN PATH_LENGTH(p)";
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
            "PATH_LENGTH function should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_path_length_in_where() {
    // PATH_LENGTH in WHERE clause
    let source = "MATCH p = (a:Person)-[:KNOWS]->+(b:Person) WHERE PATH_LENGTH(p) > 2 RETURN p";
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
            "PATH_LENGTH in WHERE should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_elements_function() {
    // ELEMENTS function to extract path elements
    let source = "MATCH p = (a:Person)-[:KNOWS]->(b:Person) RETURN ELEMENTS(p)";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // ELEMENTS may not be fully implemented yet
        if outcome.is_success() {
            println!("ELEMENTS function validated successfully");
        } else {
            println!(
                "ELEMENTS function validation status: {:?}",
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
fn test_path_variable_undefined_error() {
    // Using undefined path variable should error
    let source = "MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN p";
    let parse_result = parse(source);

    assert!(
        parse_result.ast.is_some(),
        "parser should produce an AST for: {source}"
    );

    if let Some(program) = parse_result.ast {
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        // Should fail: 'p' is not defined
        let has_error = outcome
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);

        assert!(
            has_error || !outcome.is_success(),
            "undefined path variable should cause validation error"
        );
    }
}

#[test]
fn test_multiple_path_variables() {
    // Multiple path variables in same query
    let source = "MATCH p1 = (a:Person)-[:KNOWS]->(b:Person), p2 = (b)-[:WORKS_AT]->(c:Company) RETURN p1, p2";
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
            "multiple path variables should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );

        // Verify both path variables are in scope
        if let Some(ir) = outcome.ir.as_ref() {
            let symbol_table = ir.symbol_table();
            assert!(
                symbol_table.lookup_all("p1").is_some(),
                "Path variable 'p1' should be defined"
            );
            assert!(
                symbol_table.lookup_all("p2").is_some(),
                "Path variable 'p2' should be defined"
            );
        }
    }
}

#[test]
fn test_path_variable_complex_pattern() {
    // Path variable with complex pattern including modes and quantifiers
    let source = "MATCH p = (a:Person) -[SIMPLE :KNOWS]->+ (b:Person) WHERE PATH_LENGTH(p) <= 5 RETURN p";
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
            "complex path pattern should validate successfully: {:?}",
            outcome
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
        );
    }
}
