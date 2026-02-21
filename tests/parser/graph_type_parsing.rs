//! Unit tests for graph type parsing - targeting untested code paths
//!
//! These tests target the parser/graph_type.rs module which currently has 0% coverage.
//! Focus: Testing critical functional parsing paths that impact correctness.

use gql_parser::lexer::Lexer;
use gql_parser::parser::Parser;


#[test]
fn test_empty_graph_type_specification() {
    let source = r#"
        CREATE GRAPH TYPE empty_graph AS {}
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Empty graph type should parse successfully");
}

#[test]
fn test_node_type_with_single_label() {
    let source = r#"
        CREATE GRAPH TYPE social AS {
            NODE TYPE Person LABEL Person {}
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Node type with single label should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_node_type_with_multiple_labels() {
    let source = r#"
        CREATE GRAPH TYPE org AS {
            NODE TYPE Employee LABELS Person & Worker & Employee {}
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Node type with multiple labels should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_node_type_with_properties() {
    let source = r#"
        CREATE GRAPH TYPE app AS {
            NODE TYPE User {
                id :: INT,
                name :: STRING,
                email :: STRING,
                age :: INT
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Node type with properties should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_node_type_with_property_not_null() {
    let source = r#"
        CREATE GRAPH TYPE app AS {
            NODE TYPE User {
                id :: INT NOT NULL,
                name :: STRING NOT NULL
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "NOT NULL constraints should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_node_type_with_inheritance() {
    let source = r#"
        CREATE GRAPH TYPE inheritance_test AS {
            NODE TYPE Base {},
            NODE TYPE Derived INHERITS Base {}
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Node type inheritance should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_node_type_with_multiple_inheritance() {
    let source = r#"
        CREATE GRAPH TYPE multi_inherit AS {
            NODE TYPE A {},
            NODE TYPE B {},
            NODE TYPE C INHERITS A, B {}
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Multiple inheritance should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_abstract_node_type() {
    let source = r#"
        CREATE GRAPH TYPE abstract_test AS {
            ABSTRACT NODE TYPE Entity {},
            NODE TYPE Person INHERITS Entity {}
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Abstract node types should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_directed_edge_type_basic() {
    let source = r#"
        CREATE GRAPH TYPE edges AS {
            NODE TYPE Person {},
            DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Directed edge type should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_undirected_edge_type() {
    let source = r#"
        CREATE GRAPH TYPE undirected AS {
            NODE TYPE Person {},
            UNDIRECTED EDGE TYPE FRIENDS CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Undirected edge type should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_edge_type_with_properties() {
    let source = r#"
        CREATE GRAPH TYPE rel_props AS {
            NODE TYPE Person {},
            DIRECTED EDGE TYPE KNOWS {
                since :: DATE,
                strength :: FLOAT
            } CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge type with properties should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_edge_type_with_labels() {
    let source = r#"
        CREATE GRAPH TYPE labeled_edge AS {
            NODE TYPE Person {},
            DIRECTED EDGE TYPE KNOWS LABEL Relationship {}
                CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge type with label should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_multiple_element_types_comma_separated() {
    let source = r#"
        CREATE GRAPH TYPE multi AS {
            NODE TYPE Person {},
            NODE TYPE Company {},
            NODE TYPE Item {},
            DIRECTED EDGE TYPE WORKS_AT CONNECTING (Person TO Company),
            DIRECTED EDGE TYPE BUYS CONNECTING (Person TO Item)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Multiple comma-separated types should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_trailing_comma_in_element_types() {
    let source = r#"
        CREATE GRAPH TYPE comma_test AS {
            NODE TYPE Person {},
            NODE TYPE Company {},
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Trailing comma should be allowed");
    // Test focuses on successful parsing
}

#[test]
fn test_property_type_without_typed_marker() {
    let source = r#"
        CREATE GRAPH TYPE no_marker_test AS {
            NODE TYPE User {
                id INT,
                name STRING
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Property types without typed marker should parse (per ISO GQL grammar)");
}

#[test]
fn test_property_type_with_double_colon() {
    let source = r#"
        CREATE GRAPH TYPE double_colon AS {
            NODE TYPE User {
                id :: INT,
                name :: STRING
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Double colon for property types should parse");
}

#[test]
fn test_node_type_pattern_with_alias() {
    let source = r#"
        CREATE GRAPH TYPE alias_test AS {
            (p:Person {id :: INT})
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Node type pattern with alias should parse");
}

#[test]
fn test_edge_connecting_different_node_types() {
    let source = r#"
        CREATE GRAPH TYPE heterogeneous AS {
            NODE TYPE Person {},
            NODE TYPE Company {},
            DIRECTED EDGE TYPE EMPLOYS CONNECTING (Company TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge connecting different node types should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_abstract_edge_type() {
    let source = r#"
        CREATE GRAPH TYPE abstract_edge AS {
            NODE TYPE Entity {},
            ABSTRACT DIRECTED EDGE TYPE Relationship {}
                CONNECTING (Entity TO Entity)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Abstract edge type should parse");
}

#[test]
fn test_property_types_with_various_types() {
    let source = r#"
        CREATE GRAPH TYPE type_variety AS {
            NODE TYPE Sample {
                int_val :: INT,
                str_val :: STRING,
                bool_val :: BOOL,
                float_val :: FLOAT,
                date_val :: DATE,
                time_val :: TIME,
                timestamp_val :: TIMESTAMP
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Various property types should parse");
    // Test focuses on successful parsing
}

#[test]
fn test_edge_with_inheritance() {
    let source = r#"
        CREATE GRAPH TYPE edge_inherit AS {
            NODE TYPE Person {},
            DIRECTED EDGE TYPE BaseRelationship {}
                CONNECTING (Person TO Person),
            DIRECTED EDGE TYPE Friendship INHERITS BaseRelationship
                CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Edge type inheritance should parse");
}

#[test]
fn test_complex_graph_type_with_multiple_features() {
    let source = r#"
        CREATE GRAPH TYPE complex AS {
            NODE TYPE Person LABEL Person {
                id :: INT NOT NULL,
                name :: STRING NOT NULL,
                email :: STRING,
                age :: INT
            },
            NODE TYPE Company LABEL Company {
                id :: INT NOT NULL,
                name :: STRING NOT NULL
            },
            DIRECTED EDGE TYPE WORKS_AT LABEL Employment {
                since :: DATE,
                position :: STRING
            } CONNECTING (Person TO Company),
            DIRECTED EDGE TYPE KNOWS {
                since :: DATE
            } CONNECTING (Person TO Person)
        }
    "#;

    let lexer = Lexer::new(source);
    let lex_result = lexer.tokenize();
    let parser = Parser::new(lex_result.tokens, source);
    let result = parser.parse();

    assert!(result.ast.is_some(), "Complex graph type should parse");
    // Test focuses on successful parsing
}
