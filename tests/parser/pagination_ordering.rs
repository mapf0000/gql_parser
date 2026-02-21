//! Unit tests for pagination and ordering - targeting untested code paths
//!
//! These tests target parser/query/pagination.rs which has 0% coverage.
//! Focus: ORDER BY, LIMIT, OFFSET/SKIP functionality that impacts query correctness.

use gql_parser::lexer::Lexer;
use gql_parser::parser::Parser;


#[test]
fn test_order_by_single_field_ascending() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY n.name ASC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY ASC should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_single_field_descending() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY n.name DESC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY DESC should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_ascending_keyword() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY n.name ASCENDING
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY ASCENDING should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_descending_keyword() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY n.name DESCENDING
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY DESCENDING should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_default_direction() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY n.name
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY without explicit direction should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_multiple_fields() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age
        ORDER BY n.name ASC, n.age DESC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY multiple fields should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_expression() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age
        ORDER BY n.age * 2 DESC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY expression should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_limit_clause() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        LIMIT 10
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "LIMIT clause should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_offset_clause() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        OFFSET 5
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "OFFSET clause should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_skip_clause() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        SKIP 5
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "SKIP clause should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_limit_and_offset() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        OFFSET 10
        LIMIT 20
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "OFFSET and LIMIT together should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_with_limit() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age
        ORDER BY n.age DESC
        LIMIT 5
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with LIMIT should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_with_offset_and_limit() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age
        ORDER BY n.age DESC
        OFFSET 10
        LIMIT 20
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with OFFSET and LIMIT should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_with_skip_and_limit() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age
        ORDER BY n.age DESC
        SKIP 10
        LIMIT 20
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with SKIP and LIMIT should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_limit_with_expression() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        LIMIT 5 + 10
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "LIMIT with expression should parse");
}

#[test]
fn test_offset_with_expression() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        OFFSET 10 * 2
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "OFFSET with expression should parse");
}

#[test]
fn test_order_by_with_three_fields() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age, n.city
        ORDER BY n.city ASC, n.age DESC, n.name ASC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with three fields should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_order_by_with_function_call() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        ORDER BY upper(n.name) ASC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with function call should parse");
}

#[test]
fn test_limit_zero() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        LIMIT 0
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "LIMIT 0 should parse");
}

#[test]
fn test_offset_zero() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name
        OFFSET 0
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "OFFSET 0 should parse");
}

#[test]
fn test_order_by_with_property_access_chain() {
    let source = r#"
        MATCH (n:Person)-[:LIVES_IN]->(c:City)
        RETURN n.name, c.name
        ORDER BY c.name ASC, n.name ASC
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "ORDER BY with multiple property accesses should parse");
}

#[test]
fn test_group_by_single_field() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.city, COUNT(n) AS cnt
        GROUP BY n.city
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "GROUP BY single field should parse");
}

#[test]
fn test_group_by_multiple_fields() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.city, n.age, COUNT(n) AS cnt
        GROUP BY n.city, n.age
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "GROUP BY multiple fields should parse");
}

#[test]
fn test_group_by_with_having() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.city, COUNT(n) AS cnt
        GROUP BY n.city
        HAVING COUNT(n) > 5
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "GROUP BY with HAVING should parse");
}

#[test]
fn test_complete_pagination_pipeline() {
    let source = r#"
        MATCH (n:Person)
        RETURN n.name, n.age, n.city
        ORDER BY n.city ASC, n.age DESC
        OFFSET 20
        LIMIT 10
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Complete pagination pipeline should parse");
}
