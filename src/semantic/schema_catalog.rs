//! Schema catalog system for semantic validation.
//!
//! This module provides core schema metadata types used by the `MetadataProvider`
//! trait for validation. All catalog operations go through `MetadataProvider`.
//!
//! # Architecture
//!
//! - **Core Types**: `NodeTypeMeta`, `EdgeTypeMeta`, `PropertyMeta`, etc.
//! - **Schema Views**: `SchemaSnapshot` - immutable, query-time view
//! - **Test Implementations**: `InMemorySchemaSnapshot` for testing
//! - **Session Context**: `SessionContext` - tracks active graph/schema
//!
//! # Usage
//!
//! For validation with schema metadata, use `MockMetadataProvider` from
//! the `metadata_provider` module, which implements the `MetadataProvider` trait.

use crate::ast::types::ValueType;
use smol_str::SmolStr;
use std::collections::{BTreeMap, HashMap};

// ============================================================================
// Session Context
// ============================================================================

/// Simple session context for graph/schema resolution.
///
/// This structure holds session-level information that may affect
/// which graph and schema are active (e.g., from SESSION SET commands).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionContext {
    /// The currently active graph (from SESSION SET GRAPH)
    pub active_graph: Option<SmolStr>,
    /// The currently active schema (from SESSION SET SCHEMA)
    pub active_schema: Option<SmolStr>,
}

impl SessionContext {
    /// Creates a new empty session context.
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Core Metadata Types
// ============================================================================

/// Synthetic span constant for schema-generated types.
const SYNTHETIC_SPAN: std::ops::Range<usize> = 0..0;

/// Reference to a type (node type, edge type, or property owner).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// Reference to a node type by label name
    NodeType(SmolStr),
    /// Reference to an edge type by label name
    EdgeType(SmolStr),
}

/// Metadata about a node type in the schema.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeMeta {
    /// The label name of this node type
    pub name: SmolStr,
    /// Properties defined on this node type (ordered by name for determinism)
    pub properties: BTreeMap<SmolStr, PropertyMeta>,
    /// Constraints applied to this node type
    pub constraints: Vec<ConstraintMeta>,
    /// Parent node types (for inheritance)
    pub parents: Vec<TypeRef>,
    /// Additional metadata
    pub metadata: HashMap<SmolStr, SmolStr>,
}

/// Metadata about an edge type in the schema.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypeMeta {
    /// The label name of this edge type
    pub name: SmolStr,
    /// Properties defined on this edge type (ordered by name for determinism)
    pub properties: BTreeMap<SmolStr, PropertyMeta>,
    /// Constraints applied to this edge type
    pub constraints: Vec<ConstraintMeta>,
    /// Parent edge types (for inheritance)
    pub parents: Vec<TypeRef>,
    /// Additional metadata
    pub metadata: HashMap<SmolStr, SmolStr>,
}

/// Metadata about a property.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyMeta {
    /// Property name
    pub name: SmolStr,
    /// Declared type of the property
    pub value_type: ValueType,
    /// Whether the property is required (NOT NULL)
    pub required: bool,
    /// Additional constraints on this property
    pub constraints: Vec<PropertyConstraint>,
}

impl PropertyMeta {
    /// Creates a string property.
    pub fn string(name: impl Into<SmolStr>, required: bool) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Predefined(
                crate::ast::types::PredefinedType::CharacterString(
                    crate::ast::types::CharacterStringType::String
                ),
                SYNTHETIC_SPAN,
            ),
            required,
            constraints: vec![],
        }
    }

    /// Creates an integer property.
    pub fn int(name: impl Into<SmolStr>, required: bool) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Predefined(
                crate::ast::types::PredefinedType::Numeric(
                    crate::ast::types::NumericType::Exact(
                        crate::ast::types::ExactNumericType::SignedBinary(
                            crate::ast::types::SignedBinaryExactNumericType::Int
                        )
                    )
                ),
                SYNTHETIC_SPAN,
            ),
            required,
            constraints: vec![],
        }
    }

    /// Creates a decimal property.
    pub fn decimal(name: impl Into<SmolStr>, required: bool, precision: u32, scale: u32) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Predefined(
                crate::ast::types::PredefinedType::Numeric(
                    crate::ast::types::NumericType::Exact(
                        crate::ast::types::ExactNumericType::Decimal(
                            crate::ast::types::DecimalExactNumericType {
                                kind: crate::ast::types::DecimalKind::Decimal,
                                precision: Some(precision),
                                scale: Some(scale),
                                span: SYNTHETIC_SPAN,
                            }
                        )
                    )
                ),
                SYNTHETIC_SPAN,
            ),
            required,
            constraints: vec![],
        }
    }

    /// Creates a date property.
    pub fn date(name: impl Into<SmolStr>, required: bool) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Predefined(
                crate::ast::types::PredefinedType::Temporal(
                    crate::ast::types::TemporalType::Instant(
                        crate::ast::types::TemporalInstantType::Date
                    )
                ),
                SYNTHETIC_SPAN,
            ),
            required,
            constraints: vec![],
        }
    }

    /// Creates a datetime property.
    pub fn datetime(name: impl Into<SmolStr>, required: bool) -> Self {
        Self {
            name: name.into(),
            value_type: ValueType::Predefined(
                crate::ast::types::PredefinedType::Temporal(
                    crate::ast::types::TemporalType::Instant(
                        crate::ast::types::TemporalInstantType::LocalDatetime
                    )
                ),
                SYNTHETIC_SPAN,
            ),
            required,
            constraints: vec![],
        }
    }

    /// Adds a constraint to this property.
    pub fn with_constraint(mut self, constraint: PropertyConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

/// Property-level constraints.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyConstraint {
    /// UNIQUE constraint
    Unique,
    /// CHECK constraint with expression (stored as string for now)
    Check { expression: SmolStr },
    /// DEFAULT value (stored as string for now)
    Default { value: SmolStr },
}

/// Schema-level constraints.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintMeta {
    /// Primary key constraint
    PrimaryKey {
        /// Property names forming the key
        properties: Vec<SmolStr>,
    },
    /// Unique constraint
    Unique {
        /// Property names forming the unique constraint
        properties: Vec<SmolStr>,
    },
    /// Foreign key constraint
    ForeignKey {
        /// Local properties
        properties: Vec<SmolStr>,
        /// Referenced type
        references: TypeRef,
        /// Referenced properties
        referenced_properties: Vec<SmolStr>,
    },
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during catalog operations.
///
/// These errors are returned by catalog trait methods. For diagnostic reporting,
/// convert to `Diag` using `.to_diag(span)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// The requested schema snapshot is not available
    SnapshotUnavailable { reason: SmolStr },
    /// The requested graph was not found
    GraphNotFound { graph: SmolStr },
    /// The requested schema was not found
    SchemaNotFound { schema: SmolStr },
    /// Invalid snapshot request
    InvalidRequest { reason: SmolStr },
    /// General catalog error
    General { message: SmolStr },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::SnapshotUnavailable { reason } => {
                write!(f, "Schema snapshot unavailable: {}", reason)
            }
            CatalogError::GraphNotFound { graph } => {
                write!(f, "Graph '{}' not found", graph)
            }
            CatalogError::SchemaNotFound { schema } => {
                write!(f, "Schema '{}' not found", schema)
            }
            CatalogError::InvalidRequest { reason } => {
                write!(f, "Invalid catalog request: {}", reason)
            }
            CatalogError::General { message } => {
                write!(f, "Catalog error: {}", message)
            }
        }
    }
}

impl std::error::Error for CatalogError {}

impl CatalogError {
    /// Converts this catalog error to a diagnostic at the given span.
    pub fn to_diag(&self, span: crate::ast::Span) -> crate::diag::Diag {
        use crate::diag::{Diag, DiagLabel};

        match self {
            CatalogError::SnapshotUnavailable { reason } => {
                Diag::error(format!("Schema snapshot unavailable: {}", reason))
                    .with_label(DiagLabel::primary(span, "snapshot unavailable"))
            }
            CatalogError::GraphNotFound { graph } => {
                Diag::error(format!("Graph '{}' not found", graph))
                    .with_label(DiagLabel::primary(span, "undefined graph"))
            }
            CatalogError::SchemaNotFound { schema } => {
                Diag::error(format!("Schema '{}' not found", schema))
                    .with_label(DiagLabel::primary(span, "undefined schema"))
            }
            CatalogError::InvalidRequest { reason } => {
                Diag::error(format!("Invalid catalog request: {}", reason))
                    .with_label(DiagLabel::primary(span, reason.as_str()))
            }
            CatalogError::General { message } => {
                Diag::error(format!("Catalog error: {}", message))
                    .with_label(DiagLabel::primary(span, "catalog error"))
            }
        }
    }
}

// ============================================================================
// Graph References
// ============================================================================

/// Reference to a graph in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphRef {
    /// Graph name
    pub name: SmolStr,
}

/// Reference to a schema in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaRef {
    /// Schema name
    pub name: SmolStr,
}

// ============================================================================
// Schema Snapshot Trait
// ============================================================================

/// Immutable, query-time view of schema metadata.
///
/// This trait provides read-only access to schema metadata including
/// node types, edge types, properties, constraints, and inheritance.
///
/// # Immutability
///
/// Snapshots must be immutable once created to ensure validation consistency.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support multi-threaded validation.
pub trait SchemaSnapshot: Send + Sync {
    /// Looks up metadata for a node type by label name.
    ///
    /// Returns `None` if the node type does not exist.
    fn node_type(&self, name: &str) -> Option<&NodeTypeMeta>;

    /// Looks up metadata for an edge type by label name.
    ///
    /// Returns `None` if the edge type does not exist.
    fn edge_type(&self, name: &str) -> Option<&EdgeTypeMeta>;

    /// Looks up a property on a specific type.
    ///
    /// # Arguments
    ///
    /// * `owner` - The type (node or edge) that owns the property
    /// * `property` - The property name
    ///
    /// # Inheritance
    ///
    /// This method checks the specified type first, then recursively
    /// checks parent types if the property is not found directly.
    ///
    /// Returns `None` if the property does not exist on the specified type
    /// or any of its ancestors.
    fn property(&self, owner: TypeRef, property: &str) -> Option<&PropertyMeta>;

    /// Gets all constraints defined on a specific type.
    ///
    /// Returns an empty slice if no constraints are defined.
    fn constraints(&self, owner: TypeRef) -> &[ConstraintMeta];

    /// Gets parent types for inheritance.
    ///
    /// Returns an empty slice if the type has no parents.
    fn parents(&self, owner: TypeRef) -> &[TypeRef];
}

// ============================================================================
// Variable Type Context
// ============================================================================

/// Variable type context for validation.
///
/// This structure holds inferred types for variables during validation.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableTypeContext {
    /// Map from variable name to inferred type
    pub bindings: HashMap<SmolStr, ValueType>,
}

impl VariableTypeContext {
    /// Creates a new empty variable type context.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Adds a variable binding.
    pub fn bind(&mut self, variable: SmolStr, value_type: ValueType) {
        self.bindings.insert(variable, value_type);
    }

    /// Looks up a variable binding.
    pub fn get(&self, variable: &str) -> Option<&ValueType> {
        self.bindings.get(variable)
    }
}

impl Default for VariableTypeContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// In-Memory Test Implementation
// ============================================================================

/// In-memory schema snapshot for testing.
///
/// This implementation stores schema metadata in memory.
#[derive(Debug, Clone, Default)]
pub struct InMemorySchemaSnapshot {
    /// Node types by label name
    pub node_types: HashMap<SmolStr, NodeTypeMeta>,
    /// Edge types by label name
    pub edge_types: HashMap<SmolStr, EdgeTypeMeta>,
}

impl InMemorySchemaSnapshot {
    /// Creates a new empty schema snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node type to the snapshot.
    pub fn add_node_type(&mut self, node_type: NodeTypeMeta) {
        self.node_types.insert(node_type.name.clone(), node_type);
    }

    /// Adds an edge type to the snapshot.
    pub fn add_edge_type(&mut self, edge_type: EdgeTypeMeta) {
        self.edge_types.insert(edge_type.name.clone(), edge_type);
    }

    /// Helper to find a property with inheritance support.
    fn find_property_with_inheritance(&self, owner: &TypeRef, property: &str, visited: &mut std::collections::HashSet<TypeRef>) -> Option<&PropertyMeta> {
        // Avoid infinite loops in case of circular inheritance
        if !visited.insert(owner.clone()) {
            return None;
        }

        match owner {
            TypeRef::NodeType(name) => {
                if let Some(node_meta) = self.node_types.get(name) {
                    // Check direct properties first
                    if let Some(prop) = node_meta.properties.get(property) {
                        return Some(prop);
                    }
                    // Check parents recursively
                    for parent in &node_meta.parents {
                        if let Some(prop) = self.find_property_with_inheritance(parent, property, visited) {
                            return Some(prop);
                        }
                    }
                }
            }
            TypeRef::EdgeType(name) => {
                if let Some(edge_meta) = self.edge_types.get(name) {
                    // Check direct properties first
                    if let Some(prop) = edge_meta.properties.get(property) {
                        return Some(prop);
                    }
                    // Check parents recursively
                    for parent in &edge_meta.parents {
                        if let Some(prop) = self.find_property_with_inheritance(parent, property, visited) {
                            return Some(prop);
                        }
                    }
                }
            }
        }
        None
    }

    /// Creates an example schema with common types.
    pub fn example() -> Self {
        let mut snapshot = Self::new();

        // Person node type
        let mut person_props = BTreeMap::new();
        person_props.insert("name".into(), PropertyMeta::string("name", true));
        person_props.insert("age".into(), PropertyMeta::int("age", false));

        snapshot.add_node_type(NodeTypeMeta {
            name: "Person".into(),
            properties: person_props,
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        });

        // KNOWS edge type
        let mut knows_props = BTreeMap::new();
        knows_props.insert("since".into(), PropertyMeta::int("since", false));

        snapshot.add_edge_type(EdgeTypeMeta {
            name: "KNOWS".into(),
            properties: knows_props,
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        });

        snapshot
    }
}

impl SchemaSnapshot for InMemorySchemaSnapshot {
    fn node_type(&self, name: &str) -> Option<&NodeTypeMeta> {
        self.node_types.get(name)
    }

    fn edge_type(&self, name: &str) -> Option<&EdgeTypeMeta> {
        self.edge_types.get(name)
    }

    fn property(&self, owner: TypeRef, property: &str) -> Option<&PropertyMeta> {
        let mut visited = std::collections::HashSet::new();
        self.find_property_with_inheritance(&owner, property, &mut visited)
    }

    fn constraints(&self, owner: TypeRef) -> &[ConstraintMeta] {
        match owner {
            TypeRef::NodeType(ref name) => {
                self.node_types.get(name)
                    .map(|t| t.constraints.as_slice())
                    .unwrap_or(&[])
            }
            TypeRef::EdgeType(ref name) => {
                self.edge_types.get(name)
                    .map(|t| t.constraints.as_slice())
                    .unwrap_or(&[])
            }
        }
    }

    fn parents(&self, owner: TypeRef) -> &[TypeRef] {
        match owner {
            TypeRef::NodeType(ref name) => {
                self.node_types.get(name)
                    .map(|t| t.parents.as_slice())
                    .unwrap_or(&[])
            }
            TypeRef::EdgeType(ref name) => {
                self.edge_types.get(name)
                    .map(|t| t.parents.as_slice())
                    .unwrap_or(&[])
            }
        }
    }
}

// ============================================================================
// Schema Builder
// ============================================================================

/// Builder for creating schema snapshots with a fluent API.
///
/// This builder simplifies the creation of test schemas by providing
/// a chainable interface for adding node types, edge types, and properties.
///
/// # Example
///
/// ```ignore
/// use gql_parser::semantic::schema_catalog::*;
///
/// let snapshot = SchemaSnapshotBuilder::new()
///     .with_node_type("Person", |builder| {
///         builder
///             .add_property(PropertyMeta::string("name", true))
///             .add_property(PropertyMeta::int("age", false))
///             .add_constraint(ConstraintMeta::PrimaryKey {
///                 properties: vec!["name".into()],
///             })
///     })
///     .with_edge_type("KNOWS", |builder| {
///         builder.add_property(PropertyMeta::date("since", false))
///     })
///     .build();
/// ```
pub struct SchemaSnapshotBuilder {
    snapshot: InMemorySchemaSnapshot,
}

impl SchemaSnapshotBuilder {
    /// Creates a new empty schema builder.
    pub fn new() -> Self {
        Self {
            snapshot: InMemorySchemaSnapshot::new(),
        }
    }

    /// Adds a node type to the schema using a builder closure.
    pub fn with_node_type<F>(mut self, name: impl Into<SmolStr>, builder_fn: F) -> Self
    where
        F: FnOnce(NodeTypeBuilder) -> NodeTypeBuilder,
    {
        let builder = NodeTypeBuilder::new(name.into());
        let node_type = builder_fn(builder).build();
        self.snapshot.add_node_type(node_type);
        self
    }

    /// Adds an edge type to the schema using a builder closure.
    pub fn with_edge_type<F>(mut self, name: impl Into<SmolStr>, builder_fn: F) -> Self
    where
        F: FnOnce(EdgeTypeBuilder) -> EdgeTypeBuilder,
    {
        let builder = EdgeTypeBuilder::new(name.into());
        let edge_type = builder_fn(builder).build();
        self.snapshot.add_edge_type(edge_type);
        self
    }

    /// Builds the schema snapshot.
    pub fn build(self) -> InMemorySchemaSnapshot {
        self.snapshot
    }
}

impl Default for SchemaSnapshotBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for node types.
pub struct NodeTypeBuilder {
    name: SmolStr,
    properties: BTreeMap<SmolStr, PropertyMeta>,
    constraints: Vec<ConstraintMeta>,
    parents: Vec<TypeRef>,
    metadata: HashMap<SmolStr, SmolStr>,
}

impl NodeTypeBuilder {
    /// Creates a new node type builder.
    pub fn new(name: SmolStr) -> Self {
        Self {
            name,
            properties: BTreeMap::new(),
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Adds a property to the node type.
    pub fn add_property(mut self, property: PropertyMeta) -> Self {
        self.properties.insert(property.name.clone(), property);
        self
    }

    /// Adds a constraint to the node type.
    pub fn add_constraint(mut self, constraint: ConstraintMeta) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Adds a parent type for inheritance.
    pub fn add_parent(mut self, parent: TypeRef) -> Self {
        self.parents.push(parent);
        self
    }

    /// Adds metadata to the node type.
    pub fn add_metadata(mut self, key: impl Into<SmolStr>, value: impl Into<SmolStr>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Builds the node type.
    pub fn build(self) -> NodeTypeMeta {
        NodeTypeMeta {
            name: self.name,
            properties: self.properties,
            constraints: self.constraints,
            parents: self.parents,
            metadata: self.metadata,
        }
    }
}

/// Builder for edge types.
pub struct EdgeTypeBuilder {
    name: SmolStr,
    properties: BTreeMap<SmolStr, PropertyMeta>,
    constraints: Vec<ConstraintMeta>,
    parents: Vec<TypeRef>,
    metadata: HashMap<SmolStr, SmolStr>,
}

impl EdgeTypeBuilder {
    /// Creates a new edge type builder.
    pub fn new(name: SmolStr) -> Self {
        Self {
            name,
            properties: BTreeMap::new(),
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Adds a property to the edge type.
    pub fn add_property(mut self, property: PropertyMeta) -> Self {
        self.properties.insert(property.name.clone(), property);
        self
    }

    /// Adds a constraint to the edge type.
    pub fn add_constraint(mut self, constraint: ConstraintMeta) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Adds a parent type for inheritance.
    pub fn add_parent(mut self, parent: TypeRef) -> Self {
        self.parents.push(parent);
        self
    }

    /// Adds metadata to the edge type.
    pub fn add_metadata(mut self, key: impl Into<SmolStr>, value: impl Into<SmolStr>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Builds the edge type.
    pub fn build(self) -> EdgeTypeMeta {
        EdgeTypeMeta {
            name: self.name,
            properties: self.properties,
            constraints: self.constraints,
            parents: self.parents,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_snapshot_example() {
        let snapshot = InMemorySchemaSnapshot::example();

        // Check Person node type exists
        assert!(snapshot.node_type("Person").is_some());
        let person = snapshot.node_type("Person").unwrap();
        assert_eq!(person.name, "Person");
        assert!(person.properties.contains_key("name"));
        assert!(person.properties.contains_key("age"));

        // Check KNOWS edge type exists
        assert!(snapshot.edge_type("KNOWS").is_some());
        let knows = snapshot.edge_type("KNOWS").unwrap();
        assert_eq!(knows.name, "KNOWS");
        assert!(knows.properties.contains_key("since"));
    }

    #[test]
    fn test_schema_snapshot_property_lookup() {
        let snapshot = InMemorySchemaSnapshot::example();

        // Lookup property on Person
        let name_prop = snapshot.property(
            TypeRef::NodeType("Person".into()),
            "name"
        );
        assert!(name_prop.is_some());
        assert_eq!(name_prop.unwrap().name, "name");
        assert!(name_prop.unwrap().required);

        // Lookup property on KNOWS
        let since_prop = snapshot.property(
            TypeRef::EdgeType("KNOWS".into()),
            "since"
        );
        assert!(since_prop.is_some());
        assert_eq!(since_prop.unwrap().name, "since");
        assert!(!since_prop.unwrap().required);

        // Lookup non-existent property
        let missing = snapshot.property(
            TypeRef::NodeType("Person".into()),
            "nonexistent"
        );
        assert!(missing.is_none());
    }

    #[test]
    fn test_session_context_creation() {
        let ctx = SessionContext::new();
        assert!(ctx.active_graph.is_none());
        assert!(ctx.active_schema.is_none());

        let ctx2 = SessionContext {
            active_graph: Some("mygraph".into()),
            active_schema: Some("myschema".into()),
        };
        assert_eq!(ctx2.active_graph.unwrap(), "mygraph");
        assert_eq!(ctx2.active_schema.unwrap(), "myschema");
    }

    #[test]
    fn test_variable_type_context() {
        let mut context = VariableTypeContext::new();
        assert!(context.bindings.is_empty());

        let string_type = ValueType::Predefined(
            crate::ast::types::PredefinedType::CharacterString(
                crate::ast::types::CharacterStringType::String
            ),
            0..0,
        );

        context.bind("x".into(), string_type.clone());
        assert_eq!(context.bindings.len(), 1);
        assert!(context.get("x").is_some());
        assert!(context.get("y").is_none());
    }

    #[test]
    fn test_type_ref_equality() {
        let node_ref1 = TypeRef::NodeType("Person".into());
        let node_ref2 = TypeRef::NodeType("Person".into());
        let edge_ref = TypeRef::EdgeType("KNOWS".into());

        assert_eq!(node_ref1, node_ref2);
        assert_ne!(node_ref1, edge_ref);
    }

    #[test]
    fn test_property_inheritance() {
        let mut snapshot = InMemorySchemaSnapshot::new();

        // Create a base Entity type
        let mut base_props = BTreeMap::new();
        base_props.insert("id".into(), PropertyMeta::string("id", true));
        base_props.insert("created_at".into(), PropertyMeta::datetime("created_at", true));

        snapshot.add_node_type(NodeTypeMeta {
            name: "Entity".into(),
            properties: base_props,
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        });

        // Create a Person type that inherits from Entity
        let mut person_props = BTreeMap::new();
        person_props.insert("name".into(), PropertyMeta::string("name", true));

        snapshot.add_node_type(NodeTypeMeta {
            name: "Person".into(),
            properties: person_props,
            constraints: vec![],
            parents: vec![TypeRef::NodeType("Entity".into())],
            metadata: HashMap::new(),
        });

        // Test direct properties
        assert!(snapshot.property(TypeRef::NodeType("Person".into()), "name").is_some());

        // Test inherited properties
        assert!(snapshot.property(TypeRef::NodeType("Person".into()), "id").is_some());
        assert!(snapshot.property(TypeRef::NodeType("Person".into()), "created_at").is_some());

        // Test non-existent property
        assert!(snapshot.property(TypeRef::NodeType("Person".into()), "nonexistent").is_none());
    }

    #[test]
    fn test_schema_snapshot_builder() {
        let snapshot = SchemaSnapshotBuilder::new()
            .with_node_type("User", |builder| {
                builder
                    .add_property(PropertyMeta::string("username", true))
                    .add_property(PropertyMeta::string("email", true))
                    .add_constraint(ConstraintMeta::PrimaryKey {
                        properties: vec!["username".into()],
                    })
            })
            .with_edge_type("FOLLOWS", |builder| {
                builder.add_property(PropertyMeta::datetime("since", true))
            })
            .build();

        assert!(snapshot.node_type("User").is_some());
        assert!(snapshot.edge_type("FOLLOWS").is_some());

        let user = snapshot.node_type("User").unwrap();
        assert_eq!(user.properties.len(), 2);
        assert_eq!(user.constraints.len(), 1);
    }

    #[test]
    fn test_property_meta_builders() {
        let string_prop = PropertyMeta::string("name", true);
        assert!(string_prop.required);
        assert_eq!(string_prop.name, "name");
        assert!(string_prop.constraints.is_empty());

        let int_prop = PropertyMeta::int("age", false);
        assert!(!int_prop.required);

        let decimal_prop = PropertyMeta::decimal("price", true, 10, 2);
        assert!(decimal_prop.required);

        let date_prop = PropertyMeta::date("birth_date", false);
        assert!(!date_prop.required);

        let datetime_prop = PropertyMeta::datetime("created_at", true);
        assert!(datetime_prop.required);

        // Test with_constraint
        let unique_prop = PropertyMeta::string("email", true)
            .with_constraint(PropertyConstraint::Unique);
        assert_eq!(unique_prop.constraints.len(), 1);
    }
}
