//! Integration tests for Milestone 5 Type Inference Quality improvements.

use gql_parser::ir::type_table::Type;
use gql_parser::semantic::type_metadata::{
    CallableInvocation, CastRuleSet, DefaultCastRuleSet, InferencePolicy,
    MockCastRuleSet, MockTypeCheckContextProvider, MockTypeMetadataCatalog, TypeCheckContext,
    TypeCheckContextProvider, TypeMetadataCatalog, TypeRef, UnknownCallableBehavior,
};
use gql_parser::semantic::{SemanticValidator, ValidationConfig};

#[test]
fn test_inference_policy_defaults() {
    let policy = InferencePolicy::default();
    assert!(policy.allow_any_fallback);
    assert!(policy.prefer_schema_types);
    assert_eq!(
        policy.unknown_callable_behavior,
        UnknownCallableBehavior::InferFromArguments
    );
}

#[test]
fn test_inference_policy_strict() {
    let policy = InferencePolicy::strict();
    assert!(!policy.allow_any_fallback);
    assert!(policy.prefer_schema_types);
    assert_eq!(
        policy.unknown_callable_behavior,
        UnknownCallableBehavior::ReturnNone
    );
}

#[test]
fn test_inference_policy_permissive() {
    let policy = InferencePolicy::permissive();
    assert!(policy.allow_any_fallback);
    assert!(!policy.prefer_schema_types);
    assert_eq!(
        policy.unknown_callable_behavior,
        UnknownCallableBehavior::ReturnAny
    );
}

#[test]
fn test_inference_policy_builder() {
    let policy = InferencePolicy::new()
        .with_any_fallback(false)
        .with_prefer_schema_types(false)
        .with_unknown_callable_behavior(UnknownCallableBehavior::ReturnNone);

    assert!(!policy.allow_any_fallback);
    assert!(!policy.prefer_schema_types);
    assert_eq!(
        policy.unknown_callable_behavior,
        UnknownCallableBehavior::ReturnNone
    );
}

#[test]
fn test_default_cast_rules_identity() {
    let rules = DefaultCastRuleSet::new();

    // Identity casts
    assert!(rules.can_cast(&Type::Int, &Type::Int));
    assert!(rules.can_cast(&Type::String, &Type::String));
    assert!(rules.can_cast(&Type::Boolean, &Type::Boolean));
    assert!(rules.can_cast(&Type::Float, &Type::Float));
}

#[test]
fn test_default_cast_rules_numeric_widening() {
    let rules = DefaultCastRuleSet::new();

    // Numeric widening
    assert!(rules.can_cast(&Type::Int, &Type::Float));

    // Not the reverse
    assert!(!rules.can_cast(&Type::Float, &Type::Int));
}

#[test]
fn test_default_cast_rules_to_string() {
    let rules = DefaultCastRuleSet::new();

    // Everything casts to string
    assert!(rules.can_cast(&Type::Int, &Type::String));
    assert!(rules.can_cast(&Type::Float, &Type::String));
    assert!(rules.can_cast(&Type::Boolean, &Type::String));
    assert!(rules.can_cast(&Type::Date, &Type::String));
    assert!(rules.can_cast(&Type::Time, &Type::String));
}

#[test]
fn test_default_cast_rules_string_parsing() {
    let rules = DefaultCastRuleSet::new();

    // String can parse to numeric
    assert!(rules.can_cast(&Type::String, &Type::Int));
    assert!(rules.can_cast(&Type::String, &Type::Float));
    assert!(rules.can_cast(&Type::String, &Type::Boolean));
}

#[test]
fn test_default_cast_rules_any() {
    let rules = DefaultCastRuleSet::new();

    // Any can cast to/from anything
    assert!(rules.can_cast(&Type::Any, &Type::Int));
    assert!(rules.can_cast(&Type::Int, &Type::Any));
    assert!(rules.can_cast(&Type::Any, &Type::String));
    assert!(rules.can_cast(&Type::Any, &Type::Any));
}

#[test]
fn test_default_cast_rules_null() {
    let rules = DefaultCastRuleSet::new();

    // Null can cast to anything
    assert!(rules.can_cast(&Type::Null, &Type::Int));
    assert!(rules.can_cast(&Type::Null, &Type::String));
    assert!(rules.can_cast(&Type::Null, &Type::Boolean));
    assert!(rules.can_cast(&Type::Null, &Type::Date));
}

#[test]
fn test_default_cast_rules_list() {
    let rules = DefaultCastRuleSet::new();

    // List element type compatibility
    let int_list = Type::List(Box::new(Type::Int));
    let float_list = Type::List(Box::new(Type::Float));
    let string_list = Type::List(Box::new(Type::String));

    // Integer list can cast to float list (element widening)
    assert!(rules.can_cast(&int_list, &float_list));

    // Not the reverse
    assert!(!rules.can_cast(&float_list, &int_list));

    // Integer list to string list (via toString)
    assert!(rules.can_cast(&int_list, &string_list));
}

#[test]
fn test_cast_result_type() {
    let rules = DefaultCastRuleSet::new();

    // Valid casts return target type
    assert_eq!(
        rules.cast_result_type(&Type::Int, &Type::Float),
        Type::Float
    );
    assert_eq!(
        rules.cast_result_type(&Type::Int, &Type::String),
        Type::String
    );

    // Invalid casts return Any
    assert_eq!(
        rules.cast_result_type(&Type::Float, &Type::Int),
        Type::Any
    );
}

#[test]
fn test_mock_cast_rule_set() {
    let mut rules = MockCastRuleSet::new();

    // Initially nothing is allowed
    assert!(!rules.can_cast(&Type::Int, &Type::String));

    // Allow specific cast
    rules.allow_cast(Type::Int, Type::String);
    assert!(rules.can_cast(&Type::Int, &Type::String));

    // Disallow a cast
    rules.disallow_cast(Type::Int, Type::Float);
    assert!(!rules.can_cast(&Type::Int, &Type::Float));
}

#[test]
fn test_mock_type_metadata_catalog_property_types() {
    let mut catalog = MockTypeMetadataCatalog::new();

    // Register property types for a Person node
    catalog.register_property_type(
        TypeRef::NodeType("Person".into()),
        "name",
        Type::String,
    );
    catalog.register_property_type(
        TypeRef::NodeType("Person".into()),
        "age",
        Type::Int,
    );
    catalog.register_property_type(
        TypeRef::NodeType("Person".into()),
        "salary",
        Type::Float,
    );

    // Query property types
    assert_eq!(
        catalog.property_type(&TypeRef::NodeType("Person".into()), "name"),
        Some(Type::String)
    );
    assert_eq!(
        catalog.property_type(&TypeRef::NodeType("Person".into()), "age"),
        Some(Type::Int)
    );
    assert_eq!(
        catalog.property_type(&TypeRef::NodeType("Person".into()), "salary"),
        Some(Type::Float)
    );

    // Unknown property
    assert_eq!(
        catalog.property_type(&TypeRef::NodeType("Person".into()), "unknown"),
        None
    );

    // Unknown type
    assert_eq!(
        catalog.property_type(&TypeRef::NodeType("Unknown".into()), "name"),
        None
    );
}

#[test]
fn test_mock_type_metadata_catalog_callable_returns() {
    let mut catalog = MockTypeMetadataCatalog::new();

    // Register callable return types
    catalog.register_callable_return("get_user_count", Type::Int);
    catalog.register_callable_return("get_user_name", Type::String);
    catalog.register_callable_return("is_active", Type::Boolean);

    // Query callable return types
    let call1 = CallableInvocation {
        name: "get_user_count",
        arg_types: vec![],
        is_aggregate: false,
    };
    assert_eq!(
        catalog.callable_return_type(&call1),
        Some(Type::Int)
    );

    let call2 = CallableInvocation {
        name: "get_user_name",
        arg_types: vec![Some(Type::Int)],
        is_aggregate: false,
    };
    assert_eq!(
        catalog.callable_return_type(&call2),
        Some(Type::String)
    );

    // Unknown callable
    let call3 = CallableInvocation {
        name: "unknown_func",
        arg_types: vec![],
        is_aggregate: false,
    };
    assert_eq!(catalog.callable_return_type(&call3), None);
}

#[test]
fn test_type_check_context() {
    let mut context = TypeCheckContext::new();

    // Add variable types
    context.add_variable_type("x", Type::Int);
    context.add_variable_type("y", Type::String);
    context.add_variable_type("z", Type::Boolean);

    // Query variable types
    assert_eq!(context.get_variable_type("x"), Some(&Type::Int));
    assert_eq!(context.get_variable_type("y"), Some(&Type::String));
    assert_eq!(context.get_variable_type("z"), Some(&Type::Boolean));
    assert_eq!(context.get_variable_type("unknown"), None);

    // Add expression types
    context.add_expression_type((0, 10), Type::Int);
    context.add_expression_type((10, 20), Type::String);

    // Query expression types
    assert_eq!(context.get_expression_type((0, 10)), Some(&Type::Int));
    assert_eq!(context.get_expression_type((10, 20)), Some(&Type::String));
    assert_eq!(context.get_expression_type((20, 30)), None);
}

#[test]
fn test_mock_type_check_context_provider() {
    let mut provider = MockTypeCheckContextProvider::new();

    // Create contexts for different statements
    let mut context0 = TypeCheckContext::new();
    context0.add_variable_type("x", Type::Int);

    let mut context1 = TypeCheckContext::new();
    context1.add_variable_type("y", Type::String);

    provider.register_context(0, context0);
    provider.register_context(1, context1);

    // Query contexts
    let retrieved0 = provider.type_context(0);
    assert_eq!(retrieved0.get_variable_type("x"), Some(&Type::Int));
    assert_eq!(retrieved0.get_variable_type("y"), None);

    let retrieved1 = provider.type_context(1);
    assert_eq!(retrieved1.get_variable_type("x"), None);
    assert_eq!(retrieved1.get_variable_type("y"), Some(&Type::String));

    // Unknown statement returns empty context
    let empty = provider.type_context(999);
    assert_eq!(empty.get_variable_type("x"), None);
}

#[test]
fn test_semantic_validator_with_type_metadata() {
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;

    let metadata = MockMetadataProvider::example();

    let _validator = SemanticValidator::new()
        .with_metadata_provider(&metadata);

    // Just verify it compiles and can be constructed
}

#[test]
fn test_validation_config_metadata_validation() {
    let config = ValidationConfig::default();
    assert!(!config.metadata_validation);

    // Just verify the validator was created
    let _validator = SemanticValidator::new();
}

#[test]
fn test_type_ref_equality() {
    let ref1 = TypeRef::NodeType("Person".into());
    let ref2 = TypeRef::NodeType("Person".into());
    let ref3 = TypeRef::NodeType("Company".into());
    let ref4 = TypeRef::EdgeType("KNOWS".into());

    assert_eq!(ref1, ref2);
    assert_ne!(ref1, ref3);
    assert_ne!(ref1, ref4);
}

#[test]
fn test_callable_invocation() {
    let call = CallableInvocation {
        name: "my_func",
        arg_types: vec![Some(Type::Int), Some(Type::String)],
        is_aggregate: false,
    };

    assert_eq!(call.name, "my_func");
    assert_eq!(call.arg_types.len(), 2);
    assert!(!call.is_aggregate);

    let agg_call = CallableInvocation {
        name: "sum",
        arg_types: vec![Some(Type::Int)],
        is_aggregate: true,
    };

    assert!(agg_call.is_aggregate);
}

#[test]
fn test_unknown_callable_behavior_variants() {
    let behavior1 = UnknownCallableBehavior::ReturnAny;
    let behavior2 = UnknownCallableBehavior::ReturnNone;
    let behavior3 = UnknownCallableBehavior::InferFromArguments;

    assert_eq!(behavior1, UnknownCallableBehavior::ReturnAny);
    assert_eq!(behavior2, UnknownCallableBehavior::ReturnNone);
    assert_eq!(behavior3, UnknownCallableBehavior::InferFromArguments);

    assert_ne!(behavior1, behavior2);
    assert_ne!(behavior2, behavior3);
}

#[test]
fn test_edge_type_property_metadata() {
    let mut catalog = MockTypeMetadataCatalog::new();

    // Register property types for an edge type
    catalog.register_property_type(
        TypeRef::EdgeType("KNOWS".into()),
        "since",
        Type::Date,
    );
    catalog.register_property_type(
        TypeRef::EdgeType("KNOWS".into()),
        "weight",
        Type::Float,
    );

    assert_eq!(
        catalog.property_type(&TypeRef::EdgeType("KNOWS".into()), "since"),
        Some(Type::Date)
    );
    assert_eq!(
        catalog.property_type(&TypeRef::EdgeType("KNOWS".into()), "weight"),
        Some(Type::Float)
    );
}

#[test]
fn test_general_type_ref() {
    let mut catalog = MockTypeMetadataCatalog::new();

    catalog.register_property_type(
        TypeRef::Type("CustomType".into()),
        "field1",
        Type::Int,
    );

    assert_eq!(
        catalog.property_type(&TypeRef::Type("CustomType".into()), "field1"),
        Some(Type::Int)
    );
}
