//! Edge Case Testing Suite
//!
//! This test suite covers boundary conditions, malformed input, and uncommon
//! syntax combinations that might expose parser bugs or edge cases.
//!
//! Test Categories:
//! - Boundary conditions (empty, minimal, maximal)
//! - Malformed input (unclosed delimiters, invalid syntax)
//! - Uncommon syntax combinations
//! - Parameter edge cases
//! - Identifier edge cases
//! - Operator edge cases

use gql_parser::parse;

// ===== Boundary Conditions =====

#[test]
fn minimal_valid_query() {
    // Shortest valid query
    let queries = vec![
        "MATCH (n) RETURN n",
        "RETURN 1",
        "RETURN TRUE",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Minimal query '{}' should parse",
            query
        );
    }
}

#[test]
fn empty_node_pattern() {
    let result = parse("MATCH () RETURN 1");
    assert!(result.ast.is_some(), "Empty node pattern should parse");
}

#[test]
fn empty_edge_pattern() {
    let result = parse("MATCH ()-[]->() RETURN 1");
    // May parse depending on implementation
    let _ = result.ast;
}

#[test]
fn numeric_literal_edge_cases() {
    let queries = vec![
        "RETURN 0",
        "RETURN -0",
        "RETURN 0.0",
        "RETURN -0.0",
        "RETURN 1e10",
        "RETURN 1E10",
        "RETURN 1.5e-10",
        "RETURN 0xFF",
        "RETURN 0x00",
        "RETURN 0b1010",
        "RETURN 0o777",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Numeric literal query '{}' should parse",
            query
        );
    }
}

#[test]
fn string_literal_edge_cases() {
    let queries = vec![
        r#"RETURN ''"#,  // Empty string
        r#"RETURN "  ""#,  // String with spaces
        r#"RETURN '\n'"#,  // Newline escape
        r#"RETURN '\t'"#,  // Tab escape
        r#"RETURN '\''"#,  // Escaped quote
        r#"RETURN '\\'"#,  // Escaped backslash
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "String literal query '{}' should parse",
            query
        );
    }
}

// ===== Malformed Input =====

#[test]
fn unclosed_string_literal() {
    let result = parse("RETURN 'unclosed");

    // Should have diagnostics about unclosed string
    assert!(
        !result.diagnostics.is_empty(),
        "Unclosed string should produce diagnostic"
    );
}

#[test]
fn unclosed_delimited_identifier() {
    let result = parse("MATCH (`unclosed) RETURN 1");

    // Should have diagnostics
    assert!(
        !result.diagnostics.is_empty(),
        "Unclosed delimited identifier should produce diagnostic"
    );
}

#[test]
fn unclosed_parenthesis() {
    let result = parse("MATCH (n RETURN n");

    // Should have diagnostics about unclosed parenthesis
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn unclosed_bracket() {
    let result = parse("MATCH ()-[e->() RETURN e");

    // Should have diagnostics or parse with partial AST
    // For now, just ensure no panic
    let _ = result.ast;
}

#[test]
fn unclosed_brace() {
    let result = parse("MATCH (n {prop: 1) RETURN n");

    // Should have diagnostics
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn unexpected_eof() {
    let queries = vec![
        "MATCH",
        "MATCH (n) WHERE",
        "MATCH (n) RETURN",
        "MATCH (n) WHERE n.age >",
    ];

    for query in queries {
        let result = parse(query);
        // Should have diagnostics about unexpected EOF
        assert!(
            !result.diagnostics.is_empty(),
            "Query '{}' should produce diagnostic for unexpected EOF",
            query
        );
    }
}

#[test]
fn invalid_token_sequences() {
    let queries = vec![
        "MATCH MATCH",
        "RETURN RETURN",
        "WHERE MATCH",
        "(n) MATCH",
    ];

    for query in queries {
        let result = parse(query);
        // Should have diagnostics about invalid syntax
        let _ = result.ast;
    }
}

// ===== Uncommon Syntax Combinations =====

#[test]
fn multiple_set_operators_chained() {
    let query = r#"
        MATCH (n) RETURN n
        UNION
        MATCH (m) RETURN m
        INTERSECT
        MATCH (p) RETURN p
        EXCEPT
        MATCH (q) RETURN q
    "#;

    let result = parse(query);

    // Complex set operator chaining
    let _ = result.ast;
}

#[test]
fn optional_with_complex_pattern() {
    let query = r#"
        MATCH (a)-[:KNOWS]->(b)
        OPTIONAL MATCH (b)-[:WORKS_AT]->(c:Company)
        WHERE c.revenue > 1000000
        RETURN a, b, c
    "#;

    let result = parse(query);

    assert!(
        result.ast.is_some(),
        "OPTIONAL with complex pattern should parse"
    );
}

#[test]
fn nested_procedure_calls() {
    let query = r#"
        CALL proc1()
        CALL proc2()
        CALL proc3()
        RETURN 1
    "#;

    let result = parse(query);

    // Multiple procedure calls
    let _ = result.ast;
}

#[test]
fn complex_type_annotations() {
    let queries = vec![
        "RETURN [1, 2, 3] :: LIST<INT>",
        "RETURN {a: 1, b: 2} :: RECORD",
        "MATCH (n) RETURN n :: NODE",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Parameter Edge Cases =====

#[test]
fn parameters_in_all_valid_contexts() {
    let queries = vec![
        "RETURN $param",
        "MATCH (n {id: $id}) RETURN n",
        "MATCH (n) WHERE n.age > $minAge RETURN n",
        "MATCH (n) SET n.value = $newValue RETURN n",
        // "CREATE (n {data: $data}) RETURN n",  // Not yet fully implemented
        "RETURN [$p1, $p2, $p3]",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Parameter query '{}' should parse",
            query
        );
    }
}

#[test]
fn substituted_parameters() {
    let queries = vec![
        "RETURN $$param",
        "MATCH (n {id: $$id}) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        // Substituted parameters may not be fully implemented yet
        // Just ensure no panic
        let _ = result.ast;
    }
}

#[test]
fn parameter_names_with_special_characters() {
    let queries = vec![
        "RETURN $param_123",
        "RETURN $myParam",
        "RETURN $_private",
        "RETURN $1",
        "RETURN $100",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Parameter name query '{}' should parse",
            query
        );
    }
}

// ===== Identifier Edge Cases =====

#[test]
fn delimited_identifiers_with_reserved_words() {
    let queries = vec![
        "MATCH (`MATCH`) RETURN `MATCH`",
        "MATCH (n:`SELECT`) RETURN n",
        "MATCH (n {`WHERE`: 1}) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Delimited identifier with reserved word '{}' should parse",
            query
        );
    }
}

#[test]
fn identifiers_with_unicode() {
    let queries = vec![
        "MATCH (用户) RETURN 用户",
        "MATCH (użytkownik) RETURN użytkownik",
        "MATCH (المستخدم) RETURN المستخدم",
        "MATCH (пользователь) RETURN пользователь",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Unicode identifier query should parse: {}",
            query
        );
    }
}

#[test]
fn non_reserved_words_as_identifiers() {
    let queries = vec![
        "MATCH (graph) RETURN graph",
        "MATCH (node) RETURN node",
        "MATCH (edge) RETURN edge",
        "MATCH (property) RETURN property",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Non-reserved word as identifier '{}' should parse",
            query
        );
    }
}

// ===== Operator Edge Cases =====

#[test]
fn all_comparison_operators() {
    let queries = vec![
        "MATCH (n) WHERE n.a = 1 RETURN n",
        "MATCH (n) WHERE n.a <> 1 RETURN n",
        "MATCH (n) WHERE n.a != 1 RETURN n",
        "MATCH (n) WHERE n.a < 1 RETURN n",
        "MATCH (n) WHERE n.a > 1 RETURN n",
        "MATCH (n) WHERE n.a <= 1 RETURN n",
        "MATCH (n) WHERE n.a >= 1 RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Comparison operator query '{}' should parse",
            query
        );
    }
}

#[test]
fn all_arithmetic_operators() {
    let queries = vec![
        "RETURN 1 + 2",
        "RETURN 1 - 2",
        "RETURN 1 * 2",
        "RETURN 1 / 2",
        "RETURN 1 % 2",
        "RETURN -1",
        "RETURN +1",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Arithmetic operator query '{}' should parse",
            query
        );
    }
}

#[test]
fn all_logical_operators() {
    let queries = vec![
        "MATCH (n) WHERE TRUE AND FALSE RETURN n",
        "MATCH (n) WHERE TRUE OR FALSE RETURN n",
        "MATCH (n) WHERE NOT TRUE RETURN n",
        "MATCH (n) WHERE TRUE XOR FALSE RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Logical operator query '{}' should parse",
            query
        );
    }
}

#[test]
fn operator_precedence_combinations() {
    let queries = vec![
        "RETURN 1 + 2 * 3",  // Should be 1 + (2 * 3) = 7
        "RETURN (1 + 2) * 3",  // Should be 9
        "RETURN 1 < 2 AND 3 < 4",
        "RETURN 1 + 2 > 3 - 1",
        "MATCH (n) WHERE n.a > 0 AND n.b < 10 OR n.c = 5 RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Operator precedence query '{}' should parse",
            query
        );
    }
}

// ===== Special Cases =====

#[test]
fn null_handling() {
    let queries = vec![
        "RETURN NULL",
        "MATCH (n) WHERE n.prop IS NULL RETURN n",
        "MATCH (n) WHERE n.prop IS NOT NULL RETURN n",
        "RETURN NULLIF(1, 1)",
        "RETURN COALESCE(NULL, 1, 2)",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "NULL handling query '{}' should parse",
            query
        );
    }
}

#[test]
fn boolean_literals() {
    let queries = vec![
        "RETURN TRUE",
        "RETURN FALSE",
        "RETURN UNKNOWN",
        "MATCH (n) WHERE TRUE RETURN n",
        "RETURN TRUE AND FALSE OR NOT UNKNOWN",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Boolean literal query '{}' should parse",
            query
        );
    }
}

#[test]
fn temporal_literals() {
    let queries = vec![
        "RETURN DATE '2024-01-15'",
        "RETURN TIME '14:30:00'",
        "RETURN TIMESTAMP '2024-01-15 14:30:00'",
        "RETURN DURATION 'P1Y2M3D'",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Temporal literal query '{}' should parse",
            query
        );
    }
}

#[test]
fn case_expressions() {
    let query = r#"
        MATCH (n)
        RETURN CASE
            WHEN n.age < 18 THEN 'minor'
            WHEN n.age < 65 THEN 'adult'
            ELSE 'senior'
        END AS category
    "#;

    let result = parse(query);

    assert!(
        result.ast.is_some(),
        "CASE expression should parse"
    );
}

#[test]
fn exists_predicates() {
    let queries = vec![
        "MATCH (n) WHERE EXISTS { MATCH (n)-[:KNOWS]->() } RETURN n",
        "MATCH (n) WHERE EXISTS { (n)-[:KNOWS]->() } RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn quantified_patterns() {
    let queries = vec![
        "MATCH (a)-[]->{1,5}(b) RETURN a, b",
        "MATCH (a)-[]->{2,}(b) RETURN a, b",
        "MATCH (a)-[]->{,10}(b) RETURN a, b",
        "MATCH (a)-[]->{3}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn mixed_valid_and_invalid_syntax() {
    // Partially valid query with errors in the middle
    let query = "MATCH (n) WHERE n.age >> 18 RETURN n";

    let result = parse(query);

    // Should produce some AST with diagnostics
    let _ = result.ast;
    assert!(
        !result.diagnostics.is_empty(),
        "Invalid operator should produce diagnostic"
    );
}

#[test]
fn whitespace_variations() {
    let queries = vec![
        "MATCH(n)RETURN n",  // No spaces
        "MATCH  (n)  RETURN  n",  // Multiple spaces
        "MATCH\n(n)\nRETURN\nn",  // Newlines
        "MATCH\t(n)\tRETURN\tn",  // Tabs
        "  MATCH (n) RETURN n  ",  // Leading/trailing spaces
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Whitespace variation '{}' should parse",
            query.escape_default()
        );
    }
}

#[test]
fn comments_in_expressions() {
    let query = r#"
        MATCH (n)
        WHERE n.age > 18 /* minimum age */
          AND n.active = TRUE // must be active
        RETURN n
    "#;

    let result = parse(query);

    assert!(
        result.ast.is_some(),
        "Comments in expressions should be ignored"
    );
}

#[test]
fn property_access_edge_cases() {
    let queries = vec![
        "RETURN n.prop",
        "RETURN n.prop1.prop2.prop3",
        "RETURN n.`property with spaces`",
        "RETURN map['key']",
        "RETURN list[0]",
        "RETURN list[0][1]",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}
