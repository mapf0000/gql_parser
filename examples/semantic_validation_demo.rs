//! Semantic validation demonstration
//!
//! This example shows how to use the semantic validator to validate GQL queries.

use gql_parser::diag::DiagSeverity;
use gql_parser::{parse, semantic::SemanticValidator};

fn main() {
    println!("=== Semantic Validation Demo ===\n");

    // Example 1: Valid query
    demo_valid_query();

    // Example 2: Undefined variable
    demo_undefined_variable();

    // Example 3: Disconnected pattern
    demo_disconnected_pattern();

    // Example 4: Type mismatch
    demo_type_mismatch();

    // Example 5: With schema validation
    demo_schema_validation();
}

fn demo_valid_query() {
    println!("--- Example 1: Valid Query ---");
    let source = "MATCH (n:Person) RETURN n.name";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new();

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!("✓ Query is semantically valid");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_undefined_variable() {
    println!("--- Example 2: Undefined Variable ---");
    let source = "MATCH (n:Person) RETURN m"; // 'm' is undefined

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new();

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!("✓ Query is semantically valid");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_disconnected_pattern() {
    println!("--- Example 3: Disconnected Pattern (Warning) ---");
    let source = "MATCH (a:Person), (b:Company) RETURN a, b";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new();

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!(
                "✓ Query is semantically valid (ISO-conformant disconnected patterns allowed)"
            );
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_type_mismatch() {
    println!("--- Example 4: Type Mismatch ---");
    let source = "LET x = 'hello' + 10 RETURN x";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new();

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!("✓ Query is semantically valid");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_schema_validation() {
    println!("--- Example 5: Schema Validation ---");

    use gql_parser::semantic::metadata_provider::InMemoryMetadataProvider;

    // Create metadata provider with schema snapshot
    let metadata = InMemoryMetadataProvider::example();

    // Valid label
    let valid_source = "MATCH (n:Person) RETURN n";
    println!("Query: {}", valid_source);

    let parse_result = parse(valid_source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!("✓ Valid label (Person exists in schema)");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation failed:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }

    // Invalid label
    let invalid_source = "MATCH (n:Alien) RETURN n";
    println!("\nQuery: {}", invalid_source);

    let parse_result = parse(invalid_source);
    if let Some(ast) = parse_result.ast {
        let validator = SemanticValidator::new().with_metadata_provider(&metadata);

        let outcome = validator.validate(&ast);

        if outcome.is_success() {
            println!("✓ Query is semantically valid");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Invalid label (Alien not in schema):");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }

    println!();
}
