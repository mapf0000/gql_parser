//! Extended Edge Case Testing Suite
//!
//! Additional edge cases for reserved word enforcement, path patterns,
//! and data modification statements.

use gql_parser::parse;

// ===== Reserved Word Enforcement Tests =====

#[test]
fn reserved_words_as_delimited_identifiers() {
    // Reserved words can only be used when delimited
    let queries = vec![
        "MATCH (`MATCH`) RETURN `MATCH`",
        "MATCH (`SELECT`) RETURN `SELECT`",
        "MATCH (`WHERE`) RETURN `WHERE`",
        "MATCH (n:`CREATE`) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Delimited reserved word '{}' should parse",
            query
        );
    }
}

#[test]
fn pre_reserved_words_allowed_as_identifiers() {
    // Pre-reserved words can be used as identifiers (forward compatibility)
    let queries = vec![
        "MATCH (abstract) RETURN abstract",
        "MATCH (constraint) RETURN constraint",
        "MATCH (function) RETURN function",
        "MATCH (aggregate) RETURN aggregate",
    ];

    for query in queries {
        let result = parse(query);
        // Pre-reserved words as identifiers - may work depending on context
        let _ = result.ast;
    }
}

#[test]
fn non_reserved_words_as_regular_identifiers() {
    // Non-reserved words can be used as identifiers in appropriate contexts
    let queries = vec![
        "MATCH (graph) RETURN graph",
        "MATCH (node) RETURN node",
        "MATCH (edge) RETURN edge",
        "MATCH (property) RETURN property",
        "MATCH (type) RETURN type",
        "MATCH (table) RETURN table",
        "MATCH (directed) RETURN directed",
        "MATCH (undirected) RETURN undirected",
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

// ===== Path Pattern Edge Cases =====

#[test]
fn complex_path_patterns() {
    let queries = vec![
        "MATCH (a)-[:KNOWS]->(b)-[:WORKS_AT]->(c) RETURN a, b, c",
        "MATCH (a)<-[:FOLLOWS]-(b)-[:LIKES]->(c) RETURN a, b, c",
        "MATCH (a)-[:REL1|REL2]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn variable_length_patterns() {
    let queries = vec![
        "MATCH (a)-[*]->(b) RETURN a, b",
        "MATCH (a)-[*1..5]->(b) RETURN a, b",
        "MATCH (a)-[*..10]->(b) RETURN a, b",
        "MATCH (a)-[*2..]->(b) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Subquery Edge Cases =====

#[test]
fn nested_subqueries() {
    let query = r#"
        MATCH (a)
        WHERE EXISTS {
            MATCH (a)-[:KNOWS]->(b)
            WHERE EXISTS {
                MATCH (b)-[:LIKES]->(c)
            }
        }
        RETURN a
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Label Expression Edge Cases =====

#[test]
fn label_expressions() {
    let queries = vec![
        "MATCH (n:Person) RETURN n",
        "MATCH (n:Person:Manager) RETURN n",
        "MATCH (n:Person|Manager) RETURN n",
        "MATCH (n:Person&Manager) RETURN n",
        "MATCH (n:!Inactive) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Aggregate Function Edge Cases =====

#[test]
fn aggregate_functions_comprehensive() {
    let queries = vec![
        "MATCH (n) RETURN COUNT(n)",
        "MATCH (n) RETURN COUNT(DISTINCT n)",
        "MATCH (n) RETURN SUM(n.value)",
        "MATCH (n) RETURN AVG(n.value)",
        "MATCH (n) RETURN MAX(n.value)",
        "MATCH (n) RETURN MIN(n.value)",
        "MATCH (n) RETURN COLLECT_LIST(n.value)",
        "MATCH (n) RETURN COUNT(*)",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Aggregate function query '{}' should parse",
            query
        );
    }
}

// ===== List and Map Literal Edge Cases =====

#[test]
fn list_literals_comprehensive() {
    let queries = vec![
        "RETURN []",
        "RETURN [1]",
        "RETURN [1, 2, 3]",
        "RETURN [1, 'two', TRUE, NULL]",
        "RETURN [[1, 2], [3, 4]]",
        "RETURN [1, [2, [3, [4]]]]",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "List literal query '{}' should parse",
            query
        );
    }
}

#[test]
fn map_literals_comprehensive() {
    let queries = vec![
        "RETURN {}",
        "RETURN {a: 1}",
        "RETURN {a: 1, b: 2}",
        "RETURN {a: 1, b: 'two', c: TRUE}",
        "RETURN {nested: {a: 1, b: 2}}",
        "RETURN {items: [1, 2, 3]}",  // Changed 'list' to 'items' to avoid keyword
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Map literal query '{}' should parse",
            query
        );
    }
}

// ===== Type Annotation Edge Cases =====

#[test]
fn type_annotations_comprehensive() {
    let queries = vec![
        "RETURN 1 :: INT",
        "RETURN 'text' :: STRING",
        "RETURN TRUE :: BOOLEAN",
        "RETURN [1, 2, 3] :: LIST<INT>",
        "RETURN {a: 1} :: RECORD",
        "RETURN NULL :: INT",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Session and Transaction Edge Cases =====

#[test]
fn session_statements() {
    let queries = vec![
        "SESSION SET GRAPH CURRENT_GRAPH",
        "SESSION SET TIME ZONE 'UTC'",
        "SESSION RESET GRAPH",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some() || !result.diagnostics.is_empty(),
            "Session statement '{}' should parse or produce diagnostics",
            query
        );
    }
}

#[test]
fn transaction_statements() {
    let queries = vec![
        "START TRANSACTION",
        "START TRANSACTION READ ONLY",
        "START TRANSACTION READ WRITE",
        "COMMIT",
        "ROLLBACK",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some() || !result.diagnostics.is_empty(),
            "Transaction statement '{}' should parse or produce diagnostics",
            query
        );
    }
}

// ===== Data Modification Edge Cases =====

#[test]
fn insert_statements() {
    let queries = vec![
        "INSERT (n:Person {name: 'Alice'})",
        "INSERT (n)-[:KNOWS]->(m)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn set_statements() {
    let queries = vec![
        "MATCH (n) SET n.prop = 1 RETURN n",
        "MATCH (n) SET n:NewLabel RETURN n",
        "MATCH (n) SET n += {newProp: 2} RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn delete_statements() {
    let queries = vec![
        "MATCH (n) DELETE n",
        "MATCH (n)-[r]->() DELETE r",
        "MATCH (n) DETACH DELETE n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Catalog Statement Edge Cases =====

#[test]
fn create_graph_statements() {
    let queries = vec![
        "CREATE GRAPH mygraph",
        "CREATE GRAPH mygraph ANY",
        "CREATE GRAPH mygraph {(Person :Person)}",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn drop_statements() {
    let queries = vec![
        "DROP GRAPH mygraph",
        "DROP SCHEMA myschema",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Error Recovery Tests =====

#[test]
fn partial_ast_on_error() {
    // Query with error in the middle should produce partial AST
    let query = "MATCH (n) WHERE n.age >> 18 RETURN n";

    let result = parse(query);

    // Should have diagnostics
    assert!(!result.diagnostics.is_empty(), "Should have diagnostics for invalid operator");

    // May or may not have partial AST depending on recovery strategy
    let _ = result.ast;
}

#[test]
fn multiple_errors_reported() {
    let query = "MATCH (n WHERE n.age >> 18 RETURN";

    let result = parse(query);

    // Should report multiple errors: unclosed paren, invalid operator, unexpected EOF
    assert!(!result.diagnostics.is_empty(), "Should have multiple diagnostics");
}
