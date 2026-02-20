//! Custom validation configuration demonstration
//!
//! This example shows how to configure the semantic validator with custom settings.

use gql_parser::diag::DiagSeverity;
use gql_parser::{
    parse,
    semantic::{SemanticValidator, ValidationConfig},
};

fn main() {
    println!("=== Custom Validation Configuration Demo ===\n");

    // Example 1: Default configuration
    demo_default_config();

    // Example 2: Strict mode
    demo_strict_mode();

    // Example 3: Custom configuration
    demo_custom_config();

    // Example 4: Builder pattern
    demo_builder_pattern();
}

fn demo_default_config() {
    println!("--- Example 1: Default Configuration ---");

    let validator = SemanticValidator::new();
    let source = "MATCH (a:Person), (b:Company) RETURN a, b";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let outcome = validator.validate(&ast);
        if outcome.is_success() {
            println!("✓ Valid (ISO-conformant disconnected patterns allowed)");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation errors:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_strict_mode() {
    println!("--- Example 2: Strict Mode ---");

    let validator = SemanticValidator::new().with_strict_mode(true);

    let source = "MATCH (n:Person) RETURN n";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let outcome = validator.validate(&ast);
        if outcome.is_success() {
            println!("✓ Valid in strict mode");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation errors (strict mode):");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_custom_config() {
    println!("--- Example 3: Custom Configuration ---");

    let config = ValidationConfig {
        strict_mode: true,
        schema_validation: false,  // No schema available
        catalog_validation: false, // No catalog available
        warn_on_shadowing: true,
        warn_on_disconnected_patterns: true,
        advanced_schema_validation: false,
        callable_validation: false,
    };

    let validator = SemanticValidator::with_config(config);
    let source = "MATCH (n:Person) LET n = n.name RETURN n"; // Variable shadowing

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let outcome = validator.validate(&ast);
        if outcome.is_success() {
            println!("✓ Valid");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation errors:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}

fn demo_builder_pattern() {
    println!("--- Example 4: Builder Pattern ---");

    // Build validator with chained methods
    let validator = SemanticValidator::new()
        .with_strict_mode(false)
        .with_schema_validation(false)
        .with_catalog_validation(false);

    let source = "MATCH (n:Person)-[:KNOWS]->(m:Person) RETURN n, m";

    let parse_result = parse(source);
    if let Some(ast) = parse_result.ast {
        let outcome = validator.validate(&ast);
        if outcome.is_success() {
            println!("✓ Valid query");
            println!("  Validation successful with custom configuration");
            if !outcome.diagnostics.is_empty() {
                println!("  Warnings:");
                for diag in &outcome.diagnostics {
                    if diag.severity == DiagSeverity::Warning {
                        println!("  - {}", diag.message);
                    }
                }
            }
        } else {
            println!("✗ Validation errors:");
            for diag in &outcome.diagnostics {
                if diag.severity == DiagSeverity::Error {
                    println!("  - {}", diag.message);
                }
            }
        }
    }
    println!();
}
