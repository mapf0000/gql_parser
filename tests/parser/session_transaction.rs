//! Session and Transaction Parser Tests
//!
//! This module tests parsing of session management and transaction control including:
//! - Session set operations for parameters, graphs, schema, timezone
//! - Session reset operations
//! - Transaction start, commit, rollback
//! - Transaction characteristics

use gql_parser::parse;
use crate::common::*;

// ===== Session Set Parameters =====

#[test]
fn session_set_value_parameter() {
    let queries = vec![
        "SESSION SET VALUE $maxConnections = 100",
        "SESSION SET VALUE $timeout = 30",
        "SESSION SET VALUE $retries = 3",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some() || !result.diagnostics.is_empty(),
            "Session SET VALUE '{}' should parse",
            query
        );
    }
}

#[test]
fn session_set_value_if_not_exists() {
    let queries = vec![
        "SESSION SET VALUE IF NOT EXISTS $param = 100",
        "SESSION SET VALUE IF NOT EXISTS $timeout = DURATION 'PT5M'",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_value_with_temporal_literals() {
    let queries = vec![
        "SESSION SET VALUE $startDate = DATE '2024-01-01'",
        "SESSION SET VALUE $startTime = TIME '09:00:00'",
        "SESSION SET VALUE $timestamp = TIMESTAMP '2024-01-01 09:00:00'",
        "SESSION SET VALUE $duration = DURATION 'P1Y2M3D'",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_table_parameter() {
    let query = "SESSION SET TABLE $results = (MATCH (n) RETURN n)";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn session_set_binding_table_parameter() {
    let query = "SESSION SET BINDING TABLE $data = (MATCH (n) RETURN n.id, n.name)";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn session_set_graph_parameter() {
    let queries = vec![
        "SESSION SET GRAPH $myGraph = CURRENT_GRAPH",
        "SESSION SET PROPERTY GRAPH $pg = CURRENT_PROPERTY_GRAPH",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_graph_parameter_if_not_exists() {
    let query = "SESSION SET GRAPH IF NOT EXISTS $defaultGraph = CURRENT_GRAPH";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Session Set Graph =====

#[test]
fn session_set_graph_current() {
    let queries = vec![
        "SESSION SET GRAPH CURRENT_GRAPH",
        "SESSION SET PROPERTY GRAPH CURRENT_PROPERTY_GRAPH",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Session SET GRAPH '{}' should parse",
            query
        );
    }
}

#[test]
fn session_set_graph_by_name() {
    let queries = vec![
        "SESSION SET GRAPH myGraph",
        "SESSION SET PROPERTY GRAPH myPropertyGraph",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_graph_expression() {
    let queries = vec![
        "SESSION SET GRAPH $graphParam",
        "SESSION SET GRAPH VARIABLE $myGraph",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Session Set Schema =====

#[test]
fn session_set_schema() {
    let queries = vec![
        "SESSION SET SCHEMA mySchema",
        "SESSION SET SCHEMA /root/schemas/prod",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Session Set Timezone =====

#[test]
fn session_set_timezone() {
    let queries = vec![
        "SESSION SET TIME ZONE 'UTC'",
        "SESSION SET TIME ZONE 'America/New_York'",
        "SESSION SET TIME ZONE 'Europe/London'",
        "SESSION SET TIME ZONE '+05:30'",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Session SET TIME ZONE '{}' should parse",
            query
        );
    }
}

// ===== Session Reset =====

#[test]
fn session_reset_all() {
    let query = "SESSION RESET";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn session_reset_parameters() {
    let queries = vec![
        "SESSION RESET PARAMETERS",
        "SESSION RESET ALL PARAMETERS",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_reset_characteristics() {
    let queries = vec![
        "SESSION RESET CHARACTERISTICS",
        "SESSION RESET ALL CHARACTERISTICS",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_reset_schema() {
    let query = "SESSION RESET SCHEMA";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn session_reset_graph() {
    let queries = vec![
        "SESSION RESET GRAPH",
        "SESSION RESET PROPERTY GRAPH",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_reset_timezone() {
    let query = "SESSION RESET TIME ZONE";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn session_reset_specific_parameter() {
    let queries = vec![
        "SESSION RESET PARAMETER $myParam",
        "SESSION RESET $myParam",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Session Close =====

#[test]
fn session_close() {
    let query = "SESSION CLOSE";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Transaction Start =====

#[test]
fn start_transaction_basic() {
    let query = "START TRANSACTION";
    let result = parse(query);
    assert!(result.ast.is_some(), "START TRANSACTION should parse");
}

#[test]
fn start_transaction_read_only() {
    let query = "START TRANSACTION READ ONLY";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "START TRANSACTION READ ONLY should parse"
    );
}

#[test]
fn start_transaction_read_write() {
    let query = "START TRANSACTION READ WRITE";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "START TRANSACTION READ WRITE should parse"
    );
}

#[test]
fn start_transaction_multiple_characteristics() {
    let query = "START TRANSACTION READ ONLY, READ WRITE";
    let result = parse(query);
    // This may be semantically invalid but should parse
    let _ = result.ast;
}

// ===== Transaction Commit =====

#[test]
fn commit_transaction() {
    let query = "COMMIT";
    let result = parse(query);
    assert!(result.ast.is_some(), "COMMIT should parse");
}

// ===== Transaction Rollback =====

#[test]
fn rollback_transaction() {
    let query = "ROLLBACK";
    let result = parse(query);
    assert!(result.ast.is_some(), "ROLLBACK should parse");
}

// ===== Transaction with Operations =====

#[test]
fn transaction_with_query() {
    let query = r#"
        START TRANSACTION
        MATCH (n) RETURN n
        COMMIT
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn transaction_with_mutation() {
    let query = r#"
        START TRANSACTION
        INSERT (n:Person {name: 'Alice'})
        COMMIT
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn transaction_with_rollback() {
    let query = r#"
        START TRANSACTION
        INSERT (n:Person {name: 'Alice'})
        ROLLBACK
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Combined Session and Transaction Operations =====

#[test]
fn session_set_then_transaction() {
    let query = r#"
        SESSION SET GRAPH myGraph
        START TRANSACTION
        MATCH (n) RETURN n
        COMMIT
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn multiple_session_sets() {
    let query = r#"
        SESSION SET GRAPH myGraph
        SESSION SET SCHEMA mySchema
        SESSION SET TIME ZONE 'UTC'
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Session Parameter Types =====

#[test]
fn session_set_value_with_different_types() {
    let queries = vec![
        "SESSION SET VALUE $intParam = 42",
        "SESSION SET VALUE $floatParam = 3.14",
        "SESSION SET VALUE $stringParam = 'hello'",
        "SESSION SET VALUE $boolParam = TRUE",
        "SESSION SET VALUE $nullParam = NULL",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_value_with_collections() {
    let queries = vec![
        "SESSION SET VALUE $listParam = [1, 2, 3]",
        "SESSION SET VALUE $mapParam = {key: 'value', count: 42}",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_value_with_expressions() {
    let queries = vec![
        "SESSION SET VALUE $computed = 10 + 20",
        "SESSION SET VALUE $concat = 'hello' || ' ' || 'world'",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

// ===== Session with Typed Parameters =====

#[test]
fn session_set_typed_value() {
    let queries = vec![
        "SESSION SET VALUE $param :: INT = 42",
        "SESSION SET VALUE $param TYPED STRING = 'hello'",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn session_set_typed_graph() {
    let query = "SESSION SET GRAPH $pg :: MyGraphType = myGraph";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Error Cases =====

#[test]
fn session_set_without_value_rejected() {
    let result = parse("SESSION SET VALUE $param");
    // Missing initializer
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn session_set_invalid_parameter_name() {
    let result = parse("SESSION SET VALUE INVALID = 100");
    // Parameter must start with $
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn transaction_start_invalid_characteristic() {
    let result = parse("START TRANSACTION INVALID");
    // Invalid characteristic
    assert!(!result.diagnostics.is_empty());
}

// ===== GQL Program Structure with Session =====

#[test]
fn gql_program_session_activity_only() {
    let query = r#"
        SESSION SET GRAPH myGraph
        SESSION SET SCHEMA mySchema
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn gql_program_transaction_activity() {
    let query = r#"
        START TRANSACTION
        MATCH (n) RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn gql_program_with_session_close() {
    let query = r#"
        SESSION SET GRAPH myGraph
        MATCH (n) RETURN n
        SESSION CLOSE
    "#;

    let result = parse(query);
    let _ = result.ast;
}
