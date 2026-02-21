//! Unit tests for path pattern parsing - targeting untested code paths
//!
//! These tests target parser/patterns/path.rs which has only 16% coverage.
//! Focus: Path modes, search prefixes, quantifiers, and complex path patterns.

use gql_parser::lexer::Lexer;
use gql_parser::parser::Parser;


#[test]
fn test_simple_node_to_node_path() {
    let source = r#"
        MATCH (a:Person)-[:KNOWS]->(b:Person)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Simple path pattern should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_path_with_walk_mode() {
    let source = r#"
        MATCH WALK (a)-[:KNOWS]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "WALK path mode should parse");
}

#[test]
fn test_path_with_trail_mode() {
    let source = r#"
        MATCH TRAIL (a)-[:KNOWS]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "TRAIL path mode should parse");
}

#[test]
fn test_path_with_simple_mode() {
    let source = r#"
        MATCH SIMPLE PATH (a)-[:KNOWS]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "SIMPLE PATH mode should parse");
}

#[test]
fn test_path_with_acyclic_mode() {
    let source = r#"
        MATCH ACYCLIC PATH (a)-[:KNOWS]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ACYCLIC PATH mode should parse");
}

#[test]
fn test_shortest_path() {
    let source = r#"
        MATCH SHORTEST PATH (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "SHORTEST PATH should parse");
}

#[test]
fn test_all_shortest_paths() {
    let source = r#"
        MATCH ALL SHORTEST PATHS (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ALL SHORTEST PATHS should parse");
}

#[test]
fn test_any_shortest_path() {
    let source = r#"
        MATCH ANY SHORTEST PATH (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ANY SHORTEST PATH should parse");
}

#[test]
fn test_all_paths() {
    let source = r#"
        MATCH ALL PATHS (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ALL PATHS should parse");
}

#[test]
fn test_any_path() {
    let source = r#"
        MATCH ANY PATH (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ANY PATH should parse");
}

#[test]
fn test_path_with_variable_declaration() {
    let source = r#"
        MATCH p = (a)-[:KNOWS]->(b)
        RETURN p
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Path variable declaration should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_path_with_quantifier_exact() {
    let source = r#"
        MATCH (a)-[:KNOWS*3]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Exact quantifier should parse");
}

#[test]
fn test_path_with_quantifier_range() {
    let source = r#"
        MATCH (a)-[:KNOWS*1..5]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Range quantifier should parse");
}

#[test]
fn test_path_with_quantifier_unbounded() {
    let source = r#"
        MATCH (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Unbounded quantifier should parse");
}

#[test]
fn test_path_with_quantifier_min_only() {
    let source = r#"
        MATCH (a)-[:KNOWS*2..]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Min-only quantifier should parse");
}

#[test]
fn test_path_with_quantifier_max_only() {
    let source = r#"
        MATCH (a)-[:KNOWS*..5]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Max-only quantifier should parse");
}

#[test]
fn test_undirected_edge_pattern() {
    let source = r#"
        MATCH (a)-[:KNOWS]-(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Undirected edge pattern should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_left_directed_edge_pattern() {
    let source = r#"
        MATCH (a)<-[:KNOWS]-(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Left-directed edge pattern should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_edge_pattern_with_properties() {
    let source = r#"
        MATCH (a)-[r:KNOWS {since: 2020}]->(b)
        RETURN a, r, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge pattern with properties should parse");
}

#[test]
fn test_edge_pattern_with_multiple_labels() {
    let source = r#"
        MATCH (a)-[:KNOWS|FOLLOWS]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge pattern with multiple labels should parse");
}

#[test]
fn test_edge_pattern_without_label() {
    let source = r#"
        MATCH (a)-->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge pattern without label should parse");
}

#[test]
fn test_edge_pattern_with_variable_no_label() {
    let source = r#"
        MATCH (a)-[r]->(b)
        RETURN a, r, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge pattern with variable but no label should parse");
}

#[test]
fn test_multi_hop_path() {
    let source = r#"
        MATCH (a)-[:KNOWS]->(b)-[:WORKS_AT]->(c)
        RETURN a, b, c
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Multi-hop path should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_complex_path_with_multiple_hops_and_quantifiers() {
    let source = r#"
        MATCH (a:Person)-[:KNOWS*1..3]->(b)-[:WORKS_AT]->(c:Company)
        RETURN a, b, c
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Complex path with quantifiers should parse");
}

#[test]
fn test_path_with_where_clause() {
    let source = r#"
        MATCH (a)-[r:KNOWS WHERE r.since > 2020]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Path with WHERE in edge pattern should parse");
}

#[test]
fn test_all_simple_paths() {
    let source = r#"
        MATCH ALL SIMPLE PATHS (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ALL SIMPLE PATHS should parse");
}

#[test]
fn test_all_acyclic_paths() {
    let source = r#"
        MATCH ALL ACYCLIC PATHS (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ALL ACYCLIC PATHS should parse");
}

#[test]
fn test_shortest_simple_path() {
    let source = r#"
        MATCH SHORTEST SIMPLE PATH (a)-[:KNOWS*]->(b)
        RETURN a, b
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "SHORTEST SIMPLE PATH should parse");
}

#[test]
fn test_path_pattern_with_node_labels() {
    let source = r#"
        MATCH (a:Person:Employee)-[:WORKS_AT]->(c:Company:Organization)
        RETURN a, c
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Path with multiple node labels should parse");
}

#[test]
fn test_bidirectional_path_patterns() {
    let source = r#"
        MATCH (a)<-[:KNOWS]-(b)-[:WORKS_AT]->(c)
        RETURN a, b, c
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Bidirectional path patterns should parse");
}
