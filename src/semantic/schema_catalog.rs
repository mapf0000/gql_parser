//! Advanced schema catalog system for Milestone 3.
//!
//! This module provides a comprehensive schema catalog system that supports:
//! - Schema metadata with property types, constraints, and inheritance
//! - Snapshot-based immutable schema views for query validation
//! - Graph context resolution for active graph/schema determination
//! - Variable type context for scope/type inference
//! - Test doubles and fixture loading for engine-agnostic testing
//!
//! # Architecture
//!
//! The schema catalog system follows a multi-trait design:
//!
//! - `SchemaCatalog`: Engine-facing entrypoint for obtaining schema snapshots
//! - `SchemaSnapshot`: Immutable, query-time view of schema metadata
//! - `GraphContextResolver`: Resolves active graph and schema for a session
//! - `VariableTypeContextProvider`: Provides initial type bindings for validation
//! - `SchemaFixtureLoader`: Loads test fixtures for regression testing
//!
//! All traits are `Send + Sync` and return typed errors instead of panicking.

use crate::ast::types::ValueType;
use crate::ast::Program;
use smol_str::SmolStr;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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

/// Errors that can occur during fixture loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureError {
    /// The fixture was not found
    NotFound { fixture: SmolStr },
    /// The fixture has invalid format
    InvalidFormat { fixture: SmolStr, reason: SmolStr },
    /// IO error during fixture loading
    IoError { message: SmolStr },
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureError::NotFound { fixture } => {
                write!(f, "Fixture '{}' not found", fixture)
            }
            FixtureError::InvalidFormat { fixture, reason } => {
                write!(f, "Invalid fixture '{}': {}", fixture, reason)
            }
            FixtureError::IoError { message } => {
                write!(f, "Fixture I/O error: {}", message)
            }
        }
    }
}

impl std::error::Error for FixtureError {}

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

/// Request for a schema snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSnapshotRequest {
    /// The graph to get schema for
    pub graph: GraphRef,
    /// Optional specific schema name
    pub schema: Option<SchemaRef>,
}

// ============================================================================
// Core Catalog Traits
// ============================================================================

/// Engine-facing entry point for obtaining schema snapshots.
///
/// This trait is implemented by database engines or test harnesses to provide
/// immutable schema views for validation.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support multi-threaded validation.
///
/// # Error Handling
///
/// All methods return typed `Result` types and must never panic.
pub trait SchemaCatalog: Send + Sync {
    /// Obtains an immutable schema snapshot for validation.
    ///
    /// # Arguments
    ///
    /// * `request` - Specifies which graph/schema to snapshot
    ///
    /// # Returns
    ///
    /// A reference-counted schema snapshot that remains valid for the
    /// duration of validation, or an error if the snapshot cannot be created.
    fn snapshot(&self, request: SchemaSnapshotRequest) -> Result<Arc<dyn SchemaSnapshot>, CatalogError>;
}

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

/// Resolves active graph and schema for a session.
///
/// This trait is used to determine which graph and schema should be used
/// for validation based on session context (e.g., USE GRAPH statements).
pub trait GraphContextResolver: Send + Sync {
    /// Determines the active graph for a session.
    ///
    /// # Arguments
    ///
    /// * `session` - The session context (may contain USE GRAPH, etc.)
    ///
    /// # Returns
    ///
    /// The active graph reference, or an error if no graph is active.
    fn active_graph(&self, session: &SessionContext) -> Result<GraphRef, CatalogError>;

    /// Determines the active schema for a graph.
    ///
    /// # Arguments
    ///
    /// * `graph` - The graph to get schema for
    ///
    /// # Returns
    ///
    /// The active schema reference, or an error if no schema is available.
    fn active_schema(&self, graph: &GraphRef) -> Result<SchemaRef, CatalogError>;
}

/// Provides variable type context for scope/type inference.
///
/// This trait is used during semantic validation to provide initial
/// type bindings for variables based on catalog metadata.
pub trait VariableTypeContextProvider: Send + Sync {
    /// Provides initial variable type bindings for a program.
    ///
    /// # Arguments
    ///
    /// * `graph` - The active graph
    /// * `ast` - The program AST to analyze
    ///
    /// # Returns
    ///
    /// Variable type context with initial bindings, or an error.
    fn initial_bindings(&self, graph: &GraphRef, ast: &Program) -> Result<VariableTypeContext, CatalogError>;
}

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

/// Loads schema fixtures for testing.
///
/// This trait allows test suites to load schema definitions from
/// fixture files without requiring a live database.
pub trait SchemaFixtureLoader: Send + Sync {
    /// Loads a schema snapshot from a fixture.
    ///
    /// # Arguments
    ///
    /// * `fixture` - The fixture identifier (e.g., "social_graph", "financial")
    ///
    /// # Returns
    ///
    /// A schema snapshot loaded from the fixture, or an error.
    fn load(&self, fixture: &str) -> Result<Arc<dyn SchemaSnapshot>, FixtureError>;
}

// ============================================================================
// In-Memory Test Implementations
// ============================================================================

/// In-memory schema catalog for testing.
///
/// This implementation stores schema snapshots in memory and is suitable
/// for unit tests and integration tests.
#[derive(Debug, Clone)]
pub struct InMemorySchemaCatalog {
    /// Stored snapshots by graph name
    snapshots: HashMap<SmolStr, Arc<InMemorySchemaSnapshot>>,
}

impl InMemorySchemaCatalog {
    /// Creates a new empty in-memory schema catalog.
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
        }
    }

    /// Adds a schema snapshot for a graph.
    pub fn add_snapshot(&mut self, graph: SmolStr, snapshot: InMemorySchemaSnapshot) {
        self.snapshots.insert(graph, Arc::new(snapshot));
    }
}

impl Default for InMemorySchemaCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaCatalog for InMemorySchemaCatalog {
    fn snapshot(&self, request: SchemaSnapshotRequest) -> Result<Arc<dyn SchemaSnapshot>, CatalogError> {
        self.snapshots
            .get(&request.graph.name)
            .map(|s| s.clone() as Arc<dyn SchemaSnapshot>)
            .ok_or_else(|| CatalogError::GraphNotFound { graph: request.graph.name.clone() })
    }
}

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

/// Mock graph context resolver for testing.
///
/// This implementation allows tests to control which graph and schema
/// are considered "active" without requiring a real database.
#[derive(Debug, Clone)]
pub struct MockGraphContextResolver {
    /// The default graph to return
    pub default_graph: GraphRef,
    /// The default schema to return
    pub default_schema: SchemaRef,
}

impl MockGraphContextResolver {
    /// Creates a new mock resolver with the specified defaults.
    pub fn new(graph: impl Into<SmolStr>, schema: impl Into<SmolStr>) -> Self {
        Self {
            default_graph: GraphRef { name: graph.into() },
            default_schema: SchemaRef { name: schema.into() },
        }
    }
}

impl GraphContextResolver for MockGraphContextResolver {
    fn active_graph(&self, _session: &SessionContext) -> Result<GraphRef, CatalogError> {
        Ok(self.default_graph.clone())
    }

    fn active_schema(&self, _graph: &GraphRef) -> Result<SchemaRef, CatalogError> {
        Ok(self.default_schema.clone())
    }
}

/// Mock variable type context provider for testing.
///
/// This implementation returns an empty context by default, but can be
/// configured to provide specific bindings.
#[derive(Debug, Clone, Default)]
pub struct MockVariableTypeContextProvider {
    /// Pre-configured bindings
    pub bindings: HashMap<SmolStr, ValueType>,
}

impl MockVariableTypeContextProvider {
    /// Creates a new mock provider with no initial bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pre-configured binding.
    pub fn add_binding(&mut self, variable: impl Into<SmolStr>, value_type: ValueType) {
        self.bindings.insert(variable.into(), value_type);
    }
}

impl VariableTypeContextProvider for MockVariableTypeContextProvider {
    fn initial_bindings(&self, _graph: &GraphRef, _ast: &Program) -> Result<VariableTypeContext, CatalogError> {
        Ok(VariableTypeContext {
            bindings: self.bindings.clone(),
        })
    }
}

// ============================================================================
// Fixture Loader Implementation
// ============================================================================

/// In-memory fixture loader for testing.
///
/// This implementation provides pre-defined schema fixtures that can be
/// loaded by name for regression testing.
#[derive(Debug, Clone, Default)]
pub struct InMemorySchemaFixtureLoader {
    /// Registry of fixtures by name
    fixtures: HashMap<SmolStr, Arc<InMemorySchemaSnapshot>>,
}

impl InMemorySchemaFixtureLoader {
    /// Creates a new empty fixture loader.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a fixture with the given name.
    pub fn register(&mut self, name: impl Into<SmolStr>, snapshot: InMemorySchemaSnapshot) {
        self.fixtures.insert(name.into(), Arc::new(snapshot));
    }

    /// Creates a fixture loader with standard test fixtures.
    pub fn with_standard_fixtures() -> Self {
        let mut loader = Self::new();

        // Register "social_graph" fixture
        let mut social = InMemorySchemaSnapshot::new();

        // Person node type
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
            metadata: HashMap::new(),
        });

        // KNOWS edge type
        let mut knows_props = BTreeMap::new();
        knows_props.insert("since".into(), PropertyMeta::date("since", false));

        social.add_edge_type(EdgeTypeMeta {
            name: "KNOWS".into(),
            properties: knows_props,
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        });

        loader.register("social_graph", social);

        // Register "financial" fixture
        let mut financial = InMemorySchemaSnapshot::new();

        // Account node type
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
            metadata: HashMap::new(),
        });

        // TRANSFER edge type
        let mut transfer_props = BTreeMap::new();
        transfer_props.insert("amount".into(), PropertyMeta::decimal("amount", true, 18, 2));
        transfer_props.insert("timestamp".into(), PropertyMeta::datetime("timestamp", true));

        financial.add_edge_type(EdgeTypeMeta {
            name: "TRANSFER".into(),
            properties: transfer_props,
            constraints: vec![],
            parents: vec![],
            metadata: HashMap::new(),
        });

        loader.register("financial", financial);

        loader
    }
}

impl SchemaFixtureLoader for InMemorySchemaFixtureLoader {
    fn load(&self, fixture: &str) -> Result<Arc<dyn SchemaSnapshot>, FixtureError> {
        self.fixtures
            .get(fixture)
            .map(|s| s.clone() as Arc<dyn SchemaSnapshot>)
            .ok_or_else(|| FixtureError::NotFound {
                fixture: fixture.into(),
            })
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

// ============================================================================
// Extended Fixture Examples
// ============================================================================

impl InMemorySchemaFixtureLoader {
    /// Creates a fixture loader with extended fixtures including e-commerce and healthcare.
    pub fn with_extended_fixtures() -> Self {
        let mut loader = Self::with_standard_fixtures();

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

        loader.register("ecommerce", ecommerce);

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

        loader.register("healthcare", healthcare);

        loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_catalog_creation() {
        let catalog = InMemorySchemaCatalog::new();
        assert!(catalog.snapshots.is_empty());
    }

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
    fn test_catalog_snapshot_retrieval() {
        let mut catalog = InMemorySchemaCatalog::new();
        let snapshot = InMemorySchemaSnapshot::example();
        catalog.add_snapshot("test_graph".into(), snapshot);

        let request = SchemaSnapshotRequest {
            graph: GraphRef { name: "test_graph".into() },
            schema: None,
        };

        let result = catalog.snapshot(request);
        assert!(result.is_ok());

        let snapshot = result.unwrap();
        assert!(snapshot.node_type("Person").is_some());
    }

    #[test]
    fn test_catalog_missing_graph() {
        let catalog = InMemorySchemaCatalog::new();

        let request = SchemaSnapshotRequest {
            graph: GraphRef { name: "nonexistent".into() },
            schema: None,
        };

        let result = catalog.snapshot(request);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, CatalogError::GraphNotFound { .. }));
        }
    }

    #[test]
    fn test_mock_graph_context_resolver() {
        let resolver = MockGraphContextResolver::new("my_graph", "my_schema");

        let session = SessionContext::new();

        let graph = resolver.active_graph(&session).unwrap();
        assert_eq!(graph.name, "my_graph");

        let schema = resolver.active_schema(&graph).unwrap();
        assert_eq!(schema.name, "my_schema");
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
    fn test_mock_variable_type_context_provider() {
        let mut provider = MockVariableTypeContextProvider::new();

        let string_type = ValueType::Predefined(
            crate::ast::types::PredefinedType::CharacterString(
                crate::ast::types::CharacterStringType::String
            ),
            0..0,
        );

        provider.add_binding("x", string_type);

        let graph = GraphRef { name: "test".into() };
        let ast = Program {
            statements: vec![],
            span: 0..0,
        };

        let context = provider.initial_bindings(&graph, &ast).unwrap();
        assert!(context.get("x").is_some());
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
    fn test_fixture_loader_creation() {
        let loader = InMemorySchemaFixtureLoader::new();
        assert!(loader.fixtures.is_empty());
    }

    #[test]
    fn test_fixture_loader_register_and_load() {
        let mut loader = InMemorySchemaFixtureLoader::new();
        let snapshot = InMemorySchemaSnapshot::example();
        loader.register("test", snapshot);

        let loaded = loader.load("test");
        assert!(loaded.is_ok());

        let snapshot = loaded.unwrap();
        assert!(snapshot.node_type("Person").is_some());
    }

    #[test]
    fn test_fixture_loader_missing_fixture() {
        let loader = InMemorySchemaFixtureLoader::new();
        let result = loader.load("nonexistent");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, FixtureError::NotFound { .. }));
        }
    }

    #[test]
    fn test_fixture_loader_standard_fixtures() {
        let loader = InMemorySchemaFixtureLoader::with_standard_fixtures();

        // Test social_graph fixture
        let social = loader.load("social_graph").unwrap();
        assert!(social.node_type("Person").is_some());
        assert!(social.edge_type("KNOWS").is_some());

        let person = social.node_type("Person").unwrap();
        assert!(person.properties.contains_key("name"));
        assert!(person.properties.contains_key("age"));
        assert!(person.properties.contains_key("email"));

        // Check constraints
        assert_eq!(person.constraints.len(), 1);
        assert!(matches!(
            &person.constraints[0],
            ConstraintMeta::PrimaryKey { .. }
        ));

        // Test financial fixture
        let financial = loader.load("financial").unwrap();
        assert!(financial.node_type("Account").is_some());
        assert!(financial.edge_type("TRANSFER").is_some());

        let account = financial.node_type("Account").unwrap();
        assert!(account.properties.contains_key("account_id"));
        assert!(account.properties.contains_key("balance"));

        let transfer = financial.edge_type("TRANSFER").unwrap();
        assert!(transfer.properties.contains_key("amount"));
        assert!(transfer.properties.contains_key("timestamp"));
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
}
