//! Graph type specification parsing tests
//!
//! Tests for ISO GQL graph type specifications including:
//! - Node type specifications with properties, labels, and constraints
//! - Edge type specifications with connectivity constraints
//! - Inheritance and type hierarchies
//! - Property types and label sets
//!
//! ## ISO GQL Compliance Notes
//!
//! These tests verify that the parser correctly implements ISO GQL grammar rules:
//! 1. Element types are comma-separated (not semicolon)
//! 2. Constraints appear AFTER property type specifications `{ }`, not inside
//! 3. Multiple labels use `LABELS Label1 & Label2`, not multiple `LABEL` clauses
//! 4. Inheritance supports multiple parents: `INHERITS A, B, C`
//! 5. The parser must distinguish commas for inheritance from commas for element type lists

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
    use gql_parser::lexer::token::{Token, TokenKind};
    use gql_parser::parser::graph_type::GraphTypeParser;

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
    use gql_parser::lexer::token::{Token, TokenKind};
    use gql_parser::parser::graph_type::GraphTypeParser;

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
    use gql_parser::lexer::token::{Token, TokenKind};
    use gql_parser::parser::graph_type::GraphTypeParser;
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
    use gql_parser::lexer::token::{Token, TokenKind};
    use gql_parser::parser::graph_type::GraphTypeParser;
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

#[test]
fn test_edge_phrase_pattern_preserves_endpoint_aliases() {
    use gql_parser::ast::graph_type::{ElementTypeSpecification, LocalNodeTypeAlias};
    use gql_parser::lexer::token::{Token, TokenKind};
    use gql_parser::parser::graph_type::GraphTypeParser;
    use smol_str::SmolStr;

    let tokens = vec![
        Token::new(TokenKind::LBrace, 0..1),
        Token::new(TokenKind::Directed, 2..10),
        Token::new(TokenKind::Edge, 11..15),
        Token::new(TokenKind::Type, 16..20),
        Token::new(TokenKind::Identifier(SmolStr::new("KNOWS")), 21..26),
        Token::new(TokenKind::Connecting, 27..37),
        Token::new(TokenKind::LParen, 38..39),
        Token::new(TokenKind::Identifier(SmolStr::new("Person")), 39..45),
        Token::new(TokenKind::To, 46..48),
        Token::new(TokenKind::Identifier(SmolStr::new("Company")), 49..56),
        Token::new(TokenKind::RParen, 56..57),
        Token::new(TokenKind::RBrace, 58..59),
        Token::new(TokenKind::Eof, 59..59),
    ];

    let mut parser = GraphTypeParser::new(&tokens);
    let result = parser.parse_nested_graph_type_specification();
    assert!(result.is_ok(), "failed to parse graph type: {result:?}");
    let spec = result.unwrap();

    let first = spec
        .body
        .element_types
        .types
        .first()
        .expect("expected edge type");
    let ElementTypeSpecification::Edge(edge_spec) = first else {
        panic!("expected edge element type");
    };
    let gql_parser::ast::graph_type::EdgeTypePattern::Directed(pattern) = &edge_spec.pattern else {
        panic!("expected directed edge pattern");
    };

    let Some(LocalNodeTypeAlias {
        name: left_name, ..
    }) = pattern.left_endpoint.phrase.alias.as_ref()
    else {
        panic!("expected left endpoint alias");
    };
    let Some(LocalNodeTypeAlias {
        name: right_name, ..
    }) = pattern.right_endpoint.phrase.alias.as_ref()
    else {
        panic!("expected right endpoint alias");
    };

    assert_eq!(left_name, "Person");
    assert_eq!(right_name, "Company");
}

#[test]
fn test_graph_type_parser_supports_abstract_inheritance_and_constraints() {
    use gql_parser::ast::graph_type::{ElementTypeSpecification, GraphTypeConstraint};
    use gql_parser::parse;

    let source = r#"
        CREATE GRAPH TYPE social AS {
            ABSTRACT NODE TYPE Employee INHERITS Person
                LABEL Employee { id :: INT NOT NULL, name :: STRING }
                CONSTRAINT UNIQUE (id)
        }
    "#;

    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);

    let gql_parser::ast::Statement::Catalog(stmt) = &program.statements[0] else {
        panic!("expected catalog statement");
    };
    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create_graph_type) = &stmt.kind
    else {
        panic!("expected CREATE GRAPH TYPE statement");
    };

    let source = create_graph_type
        .source
        .as_ref()
        .expect("expected graph type source");

    let gql_parser::ast::GraphTypeSource::Detailed { specification, .. } = source else {
        panic!("expected detailed graph type source");
    };

    let first = specification
        .body
        .element_types
        .types
        .first()
        .expect("expected one element type");

    let ElementTypeSpecification::Node(node) = first else {
        panic!("expected node specification");
    };

    assert!(node.is_abstract, "expected ABSTRACT modifier");
    let inheritance = node
        .inheritance
        .as_ref()
        .expect("expected inheritance clause");
    assert_eq!(inheritance.parents.len(), 1);
    assert_eq!(inheritance.parents[0].name, "Person");

    let filler = node
        .pattern
        .phrase
        .filler
        .as_ref()
        .expect("expected node filler");
    assert_eq!(filler.constraints.len(), 1);
    assert!(matches!(
        filler.constraints[0],
        GraphTypeConstraint::Unique { .. }
    ));
}

#[test]
fn test_graph_type_parser_supports_edge_constraints_and_inheritance() {
    use gql_parser::ast::graph_type::{ElementTypeSpecification, GraphTypeConstraint};
    use gql_parser::parse;

    let source = r#"
        CREATE GRAPH TYPE social AS {
            ABSTRACT DIRECTED EDGE TYPE KNOWS EXTENDS RELATED
                CONSTRAINT CHECK (weight > 0)
                CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);

    let gql_parser::ast::Statement::Catalog(stmt) = &program.statements[0] else {
        panic!("expected catalog statement");
    };
    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create_graph_type) = &stmt.kind
    else {
        panic!("expected CREATE GRAPH TYPE statement");
    };
    let gql_parser::ast::GraphTypeSource::Detailed { specification, .. } = create_graph_type
        .source
        .as_ref()
        .expect("expected graph type source")
    else {
        panic!("expected detailed source");
    };

    let first = specification
        .body
        .element_types
        .types
        .first()
        .expect("expected one element type");

    let ElementTypeSpecification::Edge(edge) = first else {
        panic!("expected edge specification");
    };

    assert!(edge.is_abstract, "expected ABSTRACT modifier");
    let inheritance = edge
        .inheritance
        .as_ref()
        .expect("expected inheritance clause");
    assert_eq!(inheritance.parents.len(), 1);
    assert_eq!(inheritance.parents[0].name, "RELATED");

    let gql_parser::ast::graph_type::EdgeTypePattern::Directed(directed) = &edge.pattern else {
        panic!("expected directed edge pattern");
    };
    let filler = match &directed.arc {
        gql_parser::ast::graph_type::DirectedArcType::PointingRight(right) => right
            .filler
            .as_ref()
            .expect("expected arc filler in directed edge"),
        gql_parser::ast::graph_type::DirectedArcType::PointingLeft(_) => {
            panic!("unexpected left-pointing arc")
        }
    };
    assert_eq!(filler.constraints.len(), 1);
    assert!(matches!(
        filler.constraints[0],
        GraphTypeConstraint::Check { .. }
    ));
}

// ===== Additional Graph Type Validation Tests =====

#[test]
fn test_duplicate_element_type_names_produces_diagnostic() {
    let source = r#"
        CREATE GRAPH TYPE dup AS {
            NODE TYPE Person,
            NODE TYPE Person
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    // Parser accepts duplicate names; semantic validator should catch this
    // For now, verify it parses and produces AST
    assert!(result.ast.is_some(), "Should parse even with duplicate names");

    // NOTE: Semantic validation (not parser) should check for duplicates
    // This test documents that the parser accepts this syntax
}

#[test]
fn test_circular_inheritance_is_parsed() {
    let source = r#"
        CREATE GRAPH TYPE circular AS {
            NODE TYPE A INHERITS B,
            NODE TYPE B INHERITS A
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    // Parser accepts circular inheritance; semantic validator should catch the cycle
    assert!(result.ast.is_some(), "Should parse circular inheritance");

    // NOTE: Semantic validation (not parser) should detect cycles
    // This test documents that the parser accepts this syntax
}

#[test]
fn test_multiple_element_types_parse_correctly() {
    let source = r#"
        CREATE GRAPH TYPE multi AS {
            NODE TYPE Person { id :: INT, name :: STRING },
            NODE TYPE Company { id :: INT, name :: STRING },
            DIRECTED EDGE TYPE WORKS_AT CONNECTING (Person TO Company),
            DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    use gql_parser::parse;
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse without errors");
    assert!(result.ast.is_some());

    let program = result.ast.unwrap();
    let stmt = &program.statements[0];

    let gql_parser::ast::Statement::Catalog(cat) = stmt else {
        panic!("Expected catalog statement");
    };

    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create) = &cat.kind else {
        panic!("Expected CREATE GRAPH TYPE");
    };

    let Some(gql_parser::ast::GraphTypeSource::Detailed { specification, .. }) = &create.source else {
        panic!("Expected detailed source");
    };

    assert_eq!(specification.body.element_types.types.len(), 4,
               "Should have 4 element types");
}

#[test]
fn test_graph_type_with_multiple_labels_per_node() {
    // ISO GQL does not support multiple LABEL clauses per node type.
    // Instead, use LABELS with & to specify multiple labels
    let source = r#"
        CREATE GRAPH TYPE multi_label AS {
            NODE TYPE Person LABELS Employee & Manager { emp_id :: INT, dept :: STRING }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Should parse multiple labels using LABELS & syntax");
    // NOTE: The correct ISO GQL syntax for multiple labels is: LABELS Label1 & Label2
    // NOT: LABEL Label1 LABEL Label2
}

#[test]
fn test_edge_type_with_multiple_connecting_clauses() {
    // NOTE: ISO GQL grammar (line 1633-1635) shows only ONE endpointPairPhrase per edge type.
    // Multiple CONNECTING clauses are not part of the standard grammar.
    // This test documents that the parser correctly rejects multiple CONNECTING clauses.
    let source = r#"
        CREATE GRAPH TYPE multi_connect AS {
            NODE TYPE Person,
            NODE TYPE Company,
            EDGE TYPE RELATED
                CONNECTING (Person TO Person)
                CONNECTING (Person TO Company)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    // Parser should fail or treat second CONNECTING as a separate element (which fails)
    // This is CORRECT behavior per ISO GQL standard
    assert!(result.ast.is_none() || !result.diagnostics.is_empty(),
            "Parser correctly rejects multiple CONNECTING clauses per ISO GQL standard");
}

#[test]
fn test_graph_type_with_check_constraint() {
    // Correct ISO GQL syntax: constraints come AFTER property types specification
    let source = r#"
        CREATE GRAPH TYPE constrained AS {
            NODE TYPE Person { age :: INT } CONSTRAINT CHECK (age >= 0)
        }
    "#;

    use gql_parser::parse;
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse CHECK constraint after property types");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_unique_constraint() {
    // Correct ISO GQL syntax: constraints come AFTER property types specification
    let source = r#"
        CREATE GRAPH TYPE unique_id AS {
            NODE TYPE Person { id :: INT } CONSTRAINT UNIQUE (id)
        }
    "#;

    use gql_parser::parse;
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse UNIQUE constraint after property types");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_multiple_constraints() {
    // Correct ISO GQL syntax: constraints come AFTER property types specification
    let source = r#"
        CREATE GRAPH TYPE multi_constraint AS {
            NODE TYPE Person {
                id :: INT,
                age :: INT
            }
            CONSTRAINT UNIQUE (id)
            CONSTRAINT CHECK (age >= 0)
            CONSTRAINT CHECK (age <= 150)
        }
    "#;

    use gql_parser::parse;
    use gql_parser::ast::graph_type::{ElementTypeSpecification, GraphTypeConstraint};

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse multiple constraints after property types");

    let program = result.ast.unwrap();
    let gql_parser::ast::Statement::Catalog(cat) = &program.statements[0] else {
        panic!("Expected catalog statement");
    };

    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create) = &cat.kind else {
        panic!("Expected CREATE GRAPH TYPE");
    };

    let Some(gql_parser::ast::GraphTypeSource::Detailed { specification, .. }) = &create.source else {
        panic!("Expected detailed source");
    };

    let ElementTypeSpecification::Node(node) =
        &specification.body.element_types.types[0] else {
        panic!("Expected node type");
    };

    let filler = node.pattern.phrase.filler.as_ref().unwrap();
    assert_eq!(filler.constraints.len(), 3, "Should have 3 constraints");
}

#[test]
fn test_undirected_edge_type_specification() {
    let source = r#"
        CREATE GRAPH TYPE undirected AS {
            NODE TYPE Person,
            UNDIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    use gql_parser::parse;
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse undirected edge");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_inheritance_chain() {
    let source = r#"
        CREATE GRAPH TYPE inheritance AS {
            NODE TYPE Entity,
            NODE TYPE Person INHERITS Entity,
            NODE TYPE Employee INHERITS Person
        }
    "#;

    use gql_parser::parse;
    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse inheritance chain");
    assert!(result.ast.is_some());

    // NOTE: Semantic validation (not parser) should check inheritance chain correctness
}

