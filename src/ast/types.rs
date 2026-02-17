//! Type system AST nodes for GQL.
//!
//! This module defines the complete type system that forms the semantic backbone of GQL.
//! The type system provides type annotations, type constructors, and type constraints used
//! throughout the language in variable declarations, CAST expressions, IS TYPED predicates,
//! graph schema definitions, and more.
//!
//! # Type Hierarchy
//!
//! ```text
//! ValueType
//! ├── Predefined
//! │   ├── Boolean (BOOL, BOOLEAN)
//! │   ├── CharacterString (STRING, CHAR, VARCHAR)
//! │   ├── ByteString (BYTES, BINARY, VARBINARY)
//! │   ├── Numeric (INT, DECIMAL, FLOAT, etc.)
//! │   ├── Temporal (DATE, TIME, TIMESTAMP, DURATION)
//! │   ├── ReferenceValue (GRAPH, NODE, EDGE, BINDING TABLE)
//! │   └── Immaterial (NULL, NOTHING)
//! ├── Path (PATH)
//! ├── List (LIST<T>, ARRAY<T>, T LIST, T ARRAY)
//! └── Record (RECORD, ANY RECORD)
//! ```

use crate::ast::Span;
use smol_str::SmolStr;

// ============================================================================
// Value Type - Top-level type union
// ============================================================================

/// Represents any value type in GQL.
///
/// This is the main entry point for all type forms, from simple predefined types
/// to complex nested constructed types.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    /// Predefined type (boolean, string, numeric, temporal, reference, immaterial)
    Predefined(PredefinedType, Span),

    /// Path value type (PATH)
    Path(PathValueType),

    /// List value type (LIST<T>, ARRAY<T>, T LIST, T ARRAY)
    List(ListValueType),

    /// Record type (ANY RECORD, RECORD with fields)
    Record(RecordType),
}

impl ValueType {
    /// Returns the span of this type
    pub fn span(&self) -> Span {
        match self {
            ValueType::Predefined(_, span) => span.clone(),
            ValueType::Path(pt) => pt.span.clone(),
            ValueType::List(lt) => lt.span.clone(),
            ValueType::Record(rt) => rt.span(),
        }
    }
}

// ============================================================================
// Predefined Types
// ============================================================================

/// Predefined type categories in GQL.
#[derive(Debug, Clone, PartialEq)]
pub enum PredefinedType {
    /// Boolean type (BOOL, BOOLEAN)
    Boolean(BooleanType),

    /// Character string type (STRING, CHAR, VARCHAR)
    CharacterString(CharacterStringType),

    /// Byte string type (BYTES, BINARY, VARBINARY)
    ByteString(ByteStringType),

    /// Numeric type (exact or approximate)
    Numeric(NumericType),

    /// Temporal type (instant or duration)
    Temporal(TemporalType),

    /// Reference value type (graph, node, edge, binding table)
    ReferenceValue(ReferenceValueType),

    /// Immaterial value type (NULL, NOTHING)
    Immaterial(ImmaterialValueType),
}

// ============================================================================
// Boolean Types
// ============================================================================

/// Boolean type variants.
///
/// Examples: `BOOL`, `BOOLEAN`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BooleanType {
    /// BOOL keyword
    Bool,
    /// BOOLEAN keyword
    Boolean,
}

// ============================================================================
// Character String Types
// ============================================================================

/// Character string type variants.
///
/// Examples: `STRING`, `CHAR(10)`, `VARCHAR(255)`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CharacterStringType {
    /// STRING - variable-length character string
    String,

    /// CHAR(n) - fixed-length character string with optional length
    /// CHAR without length is allowed
    Char(Option<u32>),

    /// VARCHAR(n) - variable-length character string with optional max length
    /// VARCHAR without length is allowed
    VarChar(Option<u32>),
}

// ============================================================================
// Byte String Types
// ============================================================================

/// Byte string type variants.
///
/// Examples: `BYTES`, `BINARY(16)`, `VARBINARY(1024)`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ByteStringType {
    /// BYTES - variable-length byte string
    Bytes,

    /// BINARY(n) - fixed-length byte string with optional length
    /// BINARY without length is allowed
    Binary(Option<u32>),

    /// VARBINARY(n) - variable-length byte string with optional max length
    /// VARBINARY without length is allowed
    VarBinary(Option<u32>),
}

// ============================================================================
// Numeric Types
// ============================================================================

/// Numeric type categories.
#[derive(Debug, Clone, PartialEq)]
pub enum NumericType {
    /// Exact numeric type (binary or decimal)
    Exact(ExactNumericType),

    /// Approximate numeric type (floating-point)
    Approximate(ApproximateNumericType),
}

/// Exact numeric type variants.
#[derive(Debug, Clone, PartialEq)]
pub enum ExactNumericType {
    /// Signed binary exact numeric (INT8, INT16, INT32, INT64, INT128, INT256, SMALLINT, INT, INTEGER, BIGINT)
    SignedBinary(SignedBinaryExactNumericType),

    /// Unsigned binary exact numeric (UINT8, UINT16, UINT32, UINT64, UINT128, UINT256, USMALLINT, UINT, UBIGINT)
    UnsignedBinary(UnsignedBinaryExactNumericType),

    /// Decimal exact numeric (DECIMAL, DEC with precision/scale)
    Decimal(DecimalExactNumericType),
}

/// Signed binary exact numeric type variants.
///
/// Examples: `INT8`, `INT32`, `BIGINT`, `INTEGER`, `SIGNED INT16`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignedBinaryExactNumericType {
    /// INT8 or SIGNED INT8
    Int8,
    /// INT16 or SIGNED INT16
    Int16,
    /// INT32 or SIGNED INT32
    Int32,
    /// INT64 or SIGNED INT64
    Int64,
    /// INT128 or SIGNED INT128
    Int128,
    /// INT256 or SIGNED INT256
    Int256,
    /// SMALLINT or SIGNED SMALLINT
    SmallInt,
    /// INT or SIGNED INT (synonym for INT32)
    Int,
    /// INTEGER or SIGNED INTEGER (synonym for INT)
    Integer,
    /// BIGINT or SIGNED BIGINT (synonym for INT64)
    BigInt,
}

/// Unsigned binary exact numeric type variants.
///
/// Examples: `UINT8`, `UINT32`, `UBIGINT`, `UNSIGNED INT16`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnsignedBinaryExactNumericType {
    /// UINT8 or UNSIGNED INT8
    UInt8,
    /// UINT16 or UNSIGNED INT16
    UInt16,
    /// UINT32 or UNSIGNED INT32
    UInt32,
    /// UINT64 or UNSIGNED INT64
    UInt64,
    /// UINT128 or UNSIGNED INT128
    UInt128,
    /// UINT256 or UNSIGNED INT256
    UInt256,
    /// USMALLINT or UNSIGNED SMALLINT
    USmallInt,
    /// UINT or UNSIGNED INT (synonym for UINT32)
    UInt,
    /// UBIGINT or UNSIGNED BIGINT (synonym for UINT64)
    UBigInt,
}

/// Decimal exact numeric type with optional precision and scale.
///
/// Examples: `DECIMAL`, `DECIMAL(10)`, `DECIMAL(10, 2)`, `DEC(8, 4)`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecimalExactNumericType {
    /// DECIMAL or DEC keyword
    pub kind: DecimalKind,
    /// Precision (total number of digits)
    pub precision: Option<u32>,
    /// Scale (number of digits after decimal point)
    pub scale: Option<u32>,
    /// Source span
    pub span: Span,
}

/// Decimal type keyword variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DecimalKind {
    /// DECIMAL keyword
    Decimal,
    /// DEC keyword (synonym for DECIMAL)
    Dec,
}

/// Approximate numeric type variants.
///
/// Examples: `FLOAT16`, `FLOAT32`, `FLOAT(53)`, `REAL`, `DOUBLE PRECISION`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApproximateNumericType {
    /// FLOAT16 - 16-bit floating point
    Float16,
    /// FLOAT32 - 32-bit floating point
    Float32,
    /// FLOAT64 - 64-bit floating point
    Float64,
    /// FLOAT128 - 128-bit floating point
    Float128,
    /// FLOAT256 - 256-bit floating point
    Float256,
    /// FLOAT(p) - floating point with optional precision
    Float(Option<u32>),
    /// REAL - single precision floating point (typically 32-bit)
    Real,
    /// DOUBLE PRECISION - double precision floating point (typically 64-bit)
    DoublePrecision,
}

// ============================================================================
// Temporal Types
// ============================================================================

/// Temporal type categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemporalType {
    /// Temporal instant type (datetime, date, time)
    Instant(TemporalInstantType),

    /// Temporal duration type
    Duration(TemporalDurationType),
}

/// Temporal instant type variants.
///
/// These represent points in time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemporalInstantType {
    /// ZONED DATETIME or TIMESTAMP WITH TIME ZONE
    ZonedDatetime,

    /// LOCAL DATETIME, TIMESTAMP, or TIMESTAMP WITHOUT TIME ZONE
    LocalDatetime,

    /// DATE
    Date,

    /// ZONED TIME or TIME WITH TIME ZONE
    ZonedTime,

    /// LOCAL TIME, TIME, or TIME WITHOUT TIME ZONE
    LocalTime,
}

/// Temporal duration type variants.
///
/// These represent intervals of time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemporalDurationType {
    /// DURATION - general duration
    Duration,

    /// DURATION YEAR TO MONTH - year-month interval
    DurationYearToMonth,

    /// DURATION DAY TO SECOND - day-time interval
    DurationDayToSecond,
}

// ============================================================================
// Immaterial Value Types
// ============================================================================

/// Immaterial value type variants.
///
/// These represent special types for null and empty values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImmaterialValueType {
    /// NULL - the null type
    Null,

    /// NULL NOT NULL - a paradoxical type (exists in grammar)
    NullNotNull,

    /// NOTHING - the empty type
    Nothing,
}

// ============================================================================
// Reference Value Types
// ============================================================================

/// Reference value type categories.
///
/// These types reference graphs, nodes, edges, and binding tables.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceValueType {
    /// Graph reference type
    Graph(GraphReferenceValueType),

    /// Binding table reference type
    BindingTable(BindingTableReferenceValueType),

    /// Node reference type
    Node(NodeReferenceValueType),

    /// Edge reference type
    Edge(EdgeReferenceValueType),
}

/// Graph reference value type variants.
///
/// Examples: `ANY PROPERTY GRAPH`, `PROPERTY GRAPH <spec>`, `ANY GRAPH NOT NULL`
#[derive(Debug, Clone, PartialEq)]
pub enum GraphReferenceValueType {
    /// ANY [PROPERTY] GRAPH [NOT NULL]
    AnyPropertyGraph {
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },

    /// PROPERTY GRAPH <nested_spec> [NOT NULL]
    PropertyGraph {
        /// Nested graph type specification (placeholder for Sprint 12)
        spec: Box<NestedGraphTypeSpecification>,
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },
}

/// Binding table reference value type.
///
/// Examples: `BINDING TABLE`, `BINDING TABLE { field1 :: INT, field2 :: STRING }`
#[derive(Debug, Clone, PartialEq)]
pub struct BindingTableReferenceValueType {
    /// Field type specifications (optional)
    pub field_types: Option<FieldTypesSpecification>,
    /// Whether NOT NULL modifier is present
    pub not_null: bool,
    /// Source span
    pub span: Span,
}

/// Node reference value type variants.
///
/// Examples: `NODE`, `VERTEX`, `ANY NODE`, `NODE NOT NULL`
#[derive(Debug, Clone, PartialEq)]
pub enum NodeReferenceValueType {
    /// [ANY] NODE [NOT NULL] or [ANY] VERTEX [NOT NULL]
    Any {
        /// Whether to use VERTEX keyword instead of NODE
        use_vertex: bool,
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },

    /// <node_type_spec> [NOT NULL]
    Typed {
        /// Node type specification (placeholder for Sprint 12)
        spec: Box<NodeTypeSpecification>,
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },
}

/// Edge reference value type variants.
///
/// Examples: `EDGE`, `RELATIONSHIP`, `ANY EDGE`, `EDGE NOT NULL`
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeReferenceValueType {
    /// [ANY] EDGE [NOT NULL] or [ANY] RELATIONSHIP [NOT NULL]
    Any {
        /// Whether to use RELATIONSHIP keyword instead of EDGE
        use_relationship: bool,
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },

    /// <edge_type_spec> [NOT NULL]
    Typed {
        /// Edge type specification (placeholder for Sprint 12)
        spec: Box<EdgeTypeSpecification>,
        /// Whether NOT NULL modifier is present
        not_null: bool,
        /// Source span
        span: Span,
    },
}

// ============================================================================
// Placeholder types for Sprint 12 (Graph Type Specifications)
// ============================================================================

/// Placeholder for nested graph type specification.
///
/// This will be fully implemented in Sprint 12 when graph type specifications are added.
/// For now, this serves as a marker in the AST structure.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedGraphTypeSpecification {
    /// Placeholder span
    pub span: Span,
    // TODO(Sprint 12): Add full graph type specification fields
}

/// Placeholder for node type specification.
///
/// This will be fully implemented in Sprint 12 when graph type specifications are added.
/// For now, this serves as a marker in the AST structure.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeTypeSpecification {
    /// Placeholder span
    pub span: Span,
    // TODO(Sprint 12): Add full node type specification fields
}

/// Placeholder for edge type specification.
///
/// This will be fully implemented in Sprint 12 when graph type specifications are added.
/// For now, this serves as a marker in the AST structure.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeTypeSpecification {
    /// Placeholder span
    pub span: Span,
    // TODO(Sprint 12): Add full edge type specification fields
}

// ============================================================================
// Constructed Types - Path
// ============================================================================

/// Path value type.
///
/// Example: `PATH`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathValueType {
    /// Source span
    pub span: Span,
}

// ============================================================================
// Constructed Types - List
// ============================================================================

/// List value type.
///
/// Examples: `LIST<INT>`, `ARRAY<STRING>`, `INT LIST`, `STRING ARRAY`
#[derive(Debug, Clone, PartialEq)]
pub struct ListValueType {
    /// Element type
    pub element_type: Box<ValueType>,
    /// Syntax form used
    pub syntax_form: ListSyntaxForm,
    /// Source span
    pub span: Span,
}

/// List syntax form variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListSyntaxForm {
    /// LIST<T> - prefix form with LIST keyword
    List,
    /// ARRAY<T> - prefix form with ARRAY keyword
    Array,
    /// T LIST - postfix form with LIST keyword
    PostfixList,
    /// T ARRAY - postfix form with ARRAY keyword
    PostfixArray,
}

// ============================================================================
// Constructed Types - Record
// ============================================================================

/// Record type variants.
///
/// Examples: `ANY RECORD`, `RECORD { field1 :: INT, field2 :: STRING }`
#[derive(Debug, Clone, PartialEq)]
pub enum RecordType {
    /// ANY RECORD - untyped record
    AnyRecord {
        /// Source span
        span: Span,
    },

    /// RECORD with field type specifications
    Record {
        /// Field type specifications
        field_types: FieldTypesSpecification,
        /// Source span
        span: Span,
    },
}

impl RecordType {
    /// Returns the span of this record type
    pub fn span(&self) -> Span {
        match self {
            RecordType::AnyRecord { span } => span.clone(),
            RecordType::Record { span, .. } => span.clone(),
        }
    }
}

/// Field type specifications for records and binding tables.
///
/// Example: `{ field1 :: INT, field2 :: STRING, field3 :: BOOL }`
#[derive(Debug, Clone, PartialEq)]
pub struct FieldTypesSpecification {
    /// List of field type specifications
    pub fields: Vec<FieldType>,
    /// Source span
    pub span: Span,
}

/// Individual field type specification.
///
/// Example: `field_name :: INT`
#[derive(Debug, Clone, PartialEq)]
pub struct FieldType {
    /// Field name
    pub field_name: SmolStr,
    /// Field type
    pub field_type: Box<ValueType>,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Type Modifiers
// ============================================================================

/// Type annotation using :: or TYPED keyword.
///
/// Examples: `expr :: INT`, `expr TYPED STRING`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    /// The operator used (:: or TYPED)
    pub operator: TypeAnnotationOperator,
    /// The type being annotated
    pub type_ref: Box<ValueType>,
    /// Source span
    pub span: Span,
}

/// Type annotation operator variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeAnnotationOperator {
    /// :: operator
    DoubleColon,
    /// TYPED keyword
    Typed,
}

/// NOT NULL constraint modifier.
///
/// Example: `INT NOT NULL`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotNullConstraint {
    /// Source span
    pub span: Span,
}
