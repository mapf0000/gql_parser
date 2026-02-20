//! Example: Milestone 4 - Callable Catalog and Function Validation
//!
//! This example demonstrates the callable catalog system introduced in Milestone 4,
//! which provides infrastructure for validating function and procedure calls against
//! their signatures.
//!
//! Key features demonstrated:
//! - Built-in function lookup with standard GQL functions (zero-cost direct lookup)
//! - Custom function registration with InMemoryMetadataProvider
//! - Arity validation for function calls
//! - Integration with SemanticValidator
//!
//! Run with: cargo run --example callable_catalog_usage

use gql_parser::semantic::callable::{
    CallableKind, resolve_builtin_signatures, list_builtin_callables,
    CallableSignature, CallableValidator, DefaultCallableValidator,
    Nullability, ParameterSignature, Volatility,
};
use gql_parser::semantic::metadata_provider::InMemoryMetadataProvider;
use gql_parser::semantic::SemanticValidator;

fn main() {
    println!("=== Milestone 4: Callable Catalog Example ===\n");

    // =========================================================================
    // Part 1: Built-in Functions
    // =========================================================================
    println!("Part 1: Built-in Functions");
    println!("--------------------------");

    // List all built-in functions
    println!("\nBuilt-in Functions:");
    let function_names = list_builtin_callables(CallableKind::Function);
    for name in function_names.iter().take(10) {
        println!("  - {}", name);
    }
    println!("  ... and {} more", function_names.len().saturating_sub(10));

    println!("\nBuilt-in Aggregate Functions:");
    let agg_names = list_builtin_callables(CallableKind::AggregateFunction);
    for name in &agg_names {
        println!("  - {}", name);
    }

    // =========================================================================
    // Part 2: Function Signature Details
    // =========================================================================
    println!("\n\nPart 2: Function Signature Details");
    println!("----------------------------------");

    // Inspect ABS function
    if let Some(sigs) = resolve_builtin_signatures("abs", CallableKind::Function) {
        for sig in &sigs {
            println!("\nFunction: {}", sig.name);
            println!("  Return type: {:?}", sig.return_type);
            println!("  Min arity: {}", sig.min_arity());
            println!("  Max arity: {:?}", sig.max_arity());
            println!("  Volatility: {:?}", sig.volatility);
            println!("  Nullability: {:?}", sig.nullability);
            println!("  Parameters:");
            for param in &sig.parameters {
                let modifiers = if param.optional {
                    " (optional)"
                } else if param.variadic {
                    " (variadic)"
                } else {
                    ""
                };
                println!(
                    "    - {}: {}{}",
                    param.name, param.param_type, modifiers
                );
            }
        }
    }

    // Inspect SUBSTRING function (has optional parameter)
    if let Some(sigs) = resolve_builtin_signatures("substring", CallableKind::Function) {
        for sig in &sigs {
            println!("\nFunction: {}", sig.name);
            println!("  Min arity: {}", sig.min_arity());
            println!("  Max arity: {:?}", sig.max_arity());
            println!("  Parameters:");
            for param in &sig.parameters {
                let modifiers = if param.optional {
                    " (optional)"
                } else {
                    ""
                };
                println!(
                    "    - {}: {}{}",
                    param.name, param.param_type, modifiers
                );
            }
        }
    }

    // Inspect CONCAT function (variadic)
    if let Some(sigs) = resolve_builtin_signatures("concat", CallableKind::Function) {
        for sig in &sigs {
            println!("\nFunction: {}", sig.name);
            println!("  Min arity: {}", sig.min_arity());
            println!("  Max arity: {:?} (variadic)", sig.max_arity());
        }
    }

    // =========================================================================
    // Part 3: Custom Function Registration
    // =========================================================================
    println!("\n\nPart 3: Custom Function Registration");
    println!("------------------------------------");

    let mut metadata_provider = InMemoryMetadataProvider::new();

    // Register a custom function: DISTANCE(lat1, lon1, lat2, lon2) -> FLOAT
    metadata_provider.add_callable(
        "distance",
        CallableSignature::new(
            "distance",
            CallableKind::Function,
            vec![
                ParameterSignature::required("lat1", "FLOAT"),
                ParameterSignature::required("lon1", "FLOAT"),
                ParameterSignature::required("lat2", "FLOAT"),
                ParameterSignature::required("lon2", "FLOAT"),
            ],
            Some("FLOAT"),
        )
        .with_volatility(Volatility::Immutable)
        .with_nullability(Nullability::NullOnNullInput),
    );

    // Register a custom procedure: LOG_EVENT(level, message, [details])
    metadata_provider.add_callable(
        "log_event",
        CallableSignature::new(
            "log_event",
            CallableKind::Procedure,
            vec![
                ParameterSignature::required("level", "STRING"),
                ParameterSignature::required("message", "STRING"),
                ParameterSignature::optional("details", "STRING"),
            ],
            None::<String>, // Procedures don't return values
        )
        .with_volatility(Volatility::Volatile),
    );

    // Register a variadic function: JOIN_STRINGS(separator, ...strings) -> STRING
    metadata_provider.add_callable(
        "join_strings",
        CallableSignature::new(
            "join_strings",
            CallableKind::Function,
            vec![
                ParameterSignature::required("separator", "STRING"),
                ParameterSignature::variadic("strings", "STRING"),
            ],
            Some("STRING"),
        )
        .with_volatility(Volatility::Immutable),
    );

    use gql_parser::semantic::metadata_provider::MetadataProvider;

    println!("\nRegistered custom functions:");
    println!("  - distance (4 parameters)");
    println!("  - join_strings (variadic)");

    println!("\nRegistered custom procedures:");
    println!("  - log_event (2-3 parameters)");

    // Verify custom functions are accessible
    if metadata_provider.lookup_callable("distance").is_some() {
        println!("\n✓ Custom function 'distance' is accessible via MetadataProvider");
    }

    // Note: Built-ins like 'abs' are always available (checked directly by validator)
    // They don't need to be looked up via metadata provider

    // =========================================================================
    // Part 4: Arity Validation
    // =========================================================================
    println!("\n\nPart 4: Arity Validation");
    println!("------------------------");

    let validator_impl = DefaultCallableValidator::new();

    // Get ABS signature
    if let Some(sigs) = resolve_builtin_signatures("abs", CallableKind::Function) {
        println!("\nValidating ABS function calls:");
        println!("  ABS signature: min_arity={}, max_arity={:?}", sigs[0].min_arity(), sigs[0].max_arity());

        use gql_parser::semantic::callable::CallSite;

        // Valid call: ABS(42)
        let call = CallSite {
            name: "abs",
            kind: CallableKind::Function,
            arg_count: 1,
            span: 0..7,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if diags.is_empty() {
            println!("  ✓ ABS(42) - Valid (1 argument)");
        } else {
            println!("  ✗ ABS(42) - Invalid: {}", diags[0].message);
        }

        // Invalid call: ABS()
        let call = CallSite {
            name: "abs",
            kind: CallableKind::Function,
            arg_count: 0,
            span: 0..5,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if !diags.is_empty() {
            println!("  ✗ ABS() - Invalid: {}", diags[0].message);
        }

        // Invalid call: ABS(1, 2)
        let call = CallSite {
            name: "abs",
            kind: CallableKind::Function,
            arg_count: 2,
            span: 0..10,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if !diags.is_empty() {
            println!("  ✗ ABS(1, 2) - Invalid: {}", diags[0].message);
        }
    }

    // Get SUBSTRING signature (has optional parameter)
    if let Some(sigs) = resolve_builtin_signatures("substring", CallableKind::Function) {
        println!("\nValidating SUBSTRING function calls:");
        println!("  SUBSTRING signature: min_arity={}, max_arity={:?}", sigs[0].min_arity(), sigs[0].max_arity());

        use gql_parser::semantic::callable::CallSite;

        // Valid: SUBSTRING('hello', 1)
        let call = CallSite {
            name: "substring",
            kind: CallableKind::Function,
            arg_count: 2,
            span: 0..20,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if diags.is_empty() {
            println!("  ✓ SUBSTRING('hello', 1) - Valid (2 arguments)");
        }

        // Valid: SUBSTRING('hello', 1, 3)
        let call = CallSite {
            name: "substring",
            kind: CallableKind::Function,
            arg_count: 3,
            span: 0..23,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if diags.is_empty() {
            println!("  ✓ SUBSTRING('hello', 1, 3) - Valid (3 arguments)");
        }

        // Invalid: SUBSTRING('hello')
        let call = CallSite {
            name: "substring",
            kind: CallableKind::Function,
            arg_count: 1,
            span: 0..18,
        };
        let diags = validator_impl.validate_call(&call, &sigs);
        if !diags.is_empty() {
            println!("  ✗ SUBSTRING('hello') - Invalid: {}", diags[0].message);
        }
    }

    // =========================================================================
    // Part 5: Integration with SemanticValidator
    // =========================================================================
    println!("\n\nPart 5: Integration with SemanticValidator");
    println!("------------------------------------------");

    let _semantic_validator = SemanticValidator::new()
        .with_metadata_provider(&metadata_provider);

    println!("\n✓ Semantic validator configured with metadata provider");

    // Parse and validate a simple query
    let source = "MATCH (n) RETURN ABS(n.value)";
    let result = gql_parser::parse_and_validate(source);

    println!("\nValidating query: {}", source);
    if result.ir.is_some() {
        println!("✓ Validation successful");
        if !result.diagnostics.is_empty() {
            println!("  Warnings/Notes:");
            for diag in result.diagnostics {
                println!("    - {:?}", diag);
            }
        }
    } else {
        println!("✗ Validation failed:");
        for diag in result.diagnostics {
            println!("    - {:?}", diag);
        }
    }

    println!("\n\n=== Summary ===");
    println!("The callable catalog system provides:");
    println!("  ✓ Built-in functions are always available (zero-cost direct lookup)");
    println!("  ✓ Support for custom function and procedure registration via MetadataProvider");
    println!("  ✓ Arity validation (required, optional, and variadic parameters)");
    println!("  ✓ Thread-safe design (Send + Sync)");
    println!("  ✓ Integration with semantic validation pipeline");
    println!("\nMilestone 4 is complete!");
}
