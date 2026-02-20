//! Integration tests for schema catalog system and MetadataProvider.

use gql_parser::{
    parse,
    semantic::{
        schema_catalog::{
            InMemorySchemaSnapshot, GraphRef, SchemaRef,
            NodeTypeMeta, PropertyMeta, TypeRef, ConstraintMeta, PropertyConstraint,
            SessionContext, SchemaSnapshot, SchemaSnapshotBuilder,
        },
        metadata_provider::{MockMetadataProvider, MetadataProvider},
        SemanticValidator,
    },
    ast::types::{ValueType, PredefinedType, CharacterStringType},
};
use std::collections::{BTreeMap, HashMap};

#[test]
fn test_schema_snapshot_node_types() {
    let snapshot = InMemorySchemaSnapshot::example();

    let person = snapshot.node_type("Person").unwrap();
    assert_eq!(person.name, "Person");
    assert!(person.properties.contains_key("name"));
    assert!(person.properties.contains_key("age"));

    let name_prop = &person.properties["name"];
    assert!(name_prop.required);
    assert_eq!(name_prop.name, "name");
}

#[test]
fn test_schema_snapshot_edge_types() {
    let snapshot = InMemorySchemaSnapshot::example();

    let knows = snapshot.edge_type("KNOWS").unwrap();
    assert_eq!(knows.name, "KNOWS");
    assert!(knows.properties.contains_key("since"));

    let since_prop = &knows.properties["since"];
    assert!(!since_prop.required);
    assert_eq!(since_prop.name, "since");
}

#[test]
fn test_schema_snapshot_property_lookup() {
    let snapshot = InMemorySchemaSnapshot::example();

    // Test property lookup by TypeRef
    let person_name = snapshot.property(
        TypeRef::NodeType("Person".into()),
        "name"
    );
    assert!(person_name.is_some());
    assert_eq!(person_name.unwrap().name, "name");

    // Test edge property lookup
    let knows_since = snapshot.property(
        TypeRef::EdgeType("KNOWS".into()),
        "since"
    );
    assert!(knows_since.is_some());
    assert_eq!(knows_since.unwrap().name, "since");

    // Test non-existent property
    let missing = snapshot.property(
        TypeRef::NodeType("Person".into()),
        "nonexistent"
    );
    assert!(missing.is_none());
}

#[test]
fn test_metadata_provider_with_standard_fixtures() {
    let provider = MockMetadataProvider::with_standard_fixtures();

    // Test social_graph fixture
    let social_graph = GraphRef { name: "social_graph".into() };
    let social = provider.get_schema_snapshot(&social_graph, None).unwrap();
    assert!(social.node_type("Person").is_some());
    assert!(social.edge_type("KNOWS").is_some());

    // Test Person properties
    let person = social.node_type("Person").unwrap();
    assert!(person.properties.contains_key("name"));
    assert!(person.properties.contains_key("age"));
    assert!(person.properties.contains_key("email"));

    // Test constraints
    assert_eq!(person.constraints.len(), 1);
    if let ConstraintMeta::PrimaryKey { properties } = &person.constraints[0] {
        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0], "name");
    } else {
        panic!("Expected PrimaryKey constraint");
    }
}

#[test]
fn test_metadata_provider_financial_fixture() {
    let provider = MockMetadataProvider::with_standard_fixtures();

    let financial_graph = GraphRef { name: "financial".into() };
    let financial = provider.get_schema_snapshot(&financial_graph, None).unwrap();
    assert!(financial.node_type("Account").is_some());
    assert!(financial.edge_type("TRANSFER").is_some());

    // Test Account properties
    let account = financial.node_type("Account").unwrap();
    assert!(account.properties.contains_key("account_id"));
    assert!(account.properties.contains_key("balance"));

    let account_id_prop = &account.properties["account_id"];
    assert!(account_id_prop.required);

    // Test TRANSFER properties
    let transfer = financial.edge_type("TRANSFER").unwrap();
    assert!(transfer.properties.contains_key("amount"));
    assert!(transfer.properties.contains_key("timestamp"));

    let amount_prop = &transfer.properties["amount"];
    assert!(amount_prop.required);
}

#[test]
fn test_metadata_provider_graph_validation() {
    let mut provider = MockMetadataProvider::new();
    let snapshot = InMemorySchemaSnapshot::example();
    provider.add_schema_snapshot("test_graph", snapshot);

    // Existing graph
    assert!(provider.validate_graph_exists("test_graph").is_ok());

    // Non-existent graph
    assert!(provider.validate_graph_exists("nonexistent").is_err());
}

#[test]
fn test_metadata_provider_resolve_active_graph() {
    let mut provider = MockMetadataProvider::new();
    let snapshot = InMemorySchemaSnapshot::example();
    provider.add_schema_snapshot("default", snapshot);

    // Empty session uses default
    let session = SessionContext::new();
    let graph = provider.resolve_active_graph(&session).unwrap();
    assert_eq!(graph.name, "default");

    // Session with active graph
    let mut session = SessionContext::new();
    session.active_graph = Some("custom".into());
    let graph = provider.resolve_active_graph(&session).unwrap();
    assert_eq!(graph.name, "custom");
}

#[test]
fn test_validator_with_metadata_provider() {
    // Create metadata provider
    let mut provider = MockMetadataProvider::new();
    let snapshot = InMemorySchemaSnapshot::example();
    provider.add_schema_snapshot("test_graph", snapshot);

    // Create validator with metadata provider
    let validator = SemanticValidator::new()
        .with_metadata_provider(&provider);

    // Parse a simple query
    let query = "MATCH (p:Person) RETURN p.name";
    let parse_result = parse(query);
    assert!(parse_result.ast.is_some());

    let program = parse_result.ast.unwrap();
    let outcome = validator.validate(&program);

    // Should succeed
    assert!(outcome.is_success());
}

#[test]
fn test_custom_schema_snapshot() {
    let mut snapshot = InMemorySchemaSnapshot::new();

    // Add custom node type using property builders
    let mut properties = BTreeMap::new();
    properties.insert("id".into(), PropertyMeta::int("id", true));

    snapshot.add_node_type(NodeTypeMeta {
        name: "CustomNode".into(),
        properties,
        constraints: vec![],
        parents: vec![],
        metadata: HashMap::new(),
    });

    // Verify the custom node type
    let custom = snapshot.node_type("CustomNode").unwrap();
    assert_eq!(custom.name, "CustomNode");
    assert!(custom.properties.contains_key("id"));
}

#[test]
fn test_constraints_and_parents() {
    let mut snapshot = InMemorySchemaSnapshot::new();

    let mut properties = BTreeMap::new();
    properties.insert("key".into(), PropertyMeta::string("key", true));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Entity".into(),
        properties,
        constraints: vec![
            ConstraintMeta::PrimaryKey {
                properties: vec!["key".into()],
            },
            ConstraintMeta::Unique {
                properties: vec!["key".into()],
            },
        ],
        parents: vec![],
        metadata: HashMap::new(),
    });

    let entity = snapshot.node_type("Entity").unwrap();
    assert_eq!(entity.constraints.len(), 2);

    let constraints = snapshot.constraints(TypeRef::NodeType("Entity".into()));
    assert_eq!(constraints.len(), 2);
}

#[test]
fn test_session_context() {
    let ctx = SessionContext::new();
    assert!(ctx.active_graph.is_none());
    assert!(ctx.active_schema.is_none());

    let ctx2 = SessionContext {
        active_graph: Some("mygraph".into()),
        active_schema: Some("myschema".into()),
    };
    assert_eq!(ctx2.active_graph.as_ref().unwrap(), "mygraph");
    assert_eq!(ctx2.active_schema.as_ref().unwrap(), "myschema");
}

#[test]
fn test_type_ref_equality_and_hashing() {
    use std::collections::HashSet;

    let node1 = TypeRef::NodeType("Person".into());
    let node2 = TypeRef::NodeType("Person".into());
    let node3 = TypeRef::NodeType("Company".into());
    let edge1 = TypeRef::EdgeType("KNOWS".into());

    assert_eq!(node1, node2);
    assert_ne!(node1, node3);
    assert_ne!(node1, edge1);

    let mut set = HashSet::new();
    set.insert(node1.clone());
    set.insert(node2.clone());
    assert_eq!(set.len(), 1); // node1 and node2 are equal

    set.insert(edge1);
    assert_eq!(set.len(), 2);
}

#[test]
fn test_schema_snapshot_builder() {
    let snapshot = SchemaSnapshotBuilder::new()
        .with_node_type("User", |builder| {
            builder
                .add_property(PropertyMeta::string("username", true))
                .add_property(PropertyMeta::string("email", true))
                .add_constraint(ConstraintMeta::PrimaryKey {
                    properties: vec!["username".into()],
                })
        })
        .with_edge_type("FOLLOWS", |builder| {
            builder.add_property(PropertyMeta::datetime("since", true))
        })
        .build();

    assert!(snapshot.node_type("User").is_some());
    assert!(snapshot.edge_type("FOLLOWS").is_some());

    let user = snapshot.node_type("User").unwrap();
    assert_eq!(user.properties.len(), 2);
    assert_eq!(user.constraints.len(), 1);
}

#[test]
fn test_extended_fixtures() {
    let provider = MockMetadataProvider::with_extended_fixtures();

    // Test e-commerce fixture
    let ecommerce_graph = GraphRef { name: "ecommerce".into() };
    let ecommerce = provider.get_schema_snapshot(&ecommerce_graph, None).unwrap();
    assert!(ecommerce.node_type("Product").is_some());
    assert!(ecommerce.node_type("Customer").is_some());
    assert!(ecommerce.node_type("Order").is_some());
    assert!(ecommerce.edge_type("CONTAINS").is_some());

    let product = ecommerce.node_type("Product").unwrap();
    assert!(product.properties.contains_key("product_id"));
    assert!(product.properties.contains_key("price"));
    assert!(product.properties.contains_key("stock_quantity"));

    // Test healthcare fixture
    let healthcare_graph = GraphRef { name: "healthcare".into() };
    let healthcare = provider.get_schema_snapshot(&healthcare_graph, None).unwrap();
    assert!(healthcare.node_type("Patient").is_some());
    assert!(healthcare.node_type("Doctor").is_some());
    assert!(healthcare.node_type("Appointment").is_some());

    let patient = healthcare.node_type("Patient").unwrap();
    assert!(patient.properties.contains_key("patient_id"));
    assert!(patient.properties.contains_key("blood_type"));
}

#[test]
fn test_property_inheritance() {
    let mut snapshot = InMemorySchemaSnapshot::new();

    // Create a base Entity type
    let mut base_props = BTreeMap::new();
    base_props.insert("id".into(), PropertyMeta::string("id", true));
    base_props.insert("created_at".into(), PropertyMeta::datetime("created_at", true));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Entity".into(),
        properties: base_props,
        constraints: vec![],
        parents: vec![],
        metadata: HashMap::new(),
    });

    // Create a Person type that inherits from Entity
    let mut person_props = BTreeMap::new();
    person_props.insert("name".into(), PropertyMeta::string("name", true));
    person_props.insert("email".into(), PropertyMeta::string("email", false));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Person".into(),
        properties: person_props,
        constraints: vec![],
        parents: vec![TypeRef::NodeType("Entity".into())],
        metadata: HashMap::new(),
    });

    // Test direct properties
    assert!(snapshot.property(TypeRef::NodeType("Person".into()), "name").is_some());
    assert!(snapshot.property(TypeRef::NodeType("Person".into()), "email").is_some());

    // Test inherited properties
    assert!(snapshot.property(TypeRef::NodeType("Person".into()), "id").is_some());
    assert!(snapshot.property(TypeRef::NodeType("Person".into()), "created_at").is_some());

    // Test non-existent property
    assert!(snapshot.property(TypeRef::NodeType("Person".into()), "nonexistent").is_none());
}

#[test]
fn test_property_meta_builders() {
    let string_prop = PropertyMeta::string("name", true);
    assert!(string_prop.required);
    assert_eq!(string_prop.name, "name");
    assert!(string_prop.constraints.is_empty());

    let int_prop = PropertyMeta::int("age", false);
    assert!(!int_prop.required);

    let decimal_prop = PropertyMeta::decimal("price", true, 10, 2);
    assert!(decimal_prop.required);

    let date_prop = PropertyMeta::date("birth_date", false);
    assert!(!date_prop.required);

    let datetime_prop = PropertyMeta::datetime("created_at", true);
    assert!(datetime_prop.required);

    // Test with_constraint
    let unique_prop = PropertyMeta::string("email", true)
        .with_constraint(PropertyConstraint::Unique);
    assert_eq!(unique_prop.constraints.len(), 1);
}

#[test]
fn test_deterministic_property_ordering() {
    // Properties should be ordered by name for deterministic iteration
    let snapshot = InMemorySchemaSnapshot::example();
    let person = snapshot.node_type("Person").unwrap();

    let keys: Vec<_> = person.properties.keys().cloned().collect();
    // BTreeMap ensures sorted order
    assert_eq!(keys, vec!["age", "name"]);
}
