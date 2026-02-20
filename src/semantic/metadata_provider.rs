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

    /// Looks up user-defined callable (function/procedure) signature.
    ///
    /// Returns the signature if the UDF exists, None otherwise.
    ///
    /// **Note**: This method should only return user-defined functions/procedures.
    /// Built-in functions are checked separately by the validator.
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
// Mock Test Double Implementation
// ============================================================================

/// Mock metadata provider for testing and examples.
///
/// **This is a test double** - not for production use.
///
/// This implementation stores metadata in memory and is suitable
/// for unit tests, integration tests, and examples.
pub struct MockMetadataProvider {
    snapshots: HashMap<SmolStr, Arc<crate::semantic::schema_catalog::InMemorySchemaSnapshot>>,
    callables: HashMap<SmolStr, CallableSignature>,
    property_types: HashMap<(TypeRef, SmolStr), ValueType>,
    default_graph: GraphRef,
    default_schema: SchemaRef,
}

impl MockMetadataProvider {
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

        // Built-ins are automatically available via default implementation
        // Just add any custom UDFs if needed

        provider
    }

    /// Creates a metadata provider with standard schema fixtures.
    ///
    /// Includes:
    /// - `social_graph`: Person nodes with KNOWS edges
    /// - `financial`: Account nodes with TRANSFER edges
    pub fn with_standard_fixtures() -> Self {
        use crate::semantic::schema_catalog::*;
        use std::collections::BTreeMap;

        let mut provider = Self::new();

        // Social graph fixture
        let mut social = InMemorySchemaSnapshot::new();

        let mut person_props = BTreeMap::new();
        person_props.insert("name".into(), PropertyMeta::string("name", true));
        person_props.insert("age".into(), PropertyMeta::int("age", false));
        person_props.insert(
            "email".into(),
            PropertyMeta::string("email", false).with_constraint(PropertyConstraint::Unique)
        );

        social.add_node_type(NodeTypeMeta {
            name: "Person".into(),
            properties: person_props,
            constraints: vec![ConstraintMeta::PrimaryKey {
                properties: vec!["name".into()],
            }],
            parents: vec![],
            metadata: std::collections::HashMap::new(),
        });

        let mut knows_props = BTreeMap::new();
        knows_props.insert("since".into(), PropertyMeta::date("since", false));

        social.add_edge_type(EdgeTypeMeta {
            name: "KNOWS".into(),
            properties: knows_props,
            constraints: vec![],
            parents: vec![],
            metadata: std::collections::HashMap::new(),
        });

        provider.add_schema_snapshot("social_graph", social);

        // Financial fixture
        let mut financial = InMemorySchemaSnapshot::new();

        let mut account_props = BTreeMap::new();
        account_props.insert(
            "account_id".into(),
            PropertyMeta::string("account_id", true).with_constraint(PropertyConstraint::Unique)
        );
        account_props.insert("balance".into(), PropertyMeta::decimal("balance", true, 18, 2));

        financial.add_node_type(NodeTypeMeta {
            name: "Account".into(),
            properties: account_props,
            constraints: vec![ConstraintMeta::PrimaryKey {
                properties: vec!["account_id".into()],
            }],
            parents: vec![],
            metadata: std::collections::HashMap::new(),
        });

        let mut transfer_props = BTreeMap::new();
        transfer_props.insert("amount".into(), PropertyMeta::decimal("amount", true, 18, 2));
        transfer_props.insert("timestamp".into(), PropertyMeta::datetime("timestamp", true));

        financial.add_edge_type(EdgeTypeMeta {
            name: "TRANSFER".into(),
            properties: transfer_props,
            constraints: vec![],
            parents: vec![],
            metadata: std::collections::HashMap::new(),
        });

        provider.add_schema_snapshot("financial", financial);

        provider
    }

    /// Creates a metadata provider with extended fixtures.
    ///
    /// Includes standard fixtures plus:
    /// - `ecommerce`: Product, Customer, Order nodes with edges
    /// - `healthcare`: Patient, Doctor, Appointment nodes with edges
    pub fn with_extended_fixtures() -> Self {
        use crate::semantic::schema_catalog::*;

        let mut provider = Self::with_standard_fixtures();

        // E-commerce fixture
        let ecommerce = SchemaSnapshotBuilder::new()
            .with_node_type("Product", |builder| {
                builder
                    .add_property(PropertyMeta::string("product_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::string("name", true))
                    .add_property(PropertyMeta::string("description", false))
                    .add_property(PropertyMeta::decimal("price", true, 10, 2))
                    .add_property(PropertyMeta::int("stock_quantity", true))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["product_id".into()],
                    })
            })
            .with_node_type("Customer", |builder| {
                builder
                    .add_property(PropertyMeta::string("customer_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::string("email", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::string("name", true))
                    .add_property(PropertyMeta::string("phone", false))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["customer_id".into()],
                    })
            })
            .with_node_type("Order", |builder| {
                builder
                    .add_property(PropertyMeta::string("order_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::datetime("order_date", true))
                    .add_property(PropertyMeta::string("status", true))
                    .add_property(PropertyMeta::decimal("total_amount", true, 12, 2))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["order_id".into()],
                    })
            })
            .with_edge_type("CONTAINS", |builder| {
                builder
                    .add_property(PropertyMeta::int("quantity", true))
                    .add_property(PropertyMeta::decimal("unit_price", true, 10, 2))
            })
            .with_edge_type("PLACED_BY", |builder| {
                builder.add_property(PropertyMeta::datetime("timestamp", true))
            })
            .build();

        provider.add_schema_snapshot("ecommerce", ecommerce);

        // Healthcare fixture
        let healthcare = SchemaSnapshotBuilder::new()
            .with_node_type("Patient", |builder| {
                builder
                    .add_property(PropertyMeta::string("patient_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::string("name", true))
                    .add_property(PropertyMeta::date("date_of_birth", true))
                    .add_property(PropertyMeta::string("blood_type", false))
                    .add_property(PropertyMeta::string("phone", false))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["patient_id".into()],
                    })
            })
            .with_node_type("Doctor", |builder| {
                builder
                    .add_property(PropertyMeta::string("doctor_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::string("name", true))
                    .add_property(PropertyMeta::string("specialty", true))
                    .add_property(PropertyMeta::string("license_number", true).with_constraint(PropertyConstraint::Unique))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["doctor_id".into()],
                    })
            })
            .with_node_type("Appointment", |builder| {
                builder
                    .add_property(PropertyMeta::string("appointment_id", true).with_constraint(PropertyConstraint::Unique))
                    .add_property(PropertyMeta::datetime("scheduled_time", true))
                    .add_property(PropertyMeta::string("status", true))
                    .add_property(PropertyMeta::string("notes", false))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["appointment_id".into()],
                    })
            })
            .with_edge_type("HAS_APPOINTMENT", |builder| {
                builder.add_property(PropertyMeta::datetime("created_at", true))
            })
            .with_edge_type("TREATS", |builder| {
                builder
                    .add_property(PropertyMeta::date("treatment_date", true))
                    .add_property(PropertyMeta::string("diagnosis", false))
            })
            .build();

        provider.add_schema_snapshot("healthcare", healthcare);

        provider
    }
}

impl Default for MockMetadataProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataProvider for MockMetadataProvider {
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
        // Only return UDFs - built-ins are checked separately by the validator
        self.callables
            .get(name)
            .cloned()
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
        let provider = MockMetadataProvider::new();
        assert!(provider.snapshots.is_empty());
        assert!(provider.callables.is_empty());
    }

    #[test]
    fn test_example_provider() {
        let provider = MockMetadataProvider::example();

        // Should have default graph
        let graph = GraphRef {
            name: "default".into(),
        };
        let result = provider.get_schema_snapshot(&graph, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_graph_validation() {
        let mut provider = MockMetadataProvider::new();
        let snapshot = crate::semantic::schema_catalog::InMemorySchemaSnapshot::new();
        provider.add_schema_snapshot("test_graph", snapshot);

        // Existing graph
        assert!(provider.validate_graph_exists("test_graph").is_ok());

        // Non-existent graph
        assert!(provider.validate_graph_exists("nonexistent").is_err());
    }

    #[test]
    fn test_resolve_active_graph() {
        let provider = MockMetadataProvider::new();

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
