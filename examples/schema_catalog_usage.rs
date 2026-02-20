//! Example demonstrating Milestone 3: Advanced Schema Catalog System
//!
//! This example shows how to use the new schema catalog system for
//! schema-aware validation with property types, constraints, and inheritance.

use gql_parser::{
    parse,
    semantic::{
        schema_catalog::{
            InMemorySchemaCatalog, InMemorySchemaSnapshot, MockGraphContextResolver,
            MockVariableTypeContextProvider, SchemaSnapshotRequest, GraphRef,
            InMemorySchemaFixtureLoader, SchemaFixtureLoader, SchemaCatalog,
        },
        SemanticValidator,
    },
};

fn main() {
    println!("=== Milestone 3: Advanced Schema Catalog Example ===\n");

    // Example 1: Using InMemorySchemaCatalog with example schema
    example_1_basic_catalog();

    // Example 2: Using SchemaFixtureLoader with standard fixtures
    example_2_fixture_loader();

    // Example 3: Integrating with SemanticValidator
    example_3_validator_integration();
}

fn example_1_basic_catalog() {
    println!("Example 1: Basic Schema Catalog");
    println!("--------------------------------");

    // Create an in-memory catalog
    let mut catalog = InMemorySchemaCatalog::new();

    // Add a schema snapshot for a graph
    let snapshot = InMemorySchemaSnapshot::example();
    catalog.add_snapshot("social_graph".into(), snapshot);

    // Retrieve the snapshot
    let request = SchemaSnapshotRequest {
        graph: GraphRef { name: "social_graph".into() },
        schema: None,
    };

    match catalog.snapshot(request) {
        Ok(snapshot) => {
            println!("✓ Successfully retrieved schema snapshot for 'social_graph'");

            // Look up node type
            if let Some(person) = snapshot.node_type("Person") {
                println!("  - Found node type: {}", person.name);
                println!("  - Properties: {:?}", person.properties.keys().collect::<Vec<_>>());
            }

            // Look up edge type
            if let Some(knows) = snapshot.edge_type("KNOWS") {
                println!("  - Found edge type: {}", knows.name);
                println!("  - Properties: {:?}", knows.properties.keys().collect::<Vec<_>>());
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
        }
    }

    println!();
}

fn example_2_fixture_loader() {
    println!("Example 2: Schema Fixture Loader");
    println!("--------------------------------");

    // Create fixture loader with standard fixtures
    let loader = InMemorySchemaFixtureLoader::with_standard_fixtures();

    // Load social_graph fixture
    match loader.load("social_graph") {
        Ok(snapshot) => {
            println!("✓ Loaded 'social_graph' fixture");

            if let Some(person) = snapshot.node_type("Person") {
                println!("  - Person properties:");
                for (name, prop) in &person.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {} ({})", name, prop.name, required);
                }

                println!("  - Person constraints: {} defined", person.constraints.len());
            }
        }
        Err(e) => {
            eprintln!("✗ Error loading fixture: {}", e);
        }
    }

    // Load financial fixture
    match loader.load("financial") {
        Ok(snapshot) => {
            println!("✓ Loaded 'financial' fixture");

            if let Some(account) = snapshot.node_type("Account") {
                println!("  - Account properties:");
                for (name, prop) in &account.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {} ({})", name, prop.name, required);
                }
            }

            if let Some(transfer) = snapshot.edge_type("TRANSFER") {
                println!("  - Transfer properties:");
                for (name, prop) in &transfer.properties {
                    let required = if prop.required { "REQUIRED" } else { "OPTIONAL" };
                    println!("    - {}: {} ({})", name, prop.name, required);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Error loading fixture: {}", e);
        }
    }

    println!();
}

fn example_3_validator_integration() {
    println!("Example 3: Validator Integration");
    println!("--------------------------------");

    // Create schema catalog
    let mut catalog = InMemorySchemaCatalog::new();
    let snapshot = InMemorySchemaSnapshot::example();
    catalog.add_snapshot("social_graph".into(), snapshot);

    // Create graph context resolver
    let resolver = MockGraphContextResolver::new("social_graph", "default_schema");

    // Create variable type context provider
    let type_provider = MockVariableTypeContextProvider::new();

    // Create validator with advanced schema validation
    let validator = SemanticValidator::new()
        .with_schema_catalog(&catalog)
        .with_graph_context_resolver(&resolver)
        .with_variable_context_provider(&type_provider)
        .with_advanced_schema_validation(true);

    // Parse and validate a query
    let query = "MATCH (p:Person) RETURN p.name, p.age";
    println!("Validating query: {}", query);

    let parse_result = parse(query);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        if outcome.is_failure() {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                println!("  - {}: {}", diag.severity, diag.message);
            }
        } else {
            println!("✓ Validation succeeded!");
            if outcome.has_diagnostics() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    println!("  - {}: {}", diag.severity, diag.message);
                }
            }
        }
    } else {
        println!("✗ Parse failed with {} diagnostics", parse_result.diagnostics.len());
    }

    println!();
}
