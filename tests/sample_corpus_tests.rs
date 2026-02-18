//! Official GQL Sample Corpus Integration Tests
//!
//! This test suite validates that the parser successfully handles all official
//! GQL sample files from the opengql-grammar repository. These samples represent
//! canonical GQL syntax and serve as conformance validation.
//!
//! Sample Location: `third_party/opengql-grammar/samples/`
//! Total Samples: 14 official GQL sample files

use gql_parser::parse;
use std::fs;
use std::path::Path;

/// Helper to load and parse a sample file
fn parse_sample(filename: &str) -> (String, gql_parser::ParseResult) {
    let path = Path::new("third_party/opengql-grammar/samples").join(filename);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read sample file {}: {}", filename, e));
    let result = parse(&source);
    (source, result)
}

/// Helper to assert sample parses successfully
fn assert_sample_parses(filename: &str, description: &str) {
    let (source, result) = parse_sample(filename);

    // Sample should produce an AST
    assert!(
        result.ast.is_some(),
        "Sample '{}' ({}) should parse successfully.\nSource:\n{}\nDiagnostics: {} issues",
        filename,
        description,
        source,
        result.diagnostics.len()
    );

    // Log diagnostics if any (for debugging)
    if !result.diagnostics.is_empty() {
        eprintln!("Sample '{}' has {} diagnostics:", filename, result.diagnostics.len());
        for diag in &result.diagnostics {
            eprintln!("  - {}", diag);
        }
    }
}

#[test]
fn sample_01_create_closed_graph_from_graph_type_double_colon() {
    // CREATE GRAPH mySocialNetwork ::socialNetworkGraphType
    assert_sample_parses(
        "create_closed_graph_from_graph_type_(double_colon).gql",
        "Graph creation with double-colon type annotation syntax"
    );
}

#[test]
fn sample_02_create_closed_graph_from_graph_type_lexical() {
    // CREATE GRAPH mySocialNetwork TYPED socialNetworkGraphType
    assert_sample_parses(
        "create_closed_graph_from_graph_type_(lexical).gql",
        "Graph creation with lexical TYPED keyword"
    );
}

#[test]
fn sample_03_create_closed_graph_from_nested_graph_type_double_colon() {
    // CREATE GRAPH mySocialNetwork ::{...inline graph type...}
    assert_sample_parses(
        "create_closed_graph_from_nested_graph_type_(double_colon).gql",
        "Graph creation with inline nested graph type specification"
    );
}

#[test]
fn sample_04_create_graph() {
    // Multiple graph creation variants: ANY, with type, LIKE, AS COPY OF
    assert_sample_parses(
        "create_graph.gql",
        "Various graph creation forms"
    );
}

#[test]
fn sample_05_create_schema() {
    // Schema creation with paths and NEXT chaining
    assert_sample_parses(
        "create_schema.gql",
        "Schema DDL with procedure chaining"
    );
}

#[test]
fn sample_06_insert_statement() {
    // Node and edge insertion with properties
    assert_sample_parses(
        "insert_statement.gql",
        "INSERT patterns with temporal literals and properties"
    );
}

#[test]
fn sample_07_match_and_insert_example() {
    // Combined MATCH and INSERT
    assert_sample_parses(
        "match_and_insert_example.gql",
        "Combined data-accessing and data-modifying query"
    );
}

#[test]
fn sample_08_match_with_exists_predicate_match_block_in_braces() {
    // EXISTS with braced MATCH block
    assert_sample_parses(
        "match_with_exists_predicate_(match_block_statement_in_braces).gql",
        "EXISTS predicate with braced MATCH block"
    );
}

#[test]
fn sample_09_match_with_exists_predicate_match_block_in_parentheses() {
    // EXISTS with parenthesized MATCH block
    assert_sample_parses(
        "match_with_exists_predicate_(match_block_statement_in_parentheses).gql",
        "EXISTS predicate with parenthesized MATCH block"
    );
}

#[test]
fn sample_10_match_with_exists_predicate_nested_match_statement() {
    // EXISTS with nested MATCH and RETURN
    assert_sample_parses(
        "match_with_exists_predicate_(nested_match_statement).gql",
        "EXISTS predicate with nested MATCH and RETURN clause"
    );
}

#[test]
fn sample_11_session_set_graph_to_current_graph() {
    // SESSION SET GRAPH CURRENT_GRAPH
    assert_sample_parses(
        "session_set_graph_to_current_graph.gql",
        "Session management with CURRENT_GRAPH function"
    );
}

#[test]
fn sample_12_session_set_graph_to_current_property_graph() {
    // SESSION SET GRAPH CURRENT_PROPERTY_GRAPH
    assert_sample_parses(
        "session_set_graph_to_current_property_graph.gql",
        "Session management with CURRENT_PROPERTY_GRAPH function"
    );
}

#[test]
fn sample_13_session_set_property_as_value() {
    // SESSION SET VALUE IF NOT EXISTS $exampleProperty = DATE '2023-10-10'
    assert_sample_parses(
        "session_set_property_as_value.gql",
        "Session parameter with conditional assignment and temporal literal"
    );
}

#[test]
fn sample_14_session_set_time_zone() {
    // SESSION SET TIME ZONE "utc"
    assert_sample_parses(
        "session_set_time_zone.gql",
        "Session timezone configuration"
    );
}

#[test]
fn all_samples_parse_successfully() {
    // Comprehensive test that all samples parse
    let samples = vec![
        "create_closed_graph_from_graph_type_(double_colon).gql",
        "create_closed_graph_from_graph_type_(lexical).gql",
        "create_closed_graph_from_nested_graph_type_(double_colon).gql",
        "create_graph.gql",
        "create_schema.gql",
        "insert_statement.gql",
        "match_and_insert_example.gql",
        "match_with_exists_predicate_(match_block_statement_in_braces).gql",
        "match_with_exists_predicate_(match_block_statement_in_parentheses).gql",
        "match_with_exists_predicate_(nested_match_statement).gql",
        "session_set_graph_to_current_graph.gql",
        "session_set_graph_to_current_property_graph.gql",
        "session_set_property_as_value.gql",
        "session_set_time_zone.gql",
    ];

    let mut failed_samples = Vec::new();
    let mut total_diagnostics = 0;

    for sample in &samples {
        let (source, result) = parse_sample(sample);
        total_diagnostics += result.diagnostics.len();

        if result.ast.is_none() {
            failed_samples.push((sample, source, result.diagnostics));
        }
    }

    if !failed_samples.is_empty() {
        eprintln!("\n==== Failed Samples ====");
        for (sample, source, diagnostics) in &failed_samples {
            eprintln!("\nSample: {}", sample);
            eprintln!("Source:\n{}", source);
            eprintln!("Diagnostics ({} issues):", diagnostics.len());
            for diag in diagnostics {
                eprintln!("  {}", diag);
            }
        }
        panic!(
            "{} of {} samples failed to parse",
            failed_samples.len(),
            samples.len()
        );
    }

    eprintln!(
        "\n✓ All {} samples parsed successfully ({} total diagnostics)",
        samples.len(),
        total_diagnostics
    );
}

#[test]
fn sample_coverage_report() {
    // Generate a coverage report showing which GQL features each sample exercises
    let samples_with_features = vec![
        ("create_closed_graph_from_graph_type_(double_colon).gql", vec!["CREATE GRAPH", "type annotation (::)"]),
        ("create_closed_graph_from_graph_type_(lexical).gql", vec!["CREATE GRAPH", "TYPED keyword"]),
        ("create_closed_graph_from_nested_graph_type_(double_colon).gql", vec!["CREATE GRAPH", "inline graph type", "node element type"]),
        ("create_graph.gql", vec!["CREATE GRAPH", "graph types", "LIKE", "AS COPY OF"]),
        ("create_schema.gql", vec!["CREATE SCHEMA", "NEXT", "graph patterns"]),
        ("insert_statement.gql", vec!["INSERT", "node patterns", "edge patterns", "temporal literals"]),
        ("match_and_insert_example.gql", vec!["MATCH", "INSERT", "combined query"]),
        ("match_with_exists_predicate_(match_block_statement_in_braces).gql", vec!["MATCH", "EXISTS", "braced block"]),
        ("match_with_exists_predicate_(match_block_statement_in_parentheses).gql", vec!["MATCH", "EXISTS", "parenthesized block"]),
        ("match_with_exists_predicate_(nested_match_statement).gql", vec!["MATCH", "EXISTS", "nested MATCH", "RETURN"]),
        ("session_set_graph_to_current_graph.gql", vec!["SESSION", "SET GRAPH", "CURRENT_GRAPH"]),
        ("session_set_graph_to_current_property_graph.gql", vec!["SESSION", "SET GRAPH", "CURRENT_PROPERTY_GRAPH"]),
        ("session_set_property_as_value.gql", vec!["SESSION", "SET VALUE", "IF NOT EXISTS", "parameters", "DATE literal"]),
        ("session_set_time_zone.gql", vec!["SESSION", "SET TIME ZONE"]),
    ];

    eprintln!("\n==== Sample Corpus Feature Coverage ====");
    for (sample, features) in samples_with_features {
        eprintln!("{}: {}", sample, features.join(", "));
    }

    // This test always passes - it's for documentation
}
