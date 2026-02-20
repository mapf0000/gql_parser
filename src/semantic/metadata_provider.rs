//! Unified metadata provider for semantic validation.
//!
//! This module provides the `MetadataProvider` trait, which serves as the
//! single interface for providing all metadata needed for validation.

use crate::ast::{Program, expression::Expression, types::ValueType};
use crate::semantic::schema_catalog::{
    SchemaSnapshot, GraphRef, SchemaRef, SessionContext,
    VariableTypeContext, CatalogError, TypeRef,
};
use crate::semantic::callable::CallableSignature;
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Arc;

/// Provides all metadata needed for enhanced semantic validation.
///
/// Database implementations expose their catalog through this trait.
/// Test implementations use in-memory structures or fixtures.
///
/// # Example - Database Implementation
///
/// ```ignore
/// struct PostgresMetadata {
///     pool: ConnectionPool,
/// }
///
/// impl MetadataProvider for PostgresMetadata {
///     fn get_schema_snapshot(
///         &self,
///         graph: &GraphRef,
///         schema: Option<&SchemaRef>,
///     ) -> Result<Arc<dyn SchemaSnapshot>, CatalogError> {
///         // Query pg_catalog tables
///         self.pool.query_schema(graph, schema)
///     }
///
///     fn lookup_callable(&self, name: &str) -> Option<CallableSignature> {
///         // Query pg_proc for user-defined functions
///         self.pool.query_callable(name)
///     }
///
///     // ... implement other methods
/// }
/// ```
pub trait MetadataProvider: Send + Sync {
    /// Gets schema snapshot for validation.
    ///
    /// This snapshot represents a consistent view of the schema metadata
    /// at a specific point in time.
    fn get_schema_snapshot(
        &self,
        graph: &GraphRef,
        schema: Option<&SchemaRef>,
    ) -> Result<Arc<dyn SchemaSnapshot>, CatalogError>;

    /// Resolves active graph from session context.
    ///
    /// Determines which graph to use based on session state
    /// (e.g., from SESSION SET GRAPH or USE GRAPH statements).
    fn resolve_active_graph(&self, session: &SessionContext) -> Result<GraphRef, CatalogError>;

    /// Resolves active schema for a graph.
    ///
    /// Returns the default schema for the given graph.
    fn resolve_active_schema(&self, graph: &GraphRef) -> Result<SchemaRef, CatalogError>;

    /// Validates that a graph exists (for USE GRAPH validation).
    fn validate_graph_exists(&self, name: &str) -> Result<(), CatalogError>;

    /// Looks up callable (function/procedure) signature.
    ///
    /// Returns the signature if the callable exists, None otherwise.
    fn lookup_callable(&self, name: &str) -> Option<CallableSignature>;

    /// Validates a callable invocation.
    ///
    /// Checks arity, argument types, and other callable-specific constraints.
    ///
    /// # Default Implementation
    ///
    /// Performs basic arity checking. Override for more sophisticated validation.
    fn validate_callable_invocation(
        &self,
        signature: &CallableSignature,
        args: &[&Expression],
    ) -> Result<(), String> {
        // Default: basic arity check
        let min_arity = signature.min_arity();
        if args.len() < min_arity {
            return Err(format!(
                "Function '{}' requires at least {} arguments, got {}",
                signature.name,
                min_arity,
                args.len()
            ));
        }
        if let Some(max_arity) = signature.max_arity() {
            if args.len() > max_arity {
                return Err(format!(
                    "Function '{}' accepts at most {} arguments, got {}",
                    signature.name,
                    max_arity,
                    args.len()
                ));
            }
        }
        Ok(())
    }

    /// Gets property type metadata for type inference.
    ///
    /// When the type inference engine encounters a property access,
    /// it uses this to determine the result type instead of
    /// falling back to Type::Any.
    ///
    /// # Default Implementation
    ///
    /// Returns None (unknown type). Override to provide property type information.
    fn get_property_metadata(&self, _owner: &TypeRef, _property: &str) -> Option<ValueType> {
        None
    }

    /// Gets callable return type metadata for type inference.
    ///
    /// Used by type inference to determine the result type of function calls.
    ///
    /// # Default Implementation
    ///
    /// Returns None (unknown type). Override to provide return type information.
    fn get_callable_return_type_metadata(&self, _name: &str) -> Option<ValueType> {
        None
    }

    /// Gets initial variable type metadata (for parameterized queries).
    ///
    /// This is used when the query has parameters whose types are
    /// known from external context.
    ///
    /// # Default Implementation
    ///
    /// Returns empty context. Override if you support parameterized queries.
    fn get_variable_type_metadata(
        &self,
        _graph: &GraphRef,
        _program: &Program,
    ) -> Result<VariableTypeContext, CatalogError> {
        Ok(VariableTypeContext::new())
    }
}

// ============================================================================
// In-Memory Test Implementation
// ============================================================================

/// In-memory metadata provider for testing.
///
/// This implementation stores metadata in memory and is suitable
/// for unit tests and integration tests.
pub struct InMemoryMetadataProvider {
    snapshots: HashMap<SmolStr, Arc<crate::semantic::schema_catalog::InMemorySchemaSnapshot>>,
    callables: HashMap<SmolStr, CallableSignature>,
    property_types: HashMap<(TypeRef, SmolStr), ValueType>,
    default_graph: GraphRef,
    default_schema: SchemaRef,
}

impl InMemoryMetadataProvider {
    /// Creates a new empty in-memory metadata provider.
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            callables: HashMap::new(),
            property_types: HashMap::new(),
            default_graph: GraphRef {
                name: "default".into(),
            },
            default_schema: SchemaRef {
                name: "public".into(),
            },
        }
    }

    /// Adds a schema snapshot for a graph.
    pub fn add_schema_snapshot(
        &mut self,
        graph: impl Into<SmolStr>,
        snapshot: crate::semantic::schema_catalog::InMemorySchemaSnapshot,
    ) {
        self.snapshots.insert(graph.into(), Arc::new(snapshot));
    }

    /// Adds a callable (function/procedure) signature.
    pub fn add_callable(&mut self, name: impl Into<SmolStr>, signature: CallableSignature) {
        self.callables.insert(name.into(), signature);
    }

    /// Adds property type metadata.
    pub fn add_property_type_metadata(
        &mut self,
        owner: TypeRef,
        property: impl Into<SmolStr>,
        value_type: ValueType,
    ) {
        self.property_types
            .insert((owner, property.into()), value_type);
    }

    /// Creates an example metadata provider with common fixtures.
    pub fn example() -> Self {
        let mut provider = Self::new();

        // Add schema
        let snapshot = crate::semantic::schema_catalog::InMemorySchemaSnapshot::example();
        provider.add_schema_snapshot("default", snapshot);

        // Add built-in callables (these would normally come from BuiltinCallableCatalog)
        // For simplicity, we'll let the actual CallableCatalog handle this

        provider
    }
}

impl Default for InMemoryMetadataProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataProvider for InMemoryMetadataProvider {
    fn get_schema_snapshot(
        &self,
        graph: &GraphRef,
        _schema: Option<&SchemaRef>,
    ) -> Result<Arc<dyn SchemaSnapshot>, CatalogError> {
        self.snapshots
            .get(&graph.name)
            .map(|s| s.clone() as Arc<dyn SchemaSnapshot>)
            .ok_or_else(|| CatalogError::GraphNotFound {
                graph: graph.name.clone(),
            })
    }

    fn resolve_active_graph(&self, session: &SessionContext) -> Result<GraphRef, CatalogError> {
        Ok(session
            .active_graph
            .as_ref()
            .map(|name| GraphRef { name: name.clone() })
            .unwrap_or_else(|| self.default_graph.clone()))
    }

    fn resolve_active_schema(&self, _graph: &GraphRef) -> Result<SchemaRef, CatalogError> {
        Ok(self.default_schema.clone())
    }

    fn validate_graph_exists(&self, name: &str) -> Result<(), CatalogError> {
        if self.snapshots.contains_key(name) {
            Ok(())
        } else {
            Err(CatalogError::GraphNotFound {
                graph: name.into(),
            })
        }
    }

    fn lookup_callable(&self, name: &str) -> Option<CallableSignature> {
        self.callables.get(name).cloned()
    }

    fn get_property_metadata(&self, owner: &TypeRef, property: &str) -> Option<ValueType> {
        self.property_types
            .get(&(owner.clone(), property.into()))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_provider_creation() {
        let provider = InMemoryMetadataProvider::new();
        assert!(provider.snapshots.is_empty());
        assert!(provider.callables.is_empty());
    }

    #[test]
    fn test_example_provider() {
        let provider = InMemoryMetadataProvider::example();

        // Should have default graph
        let graph = GraphRef {
            name: "default".into(),
        };
        let result = provider.get_schema_snapshot(&graph, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_graph_validation() {
        let mut provider = InMemoryMetadataProvider::new();
        let snapshot = crate::semantic::schema_catalog::InMemorySchemaSnapshot::new();
        provider.add_schema_snapshot("test_graph", snapshot);

        // Existing graph
        assert!(provider.validate_graph_exists("test_graph").is_ok());

        // Non-existent graph
        assert!(provider.validate_graph_exists("nonexistent").is_err());
    }

    #[test]
    fn test_resolve_active_graph() {
        let provider = InMemoryMetadataProvider::new();

        // Empty session uses default
        let session = SessionContext::new();
        let graph = provider.resolve_active_graph(&session).unwrap();
        assert_eq!(graph.name, "default");

        // Session with active graph
        let mut session = SessionContext::new();
        session.active_graph = Some("custom".into());
        let graph = provider.resolve_active_graph(&session).unwrap();
        assert_eq!(graph.name, "custom");
    }
}
