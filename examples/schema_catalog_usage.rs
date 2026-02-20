//! Example demonstrating the Schema Catalog System with MetadataProvider.
//!
//! This example shows how to use the metadata provider for schema-aware
//! validation with property types, constraints, and inheritance.

use gql_parser::{
    parse,
    semantic::{
        schema_catalog::{
            InMemorySchemaSnapshot, GraphRef,
        },
        metadata_provider::{MockMetadataProvider, MetadataProvider},
        SemanticValidator,
    },
};

fn main() {
    println!("=== Schema Catalog with MetadataProvider Example ===\n");

    // Example 1: Using MockMetadataProvider with example schema
    example_1_basic_metadata_provider();

    // Example 2: Using standard fixtures
    example_2_standard_fixtures();

    // Example 3: Integrating with SemanticValidator
    example_3_validator_integration();
}

fn example_1_basic_metadata_provider() {
    println!("Example 1: Basic Metadata Provider");
    println!("-----------------------------------");

    // Create a metadata provider
    let mut provider = MockMetadataProvider::new();

    // Add a schema snapshot for a graph
    let snapshot = InMemorySchemaSnapshot::example();
    provider.add_schema_snapshot("social_graph", snapshot);

    // Retrieve the snapshot
    let graph = GraphRef { name: "social_graph".into() };

    match provider.get_schema_snapshot(&graph, None) {
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

fn example_2_standard_fixtures() {
    println!("Example 2: Standard Fixtures");
    println!("----------------------------");

    // Create provider with standard fixtures
    let provider = MockMetadataProvider::with_standard_fixtures();

    // Access social_graph fixture
    let graph = GraphRef { name: "social_graph".into() };
    match provider.get_schema_snapshot(&graph, None) {
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

    // Access financial fixture
    let graph = GraphRef { name: "financial".into() };
    match provider.get_schema_snapshot(&graph, None) {
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

    // Create metadata provider
    let mut provider = MockMetadataProvider::new();
    let snapshot = InMemorySchemaSnapshot::example();
    provider.add_schema_snapshot("social_graph", snapshot);

    // Create validator with metadata provider
    let validator = SemanticValidator::new()
        .with_metadata_provider(&provider);

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
