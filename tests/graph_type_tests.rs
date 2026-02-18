//! Integration tests for graph type specification parsing.

use gql_parser::lexer::Lexer;
use gql_parser::parser::Parser;

#[test]
fn test_basic_compilation() {
    // Just verify that the module compiles and basic parsing works
    let source = "RETURN 1";
    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_parser_module_exists() {
    // Verify the graph_type parser module is accessible
    use gql_parser::parser::graph_type::GraphTypeParser;
    use gql_parser::lexer::token::{Token, TokenKind};

    let tokens = vec![
        Token::new(TokenKind::LBrace, 0..1),
        Token::new(TokenKind::RBrace, 1..2),
        Token::new(TokenKind::Eof, 2..2),
    ];

    let mut parser = GraphTypeParser::new(&tokens);
    let result = parser.parse_nested_graph_type_specification();

    // Should parse an empty graph type spec successfully
    assert!(result.is_ok());
}

#[test]
fn test_property_types_specification_empty() {
    use gql_parser::parser::graph_type::GraphTypeParser;
    use gql_parser::lexer::token::{Token, TokenKind};

    let tokens = vec![
        Token::new(TokenKind::LBrace, 0..1),
        Token::new(TokenKind::RBrace, 1..2),
        Token::new(TokenKind::Eof, 2..2),
    ];

    let mut parser = GraphTypeParser::new(&tokens);
    let result = parser.parse_property_types_specification();

    assert!(result.is_ok());
    let prop_spec = result.unwrap();
    assert!(prop_spec.property_types.is_none());
}

#[test]
fn test_label_set_phrase_single_label() {
    use gql_parser::parser::graph_type::GraphTypeParser;
    use gql_parser::lexer::token::{Token, TokenKind};
    use smol_str::SmolStr;

    let tokens = vec![
        Token::new(TokenKind::Label, 0..5),
        Token::new(TokenKind::Identifier(SmolStr::new("Person")), 6..12),
        Token::new(TokenKind::Eof, 12..12),
    ];

    let mut parser = GraphTypeParser::new(&tokens);
    let result = parser.parse_label_set_phrase();

    assert!(result.is_ok());
}

#[test]
fn test_label_set_specification_multiple_labels() {
    use gql_parser::parser::graph_type::GraphTypeParser;
    use gql_parser::lexer::token::{Token, TokenKind};
    use smol_str::SmolStr;

    let tokens = vec![
        Token::new(TokenKind::Identifier(SmolStr::new("Person")), 0..6),
        Token::new(TokenKind::Ampersand, 7..8),
        Token::new(TokenKind::Identifier(SmolStr::new("Employee")), 9..17),
        Token::new(TokenKind::Eof, 17..17),
    ];

    let mut parser = GraphTypeParser::new(&tokens);
    let result = parser.parse_label_set_specification();

    assert!(result.is_ok());
    let label_set = result.unwrap();
    assert_eq!(label_set.labels.len(), 2);
}
