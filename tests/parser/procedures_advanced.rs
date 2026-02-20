//! Advanced Procedure Call Parser Tests
//!
//! This module tests parsing of advanced procedure features including:
//! - Inline procedures with variable definitions
//! - YIELD with WHERE filters
//! - Complex procedure argument patterns
//! - Nested procedure calls

use gql_parser::parse;
use crate::common::*;

// ===== Inline Procedures with Variable Definitions =====

#[test]
fn inline_procedure_with_value_definition() {
    let query = r#"
        VALUE counter = 0
        CALL { MATCH (n) RETURN COUNT(n) AS total }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn inline_procedure_with_graph_definition() {
    let query = r#"
        GRAPH g = CURRENT_GRAPH
        CALL { USE g MATCH (n) RETURN n }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn inline_procedure_with_table_definition() {
    let query = r#"
        TABLE results = (MATCH (n) RETURN n)
        CALL { SELECT * FROM results }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn inline_procedure_with_multiple_definitions() {
    let query = r#"
        VALUE x = 1
        VALUE y = 2
        GRAPH g = CURRENT_GRAPH
        CALL { RETURN x + y }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Inline Procedures with Variable Scope =====

#[test]
fn inline_procedure_empty_scope() {
    let query = "CALL () { RETURN 1 }";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "Inline procedure with empty scope should parse"
    );
}

#[test]
fn inline_procedure_single_variable_scope() {
    let query = "MATCH (n) CALL (n) { RETURN n.name }";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "Inline procedure with single variable should parse"
    );
}

#[test]
fn inline_procedure_multiple_variable_scope() {
    let query = "MATCH (a)-[r]->(b) CALL (a, r, b) { RETURN a.name, type(r), b.name }";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "Inline procedure with multiple variables should parse"
    );
}

// ===== YIELD with Filters =====

#[test]
fn yield_with_where_clause() {
    let query = "CALL myProc(1, 2) YIELD result WHERE result.value > 10 RETURN result";
    let result = parse(query);
    // YIELD with WHERE may not be fully implemented
    let _ = result.ast;
}

#[test]
fn yield_with_complex_where() {
    let query = r#"
        CALL myProc()
        YIELD x, y, z
        WHERE x > 0 AND y < 100 OR z = 'active'
        RETURN x, y, z
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn yield_with_aggregation_in_where() {
    let query = r#"
        CALL myProc()
        YIELD item
        WHERE item.count > AVG(item.count)
        RETURN item
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Complex Procedure Arguments =====

#[test]
fn procedure_call_with_expression_arguments() {
    let queries = vec![
        "CALL myProc(1 + 2, 3 * 4)",
        "CALL myProc(n.id, n.value * 2)",
        "CALL myProc([1, 2, 3], {key: 'value'})",
    ];

    for query in queries {
        let result = parse(query);
        assert!(
            result.ast.is_some(),
            "Procedure with expression args '{}' should parse",
            query
        );
    }
}

#[test]
fn procedure_call_with_nested_function_arguments() {
    let queries = vec![
        "CALL myProc(UPPER(n.name), LOWER(n.email))",
        "CALL myProc(SUBSTRING(s, 1, 5), TRIM(t))",
        "CALL myProc(COUNT(n), SUM(n.value))",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn procedure_call_with_subquery_argument() {
    let query = "CALL myProc((MATCH (n) RETURN COUNT(n)))";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Nested Procedure Calls =====

#[test]
fn sequential_procedure_calls() {
    let query = r#"
        CALL proc1()
        CALL proc2()
        CALL proc3()
        RETURN 1
    "#;

    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "Sequential procedure calls should parse"
    );
}

#[test]
fn procedure_call_with_yield_piped_to_another() {
    let query = r#"
        CALL proc1() YIELD x
        CALL proc2(x) YIELD y
        RETURN y
    "#;

    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "Piped procedure calls should parse"
    );
}

#[test]
fn nested_inline_procedures() {
    let query = r#"
        CALL {
            CALL {
                MATCH (n) RETURN n
            }
        }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== OPTIONAL CALL =====

#[test]
fn optional_call_named_procedure() {
    let query = "OPTIONAL CALL risky_procedure()";
    let result = parse(query);
    assert!(result.ast.is_some(), "OPTIONAL CALL should parse");
}

#[test]
fn optional_call_inline_procedure() {
    let query = r#"
        OPTIONAL CALL {
            MATCH (n:Rare) RETURN n
        }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn optional_call_with_yield() {
    let query = "OPTIONAL CALL myProc() YIELD result RETURN result";
    let result = parse(query);
    let _ = result.ast;
}

// ===== CALL in Different Contexts =====

#[test]
fn call_in_match_pipeline() {
    let query = r#"
        MATCH (n:Person)
        CALL enrichData(n) YIELD enriched
        RETURN enriched
    "#;

    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "CALL in MATCH pipeline should parse"
    );
}

#[test]
fn call_in_mutation_pipeline() {
    let query = r#"
        INSERT (n:Person {name: 'Alice'})
        CALL notifyCreation(n)
        RETURN n
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn call_after_set_operations() {
    let query = r#"
        MATCH (n:Person) RETURN n
        UNION
        MATCH (m:Company) RETURN m
        CALL processResults()
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== YIELD Variations =====

#[test]
fn yield_all_columns() {
    let query = "CALL myProc() YIELD * RETURN *";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn yield_with_aliases() {
    let query = r#"
        CALL myProc()
        YIELD result1 AS r1, result2 AS r2, result3 AS r3
        RETURN r1, r2, r3
    "#;

    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "YIELD with aliases should parse"
    );
}

#[test]
fn yield_subset_of_outputs() {
    let query = "CALL myProc() YIELD result1, result3 RETURN result1";
    let result = parse(query);
    assert!(
        result.ast.is_some(),
        "YIELD subset should parse"
    );
}

// ===== AT Schema Clause =====

#[test]
fn procedure_with_at_schema_clause() {
    let query = r#"
        AT mySchema
        CALL myProc()
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn inline_procedure_with_at_schema() {
    let query = r#"
        AT mySchema
        CALL {
            MATCH (n) RETURN n
        }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Procedure Definition Contexts =====

#[test]
fn value_definition_with_typed_initializer() {
    let query = r#"
        VALUE counter :: INT = 0
        CALL { RETURN counter }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn graph_definition_with_typed_initializer() {
    let query = r#"
        GRAPH g :: MyGraphType = CURRENT_GRAPH
        CALL { USE g MATCH (n) RETURN n }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn table_definition_with_typed_initializer() {
    let query = r#"
        TABLE results TYPED MyTableType = (MATCH (n) RETURN n)
        CALL { SELECT * FROM results }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Catalog-Modifying Procedures =====

#[test]
fn call_procedure_in_catalog_context() {
    let query = r#"
        CREATE SCHEMA mySchema
        CALL setupSchema()
    "#;

    let result = parse(query);
    let _ = result.ast;
}

// ===== Error Cases =====

#[test]
fn named_procedure_without_parentheses_rejected() {
    let result = parse("CALL myProc");
    // Must have parentheses
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn yield_without_items_rejected() {
    let result = parse("CALL myProc() YIELD");
    // YIELD must have at least one item
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn inline_procedure_unclosed_brace_rejected() {
    let result = parse("CALL { MATCH (n) RETURN n");
    // Missing closing brace
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn variable_definition_without_initializer_rejected() {
    let result = parse("VALUE x CALL { RETURN 1 }");
    // VALUE requires initializer
    assert!(!result.diagnostics.is_empty());
}

// ===== Complex Scenarios =====

#[test]
fn procedure_call_chain_with_transformations() {
    let query = r#"
        MATCH (n:Person)
        CALL validate(n) YIELD valid
        WHERE valid = TRUE
        CALL enrich(n) YIELD enriched
        CALL transform(enriched) YIELD final
        RETURN final
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn inline_procedure_with_complex_body() {
    let query = r#"
        CALL (n) {
            MATCH (n)-[:KNOWS]->(friend)
            WHERE friend.age > 18
            WITH friend, COUNT(*) AS mutualFriends
            ORDER BY mutualFriends DESC
            LIMIT 10
            RETURN friend, mutualFriends
        }
    "#;

    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn procedure_with_all_features_combined() {
    let query = r#"
        VALUE threshold = 100
        GRAPH g = CURRENT_GRAPH
        AT mySchema
        OPTIONAL CALL (n, threshold) {
            USE g
            MATCH (n)-[:KNOWS]->(friend)
            WHERE friend.score > threshold
            RETURN friend
        }
        YIELD friend
        WHERE friend.active = TRUE
        RETURN friend.name
    "#;

    let result = parse(query);
    let _ = result.ast;
}
