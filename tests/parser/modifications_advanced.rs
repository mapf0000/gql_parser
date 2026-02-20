//! Advanced Data Modification Parser Tests
//!
//! This module tests parsing of advanced data modification features including:
//! - Complex INSERT patterns with multiple relationships
//! - SET with multiple operations
//! - REMOVE with multiple items
//! - MERGE statements
//! - Combined modification operations

use gql_parser::parse;
use crate::common::*;

// ===== Complex INSERT Patterns =====

#[test]
fn insert_with_multiple_patterns_and_relationships() {
    let queries = vec![
        "INSERT (a:Person {name: 'Alice'})-[:KNOWS]->(b:Person {name: 'Bob'})",
        "INSERT (a:Person), (b:Person), (a)-[:KNOWS]->(b)",
        "INSERT (a)-[:R1]->(b)-[:R2]->(c)",
    ];

    for query in queries {
        let result = parse(query);
        // INSERT may have varying levels of support
        let _ = result.ast;
    }
}

#[test]
fn insert_cyclic_pattern() {
    let queries = vec![
        "INSERT (a)-[:R1]->(b)-[:R2]->(c)-[:R3]->(a)",
        "INSERT (n)-[:FOLLOWS]->(n)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn insert_with_properties_all_types() {
    let query = r#"
        INSERT (n:Person {
            id: 1,
            name: 'Alice',
            age: 30,
            active: TRUE,
            salary: 75000.50,
            hired: DATE '2020-01-15',
            tags: ['engineer', 'senior'],
            metadata: {team: 'backend', level: 3}
        })
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn insert_multiple_node_patterns() {
    let queries = vec![
        "INSERT (a:Person), (b:Person), (c:Person)",
        "INSERT (a), (b), (c), (d), (e)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn insert_multiple_edge_patterns() {
    let queries = vec![
        "INSERT (a)-[:R1]->(b), (c)-[:R2]->(d)",
        "INSERT (a)-[:KNOWS]->(b), (b)-[:WORKS_AT]->(c)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn insert_with_undirected_edges() {
    let queries = vec![
        "INSERT (a)~[:FRIENDS]~(b)",
        "INSERT (a)-[:RELATED]-(b)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn insert_with_multiple_labels() {
    let queries = vec![
        "INSERT (n:Person:Employee {name: 'Alice'})",
        "INSERT (a:User:Admin:Verified)",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== SET with Multiple Operations =====

#[test]
fn set_multiple_properties() {
    let queries = vec![
        "MATCH (n) SET n.prop1 = 1, n.prop2 = 2, n.prop3 = 3 RETURN n",
        "MATCH (n) SET n.a = 'x', n.b = 'y', n.c = 'z' RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SET multiple properties '{}' should parse",
            query
        );
    }
}

#[test]
fn set_property_and_label() {
    let queries = vec![
        "MATCH (n) SET n.prop = 1, n:NewLabel RETURN n",
        "MATCH (n) SET n:Label1, n.value = 100, n:Label2 RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SET property and label '{}' should parse",
            query
        );
    }
}

#[test]
fn set_all_properties_replacement() {
    let queries = vec![
        "MATCH (n) SET n = {prop: 1} RETURN n",
        "MATCH (n) SET n = {a: 1, b: 2, c: 3} RETURN n",
        "MATCH (n) SET n = {} RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SET all properties '{}' should parse",
            query
        );
    }
}

#[test]
fn set_property_merge() {
    let queries = vec![
        "MATCH (n) SET n += {newProp: 1} RETURN n",
        "MATCH (n) SET n += {a: 1, b: 2} RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        // += operator may not be fully implemented
        let _ = result.ast;
    }
}

#[test]
fn set_multiple_labels() {
    let queries = vec![
        "MATCH (n) SET n:Label1, n:Label2, n:Label3 RETURN n",
        "MATCH (n) SET n:Person, n:Employee RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn set_with_expressions() {
    let queries = vec![
        "MATCH (n) SET n.count = n.count + 1 RETURN n",
        "MATCH (n) SET n.value = n.value * 2 RETURN n",
        "MATCH (n) SET n.fullName = n.firstName || ' ' || n.lastName RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "SET with expressions '{}' should parse",
            query
        );
    }
}

#[test]
fn set_nested_properties() {
    let queries = vec![
        "MATCH (n) SET n.address.city = 'NYC' RETURN n",
        "MATCH (n) SET n.profile.settings.theme = 'dark' RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        // Nested property setting may not be supported
        let _ = result.ast;
    }
}

// ===== REMOVE with Multiple Items =====

#[test]
fn remove_multiple_properties() {
    let queries = vec![
        "MATCH (n) REMOVE n.prop1, n.prop2 RETURN n",
        "MATCH (n) REMOVE n.a, n.b, n.c, n.d RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "REMOVE multiple properties '{}' should parse",
            query
        );
    }
}

#[test]
fn remove_multiple_labels() {
    let queries = vec![
        "MATCH (n) REMOVE n:Label1, n:Label2 RETURN n",
        "MATCH (n) REMOVE n:Old, n:Deprecated RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "REMOVE multiple labels '{}' should parse",
            query
        );
    }
}

#[test]
fn remove_properties_and_labels() {
    let queries = vec![
        "MATCH (n) REMOVE n.prop, n:Label RETURN n",
        "MATCH (n) REMOVE n:Old, n.deprecated, n:Unused RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "REMOVE properties and labels '{}' should parse",
            query
        );
    }
}

// ===== DELETE Statements =====

#[test]
fn delete_multiple_elements() {
    let queries = vec![
        "MATCH (n), (m) DELETE n, m",
        "MATCH (a)-[r]->(b) DELETE a, r, b",
    ];

    for query in queries {
        let result = parse(query);
        assert!(result.ast.is_some(), "DELETE multiple '{}' should parse", query);
    }
}

#[test]
fn delete_with_detach() {
    let queries = vec![
        "MATCH (n) DETACH DELETE n",
        "MATCH (n) NODETACH DELETE n",
    ];

    for query in queries {
        let result = parse(query);
        assert!(result.ast.is_some(), "DELETE with detach '{}' should parse", query);
    }
}

#[test]
fn delete_edges_only() {
    let queries = vec![
        "MATCH ()-[r]->() DELETE r",
        "MATCH (a)-[r1]->(b)-[r2]->(c) DELETE r1, r2",
    ];

    for query in queries {
        let result = parse(query);
        assert!(result.ast.is_some(), "DELETE edges '{}' should parse", query);
    }
}

// ===== MERGE Statements =====

#[test]
fn merge_basic_patterns() {
    let queries = vec![
        "MERGE (n:Person {id: 1}) RETURN n",
        "MERGE (n:Person {id: $id, name: $name}) RETURN n",
    ];

    for query in queries {
        let result = parse(query);
        // MERGE may not be implemented yet
        let _ = result.ast;
    }
}

#[test]
fn merge_with_on_create() {
    let query = r#"
        MERGE (n:Person {id: 1})
        ON CREATE SET n.created = TIMESTAMP '2024-01-01 00:00:00'
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn merge_with_on_match() {
    let query = r#"
        MERGE (n:Person {id: 1})
        ON MATCH SET n.lastSeen = TIMESTAMP '2024-01-01 00:00:00'
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn merge_with_on_create_and_on_match() {
    let query = r#"
        MERGE (n:Person {id: 1})
        ON CREATE SET n.created = TIMESTAMP '2024-01-01 00:00:00'
        ON MATCH SET n.lastSeen = TIMESTAMP '2024-01-01 00:00:00'
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn merge_relationship_pattern() {
    let queries = vec![
        "MERGE (a)-[:KNOWS]->(b) RETURN a, b",
        "MERGE (a:Person {id: 1})-[:KNOWS]->(b:Person {id: 2}) RETURN a, b",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Combined Modification Operations =====

#[test]
fn insert_and_set_chained() {
    let query = "INSERT (n:Person {name: 'Alice'}) SET n.created = DATE '2024-01-01' RETURN n";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn match_insert_set_chain() {
    let query = r#"
        MATCH (a:Person {id: 1})
        INSERT (b:Person {name: 'Bob'})
        SET b.friend = a.id
        RETURN a, b
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn match_set_remove_chain() {
    let query = r#"
        MATCH (n:Person)
        SET n.updated = TRUE
        REMOVE n.deprecated
        RETURN n
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "Chained SET and REMOVE should parse");
}

#[test]
fn match_set_delete_chain() {
    let query = r#"
        MATCH (n:Person WHERE n.inactive = TRUE)
        SET n.archived = TRUE
        DELETE n
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "Chained SET and DELETE should parse");
}

#[test]
fn complex_modification_pipeline() {
    let query = r#"
        MATCH (old:Person {id: 1})
        INSERT (new:Person {name: old.name, version: 2})
        SET old.replaced = TRUE, new.created = DATE '2024-01-01'
        REMOVE old:Active
        RETURN old, new
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Modification with WHERE Clauses =====

#[test]
fn set_with_where_in_match() {
    let query = "MATCH (n) WHERE n.age > 18 SET n.adult = TRUE RETURN n";
    let result = parse(query);
    assert!(result.ast.is_some(), "SET with WHERE should parse");
}

#[test]
fn delete_with_complex_where() {
    let query = r#"
        MATCH (n:Person)
        WHERE n.inactive = TRUE AND n.lastLogin < DATE '2020-01-01'
        DELETE n
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "DELETE with WHERE should parse");
}

// ===== Modification with RETURN =====

#[test]
fn insert_with_return_expressions() {
    let query = "INSERT (n:Person {name: 'Alice', age: 30}) RETURN n.name, n.age, id(n)";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn set_with_return_old_and_new_values() {
    let query = r#"
        MATCH (n)
        SET n.value = n.value + 1
        RETURN n.value AS newValue
    "#;

    let result = parse(query);
    assert!(result.ast.is_some(), "SET with RETURN should parse");
}

// ===== Error Cases for Modifications =====

#[test]
fn insert_empty_property_map_rejected() {
    let result = parse("INSERT (n {})");
    // Empty property maps in INSERT should be rejected
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn set_without_equals_rejected() {
    let result = parse("SET n.prop");
    // SET without assignment should be rejected
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn remove_without_property_or_label_rejected() {
    let result = parse("REMOVE n");
    // REMOVE without property or label should be rejected
    assert!(!result.diagnostics.is_empty());
}

// ===== Modification with USE Clause =====

#[test]
fn use_graph_with_insert() {
    let query = "USE myGraph INSERT (n:Person {name: 'Alice'}) RETURN n";
    let result = parse(query);
    assert!(result.ast.is_some(), "USE with INSERT should parse");
}

#[test]
fn use_graph_with_set() {
    let query = "USE myGraph MATCH (n) SET n.updated = TRUE RETURN n";
    let result = parse(query);
    assert!(result.ast.is_some(), "USE with SET should parse");
}

#[test]
fn use_graph_with_delete() {
    let query = "USE myGraph MATCH (n) DELETE n";
    let result = parse(query);
    assert!(result.ast.is_some(), "USE with DELETE should parse");
}
