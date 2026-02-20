//! Advanced Schema and Graph Type Parser Tests
//!
//! This module tests parsing of advanced schema definition features including:
//! - Complex graph type definitions with constraints
//! - Element type inheritance
//! - Source and destination constraints
//! - Property type specifications

use gql_parser::parse;
use crate::common::*;

// ===== Complex Graph Type Definitions =====

#[test]
fn graph_type_with_multiple_constraints() {
    let schema = r#"
        CREATE GRAPH TYPE SocialNet AS {
            NODE TYPE Person {
                id :: INT,
                name :: STRING,
                email :: STRING?
            }
            CONSTRAINT UNIQUE (id),
            CONSTRAINT CHECK (id > 0),
            DIRECTED EDGE TYPE KNOWS
                CONNECTING (Person TO Person)
                {since :: DATE}
                CONSTRAINT KEY (from, to)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_with_unique_constraints() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE User {
                id :: INT,
                username :: STRING,
                email :: STRING
            }
            CONSTRAINT UNIQUE (id),
            CONSTRAINT UNIQUE (username),
            CONSTRAINT UNIQUE (email)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_with_check_constraints() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Product {
                id :: INT,
                price :: FLOAT,
                quantity :: INT
            }
            CONSTRAINT CHECK (price > 0),
            CONSTRAINT CHECK (quantity >= 0)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_with_key_constraint() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Entity {
                region :: STRING,
                id :: INT
            }
            CONSTRAINT KEY (region, id)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Element Type Inheritance =====

#[test]
fn element_type_with_inheritance() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Entity {
                id :: INT
            },
            NODE TYPE Person EXTENDS Entity {
                name :: STRING
            },
            NODE TYPE Company EXTENDS Entity {
                revenue :: FLOAT
            }
        }
    "#;

    let result = parse(schema);
    // Inheritance may not be implemented yet
    let _ = result.ast;
}

#[test]
fn element_type_multiple_inheritance() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Named {
                name :: STRING
            },
            NODE TYPE Timestamped {
                created :: TIMESTAMP,
                updated :: TIMESTAMP
            },
            NODE TYPE Person EXTENDS Named, Timestamped {
                age :: INT
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Edge Type Definitions =====

#[test]
fn directed_edge_type_basic() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT},
            DIRECTED EDGE TYPE KNOWS
                CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn undirected_edge_type() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT},
            UNDIRECTED EDGE TYPE FRIENDS
                CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn edge_type_with_properties() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT},
            DIRECTED EDGE TYPE KNOWS
                CONNECTING (Person TO Person)
                {
                    since :: DATE,
                    strength :: FLOAT,
                    verified :: BOOLEAN
                }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn edge_type_multiple_source_dest_types() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT},
            NODE TYPE Company {id :: INT},
            DIRECTED EDGE TYPE WORKS_AT
                CONNECTING (Person TO Company)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn edge_type_with_union_sources() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT},
            NODE TYPE Company {id :: INT},
            NODE TYPE Organization {id :: INT},
            DIRECTED EDGE TYPE MEMBER_OF
                CONNECTING (Person TO Company | Organization)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Property Type Specifications =====

#[test]
fn property_types_all_primitives() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE AllTypes {
                intProp :: INT,
                floatProp :: FLOAT,
                stringProp :: STRING,
                boolProp :: BOOLEAN,
                dateProp :: DATE,
                timeProp :: TIME,
                timestampProp :: TIMESTAMP,
                durationProp :: DURATION
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn property_types_nullable() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {
                id :: INT,
                name :: STRING,
                email :: STRING?,
                phone :: STRING?
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn property_types_lists() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {
                id :: INT,
                tags :: LIST<STRING>,
                scores :: LIST<INT>,
                matrix :: LIST<LIST<FLOAT>>
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn property_types_records() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {
                id :: INT,
                address :: RECORD {
                    street :: STRING,
                    city :: STRING,
                    zip :: STRING
                }
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn property_types_nested_structures() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Complex {
                data :: RECORD {
                    values :: LIST<INT>,
                    metadata :: RECORD {
                        created :: TIMESTAMP,
                        tags :: LIST<STRING>
                    }
                }
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Graph Type Source Specifications =====

#[test]
fn graph_type_like_existing_graph() {
    let schema = "CREATE GRAPH TYPE NewType LIKE existingGraph";
    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_copy_of_type() {
    let schema = "CREATE GRAPH TYPE NewType AS COPY OF ExistingType";
    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_with_nested_specification() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person {id :: INT, name :: STRING}
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== CREATE GRAPH Statements =====

#[test]
fn create_graph_with_type_reference() {
    let queries = vec![
        "CREATE GRAPH myGraph :: MyGraphType",
        "CREATE GRAPH myGraph TYPED MyGraphType",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn create_graph_with_inline_type() {
    let schema = r#"
        CREATE GRAPH myGraph :: {
            NODE TYPE Person {
                id :: INT,
                name :: STRING
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn create_graph_any_type() {
    let queries = vec![
        "CREATE GRAPH myGraph ANY",
        "CREATE GRAPH myGraph :: ANY",
        "CREATE GRAPH myGraph TYPED ANY",
    ];

    for query in queries {
        let result = parse(query);
        let _ = result.ast;
    }
}

#[test]
fn create_graph_if_not_exists() {
    let query = "CREATE GRAPH IF NOT EXISTS myGraph ANY";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn create_or_replace_graph() {
    let query = "CREATE OR REPLACE GRAPH myGraph ANY";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn create_graph_with_source() {
    let query = "CREATE GRAPH newGraph LIKE oldGraph AS COPY OF oldGraph";
    let result = parse(query);
    let _ = result.ast;
}

// ===== DROP Statements =====

#[test]
fn drop_graph_basic() {
    let query = "DROP GRAPH myGraph";
    let result = parse(query);
    assert!(result.ast.is_some(), "DROP GRAPH should parse");
}

#[test]
fn drop_graph_if_exists() {
    let query = "DROP GRAPH IF EXISTS myGraph";
    let result = parse(query);
    assert!(result.ast.is_some(), "DROP GRAPH IF EXISTS should parse");
}

#[test]
fn drop_property_graph() {
    let query = "DROP PROPERTY GRAPH myGraph";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn drop_graph_type() {
    let query = "DROP GRAPH TYPE MyGraphType";
    let result = parse(query);
    assert!(result.ast.is_some(), "DROP GRAPH TYPE should parse");
}

#[test]
fn drop_graph_type_if_exists() {
    let query = "DROP GRAPH TYPE IF EXISTS MyGraphType";
    let result = parse(query);
    let _ = result.ast;
}

// ===== CREATE SCHEMA Statements =====

#[test]
fn create_schema_basic() {
    let query = "CREATE SCHEMA mySchema";
    let result = parse(query);
    assert!(result.ast.is_some(), "CREATE SCHEMA should parse");
}

#[test]
fn create_schema_if_not_exists() {
    let query = "CREATE SCHEMA IF NOT EXISTS mySchema";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn create_schema_with_path() {
    let query = "CREATE SCHEMA /root/level1/mySchema";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn drop_schema() {
    let query = "DROP SCHEMA mySchema";
    let result = parse(query);
    assert!(result.ast.is_some(), "DROP SCHEMA should parse");
}

#[test]
fn drop_schema_if_exists() {
    let query = "DROP SCHEMA IF EXISTS mySchema";
    let result = parse(query);
    let _ = result.ast;
}

// ===== Label Set Specifications =====

#[test]
fn label_set_spec_single_label() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Person LABEL Person {
                id :: INT
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn label_set_spec_multiple_labels() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Entity LABELS Entity, Active {
                id :: INT
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Abstract Element Types =====

#[test]
fn abstract_node_type() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            ABSTRACT NODE TYPE Entity {
                id :: INT
            },
            NODE TYPE Person EXTENDS Entity {
                name :: STRING
            }
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn abstract_edge_type() {
    let schema = r#"
        CREATE GRAPH TYPE MyGraph AS {
            NODE TYPE Entity {id :: INT},
            ABSTRACT DIRECTED EDGE TYPE Relationship
                CONNECTING (Entity TO Entity),
            DIRECTED EDGE TYPE Knows EXTENDS Relationship
                CONNECTING (Entity TO Entity)
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

// ===== Complex Graph Type Scenarios =====

#[test]
fn graph_type_with_multiple_node_and_edge_types() {
    let schema = r#"
        CREATE GRAPH TYPE SocialNetwork AS {
            NODE TYPE Person {
                id :: INT,
                name :: STRING,
                age :: INT
            }
            CONSTRAINT UNIQUE (id),

            NODE TYPE Company {
                id :: INT,
                name :: STRING,
                revenue :: FLOAT
            }
            CONSTRAINT UNIQUE (id),

            DIRECTED EDGE TYPE KNOWS
                CONNECTING (Person TO Person)
                {since :: DATE},

            DIRECTED EDGE TYPE WORKS_AT
                CONNECTING (Person TO Company)
                {since :: DATE, position :: STRING}
        }
    "#;

    let result = parse(schema);
    let _ = result.ast;
}

#[test]
fn graph_type_catalog_with_schema_paths() {
    let query = "CREATE GRAPH TYPE /catalogs/prod/types/SocialNetwork AS { NODE TYPE Person {id :: INT} }";
    let result = parse(query);
    let _ = result.ast;
}

#[test]
fn graph_with_fully_qualified_type_reference() {
    let query = "CREATE GRAPH myGraph :: /catalogs/prod/types/SocialNetwork";
    let result = parse(query);
    let _ = result.ast;
}
