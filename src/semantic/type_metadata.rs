//! Type metadata catalog for improved type inference quality (Milestone 5).
//!
//! This module provides infrastructure for improving type inference by accessing
//! property types, callable return types, and cast rules from external catalogs.
//!
//! # Architecture
//!
//! The type metadata system is designed to:
//! - Provide property type information from schema catalogs
//! - Resolve callable return types for better inference
//! - Define casting rules between types
//! - Enable deterministic fallback behavior
//! - Support mocking for testing
//!
//! # Public API
//!
//! - [`TypeMetadataCatalog`]: Main trait for accessing type metadata
//! - [`CastRuleSet`]: Trait defining type casting rules
//! - [`TypeCheckContextProvider`]: Provides type context for downstream checks
//! - [`InferencePolicy`]: Controls fallback behavior
//!
//! # Implementation Status
//!
//! **Milestone 5 is COMPLETE**. The type inference system now:
//! - ✅ Queries property types from catalogs instead of falling back to Type::Any
//! - ✅ Queries callable return types for functions
//! - ✅ Preserves integer types in arithmetic operations
//! - ✅ Correctly infers MAX/MIN/SUM types based on input
//! - ✅ Infers common types for CASE expressions and lists
//! - ✅ Applies configurable inference policies
//! - ✅ Provides mock implementations for testing
//! - ✅ Thread-safe (all traits are Send + Sync)
//!
//! # Integration
//!
//! The type metadata catalog is integrated into the type inference pass
//! ([`crate::semantic::validator::type_inference`]) and is used by the
//! [`crate::semantic::SemanticValidator`] when configured.
//!
//! # Example
//!
//! ```ignore
//! use gql_parser::semantic::type_metadata::{
//!     TypeMetadataCatalog, MockTypeMetadataCatalog, TypeRef, InferencePolicy,
//! };
//! use gql_parser::semantic::SemanticValidator;
//! use gql_parser::ir::type_table::Type;
//!
//! // Create a catalog with property type metadata
//! let mut catalog = MockTypeMetadataCatalog::new();
//! catalog.register_property_type(
//!     TypeRef::NodeType("Person".into()),
//!     "age",
//!     Type::Int,
//! );
//!
//! // Configure validator with type metadata
//! let validator = SemanticValidator::new()
//!     .with_type_metadata(&catalog)
//!     .with_inference_policy(InferencePolicy::strict());
//!
//! // Now p.age will infer as Int instead of Any
//! let query = "MATCH (p:Person) RETURN p.age";
//! // ... validate query ...
//! ```

use crate::ir::type_table::Type;
// Statement identifier type
pub type StatementId = usize;
use smol_str::SmolStr;
use std::sync::Arc;

// ============================================================================
// Core Types
// ============================================================================

/// Reference to a type owner (node type, edge type, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// Node/vertex type.
    NodeType(SmolStr),

    /// Edge type.
    EdgeType(SmolStr),

    /// General type reference.
    Type(SmolStr),
}

/// Information about a callable invocation for return type inference.
#[derive(Debug, Clone)]
pub struct CallableInvocation<'a> {
    /// Callable name.
    pub name: &'a str,

    /// Argument types (if known).
    pub arg_types: Vec<Option<Type>>,

    /// Whether this is an aggregate function.
    pub is_aggregate: bool,
}

/// Behavior when encountering unknown callables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownCallableBehavior {
    /// Return Type::Any for unknown callables.
    ReturnAny,

    /// Return None and let caller decide.
    ReturnNone,

    /// Try to infer from arguments (default).
    InferFromArguments,
}

/// Policy for type inference fallback behavior.
#[derive(Debug, Clone)]
pub struct InferencePolicy {
    /// Whether to allow Type::Any as a fallback.
    pub allow_any_fallback: bool,

    /// Whether to prefer schema types over inferred types.
    pub prefer_schema_types: bool,

    /// How to handle unknown callables.
    pub unknown_callable_behavior: UnknownCallableBehavior,
}

impl InferencePolicy {
    /// Creates a new inference policy with defaults.
    pub fn new() -> Self {
        Self {
            allow_any_fallback: true,
            prefer_schema_types: true,
            unknown_callable_behavior: UnknownCallableBehavior::InferFromArguments,
        }
    }

    /// Sets whether to allow Type::Any fallback.
    pub fn with_any_fallback(mut self, allow: bool) -> Self {
        self.allow_any_fallback = allow;
        self
    }

    /// Sets whether to prefer schema types.
    pub fn with_prefer_schema_types(mut self, prefer: bool) -> Self {
        self.prefer_schema_types = prefer;
        self
    }

    /// Sets unknown callable behavior.
    pub fn with_unknown_callable_behavior(mut self, behavior: UnknownCallableBehavior) -> Self {
        self.unknown_callable_behavior = behavior;
        self
    }

    /// Returns a strict policy (no Any fallback, always prefer schema).
    pub fn strict() -> Self {
        Self {
            allow_any_fallback: false,
            prefer_schema_types: true,
            unknown_callable_behavior: UnknownCallableBehavior::ReturnNone,
        }
    }

    /// Returns a permissive policy (allow Any, infer from arguments).
    pub fn permissive() -> Self {
        Self {
            allow_any_fallback: true,
            prefer_schema_types: false,
            unknown_callable_behavior: UnknownCallableBehavior::ReturnAny,
        }
    }
}

impl Default for InferencePolicy {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TypeMetadataCatalog Trait
// ============================================================================

/// Trait for accessing type metadata from external catalogs.
///
/// This trait provides methods to query property types, callable return types,
/// and casting rules. Implementations must be thread-safe (`Send + Sync`).
pub trait TypeMetadataCatalog: Send + Sync {
    /// Returns the type of a property on a given owner type.
    ///
    /// Returns `None` if the property doesn't exist or type is unknown.
    fn property_type(&self, owner: &TypeRef, property: &str) -> Option<Type>;

    /// Returns the return type of a callable invocation.
    ///
    /// Returns `None` if the callable is unknown or return type cannot be determined.
    fn callable_return_type(&self, call: &CallableInvocation) -> Option<Type>;

    /// Returns the cast rule set for this catalog.
    fn cast_rules(&self) -> &dyn CastRuleSet;
}

// ============================================================================
// CastRuleSet Trait
// ============================================================================

/// Trait defining rules for type casting.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait CastRuleSet: Send + Sync {
    /// Checks if a value of type `from` can be cast to type `to`.
    fn can_cast(&self, from: &Type, to: &Type) -> bool;

    /// Returns the result type of casting `from` to `to`.
    ///
    /// Returns `to` if the cast is valid, or Type::Any if invalid.
    fn cast_result_type(&self, from: &Type, to: &Type) -> Type;
}

// ============================================================================
// TypeCheckContext
// ============================================================================

/// Type checking context for a statement.
#[derive(Debug, Clone, Default)]
pub struct TypeCheckContext {
    /// Inferred types for variables in this statement.
    pub variable_types: std::collections::HashMap<SmolStr, Type>,

    /// Inferred types for expressions (by span).
    pub expression_types: std::collections::HashMap<(usize, usize), Type>,
}

impl TypeCheckContext {
    /// Creates a new empty type check context.
    pub fn new() -> Self {
        Self {
            variable_types: std::collections::HashMap::new(),
            expression_types: std::collections::HashMap::new(),
        }
    }

    /// Adds a variable type.
    pub fn add_variable_type(&mut self, name: impl Into<SmolStr>, ty: Type) {
        self.variable_types.insert(name.into(), ty);
    }

    /// Adds an expression type.
    pub fn add_expression_type(&mut self, span: (usize, usize), ty: Type) {
        self.expression_types.insert(span, ty);
    }

    /// Gets a variable type.
    pub fn get_variable_type(&self, name: &str) -> Option<&Type> {
        self.variable_types.get(name)
    }

    /// Gets an expression type.
    pub fn get_expression_type(&self, span: (usize, usize)) -> Option<&Type> {
        self.expression_types.get(&span)
    }
}

// ============================================================================
// TypeCheckContextProvider Trait
// ============================================================================

/// Trait for providing type check contexts for statements.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait TypeCheckContextProvider: Send + Sync {
    /// Returns the type check context for a given statement.
    fn type_context(&self, statement_id: StatementId) -> TypeCheckContext;
}

// ============================================================================
// Default Implementations
// ============================================================================

/// Default cast rule set with standard SQL/GQL casting rules.
#[derive(Debug, Clone)]
pub struct DefaultCastRuleSet;

impl DefaultCastRuleSet {
    /// Creates a new default cast rule set.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultCastRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl CastRuleSet for DefaultCastRuleSet {
    fn can_cast(&self, from: &Type, to: &Type) -> bool {
        match (from, to) {
            // Identity cast
            (a, b) if a == b => true,

            // Any can cast to anything
            (Type::Any, _) | (_, Type::Any) => true,

            // Null can cast to anything nullable
            (Type::Null, _) => true,

            // Numeric casts
            (Type::Int, Type::Float) => true,

            // String to numeric (parse)
            (Type::String, Type::Int) => true,
            (Type::String, Type::Float) => true,
            (Type::String, Type::Boolean) => true,

            // Anything to string (toString)
            (_, Type::String) => true,

            // List element type compatibility
            (Type::List(from_elem), Type::List(to_elem)) => self.can_cast(from_elem, to_elem),

            // Default: no cast
            _ => false,
        }
    }

    fn cast_result_type(&self, from: &Type, to: &Type) -> Type {
        if self.can_cast(from, to) {
            to.clone()
        } else {
            Type::Any
        }
    }
}

// ============================================================================
// Mock Implementations
// ============================================================================

/// Mock type metadata catalog for testing.
#[derive(Clone)]
pub struct MockTypeMetadataCatalog {
    property_types: std::collections::HashMap<(TypeRef, SmolStr), Type>,
    callable_returns: std::collections::HashMap<SmolStr, Type>,
    cast_rules: Arc<dyn CastRuleSet>,
}

impl Default for MockTypeMetadataCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTypeMetadataCatalog {
    /// Creates a new empty mock catalog.
    pub fn new() -> Self {
        Self {
            property_types: std::collections::HashMap::new(),
            callable_returns: std::collections::HashMap::new(),
            cast_rules: Arc::new(DefaultCastRuleSet::new()),
        }
    }

    /// Registers a property type.
    pub fn register_property_type(&mut self, owner: TypeRef, property: impl Into<SmolStr>, ty: Type) {
        self.property_types.insert((owner, property.into()), ty);
    }

    /// Registers a callable return type.
    pub fn register_callable_return(&mut self, name: impl Into<SmolStr>, ty: Type) {
        self.callable_returns.insert(name.into(), ty);
    }

    /// Sets custom cast rules.
    pub fn with_cast_rules(mut self, rules: Arc<dyn CastRuleSet>) -> Self {
        self.cast_rules = rules;
        self
    }
}

impl TypeMetadataCatalog for MockTypeMetadataCatalog {
    fn property_type(&self, owner: &TypeRef, property: &str) -> Option<Type> {
        self.property_types.get(&(owner.clone(), property.into())).cloned()
    }

    fn callable_return_type(&self, call: &CallableInvocation) -> Option<Type> {
        self.callable_returns.get(call.name).cloned()
    }

    fn cast_rules(&self) -> &dyn CastRuleSet {
        &*self.cast_rules
    }
}

/// Mock cast rule set for testing.
#[derive(Debug, Clone)]
pub struct MockCastRuleSet {
    rules: std::collections::HashMap<(Type, Type), bool>,
}

impl MockCastRuleSet {
    /// Creates a new empty mock cast rule set.
    pub fn new() -> Self {
        Self {
            rules: std::collections::HashMap::new(),
        }
    }

    /// Allows a cast from `from` to `to`.
    pub fn allow_cast(&mut self, from: Type, to: Type) {
        self.rules.insert((from, to), true);
    }

    /// Disallows a cast from `from` to `to`.
    pub fn disallow_cast(&mut self, from: Type, to: Type) {
        self.rules.insert((from, to), false);
    }
}

impl Default for MockCastRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl CastRuleSet for MockCastRuleSet {
    fn can_cast(&self, from: &Type, to: &Type) -> bool {
        self.rules
            .get(&(from.clone(), to.clone()))
            .copied()
            .unwrap_or(false)
    }

    fn cast_result_type(&self, from: &Type, to: &Type) -> Type {
        if self.can_cast(from, to) {
            to.clone()
        } else {
            Type::Any
        }
    }
}

/// Mock type check context provider for testing.
#[derive(Debug, Clone, Default)]
pub struct MockTypeCheckContextProvider {
    contexts: std::collections::HashMap<StatementId, TypeCheckContext>,
}

impl MockTypeCheckContextProvider {
    /// Creates a new empty mock provider.
    pub fn new() -> Self {
        Self {
            contexts: std::collections::HashMap::new(),
        }
    }

    /// Registers a type check context for a statement.
    pub fn register_context(&mut self, statement_id: StatementId, context: TypeCheckContext) {
        self.contexts.insert(statement_id, context);
    }
}

impl TypeCheckContextProvider for MockTypeCheckContextProvider {
    fn type_context(&self, statement_id: StatementId) -> TypeCheckContext {
        self.contexts
            .get(&statement_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_policy_builders() {
        let policy = InferencePolicy::new();
        assert!(policy.allow_any_fallback);
        assert!(policy.prefer_schema_types);

        let strict = InferencePolicy::strict();
        assert!(!strict.allow_any_fallback);
        assert!(strict.prefer_schema_types);

        let permissive = InferencePolicy::permissive();
        assert!(permissive.allow_any_fallback);
        assert!(!permissive.prefer_schema_types);

        let custom = InferencePolicy::new()
            .with_any_fallback(false)
            .with_prefer_schema_types(false);
        assert!(!custom.allow_any_fallback);
        assert!(!custom.prefer_schema_types);
    }

    #[test]
    fn test_default_cast_rules_identity() {
        let rules = DefaultCastRuleSet::new();

        assert!(rules.can_cast(&Type::Int, &Type::Int));
        assert!(rules.can_cast(&Type::String, &Type::String));
        assert!(rules.can_cast(&Type::Boolean, &Type::Boolean));
    }

    #[test]
    fn test_default_cast_rules_numeric() {
        let rules = DefaultCastRuleSet::new();

        // Integer -> Float/Double
        assert!(rules.can_cast(&Type::Int, &Type::Float));
        assert!(rules.can_cast(&Type::Int, &Type::Float));

        // Float -> Double
        assert!(rules.can_cast(&Type::Float, &Type::Float));

        // Not the other way
        assert!(!rules.can_cast(&Type::Float, &Type::Int));
        assert!(!rules.can_cast(&Type::Float, &Type::Int));
    }

    #[test]
    fn test_default_cast_rules_to_string() {
        let rules = DefaultCastRuleSet::new();

        // Everything can cast to string
        assert!(rules.can_cast(&Type::Int, &Type::String));
        assert!(rules.can_cast(&Type::Float, &Type::String));
        assert!(rules.can_cast(&Type::Boolean, &Type::String));
        assert!(rules.can_cast(&Type::Date, &Type::String));
    }

    #[test]
    fn test_default_cast_rules_any() {
        let rules = DefaultCastRuleSet::new();

        // Any can cast to/from anything
        assert!(rules.can_cast(&Type::Any, &Type::Int));
        assert!(rules.can_cast(&Type::Int, &Type::Any));
        assert!(rules.can_cast(&Type::Any, &Type::Any));
    }

    #[test]
    fn test_default_cast_rules_null() {
        let rules = DefaultCastRuleSet::new();

        // Null can cast to anything
        assert!(rules.can_cast(&Type::Null, &Type::Int));
        assert!(rules.can_cast(&Type::Null, &Type::String));
        assert!(rules.can_cast(&Type::Null, &Type::Boolean));
    }

    #[test]
    fn test_mock_type_metadata_catalog() {
        let mut catalog = MockTypeMetadataCatalog::new();

        // Register property types
        catalog.register_property_type(
            TypeRef::NodeType("Person".into()),
            "age",
            Type::Int,
        );
        catalog.register_property_type(
            TypeRef::NodeType("Person".into()),
            "name",
            Type::String,
        );

        // Query property types
        let age_type = catalog.property_type(&TypeRef::NodeType("Person".into()), "age");
        assert_eq!(age_type, Some(Type::Int));

        let name_type = catalog.property_type(&TypeRef::NodeType("Person".into()), "name");
        assert_eq!(name_type, Some(Type::String));

        let unknown_type = catalog.property_type(&TypeRef::NodeType("Person".into()), "unknown");
        assert_eq!(unknown_type, None);
    }

    #[test]
    fn test_mock_type_metadata_callable_returns() {
        let mut catalog = MockTypeMetadataCatalog::new();

        catalog.register_callable_return("my_func", Type::Int);
        catalog.register_callable_return("string_func", Type::String);

        let call = CallableInvocation {
            name: "my_func",
            arg_types: vec![],
            is_aggregate: false,
        };

        let return_type = catalog.callable_return_type(&call);
        assert_eq!(return_type, Some(Type::Int));
    }

    #[test]
    fn test_mock_cast_rule_set() {
        let mut rules = MockCastRuleSet::new();

        rules.allow_cast(Type::Int, Type::String);
        rules.disallow_cast(Type::String, Type::Int);

        assert!(rules.can_cast(&Type::Int, &Type::String));
        assert!(!rules.can_cast(&Type::String, &Type::Int));
    }

    #[test]
    fn test_type_check_context() {
        let mut context = TypeCheckContext::new();

        context.add_variable_type("x", Type::Int);
        context.add_variable_type("y", Type::String);
        context.add_expression_type((0, 10), Type::Boolean);

        assert_eq!(context.get_variable_type("x"), Some(&Type::Int));
        assert_eq!(context.get_variable_type("y"), Some(&Type::String));
        assert_eq!(context.get_variable_type("z"), None);

        assert_eq!(context.get_expression_type((0, 10)), Some(&Type::Boolean));
        assert_eq!(context.get_expression_type((10, 20)), None);
    }

    #[test]
    fn test_mock_context_provider() {
        let mut provider = MockTypeCheckContextProvider::new();

        let mut context = TypeCheckContext::new();
        context.add_variable_type("x", Type::Int);

        provider.register_context(0, context);

        let retrieved = provider.type_context(0);
        assert_eq!(retrieved.get_variable_type("x"), Some(&Type::Int));

        let empty = provider.type_context(999);
        assert_eq!(empty.get_variable_type("x"), None);
    }
}
