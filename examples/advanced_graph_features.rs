//! Example demonstrating advanced Milestone 3 features:
//! - SchemaSnapshotBuilder for easy schema creation
//! - Extended fixtures (e-commerce, healthcare)
//! - Property inheritance

use gql_parser::semantic::schema_catalog::{
    InMemorySchemaFixtureLoader, SchemaFixtureLoader, SchemaSnapshotBuilder, SchemaSnapshot,
    PropertyMeta, ConstraintMeta, TypeRef, PropertyConstraint,
    InMemorySchemaSnapshot, NodeTypeMeta,
};
use std::collections::{BTreeMap, HashMap};

fn main() {
    println!("=== Milestone 3: Advanced Features Example ===\n");

    // Example 1: Using SchemaSnapshotBuilder
    example_1_schema_builder();

    // Example 2: Extended fixtures (e-commerce, healthcare)
    example_2_extended_fixtures();

    // Example 3: Property inheritance
    example_3_property_inheritance();
}

fn example_1_schema_builder() {
    println!("Example 1: Schema Builder");
    println!("-------------------------");

    // Create a schema using the fluent builder API
    let snapshot = SchemaSnapshotBuilder::new()
        .with_node_type("User", |builder| {
            builder
                .add_property(PropertyMeta::string("user_id", true)
                    .with_constraint(PropertyConstraint::Unique))
                .add_property(PropertyMeta::string("username", true))
                .add_property(PropertyMeta::string("email", true)
                    .with_constraint(PropertyConstraint::Unique))
                .add_property(PropertyMeta::datetime("created_at", true))
                .add_constraint(ConstraintMeta::PrimaryKey {
                    properties: vec!["user_id".into()],
                })
        })
        .with_node_type("Post", |builder| {
            builder
                .add_property(PropertyMeta::string("post_id", true))
                .add_property(PropertyMeta::string("title", true))
                .add_property(PropertyMeta::string("content", false))
                .add_property(PropertyMeta::datetime("published_at", true))
                .add_property(PropertyMeta::int("likes", false))
                .add_constraint(ConstraintMeta::PrimaryKey {
                    properties: vec!["post_id".into()],
                })
        })
        .with_edge_type("AUTHORED", |builder| {
            builder.add_property(PropertyMeta::datetime("timestamp", true))
        })
        .with_edge_type("LIKES", |builder| {
            builder.add_property(PropertyMeta::datetime("liked_at", true))
        })
        .build();

    println!("✓ Created schema with builder");
    println!("  - Node types: User, Post");
    println!("  - Edge types: AUTHORED, LIKES");

    if let Some(user) = snapshot.node_type("User") {
        println!("  - User has {} properties", user.properties.len());
        println!("  - User has {} constraints", user.constraints.len());
    }

    println!();
}

fn example_2_extended_fixtures() {
    println!("Example 2: Extended Fixtures");
    println!("----------------------------");

    let loader = InMemorySchemaFixtureLoader::with_extended_fixtures();

    // E-commerce fixture
    println!("E-commerce Schema:");
    match loader.load("ecommerce") {
        Ok(snapshot) => {
            println!("✓ Loaded e-commerce fixture");

            if let Some(product) = snapshot.node_type("Product") {
                println!("  Product properties:");
                for (name, prop) in &product.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {}", name, required);
                }
            }

            if let Some(customer) = snapshot.node_type("Customer") {
                println!("  Customer properties:");
                for (name, prop) in &customer.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {}", name, required);
                }
            }
        }
        Err(e) => eprintln!("✗ Error: {}", e),
    }

    println!();

    // Healthcare fixture
    println!("Healthcare Schema:");
    match loader.load("healthcare") {
        Ok(snapshot) => {
            println!("✓ Loaded healthcare fixture");

            if let Some(patient) = snapshot.node_type("Patient") {
                println!("  Patient properties:");
                for (name, prop) in &patient.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {}", name, required);
                }
            }

            if let Some(doctor) = snapshot.node_type("Doctor") {
                println!("  Doctor properties:");
                for (name, prop) in &doctor.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {}", name, required);
                }
            }
        }
        Err(e) => eprintln!("✗ Error: {}", e),
    }

    println!();
}

fn example_3_property_inheritance() {
    println!("Example 3: Property Inheritance");
    println!("-------------------------------");

    let mut snapshot = InMemorySchemaSnapshot::new();

    // Create base Entity type
    let mut entity_props = BTreeMap::new();
    entity_props.insert("id".into(), PropertyMeta::string("id", true)
        .with_constraint(PropertyConstraint::Unique));
    entity_props.insert("created_at".into(), PropertyMeta::datetime("created_at", true));
    entity_props.insert("updated_at".into(), PropertyMeta::datetime("updated_at", false));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Entity".into(),
        properties: entity_props,
        constraints: vec![ConstraintMeta::PrimaryKey {
            properties: vec!["id".into()],
        }],
        parents: vec![],
        metadata: HashMap::new(),
    });

    // Create Person that inherits from Entity
    let mut person_props = BTreeMap::new();
    person_props.insert("name".into(), PropertyMeta::string("name", true));
    person_props.insert("email".into(), PropertyMeta::string("email", false)
        .with_constraint(PropertyConstraint::Unique));
    person_props.insert("age".into(), PropertyMeta::int("age", false));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Person".into(),
        properties: person_props,
        constraints: vec![],
        parents: vec![TypeRef::NodeType("Entity".into())],
        metadata: HashMap::new(),
    });

    // Create Company that inherits from Entity
    let mut company_props = BTreeMap::new();
    company_props.insert("name".into(), PropertyMeta::string("name", true));
    company_props.insert("industry".into(), PropertyMeta::string("industry", false));

    snapshot.add_node_type(NodeTypeMeta {
        name: "Company".into(),
        properties: company_props,
        constraints: vec![],
        parents: vec![TypeRef::NodeType("Entity".into())],
        metadata: HashMap::new(),
    });

    println!("✓ Created schema with inheritance");
    println!("  Base type: Entity");
    println!("  Derived types: Person, Company");
    println!();

    // Demonstrate property lookup with inheritance
    println!("Testing property inheritance:");

    // Person's own properties
    if snapshot.property(TypeRef::NodeType("Person".into()), "name").is_some() {
        println!("  ✓ Person has 'name' (direct property)");
    }

    if snapshot.property(TypeRef::NodeType("Person".into()), "email").is_some() {
        println!("  ✓ Person has 'email' (direct property)");
    }

    // Inherited properties from Entity
    if snapshot.property(TypeRef::NodeType("Person".into()), "id").is_some() {
        println!("  ✓ Person has 'id' (inherited from Entity)");
    }

    if snapshot.property(TypeRef::NodeType("Person".into()), "created_at").is_some() {
        println!("  ✓ Person has 'created_at' (inherited from Entity)");
    }

    if snapshot.property(TypeRef::NodeType("Person".into()), "updated_at").is_some() {
        println!("  ✓ Person has 'updated_at' (inherited from Entity)");
    }

    println!();

    // Company also inherits from Entity
    if snapshot.property(TypeRef::NodeType("Company".into()), "id").is_some() {
        println!("  ✓ Company has 'id' (inherited from Entity)");
    }

    if snapshot.property(TypeRef::NodeType("Company".into()), "name").is_some() {
        println!("  ✓ Company has 'name' (direct property)");
    }

    println!();
}
