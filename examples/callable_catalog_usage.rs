//! Example: Milestone 4 - Callable Catalog and Function Validation
//!
//! This example demonstrates the callable catalog system introduced in Milestone 4,
//! which provides infrastructure for validating function and procedure calls against
//! their signatures.
//!
//! Key features demonstrated:
//! - Built-in function catalog with standard GQL functions
//! - Custom function registration with InMemoryCallableCatalog
//! - Composite catalog combining built-ins and custom functions
//! - Arity validation for function calls
//! - Integration with SemanticValidator
//!
//! Run with: cargo run --example milestone4_callable_catalog

use gql_parser::semantic::callable::{
    BuiltinCallableCatalog, CallableCatalog, CallableKind, CallableLookupContext,
    CallableSignature, CallableValidator, CompositeCallableCatalog, DefaultCallableValidator,
    InMemoryCallableCatalog, Nullability, ParameterSignature, Volatility,
};
use gql_parser::semantic::SemanticValidator;

fn main() {
    println!("=== Milestone 4: Callable Catalog Example ===\n");

    // =========================================================================
    // Part 1: Built-in Callable Catalog
    // =========================================================================
    println!("Part 1: Built-in Functions");
    println!("--------------------------");

    let builtins = BuiltinCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // List all built-in functions
    println!("\nBuilt-in Functions:");
    let function_names = builtins.list(CallableKind::Function, &ctx);
    for name in function_names.iter().take(10) {
        println!("  - {}", name);
    }
    println!("  ... and {} more", function_names.len().saturating_sub(10));

    // List all aggregate functions
    println!("\nBuilt-in Aggregate Functions:");
    let agg_names = builtins.list(CallableKind::AggregateFunction, &ctx);
    for name in &agg_names {
        println!("  - {}", name);
    }

    // =========================================================================
    // Part 2: Function Signature Details
    // =========================================================================
    println!("\n\nPart 2: Function Signature Details");
    println!("----------------------------------");

    // Inspect ABS function
    if let Ok(sigs) = builtins.resolve("abs", CallableKind::Function, &ctx) {
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
    if let Ok(sigs) = builtins.resolve("substring", CallableKind::Function, &ctx) {
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
    if let Ok(sigs) = builtins.resolve("concat", CallableKind::Function, &ctx) {
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

    let mut custom_catalog = InMemoryCallableCatalog::new();

    // Register a custom function: DISTANCE(lat1, lon1, lat2, lon2) -> FLOAT
    custom_catalog.register(
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
    custom_catalog.register(
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
    custom_catalog.register(
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

    println!("\nRegistered custom functions:");
    let custom_functions = custom_catalog.list(CallableKind::Function, &ctx);
    for name in &custom_functions {
        if let Ok(sigs) = custom_catalog.resolve(name, CallableKind::Function, &ctx) {
            for sig in &sigs {
                println!("  - {} ({} parameters)", sig.name, sig.parameters.len());
            }
        }
    }

    println!("\nRegistered custom procedures:");
    let custom_procedures = custom_catalog.list(CallableKind::Procedure, &ctx);
    for name in &custom_procedures {
        if let Ok(sigs) = custom_catalog.resolve(name, CallableKind::Procedure, &ctx) {
            for sig in &sigs {
                println!("  - {} ({} parameters)", sig.name, sig.parameters.len());
            }
        }
    }

    // =========================================================================
    // Part 4: Composite Catalog
    // =========================================================================
    println!("\n\nPart 4: Composite Catalog");
    println!("-------------------------");

    let composite = CompositeCallableCatalog::new(builtins, custom_catalog);

    println!("\nComposite catalog contains:");
    let all_functions = composite.list(CallableKind::Function, &ctx);
    println!("  - {} functions total", all_functions.len());
    println!(
        "  - Including built-ins: abs, sqrt, concat, coalesce, ..."
    );
    println!("  - Including custom: distance, join_strings");

    // Verify both built-in and custom are accessible
    if composite
        .resolve("abs", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0
    {
        println!("\n✓ Built-in function 'abs' is accessible");
    }

    if composite
        .resolve("distance", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0
    {
        println!("✓ Custom function 'distance' is accessible");
    }

    // =========================================================================
    // Part 5: Arity Validation
    // =========================================================================
    println!("\n\nPart 5: Arity Validation");
    println!("------------------------");

    let validator_impl = DefaultCallableValidator::new();

    // Get ABS signature
    if let Ok(sigs) = composite.resolve("abs", CallableKind::Function, &ctx) {
        println!("\nValidating ABS function calls:");
        println!("  ABS signature: min_arity={}, max_arity={:?}", sigs[0].min_arity(), sigs[0].max_arity());

        use gql_parser::ast::Span;
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
    if let Ok(sigs) = composite.resolve("substring", CallableKind::Function, &ctx) {
        println!("\nValidating SUBSTRING function calls:");
        println!("  SUBSTRING signature: min_arity={}, max_arity={:?}", sigs[0].min_arity(), sigs[0].max_arity());

        use gql_parser::ast::Span;
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
    // Part 6: Integration with SemanticValidator
    // =========================================================================
    println!("\n\nPart 6: Integration with SemanticValidator");
    println!("------------------------------------------");

    let catalog_instance = BuiltinCallableCatalog::new();
    let validator_instance = DefaultCallableValidator::new();

    let semantic_validator = SemanticValidator::new()
        .with_callable_catalog(&catalog_instance)
        .with_callable_validator(&validator_instance);

    println!("\n✓ Semantic validator configured with callable catalog");

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

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n\n=== Summary ===");
    println!("The callable catalog system provides:");
    println!("  ✓ Built-in function catalog with standard GQL functions");
    println!("  ✓ Support for custom function and procedure registration");
    println!("  ✓ Composite catalogs combining built-in and external callables");
    println!("  ✓ Arity validation (required, optional, and variadic parameters)");
    println!("  ✓ Thread-safe trait design (Send + Sync)");
    println!("  ✓ Integration with semantic validation pipeline");
    println!("\nMilestone 4 is complete!");
}
