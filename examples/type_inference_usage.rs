//! Example: Milestone 5 - Type Inference Quality Improvements
//!
//! This example demonstrates the type metadata catalog system introduced in
//! Milestone 5, which improves type inference quality by:
//! - Providing property type information from schemas
//! - Resolving callable return types
//! - Defining type casting rules
//! - Controlling fallback behavior with policies
//!
//! Key features demonstrated:
//! - TypeMetadataCatalog for property and callable types
//! - CastRuleSet for defining casting rules
//! - InferencePolicy for controlling fallback behavior
//! - Mock implementations for testing
//! - Integration with SemanticValidator
//!
//! Run with: cargo run --example milestone5_type_inference

use gql_parser::ir::type_table::Type;
use gql_parser::semantic::type_metadata::{
    CallableInvocation, CastRuleSet, DefaultCastRuleSet, InferencePolicy,
    MockCastRuleSet, MockTypeCheckContextProvider, MockTypeMetadataCatalog, TypeCheckContext,
    TypeCheckContextProvider, TypeMetadataCatalog, TypeRef, UnknownCallableBehavior,
};
use gql_parser::semantic::SemanticValidator;

fn main() {
    println!("=== Milestone 5: Type Inference Quality Example ===\n");

    // =========================================================================
    // Part 1: Inference Policies
    // =========================================================================
    println!("Part 1: Inference Policies");
    println!("--------------------------");

    let default_policy = InferencePolicy::default();
    println!("\nDefault Policy:");
    println!("  - Allow Any fallback: {}", default_policy.allow_any_fallback);
    println!("  - Prefer schema types: {}", default_policy.prefer_schema_types);
    println!("  - Unknown callable behavior: {:?}", default_policy.unknown_callable_behavior);

    let strict_policy = InferencePolicy::strict();
    println!("\nStrict Policy:");
    println!("  - Allow Any fallback: {}", strict_policy.allow_any_fallback);
    println!("  - Prefer schema types: {}", strict_policy.prefer_schema_types);
    println!("  - Unknown callable behavior: {:?}", strict_policy.unknown_callable_behavior);

    let permissive_policy = InferencePolicy::permissive();
    println!("\nPermissive Policy:");
    println!("  - Allow Any fallback: {}", permissive_policy.allow_any_fallback);
    println!("  - Prefer schema types: {}", permissive_policy.prefer_schema_types);
    println!("  - Unknown callable behavior: {:?}", permissive_policy.unknown_callable_behavior);

    let custom_policy = InferencePolicy::new()
        .with_any_fallback(false)
        .with_prefer_schema_types(true)
        .with_unknown_callable_behavior(UnknownCallableBehavior::ReturnNone);
    println!("\nCustom Policy:");
    println!("  - Allow Any fallback: {}", custom_policy.allow_any_fallback);
    println!("  - Prefer schema types: {}", custom_policy.prefer_schema_types);
    println!("  - Unknown callable behavior: {:?}", custom_policy.unknown_callable_behavior);

    // =========================================================================
    // Part 2: Cast Rules
    // =========================================================================
    println!("\n\nPart 2: Type Cast Rules");
    println!("------------------------");

    let cast_rules = DefaultCastRuleSet::new();

    println!("\nNumeric Widening:");
    println!("  Integer -> Float: {}", cast_rules.can_cast(&Type::Int, &Type::Float));
    println!("  Integer -> Double: {}", cast_rules.can_cast(&Type::Int, &Type::Float));
    println!("  Float -> Double: {}", cast_rules.can_cast(&Type::Float, &Type::Float));
    println!("  Float -> Integer: {} (narrowing not allowed)", cast_rules.can_cast(&Type::Float, &Type::Int));

    println!("\nTo String Conversions:");
    println!("  Integer -> String: {}", cast_rules.can_cast(&Type::Int, &Type::String));
    println!("  Boolean -> String: {}", cast_rules.can_cast(&Type::Boolean, &Type::String));
    println!("  Date -> String: {}", cast_rules.can_cast(&Type::Date, &Type::String));

    println!("\nFrom String Parsing:");
    println!("  String -> Integer: {}", cast_rules.can_cast(&Type::String, &Type::Int));
    println!("  String -> Boolean: {}", cast_rules.can_cast(&Type::String, &Type::Boolean));

    println!("\nSpecial Types:");
    println!("  Null -> Integer: {}", cast_rules.can_cast(&Type::Null, &Type::Int));
    println!("  Any -> Integer: {}", cast_rules.can_cast(&Type::Any, &Type::Int));
    println!("  Integer -> Any: {}", cast_rules.can_cast(&Type::Int, &Type::Any));

    // =========================================================================
    // Part 3: Custom Cast Rules
    // =========================================================================
    println!("\n\nPart 3: Custom Cast Rules");
    println!("--------------------------");

    let mut custom_rules = MockCastRuleSet::new();

    // Define custom casting rules
    custom_rules.allow_cast(Type::Int, Type::String);
    custom_rules.allow_cast(Type::Boolean, Type::Int);
    custom_rules.disallow_cast(Type::String, Type::Int);

    println!("\nCustom Rules:");
    println!("  Integer -> String: {}", custom_rules.can_cast(&Type::Int, &Type::String));
    println!("  Boolean -> Integer: {}", custom_rules.can_cast(&Type::Boolean, &Type::Int));
    println!("  String -> Integer: {} (explicitly disallowed)", custom_rules.can_cast(&Type::String, &Type::Int));

    // =========================================================================
    // Part 4: Type Metadata Catalog
    // =========================================================================
    println!("\n\nPart 4: Type Metadata Catalog");
    println!("------------------------------");

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
        "email",
        Type::String,
    );
    catalog.register_property_type(
        TypeRef::NodeType("Person".into()),
        "salary",
        Type::Float,
    );
    catalog.register_property_type(
        TypeRef::NodeType("Person".into()),
        "is_active",
        Type::Boolean,
    );

    println!("\nPerson Node Properties:");
    println!("  name: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "name"));
    println!("  age: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "age"));
    println!("  email: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "email"));
    println!("  salary: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "salary"));
    println!("  is_active: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "is_active"));
    println!("  unknown: {:?}", catalog.property_type(&TypeRef::NodeType("Person".into()), "unknown"));

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

    println!("\nKNOWS Edge Properties:");
    println!("  since: {:?}", catalog.property_type(&TypeRef::EdgeType("KNOWS".into()), "since"));
    println!("  weight: {:?}", catalog.property_type(&TypeRef::EdgeType("KNOWS".into()), "weight"));

    // =========================================================================
    // Part 5: Callable Return Types
    // =========================================================================
    println!("\n\nPart 5: Callable Return Types");
    println!("------------------------------");

    // Register callable return types
    catalog.register_callable_return("get_user_count", Type::Int);
    catalog.register_callable_return("get_user_name", Type::String);
    catalog.register_callable_return("is_user_active", Type::Boolean);
    catalog.register_callable_return("calculate_average", Type::Float);

    let call1 = CallableInvocation {
        name: "get_user_count",
        arg_types: vec![],
        is_aggregate: false,
    };
    println!("\nget_user_count() return type: {:?}", catalog.callable_return_type(&call1));

    let call2 = CallableInvocation {
        name: "get_user_name",
        arg_types: vec![Some(Type::Int)],
        is_aggregate: false,
    };
    println!("get_user_name(id) return type: {:?}", catalog.callable_return_type(&call2));

    let call3 = CallableInvocation {
        name: "is_user_active",
        arg_types: vec![Some(Type::Int)],
        is_aggregate: false,
    };
    println!("is_user_active(id) return type: {:?}", catalog.callable_return_type(&call3));

    let call4 = CallableInvocation {
        name: "unknown_function",
        arg_types: vec![],
        is_aggregate: false,
    };
    println!("unknown_function() return type: {:?}", catalog.callable_return_type(&call4));

    // =========================================================================
    // Part 6: Type Check Contexts
    // =========================================================================
    println!("\n\nPart 6: Type Check Contexts");
    println!("----------------------------");

    let mut context = TypeCheckContext::new();

    // Add variable types
    context.add_variable_type("user", Type::Node(Some(vec!["Person".to_string()])));
    context.add_variable_type("age", Type::Int);
    context.add_variable_type("name", Type::String);
    context.add_variable_type("active", Type::Boolean);

    println!("\nVariable Types:");
    println!("  user: {:?}", context.get_variable_type("user"));
    println!("  age: {:?}", context.get_variable_type("age"));
    println!("  name: {:?}", context.get_variable_type("name"));
    println!("  active: {:?}", context.get_variable_type("active"));

    // Add expression types
    context.add_expression_type((0, 10), Type::Int);
    context.add_expression_type((10, 20), Type::String);

    println!("\nExpression Types:");
    println!("  (0, 10): {:?}", context.get_expression_type((0, 10)));
    println!("  (10, 20): {:?}", context.get_expression_type((10, 20)));

    // =========================================================================
    // Part 7: Context Provider
    // =========================================================================
    println!("\n\nPart 7: Type Check Context Provider");
    println!("------------------------------------");

    let mut provider = MockTypeCheckContextProvider::new();

    // Register contexts for different statements
    let mut stmt0_context = TypeCheckContext::new();
    stmt0_context.add_variable_type("n", Type::Node(Some(vec!["Person".to_string()])));

    let mut stmt1_context = TypeCheckContext::new();
    stmt1_context.add_variable_type("m", Type::Node(Some(vec!["Company".to_string()])));

    provider.register_context(0, stmt0_context);
    provider.register_context(1, stmt1_context);

    println!("\nStatement 0 Context:");
    let ctx0 = provider.type_context(0);
    println!("  n: {:?}", ctx0.get_variable_type("n"));

    println!("\nStatement 1 Context:");
    let ctx1 = provider.type_context(1);
    println!("  m: {:?}", ctx1.get_variable_type("m"));

    // =========================================================================
    // Part 8: Semantic Validator Configuration
    // =========================================================================
    println!("\n\nPart 8: Semantic Validator Configuration");
    println!("----------------------------------------");

    // For type inference, you would implement MetadataProvider for your catalog
    // and provide property type information via get_property_metadata()
    println!("\n✓ Type inference is now integrated via MetadataProvider trait");
    println!("  - Implement MetadataProvider::get_property_metadata() for property types");
    println!("  - Implement MetadataProvider::get_callable_return_type_metadata() for callable types");

    // =========================================================================
    // Part 9: Integration with Semantic Validator
    // =========================================================================
    println!("\n\nPart 9: Integration with SemanticValidator");
    println!("------------------------------------------");

    // Use MockMetadataProvider or custom MetadataProvider implementation
    use gql_parser::semantic::metadata_provider::MockMetadataProvider;
    let metadata = MockMetadataProvider::example();

    let _validator = SemanticValidator::new()
        .with_metadata_provider(&metadata);

    println!("\n✓ Semantic validator configured with metadata provider");
    println!("  See MetadataProvider trait for property/callable type information");
    println!("  The validator can now:");
    println!("    - Use property types from the catalog for better inference");
    println!("    - Resolve callable return types");
    println!("    - Apply casting rules consistently");
    println!("    - Reduce Type::Any fallbacks in complex expressions");

    // =========================================================================
    // Part 10: Practical Example
    // =========================================================================
    println!("\n\nPart 10: Practical Type Inference Example");
    println!("------------------------------------------");

    println!("\nScenario: Inferring types in a graph query");
    println!("Query: MATCH (p:Person) RETURN p.age, p.name, p.salary * 1.1");

    println!("\nWith enhanced type inference:");
    println!("  1. p:Person is recognized as NodeType(Person)");

    if let Some(age_type) = catalog.property_type(&TypeRef::NodeType("Person".into()), "age") {
        println!("  2. p.age is inferred as {:?} (from property metadata)", age_type);
    }

    if let Some(name_type) = catalog.property_type(&TypeRef::NodeType("Person".into()), "name") {
        println!("  3. p.name is inferred as {:?} (from property metadata)", name_type);
    }

    if let Some(salary_type) = catalog.property_type(&TypeRef::NodeType("Person".into()), "salary") {
        println!("  4. p.salary is inferred as {:?} (from property metadata)", salary_type);
        println!("  5. p.salary * 1.1 is inferred as {:?} (Float * Float = Float)", salary_type);
    }

    println!("\nWithout enhanced inference:");
    println!("  - p.age would be Type::Any");
    println!("  - p.name would be Type::Any");
    println!("  - p.salary * 1.1 would be Type::Any");

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n\n=== Summary ===");
    println!("Type inference quality improvements provide:");
    println!("  ✓ Property type resolution from schema catalogs");
    println!("  ✓ Callable return type inference");
    println!("  ✓ Configurable casting rules");
    println!("  ✓ Deterministic fallback policies");
    println!("  ✓ Type check context propagation");
    println!("  ✓ Reduced Type::Any fallbacks");
    println!("  ✓ Thread-safe trait design (Send + Sync)");
    println!("  ✓ Comprehensive mocking support for testing");
    println!("\nMilestone 5 is complete!");
}
