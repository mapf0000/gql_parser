//! Semantic validation tests for procedure statements.
//!
//! Tests procedure signature matching, YIELD validation, and
//! variable scoping in inline procedures.

use gql_parser::parse;
use gql_parser::semantic::validator::{SemanticValidator, ValidationConfig};
use gql_parser::semantic::metadata_provider::MetadataProvider;
use gql_parser::ir::ValidationOutcome;
use gql_parser::semantic::callable::{
    CallableCatalog, CallableSignature, CallableKind,
    ParameterSignature, InMemoryCallableCatalog, Volatility, Nullability,
};
use gql_parser::semantic::metadata_provider::InMemoryMetadataProvider;
use smol_str::SmolStr;

fn validate_with_procedures(source: &str, catalog: &impl MetadataProvider)
    -> ValidationOutcome
{
    use gql_parser::ast::program::Statement;

    let parse_result = parse(source);
    eprintln!("Parse result AST is_some: {}", parse_result.ast.is_some());
    if let Some(ref program) = parse_result.ast {
        eprintln!("Program statements count: {}", program.statements.len());
        for (i, stmt) in program.statements.iter().enumerate() {
            eprintln!("Statement {}: {:?}", i, match stmt {
                Statement::Query(_) => "Query",
                Statement::Mutation(_) => "Mutation",
                Statement::Session(_) => "Session",
                Statement::Transaction(_) => "Transaction",
                Statement::Catalog(_) => "Catalog",
                Statement::Empty(_) => "Empty",
            });
        }
    }
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let config = ValidationConfig {
        metadata_validation: true, // Enable metadata validation for callable checks
        ..Default::default()
    };
    let validator = SemanticValidator::with_config(config).with_metadata_provider(catalog);
    validator.validate(parse_result.ast.as_ref().unwrap())
}

// ===== Test 1-5: Procedure Existence & Arguments =====

#[test]
fn test_builtin_procedure_validates() {
    let source = "CALL abs(-5) RETURN 1";

    // Use metadata provider which has built-ins
    let catalog = InMemoryMetadataProvider::new();
    let outcome = validate_with_procedures(source, &catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_unknown_procedure_fails_with_validation_enabled() {
    let source = "CALL nonexistent_procedure() RETURN 1";

    let catalog = InMemoryCallableCatalog::new();
    let outcome = validate_with_procedures(source, &catalog);

    // Debug output
    eprintln!("=== Test: unknown procedure ===");
    eprintln!("Is success: {}", outcome.is_success());
    eprintln!("Diagnostics count: {}", outcome.diagnostics.len());
    for diag in &outcome.diagnostics {
        eprintln!("  - {:?}: {}", diag.severity, diag.message);
    }

    // Should fail - procedure doesn't exist
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d| d.message.contains("not found")),
            "Expected undefined procedure diagnostic");
}

#[test]
fn test_procedure_with_correct_arity_validates() {
    let mut catalog = InMemoryCallableCatalog::new();

    // Register a procedure that takes 2 arguments
    catalog.register(CallableSignature {
        name: SmolStr::new("my_proc"),
        kind: CallableKind::Procedure,
        parameters: vec![
            ParameterSignature {
                name: SmolStr::new("arg1"),
                param_type: SmolStr::new("ANY"),
                optional: false,
                variadic: false,
            },
            ParameterSignature {
                name: SmolStr::new("arg2"),
                param_type: SmolStr::new("ANY"),
                optional: false,
                variadic: false,
            },
        ],
        return_type: None,
        volatility: Volatility::Volatile,
        nullability: Nullability::NullOnNullInput,
    });

    let source = "CALL my_proc(1, 2) RETURN 1";
    let outcome = validate_with_procedures(source, &catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_procedure_with_wrong_arity_fails() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(CallableSignature {
        name: SmolStr::new("my_proc"),
        kind: CallableKind::Procedure,
        parameters: vec![
            ParameterSignature {
                name: SmolStr::new("arg1"),
                param_type: SmolStr::new("ANY"),
                optional: false,
                variadic: false,
            },
        ],
        return_type: None,
        volatility: Volatility::Volatile,
        nullability: Nullability::NullOnNullInput,
    });

    let source = "CALL my_proc(1, 2, 3) RETURN 1";
    let outcome = validate_with_procedures(source, &catalog);

    // Should fail - wrong number of arguments
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("argument") || d.message.contains("arity")
            ));
}

#[test]
fn test_optional_call_validates() {
    let source = "OPTIONAL CALL abs(5) RETURN 1";

    // Built-ins are always available (checked directly by validator)
    let catalog = InMemoryMetadataProvider::new();
    let outcome = validate_with_procedures(source, &catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

// ===== Test 6-10: YIELD & Inline Procedures =====

#[test]
fn test_yield_valid_field_validates() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(CallableSignature {
        name: SmolStr::new("my_proc"),
        kind: CallableKind::Procedure,
        parameters: vec![],
        return_type: Some(SmolStr::new("result")),
        volatility: Volatility::Volatile,
        nullability: Nullability::NullOnNullInput,
    });

    let source = "CALL my_proc() YIELD result RETURN 1";
    let outcome = validate_with_procedures(source, &catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_yield_invalid_field_fails() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(CallableSignature {
        name: SmolStr::new("my_proc"),
        kind: CallableKind::Procedure,
        parameters: vec![],
        return_type: Some(SmolStr::new("result")),
        volatility: Volatility::Volatile,
        nullability: Nullability::NullOnNullInput,
    });

    let source = "CALL my_proc() YIELD nonexistent RETURN 1";
    let outcome = validate_with_procedures(source, &catalog);

    // Should fail or warn - field doesn't exist
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("nonexistent") || d.message.contains("field")
            ));
}

#[test]
fn test_inline_procedure_validates() {
    let source = "CALL { MATCH (n) RETURN n }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_inline_procedure_with_scope_validates() {
    let source = "MATCH (x) CALL (x) { MATCH (y) WHERE y.id = x.id RETURN y }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    // 'x' should be in scope within the inline procedure
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_inline_procedure_out_of_scope_variable_fails() {
    let source = "CALL (x) { RETURN y }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    // 'y' is not in scope (only 'x' is)
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("undefined") || d.message.contains("scope")
            ));
}
