//! Graph Type Specification AST nodes for GQL.
//!
//! This module defines the complete graph type specification system for schema definition in GQL.
//! Graph type specifications enable comprehensive schema modeling for property graphs, including
//! node types, edge types, property types, label sets, and connectivity constraints.
//!
//! # Graph Type Specification Hierarchy
//!
//! ```text
//! NestedGraphTypeSpecification
//! └── body: GraphTypeSpecificationBody
//!     └── element_types: ElementTypeList
//!         └── types: Vec<ElementTypeSpecification>
//!             ├── Node(NodeTypeSpecification)
//!             │   └── pattern: NodeTypePattern
//!             │       └── phrase: NodeTypePhrase
//!             │           ├── filler: Option<NodeTypeFiller>
//!             │           │   ├── label_set: Option<NodeTypeLabelSet>
//!             │           │   ├── property_types: Option<NodeTypePropertyTypes>
//!             │           │   ├── key_label_set: Option<NodeTypeKeyLabelSet>
//!             │           │   └── implied_content: Option<NodeTypeImpliedContent>
//!             │           └── alias: Option<LocalNodeTypeAlias>
//!             └── Edge(EdgeTypeSpecification)
//!                 └── pattern: EdgeTypePattern
//!                     ├── Directed(EdgeTypePatternDirected)
//!                     │   ├── left_endpoint: NodeTypePattern
//!                     │   ├── arc: DirectedArcType
//!                     │   └── right_endpoint: NodeTypePattern
//!                     └── Undirected(EdgeTypePatternUndirected)
//!                         ├── left_endpoint: NodeTypePattern
//!                         ├── arc: ArcTypeUndirected
//!                         └── right_endpoint: NodeTypePattern
//! ```
//!
//! # Grammar References
//!
//! This module implements the following GQL grammar rules:
//! - `nestedGraphTypeSpecification` (Line 1482)
//! - `graphTypeSpecificationBody` (Line 1486)
//! - `elementTypeList` (Line 1490)
//! - `elementTypeSpecification` (Line 1494)
//! - `nodeTypeSpecification` (Line 1501)
//! - `edgeTypeSpecification` (Line 1548)
//! - `propertyTypesSpecification` (Line 1691)
//! - `labelSetPhrase` (Line 1679)

use crate::ast::{Span, ValueType};
use smol_str::SmolStr;

// ============================================================================
// Nested Graph Type Specification (Top-level)
// ============================================================================

/// Nested graph type specification enclosed in braces.
///
/// Defines the structure of a property graph including node types, edge types,
/// properties, labels, and connectivity constraints.
///
/// Syntax: `{ element_type_list }`
///
/// Example:
/// ```gql
/// {
///   NODE TYPE Person LABEL Person { name :: STRING, age :: INT },
///   DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NestedGraphTypeSpecification {
    /// The graph type specification body
    pub body: GraphTypeSpecificationBody,
    /// Source span
    pub span: Span,
}

/// Graph type specification body containing element type definitions.
///
/// This represents the content within braces of a graph type specification.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphTypeSpecificationBody {
    /// List of element type definitions (nodes and edges)
    pub element_types: ElementTypeList,
    /// Source span
    pub span: Span,
}

/// List of element type specifications (comma-separated).
///
/// Element types can be node types or edge types.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementTypeList {
    /// Vector of element type specifications
    pub types: Vec<ElementTypeSpecification>,
    /// Source span
    pub span: Span,
}

/// An element type specification (node or edge).
///
/// This enum distinguishes between node type definitions and edge type definitions
/// within a graph type specification.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementTypeSpecification {
    /// Node type definition
    Node(Box<NodeTypeSpecification>),
    /// Edge type definition
    Edge(Box<EdgeTypeSpecification>),
}

/// Type inheritance clause for node/edge type specifications.
///
/// Syntax examples:
/// - `INHERITS Person`
/// - `EXTENDS Person, NamedEntity`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInheritanceClause {
    /// Parent types declared by the inheritance clause.
    pub parents: Vec<InheritedTypeReference>,
    /// Source span.
    pub span: Span,
}

/// Parent type reference used in an inheritance clause.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InheritedTypeReference {
    /// Parent type name.
    pub name: SmolStr,
    /// Source span.
    pub span: Span,
}

/// Graph-type constraint clause.
///
/// Constraints are captured in parsed form and preserve any raw argument payload
/// for downstream validation/normalization.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphTypeConstraint {
    /// `... CONSTRAINT KEY (...)`
    Key {
        arguments: Vec<GraphTypeConstraintArgument>,
        span: Span,
    },
    /// `... CONSTRAINT UNIQUE (...)`
    Unique {
        arguments: Vec<GraphTypeConstraintArgument>,
        span: Span,
    },
    /// `... CONSTRAINT MANDATORY (...)`
    Mandatory {
        arguments: Vec<GraphTypeConstraintArgument>,
        span: Span,
    },
    /// `... CONSTRAINT CHECK (...)`
    Check {
        arguments: Vec<GraphTypeConstraintArgument>,
        span: Span,
    },
    /// `... CONSTRAINT <custom>(...)`
    Custom {
        name: SmolStr,
        arguments: Vec<GraphTypeConstraintArgument>,
        span: Span,
    },
}

impl GraphTypeConstraint {
    /// Returns the source span of this constraint clause.
    pub fn span(&self) -> Span {
        match self {
            GraphTypeConstraint::Key { span, .. }
            | GraphTypeConstraint::Unique { span, .. }
            | GraphTypeConstraint::Mandatory { span, .. }
            | GraphTypeConstraint::Check { span, .. }
            | GraphTypeConstraint::Custom { span, .. } => span.clone(),
        }
    }
}

/// Raw argument payload for a graph-type constraint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphTypeConstraintArgument {
    /// Normalized token text for the argument segment.
    pub raw: SmolStr,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Node Type Specifications
// ============================================================================

/// Node type specification defining the structure of a node in the graph.
///
/// Includes labels, properties, keys, and optional local aliases.
///
/// Syntax: `node_type_pattern`
///
/// Example:
/// ```gql
/// NODE TYPE Person LABEL Person { name :: STRING, age :: INT } KEY name
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeSpecification {
    /// Whether `ABSTRACT` modifier is present.
    pub is_abstract: bool,
    /// Optional inheritance clause.
    pub inheritance: Option<TypeInheritanceClause>,
    /// The node type pattern
    pub pattern: NodeTypePattern,
    /// Source span
    pub span: Span,
}

/// Node type pattern containing the node type phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypePattern {
    /// The node type phrase
    pub phrase: NodeTypePhrase,
    /// Source span
    pub span: Span,
}

/// Node type phrase with optional filler content and local alias.
///
/// Syntax: `[NODE [TYPE]] [node_type_filler] [AS alias]`
///
/// Example:
/// ```gql
/// NODE TYPE Person LABEL Person { name :: STRING } AS p
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypePhrase {
    /// Optional node type filler (labels, properties, keys, implied content)
    pub filler: Option<NodeTypeFiller>,
    /// Optional local type alias (AS clause)
    pub alias: Option<LocalNodeTypeAlias>,
    /// Source span
    pub span: Span,
}

/// Local node type alias (AS clause).
///
/// Example: `AS PersonType`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalNodeTypeAlias {
    /// The alias name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Node Type Filler
// ============================================================================

/// Node type filler containing labels, properties, keys, and implied content.
///
/// All components are optional, allowing flexible node type definitions.
///
/// Example:
/// ```gql
/// LABEL Person { name :: STRING, age :: INT } KEY name
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeFiller {
    /// Optional label set specification
    pub label_set: Option<NodeTypeLabelSet>,
    /// Optional property types specification
    pub property_types: Option<NodeTypePropertyTypes>,
    /// Optional key label set for key constraints
    pub key_label_set: Option<NodeTypeKeyLabelSet>,
    /// Optional implied content for defaults
    pub implied_content: Option<NodeTypeImpliedContent>,
    /// Explicit graph-type constraints.
    pub constraints: Vec<GraphTypeConstraint>,
    /// Source span
    pub span: Span,
}

/// Node type label set specification.
///
/// Defines which labels a node type has.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeLabelSet {
    /// The label set phrase
    pub label_set_phrase: LabelSetPhrase,
    /// Source span
    pub span: Span,
}

/// Node type property types specification.
///
/// Defines the properties of a node type.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypePropertyTypes {
    /// The property types specification
    pub specification: PropertyTypesSpecification,
    /// Source span
    pub span: Span,
}

/// Node type key label set for key constraints.
///
/// Defines which labels form the key for this node type.
///
/// Syntax: `KEY label_set_specification`
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeKeyLabelSet {
    /// The label set specification
    pub label_set: LabelSetSpecification,
    /// Source span
    pub span: Span,
}

/// Node type implied content for default values.
///
/// Specifies default content for a node type.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeImpliedContent {
    /// The implied node type content
    pub content: Box<NodeTypeFiller>,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Edge Type Specifications
// ============================================================================

/// Edge type specification defining the structure of an edge in the graph.
///
/// Includes edge direction, labels, properties, and endpoint connectivity.
///
/// Syntax: `edge_type_pattern`
///
/// Examples:
/// ```gql
/// DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
/// (Person)-[KNOWS]->(Person)
/// (Person)~[SIMILAR_TO]~(Person)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypeSpecification {
    /// Whether `ABSTRACT` modifier is present.
    pub is_abstract: bool,
    /// Optional inheritance clause.
    pub inheritance: Option<TypeInheritanceClause>,
    /// The edge type pattern
    pub pattern: EdgeTypePattern,
    /// Source span
    pub span: Span,
}

/// Edge type pattern (directed or undirected).
///
/// Distinguishes between directed edges (with arrows) and undirected edges (with tildes).
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeTypePattern {
    /// Directed edge type pattern: `-[edge]->` or `<-[edge]-`
    Directed(EdgeTypePatternDirected),
    /// Undirected edge type pattern: `~[edge]~`
    Undirected(EdgeTypePatternUndirected),
}

/// Directed edge type pattern.
///
/// Syntax: `node_type_pattern arc_type node_type_pattern`
///
/// Example:
/// ```gql
/// (Person)-[KNOWS { since :: DATE }]->(Person)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypePatternDirected {
    /// Left endpoint node type
    pub left_endpoint: NodeTypePattern,
    /// Directed arc type (pointing right or left)
    pub arc: DirectedArcType,
    /// Right endpoint node type
    pub right_endpoint: NodeTypePattern,
    /// Source span
    pub span: Span,
}

/// Undirected edge type pattern.
///
/// Syntax: `node_type_pattern ~[edge_type_filler]~ node_type_pattern`
///
/// Example:
/// ```gql
/// (Person)~[SIMILAR_TO]~(Person)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypePatternUndirected {
    /// Left endpoint node type
    pub left_endpoint: NodeTypePattern,
    /// Undirected arc type
    pub arc: ArcTypeUndirected,
    /// Right endpoint node type
    pub right_endpoint: NodeTypePattern,
    /// Source span
    pub span: Span,
}

/// Directed arc type (pointing right or left).
#[derive(Debug, Clone, PartialEq)]
pub enum DirectedArcType {
    /// Arc pointing right: `-[edge]->`
    PointingRight(ArcTypePointingRight),
    /// Arc pointing left: `<-[edge]-`
    PointingLeft(ArcTypePointingLeft),
}

/// Arc type pointing right (source to destination).
///
/// Syntax: `-[edge_type_filler?]->`
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTypePointingRight {
    /// Optional edge type filler
    pub filler: Option<EdgeTypeFiller>,
    /// Source span
    pub span: Span,
}

/// Arc type pointing left (destination to source).
///
/// Syntax: `<-[edge_type_filler?]-`
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTypePointingLeft {
    /// Optional edge type filler
    pub filler: Option<EdgeTypeFiller>,
    /// Source span
    pub span: Span,
}

/// Undirected arc type.
///
/// Syntax: `~[edge_type_filler?]~`
#[derive(Debug, Clone, PartialEq)]
pub struct ArcTypeUndirected {
    /// Optional edge type filler
    pub filler: Option<EdgeTypeFiller>,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Edge Type Filler and Phrases
// ============================================================================

/// Edge type filler containing the edge type phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypeFiller {
    /// The edge type phrase
    pub phrase: EdgeTypePhrase,
    /// Explicit graph-type constraints.
    pub constraints: Vec<GraphTypeConstraint>,
    /// Source span
    pub span: Span,
}

/// Edge type phrase with edge kind, labels, properties, and endpoints.
///
/// Syntax: `[edge_kind] EDGE [TYPE] [labels/properties] CONNECTING endpoint_pair`
///
/// Example:
/// ```gql
/// DIRECTED EDGE TYPE KNOWS LABEL Knows { since :: DATE } CONNECTING (Person TO Person)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypePhrase {
    /// Edge kind (directed, undirected, or inferred)
    pub edge_kind: EdgeKind,
    /// Optional filler content (labels and properties)
    pub filler_content: Option<EdgeTypePhraseContent>,
    /// Endpoint pair phrase specifying connectivity
    pub endpoint_pair_phrase: EndpointPairPhrase,
    /// Source span
    pub span: Span,
}

/// Edge type phrase content (labels and properties).
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypePhraseContent {
    /// Optional label set
    pub label_set: Option<EdgeTypeLabelSet>,
    /// Optional property types
    pub property_types: Option<EdgeTypePropertyTypes>,
    /// Source span
    pub span: Span,
}

/// Edge type label set specification.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypeLabelSet {
    /// The label set phrase
    pub label_set_phrase: LabelSetPhrase,
    /// Source span
    pub span: Span,
}

/// Edge type property types specification.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypePropertyTypes {
    /// The property types specification
    pub specification: PropertyTypesSpecification,
    /// Source span
    pub span: Span,
}

/// Edge kind (directed or undirected).
///
/// Specifies whether an edge is directional or bidirectional.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// DIRECTED EDGE - edge has direction from source to destination
    Directed,
    /// UNDIRECTED EDGE - edge has no specific direction
    Undirected,
    /// Edge kind inferred from pattern syntax (arrow vs tilde)
    Inferred,
}

// ============================================================================
// Endpoint Pairs
// ============================================================================

/// Endpoint pair phrase specifying connectivity constraints.
///
/// Syntax: `CONNECTING (endpoint_pair)`
///
/// Example:
/// ```gql
/// CONNECTING (Person TO Company)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointPairPhrase {
    /// The endpoint pair
    pub endpoint_pair: EndpointPair,
    /// Source span
    pub span: Span,
}

/// Endpoint pair specifying source and destination node types.
///
/// Syntax: `source_node_type TO destination_node_type`
///
/// Example:
/// ```gql
/// Person TO Company
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointPair {
    /// Source node type
    pub source: NodeTypeReference,
    /// Destination node type
    pub destination: NodeTypeReference,
    /// Source span
    pub span: Span,
}

/// Node type reference within an endpoint pair.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeReference {
    /// The referenced node type pattern
    pub node_type: NodeTypePattern,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Property Types Specification
// ============================================================================

/// Property types specification enclosed in braces.
///
/// Defines the properties of a node or edge type.
///
/// Syntax: `{ property_type_list? }`
///
/// Examples:
/// ```gql
/// { }
/// { name :: STRING, age :: INT NOT NULL }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTypesSpecification {
    /// Optional property type list (empty braces allowed)
    pub property_types: Option<PropertyTypeList>,
    /// Source span
    pub span: Span,
}

/// List of property types (comma-separated).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTypeList {
    /// Vector of property types
    pub types: Vec<PropertyType>,
    /// Source span
    pub span: Span,
}

/// Individual property type definition.
///
/// Syntax: `property_name :: value_type [NOT NULL]`
///
/// Example:
/// ```gql
/// name :: STRING NOT NULL
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyType {
    /// Property name
    pub name: PropertyName,
    /// Property value type
    pub value_type: PropertyValueType,
    /// Whether NOT NULL constraint is present
    pub not_null: bool,
    /// Source span
    pub span: Span,
}

/// Property name identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropertyName {
    /// The property name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
}

/// Property value type wrapper.
///
/// References a value type from the type system.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyValueType {
    /// The value type
    pub value_type: ValueType,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Label Set Phrases and Specifications
// ============================================================================

/// Label set phrase with multiple syntax variants.
///
/// Syntax variants:
/// - `LABEL label_name` - single label
/// - `LABELS label_set_specification` - multiple labels
/// - `IS label_set_specification` - IS operator
/// - `: label_set_specification` - colon operator
///
/// Examples:
/// ```gql
/// LABEL Person
/// LABELS Person & Employee
/// IS Person & Employee
/// : Person & Employee
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum LabelSetPhrase {
    /// LABEL <label_name> - single label
    Label(LabelName),
    /// LABELS <label_set_specification> - multiple labels
    Labels(LabelSetSpecification),
    /// IS <label_set_specification> or : <label_set_specification>
    IsLabelSet(LabelSetSpecification),
}

/// Label set specification with ampersand-separated labels.
///
/// Syntax: `label1 & label2 & label3 & ...`
///
/// Example:
/// ```gql
/// Person & Employee & Manager
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LabelSetSpecification {
    /// Vector of label names (ampersand-separated)
    pub labels: Vec<LabelName>,
    /// Source span
    pub span: Span,
}

/// Label name identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabelName {
    /// The label name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
}
