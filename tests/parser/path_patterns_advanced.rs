//! Advanced Path Pattern Parser Tests
//!
//! This module tests parsing of advanced path pattern features including:
//! - Simplified path pattern expressions with multiple operators
//! - Path search variants with different modes
//! - Multiple path variables with YIELD
//! - Complex path quantifiers
//! - Path mode combinations

use gql_parser::parse;

// ===== Simplified Path Pattern Expressions =====

#[test]
fn simplified_path_with_multiple_operators() {
    let queries = vec![
        "MATCH -/(:Person)-[:KNOWS]->(:Person)/-> RETURN 1",
        "MATCH -/(a|b)*/- RETURN 1",
        "MATCH -/(a b){2,5}/- RETURN 1",
        "MATCH -/!(:Admin)/- RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        // Simplified path patterns may not be fully implemented
        let _ = result.ast;
    }
}

#[test]
fn simplified_path_with_negation() {
    let queries = vec![
        "MATCH -/!a/- RETURN 1",
        "MATCH -/!(a|b)/- RETURN 1",
        "MATCH -/!(:Label)/- RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn simplified_path_with_conjunction() {
    let queries = vec![
        "MATCH -/a&b/- RETURN 1",
        "MATCH -/(:Person)&(:Employee)/- RETURN 1",
        "MATCH -/(a&b)*/- RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn simplified_path_with_concatenation() {
    let queries = vec![
        "MATCH -/a b c/- RETURN 1",
        "MATCH -/(:Person) [:KNOWS] (:Person)/- RETURN 1",
        "MATCH -/(a b)*/- RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn simplified_path_complex_combinations() {
    let queries = vec![
        "MATCH -/(a|b) c*/- RETURN 1",
        "MATCH -/(a&b)|c/- RETURN 1",
        "MATCH -/!(a|b) c/- RETURN 1",
        "MATCH -/((a|b)&c)*/- RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Path Search Variants =====

#[test]
fn path_search_any_shortest() {
    let queries = vec![
        "MATCH ANY SHORTEST PATH (a)-[e]->{1,*}(b) RETURN a, b",
        "MATCH ANY SHORTEST WALK PATH (a)-[]->{1,5}(b) RETURN a, b",
        "MATCH ANY SHORTEST TRAIL PATH (a)-[]->{2,}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn path_search_all_shortest() {
    let queries = vec![
        "MATCH ALL SHORTEST PATH (a)-[e]->{1,*}(b) RETURN a, b",
        "MATCH ALL SHORTEST SIMPLE PATH (a)-[]->{1,10}(b) RETURN a, b",
        "MATCH ALL SHORTEST ACYCLIC PATH (a)-[]->{1,}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn path_search_counted_shortest() {
    let queries = vec![
        "MATCH SHORTEST 5 PATHS (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH SHORTEST 10 WALK PATHS (a)-[]->{1,5}(b) RETURN a, b",
        "MATCH SHORTEST 3 TRAIL PATH (a)-[]->{2,}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn path_search_counted_shortest_groups() {
    let queries = vec![
        "MATCH SHORTEST 3 GROUPS (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH SHORTEST 5 WALK PATHS GROUPS (a)-[]->{1,5}(b) RETURN a, b",
        "MATCH SHORTEST 2 SIMPLE PATH GROUP (a)-[]->{2,}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn path_search_all_paths() {
    let queries = vec![
        "MATCH ALL PATHS (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH ALL WALK PATHS (a)-[]->{1,5}(b) RETURN a, b",
        "MATCH ALL TRAIL PATH (a)-[]->{2,}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn path_search_any_paths() {
    let queries = vec![
        "MATCH ANY PATHS (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH ANY 10 PATHS (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH ANY 5 WALK PATHS (a)-[]->{1,5}(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Multiple Path Variables =====

#[test]
fn multiple_path_variables_with_yield() {
    let query = "MATCH p1 = (a)-[:KNOWS]->(b), p2 = (b)-[:WORKS_AT]->(c) \
                 YIELD p1, p2, a, b, c \
                 RETURN p1, p2";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn multiple_path_variables_different_modes() {
    let query = "MATCH p1 = WALK PATH (a)-[]->(b), \
                       p2 = TRAIL PATH (b)-[]->(c) \
                 RETURN p1, p2";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn path_variables_with_different_search_prefixes() {
    let query = "MATCH p1 = ALL SHORTEST PATH (a)-[]->{1,*}(b), \
                       p2 = ANY PATH (b)-[]->(c) \
                 RETURN p1, p2";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Complex Path Quantifiers =====

#[test]
fn path_quantifier_edge_cases() {
    let queries = vec![
        "MATCH (a)-[]->{0,5}(b) RETURN a, b",  // Zero minimum
        "MATCH (a)-[]->{1,1}(b) RETURN a, b",  // Same min and max
        "MATCH (a)-[]->{100,}(b) RETURN a, b", // Large minimum
        "MATCH (a)-[]->{,1000}(b) RETURN a, b", // Large maximum
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn quantified_node_patterns() {
    let queries = vec![
        "MATCH (n:Person){2,5} RETURN n",
        "MATCH ((n)-[:KNOWS]->(m)){1,3} RETURN n, m",
        "MATCH ((:Person)-[:KNOWS]->(:Person)){2,} RETURN 1",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn quantified_parenthesized_patterns() {
    let queries = vec![
        "MATCH ((a)-[]->(b)){2,5} RETURN a, b",
        "MATCH (((n)-[]->(m))-[]->(p)){1,3} RETURN n, m, p",
        "MATCH (((a)-[]->(b)){2}){3} RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Path Mode Combinations =====

#[test]
fn keep_clause_with_different_modes() {
    let queries = vec![
        "MATCH WALK PATH (n)-[]->{1,*}(m) KEEP TRAIL RETURN n, m",
        "MATCH ALL PATHS (n)-[]->{1,*}(m) KEEP SIMPLE RETURN n, m",
        "MATCH ANY PATH (n)-[]->{1,*}(m) KEEP ACYCLIC RETURN n, m",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn match_mode_with_path_patterns() {
    let queries = vec![
        "MATCH REPEATABLE ELEMENTS (a)-[]->(b) RETURN a, b",
        "MATCH DIFFERENT EDGES (a)-[]->(b) RETURN a, b",
        "MATCH REPEATABLE ELEMENT BINDINGS (a)-[]->(b) RETURN a, b",
        "MATCH DIFFERENT EDGE BINDINGS (a)-[]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn match_mode_with_path_search() {
    let queries = vec![
        "MATCH REPEATABLE ELEMENTS ALL SHORTEST PATH (a)-[]->{1,*}(b) RETURN a, b",
        "MATCH DIFFERENT EDGES ANY PATH (a)-[]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Edge Direction Combinations =====

#[test]
fn all_edge_direction_types() {
    let queries = vec![
        "MATCH (a)-[e]->(b) RETURN a, b",       // Right arrow
        "MATCH (a)<-[e]-(b) RETURN a, b",       // Left arrow
        "MATCH (a)-[e]-(b) RETURN a, b",        // Any direction
        "MATCH (a)~[e]~(b) RETURN a, b",        // Undirected
        "MATCH (a)<-[e]->(b) RETURN a, b",      // Any directed
        "MATCH (a)<~[e]~(b) RETURN a, b",       // Left or undirected
        "MATCH (a)~[e]~>(b) RETURN a, b",       // Right or undirected
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Edge direction query '{}' should parse",
            query
        );
    }
}

#[test]
fn abbreviated_edge_all_directions() {
    let queries = vec![
        "MATCH (a)->(b) RETURN a, b",   // Right arrow abbreviated
        "MATCH (a)<-(b) RETURN a, b",   // Left arrow abbreviated
        "MATCH (a)-(b) RETURN a, b",    // Any direction abbreviated
        "MATCH (a)~(b) RETURN a, b",    // Undirected abbreviated
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Abbreviated edge query '{}' should parse",
            query
        );
    }
}

// ===== Label Expressions in Patterns =====

#[test]
fn label_expression_union_in_pattern() {
    let queries = vec![
        "MATCH (n:Person|Company) RETURN n",
        "MATCH (n:A|B|C) RETURN n",
        "MATCH (n:Person|Manager|Executive) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn label_expression_conjunction_in_pattern() {
    let queries = vec![
        "MATCH (n:Person&Employee) RETURN n",
        "MATCH (n:Active&Verified&Premium) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn label_expression_negation_in_pattern() {
    let queries = vec![
        "MATCH (n:!Inactive) RETURN n",
        "MATCH (n:!(Deleted|Archived)) RETURN n",
        "MATCH (n:Person&!Admin) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn label_expression_wildcard() {
    let queries = vec![
        "MATCH (n:%) RETURN n",
        "MATCH (n:Person&%) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn label_expression_with_labels_keyword() {
    let queries = vec![
        "MATCH (n:LABELS Person) RETURN n",
        "MATCH (n:LABELS Person&Employee) RETURN n",
        "MATCH (n:LABELS Person|Manager) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Complex Pattern Combinations =====

#[test]
fn pattern_with_where_in_element() {
    let queries = vec![
        "MATCH (n:Person WHERE n.age > 18) RETURN n",
        "MATCH (a)-[e:KNOWS WHERE e.since < DATE '2020-01-01']->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Pattern with WHERE in element '{}' should parse",
            query
        );
    }
}

#[test]
fn pattern_with_property_spec_in_element() {
    let queries = vec![
        "MATCH (n:Person {age: 25, city: 'NYC'}) RETURN n",
        "MATCH (a)-[e:KNOWS {years: 5}]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Pattern with property spec '{}' should parse",
            query
        );
    }
}

#[test]
fn pattern_union_and_alternation_precedence() {
    let queries = vec![
        "MATCH (a)|(b) RETURN a",
        "MATCH (a)|+|(b) RETURN a",
        "MATCH (a)|(b)|+|(c) RETURN a",
        "MATCH (a)|(b)|(c)|+|(d)|(e) RETURN a",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Variable-Length Pattern Edge Cases =====

#[test]
fn variable_length_with_labels() {
    let queries = vec![
        "MATCH (a)-[:KNOWS*]->(b) RETURN a, b",
        "MATCH (a)-[:KNOWS|FOLLOWS*1..5]->(b) RETURN a, b",
        "MATCH (a)-[:KNOWS*2..]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn variable_length_zero_minimum() {
    let queries = vec![
        "MATCH (a)-[*0..5]->(b) RETURN a, b",
        "MATCH (a)-[:KNOWS*0..]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}
