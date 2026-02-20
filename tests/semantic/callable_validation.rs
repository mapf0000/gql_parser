//! Integration tests for Milestone 4 Callable Catalog functionality.

use gql_parser::semantic::callable::{
    BuiltinCallableCatalog, CallableCatalog, CallableKind, CallableLookupContext,
    CallableSignature, CallableValidator, CompositeCallableCatalog, DefaultCallableValidator,
    InMemoryCallableCatalog, ParameterSignature,
};
use gql_parser::semantic::{SemanticValidator, ValidationConfig};

#[test]
fn test_builtin_callable_catalog_coverage() {
    let catalog = BuiltinCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // Test numeric functions
    assert!(catalog
        .resolve("abs", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("mod", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("floor", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("ceil", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("sqrt", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("power", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("exp", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("ln", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("log", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);

    // Test string functions
    assert!(catalog
        .resolve("length", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("substring", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("upper", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("lower", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("trim", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);

    // Test temporal functions
    assert!(catalog
        .resolve("current_date", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("current_time", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("current_timestamp", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);

    // Test aggregate functions
    assert!(catalog
        .resolve("count", CallableKind::AggregateFunction, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("sum", CallableKind::AggregateFunction, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("avg", CallableKind::AggregateFunction, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("min", CallableKind::AggregateFunction, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("max", CallableKind::AggregateFunction, &ctx)
        .unwrap()
        .len()
        > 0);

    // Test other functions
    assert!(catalog
        .resolve("coalesce", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
    assert!(catalog
        .resolve("nullif", CallableKind::Function, &ctx)
        .unwrap()
        .len()
        > 0);
}

#[test]
fn test_builtin_function_arity() {
    let catalog = BuiltinCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // ABS: requires exactly 1 argument
    let sigs = catalog
        .resolve("abs", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].min_arity(), 1);
    assert_eq!(sigs[0].max_arity(), Some(1));

    // MOD: requires exactly 2 arguments
    let sigs = catalog
        .resolve("mod", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].min_arity(), 2);
    assert_eq!(sigs[0].max_arity(), Some(2));

    // SUBSTRING: requires 2-3 arguments (third is optional)
    let sigs = catalog
        .resolve("substring", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].min_arity(), 2);
    assert_eq!(sigs[0].max_arity(), Some(3));

    // ROUND: requires 1-2 arguments (second is optional)
    let sigs = catalog
        .resolve("round", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].min_arity(), 1);
    assert_eq!(sigs[0].max_arity(), Some(2));

    // CONCAT: variadic (unlimited arguments)
    let sigs = catalog
        .resolve("concat", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].max_arity(), None); // variadic

    // COALESCE: variadic
    let sigs = catalog
        .resolve("coalesce", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs[0].max_arity(), None); // variadic
}

#[test]
fn test_inmemory_catalog_registration() {
    let mut catalog = InMemoryCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // Initially empty
    assert!(catalog.is_empty());

    // Register a custom function
    catalog.register(CallableSignature::new(
        "my_custom_func",
        CallableKind::Function,
        vec![
            ParameterSignature::required("x", "INT"),
            ParameterSignature::required("y", "STRING"),
        ],
        Some("BOOL"),
    ));

    assert_eq!(catalog.len(), 1);

    // Resolve it
    let sigs = catalog
        .resolve("my_custom_func", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].name, "my_custom_func");
    assert_eq!(sigs[0].min_arity(), 2);
    assert_eq!(sigs[0].max_arity(), Some(2));
    assert_eq!(sigs[0].return_type, Some("BOOL".into()));

    // Register an overload
    catalog.register(CallableSignature::new(
        "my_custom_func",
        CallableKind::Function,
        vec![ParameterSignature::required("x", "STRING")],
        Some("INT"),
    ));

    assert_eq!(catalog.len(), 2);

    // Both overloads should be returned
    let sigs = catalog
        .resolve("my_custom_func", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs.len(), 2);

    // Unregister
    catalog.unregister("my_custom_func", CallableKind::Function);
    let sigs = catalog
        .resolve("my_custom_func", CallableKind::Function, &ctx)
        .unwrap();
    assert!(sigs.is_empty());
}

#[test]
fn test_composite_catalog_composition() {
    let builtins = BuiltinCallableCatalog::new();
    let mut custom = InMemoryCallableCatalog::new();

    // Add custom function
    custom.register(CallableSignature::new(
        "my_func",
        CallableKind::Function,
        vec![ParameterSignature::required("x", "INT")],
        Some("INT"),
    ));

    let catalog = CompositeCallableCatalog::new(builtins, custom);
    let ctx = CallableLookupContext::new();

    // Can resolve built-in
    let sigs = catalog.resolve("abs", CallableKind::Function, &ctx).unwrap();
    assert_eq!(sigs.len(), 1);

    // Can resolve custom
    let sigs = catalog
        .resolve("my_func", CallableKind::Function, &ctx)
        .unwrap();
    assert_eq!(sigs.len(), 1);

    // Can list both
    let names = catalog.list(CallableKind::Function, &ctx);
    assert!(names.contains(&"abs".into()));
    assert!(names.contains(&"my_func".into()));

    // Can disable built-ins
    let ctx_no_builtins = CallableLookupContext::new().with_builtins(false);
    let sigs = catalog
        .resolve("abs", CallableKind::Function, &ctx_no_builtins)
        .unwrap();
    assert!(sigs.is_empty());

    let sigs = catalog
        .resolve("my_func", CallableKind::Function, &ctx_no_builtins)
        .unwrap();
    assert_eq!(sigs.len(), 1); // custom still works
}

#[test]
fn test_default_callable_validator_correct_arity() {
    use gql_parser::ast::Span;
    use gql_parser::semantic::callable::CallSite;

    let validator = DefaultCallableValidator::new();

    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![ParameterSignature::required("x", "INT")],
        Some("INT"),
    );

    // Correct arity
    let call = CallSite {
        name: "test",
        kind: CallableKind::Function,
        arg_count: 1,
        span: 0..4,
    };

    let diags = validator.validate_call(&call, &[sig]);
    assert!(diags.is_empty());
}

#[test]
fn test_default_callable_validator_wrong_arity() {
    use gql_parser::ast::Span;
    use gql_parser::diag::DiagSeverity;
    use gql_parser::semantic::callable::CallSite;

    let validator = DefaultCallableValidator::new();

    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![ParameterSignature::required("x", "INT")],
        Some("INT"),
    );

    // Wrong arity: too many arguments
    let call = CallSite {
        name: "test",
        kind: CallableKind::Function,
        arg_count: 2,
        span: 0..4,
    };

    let diags = validator.validate_call(&call, &[sig.clone()]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagSeverity::Error);
    assert!(diags[0].message.contains("expects"));

    // Wrong arity: too few arguments
    let call = CallSite {
        name: "test",
        kind: CallableKind::Function,
        arg_count: 0,
        span: 0..4,
    };

    let diags = validator.validate_call(&call, &[sig]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagSeverity::Error);
}

#[test]
fn test_default_callable_validator_undefined_function() {
    use gql_parser::ast::Span;
    use gql_parser::diag::DiagSeverity;
    use gql_parser::semantic::callable::CallSite;

    let validator = DefaultCallableValidator::new();

    let call = CallSite {
        name: "undefined_func",
        kind: CallableKind::Function,
        arg_count: 1,
        span: 0..14,
    };

    // Empty signature list means function not found
    let diags = validator.validate_call(&call, &[]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagSeverity::Error);
    assert!(diags[0].message.contains("not defined"));
}

#[test]
fn test_default_callable_validator_variadic() {
    use gql_parser::ast::Span;
    use gql_parser::semantic::callable::CallSite;

    let validator = DefaultCallableValidator::new();

    let sig = CallableSignature::new(
        "concat",
        CallableKind::Function,
        vec![ParameterSignature::variadic("args", "STRING")],
        Some("STRING"),
    );

    // Variadic accepts any number of arguments >= min_arity
    for arg_count in 0..10 {
        let call = CallSite {
            name: "concat",
            kind: CallableKind::Function,
            arg_count,
            span: 0..6,
        };

        let diags = validator.validate_call(&call, &[sig.clone()]);
        assert!(diags.is_empty(), "Failed for arg_count={}", arg_count);
    }
}

#[test]
fn test_semantic_validator_with_callable_catalog() {
    let catalog = BuiltinCallableCatalog::new();
    let validator_impl = DefaultCallableValidator::new();

    let _validator = SemanticValidator::new()
        .with_callable_catalog(&catalog)
        .with_callable_validator(&validator_impl);

    // Just verify it compiles and can be constructed
}

#[test]
fn test_callable_validation_config_defaults() {
    let config = ValidationConfig::default();
    assert!(!config.callable_validation);
}

#[test]
fn test_signature_arity_helpers() {
    // Fixed arity
    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![
            ParameterSignature::required("a", "INT"),
            ParameterSignature::required("b", "INT"),
        ],
        Some("INT"),
    );
    assert_eq!(sig.min_arity(), 2);
    assert_eq!(sig.max_arity(), Some(2));
    assert!(sig.matches_arity(2));
    assert!(!sig.matches_arity(1));
    assert!(!sig.matches_arity(3));

    // With optional parameter
    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![
            ParameterSignature::required("a", "INT"),
            ParameterSignature::optional("b", "INT"),
        ],
        Some("INT"),
    );
    assert_eq!(sig.min_arity(), 1);
    assert_eq!(sig.max_arity(), Some(2));
    assert!(sig.matches_arity(1));
    assert!(sig.matches_arity(2));
    assert!(!sig.matches_arity(0));
    assert!(!sig.matches_arity(3));

    // Variadic
    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![ParameterSignature::variadic("args", "ANY")],
        Some("ANY"),
    );
    assert_eq!(sig.min_arity(), 0);
    assert_eq!(sig.max_arity(), None);
    assert!(sig.matches_arity(0));
    assert!(sig.matches_arity(1));
    assert!(sig.matches_arity(100));

    // Required + variadic
    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![
            ParameterSignature::required("first", "INT"),
            ParameterSignature::variadic("rest", "INT"),
        ],
        Some("INT"),
    );
    assert_eq!(sig.min_arity(), 1);
    assert_eq!(sig.max_arity(), None);
    assert!(!sig.matches_arity(0));
    assert!(sig.matches_arity(1));
    assert!(sig.matches_arity(100));
}

#[test]
fn test_callable_lookup_context_builder() {
    let ctx = CallableLookupContext::new();
    assert!(ctx.include_builtins);
    assert!(ctx.schema.is_none());
    assert!(ctx.graph.is_none());

    let ctx = CallableLookupContext::new()
        .with_schema("my_schema")
        .with_graph("my_graph")
        .with_builtins(false);

    assert!(!ctx.include_builtins);
    assert_eq!(ctx.schema, Some("my_schema".into()));
    assert_eq!(ctx.graph, Some("my_graph".into()));
}

#[test]
fn test_parameter_signature_constructors() {
    let param = ParameterSignature::required("x", "INT");
    assert!(!param.optional);
    assert!(!param.variadic);
    assert_eq!(param.name, "x");
    assert_eq!(param.param_type, "INT");

    let param = ParameterSignature::optional("y", "STRING");
    assert!(param.optional);
    assert!(!param.variadic);

    let param = ParameterSignature::variadic("args", "ANY");
    assert!(!param.optional);
    assert!(param.variadic);
}

#[test]
fn test_callable_signature_builder() {
    use gql_parser::semantic::callable::{Nullability, Volatility};

    let sig = CallableSignature::new(
        "test",
        CallableKind::Function,
        vec![ParameterSignature::required("x", "INT")],
        Some("INT"),
    )
    .with_volatility(Volatility::Volatile)
    .with_nullability(Nullability::CalledOnNullInput);

    assert_eq!(sig.volatility, Volatility::Volatile);
    assert_eq!(sig.nullability, Nullability::CalledOnNullInput);
}

#[test]
fn test_builtin_catalog_case_insensitive() {
    let catalog = BuiltinCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    // Test case insensitivity
    let sigs_lower = catalog
        .resolve("abs", CallableKind::Function, &ctx)
        .unwrap();
    let sigs_upper = catalog
        .resolve("ABS", CallableKind::Function, &ctx)
        .unwrap();
    let sigs_mixed = catalog
        .resolve("AbS", CallableKind::Function, &ctx)
        .unwrap();

    assert_eq!(sigs_lower.len(), sigs_upper.len());
    assert_eq!(sigs_lower.len(), sigs_mixed.len());
}

#[test]
fn test_inmemory_catalog_case_insensitive() {
    let mut catalog = InMemoryCallableCatalog::new();
    let ctx = CallableLookupContext::new();

    catalog.register(CallableSignature::new(
        "MyFunc",
        CallableKind::Function,
        vec![],
        Some("INT"),
    ));

    // Should resolve regardless of case
    assert!(!catalog
        .resolve("myfunc", CallableKind::Function, &ctx)
        .unwrap()
        .is_empty());
    assert!(!catalog
        .resolve("MYFUNC", CallableKind::Function, &ctx)
        .unwrap()
        .is_empty());
    assert!(!catalog
        .resolve("MyFunc", CallableKind::Function, &ctx)
        .unwrap()
        .is_empty());
}
