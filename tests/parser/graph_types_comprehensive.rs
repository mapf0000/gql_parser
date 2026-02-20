//! Comprehensive graph type parser tests for ISO GQL compliance
//!
//! This test file provides extensive coverage of edge cases and ensures
//! future maintainability by documenting correct ISO GQL syntax.

use gql_parser::parse;

// ============================================================================
// Multiple Inheritance Tests
// ============================================================================

#[test]
fn test_multiple_inheritance_single_parent() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Base,
            NODE TYPE Derived INHERITS Base
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse single inheritance");
    assert!(result.ast.is_some());
}

#[test]
fn test_multiple_inheritance_two_parents() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Base1,
            NODE TYPE Base2,
            NODE TYPE Derived INHERITS Base1, Base2
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse multiple inheritance");
    assert!(result.ast.is_some());
}

#[test]
fn test_multiple_inheritance_three_parents() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE A,
            NODE TYPE B,
            NODE TYPE C,
            NODE TYPE D INHERITS A, B, C
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse three-parent inheritance");
    assert!(result.ast.is_some());
}

#[test]
fn test_inheritance_followed_by_another_type() {
    // Critical test: comma after inheritance parent should not consume next element type
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE A,
            NODE TYPE B INHERITS A,
            NODE TYPE C
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(),
            "Parser should correctly distinguish inheritance commas from element type commas");
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

    assert_eq!(specification.body.element_types.types.len(), 3,
               "Should have exactly 3 node types, not treat C as an inheritance parent");
}

// ============================================================================
// Constraint Placement Tests
// ============================================================================

#[test]
fn test_constraint_after_property_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person { id :: INT, name :: STRING } CONSTRAINT UNIQUE (id)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Constraints come AFTER property types");
    assert!(result.ast.is_some());
}

#[test]
fn test_multiple_constraints_after_property_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person {
                id :: INT,
                age :: INT
            }
            CONSTRAINT UNIQUE (id)
            CONSTRAINT CHECK (age >= 0)
            CONSTRAINT CHECK (age <= 150)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Multiple constraints after property types");
    assert!(result.ast.is_some());
}

#[test]
fn test_constraint_without_property_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person CONSTRAINT UNIQUE (id)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Constraints without property types block");
    assert!(result.ast.is_some());
}

#[test]
fn test_constraint_with_label_and_properties() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person LABEL Person { id :: INT } CONSTRAINT UNIQUE (id)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Constraints with labels and properties");
    assert!(result.ast.is_some());
}

// ============================================================================
// Label Set Tests
// ============================================================================

#[test]
fn test_single_label() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person LABEL Person { name :: STRING }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Single label");
    assert!(result.ast.is_some());
}

#[test]
fn test_multiple_labels_with_ampersand() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person LABELS Person & Employee { id :: INT, name :: STRING }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Multiple labels using & syntax");
    assert!(result.ast.is_some());
}

#[test]
fn test_three_labels_with_ampersand() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Entity LABELS Person & Employee & Manager
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Three labels using & syntax");
    assert!(result.ast.is_some());
}

// ============================================================================
// Edge Type Tests
// ============================================================================

#[test]
fn test_directed_edge_basic() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Basic directed edge");
    assert!(result.ast.is_some());
}

#[test]
fn test_undirected_edge_basic() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            UNDIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Basic undirected edge");
    assert!(result.ast.is_some());
}

#[test]
fn test_edge_with_properties() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            DIRECTED EDGE TYPE KNOWS { since :: DATE } CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Edge with properties");
    assert!(result.ast.is_some());
}

#[test]
fn test_edge_with_label_and_properties() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            DIRECTED EDGE TYPE RELATIONSHIP LABEL KNOWS { since :: DATE }
                CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Edge with label and properties");
    assert!(result.ast.is_some());
}

#[test]
fn test_edge_inheritance() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            DIRECTED EDGE TYPE Relationship CONNECTING (Person TO Person),
            DIRECTED EDGE TYPE KNOWS INHERITS Relationship CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Edge with inheritance");
    assert!(result.ast.is_some());
}

// ============================================================================
// Complex Scenarios
// ============================================================================

#[test]
fn test_comprehensive_graph_type() {
    let source = r#"
        CREATE GRAPH TYPE social_network AS {
            NODE TYPE Entity,
            NODE TYPE Person INHERITS Entity
                LABEL Person {
                    id :: INT,
                    name :: STRING,
                    email :: STRING,
                    age :: INT
                }
                CONSTRAINT UNIQUE (id)
                CONSTRAINT UNIQUE (email)
                CONSTRAINT CHECK (age >= 0)
                CONSTRAINT CHECK (age <= 150),
            NODE TYPE Company INHERITS Entity
                LABEL Company {
                    id :: INT,
                    name :: STRING
                }
                CONSTRAINT UNIQUE (id),
            DIRECTED EDGE TYPE WORKS_AT
                LABEL WORKS_AT {
                    since :: DATE,
                    position :: STRING
                }
                CONNECTING (Person TO Company),
            DIRECTED EDGE TYPE KNOWS
                CONNECTING (Person TO Person),
            UNDIRECTED EDGE TYPE PARTNERS_WITH
                CONNECTING (Company TO Company)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Comprehensive graph type with all features");
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

    assert_eq!(specification.body.element_types.types.len(), 6,
               "Should have 3 node types and 3 edge types");
}

#[test]
fn test_abstract_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            ABSTRACT NODE TYPE Entity,
            NODE TYPE Person INHERITS Entity,
            ABSTRACT DIRECTED EDGE TYPE Relationship CONNECTING (Entity TO Entity),
            DIRECTED EDGE TYPE KNOWS INHERITS Relationship CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Abstract node and edge types");
    assert!(result.ast.is_some());
}

#[test]
fn test_empty_property_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person { },
            DIRECTED EDGE TYPE KNOWS { } CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Empty property type specifications");
    assert!(result.ast.is_some());
}

#[test]
fn test_property_types_with_not_null() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person {
                id :: INT NOT NULL,
                name :: STRING NOT NULL,
                age :: INT
            }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Property types with NOT NULL");
    assert!(result.ast.is_some());
}

#[test]
fn test_trailing_comma_in_element_types() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person,
            NODE TYPE Company,
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Trailing comma after last element type");
    assert!(result.ast.is_some());
}

#[test]
fn test_key_label_set() {
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person
                LABEL Person { id :: INT, name :: STRING }
                KEY Person
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "KEY label set specification");
    assert!(result.ast.is_some());
}

// ============================================================================
// Error Cases (should fail gracefully)
// ============================================================================

#[test]
fn test_invalid_constraint_inside_property_block() {
    // This is INVALID ISO GQL syntax - constraints must be AFTER the { } block
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person {
                id :: INT,
                CONSTRAINT UNIQUE (id)
            }
        }
    "#;

    let result = parse(source);
    // Should have diagnostics because CONSTRAINT is not a valid property name
    assert!(!result.diagnostics.is_empty() || result.ast.is_none(),
            "Parser correctly rejects constraints inside property types block");
}

#[test]
fn test_multiple_label_clauses_not_supported() {
    // This is INVALID ISO GQL syntax - use LABELS with & instead
    let source = r#"
        CREATE GRAPH TYPE test AS {
            NODE TYPE Person
                LABEL Employee
                LABEL Manager
        }
    "#;

    let result = parse(source);
    // Should fail or have diagnostics
    assert!(!result.diagnostics.is_empty() || result.ast.is_none(),
            "Parser correctly rejects multiple LABEL clauses (use LABELS & instead)");
}
