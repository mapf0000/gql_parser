//! Token types and representations for GQL lexical analysis.

use crate::ast::Span;
use smol_str::SmolStr;
use std::fmt;

/// The kind of a lexical token in GQL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Keywords - Reserved
    Match,
    Where,
    Return,
    Create,
    Delete,
    Detach,
    Nodetach,
    Insert,
    Set,
    Remove,
    With,
    Call,
    Yield,
    Union,
    Intersect,
    Except,
    Otherwise,
    Optional,
    Use,
    At,
    Next,
    Finish,
    Let,
    For,
    Filter,
    Order,
    By,
    Asc,
    Ascending,
    Desc,
    Descending,
    Skip,
    Limit,
    Offset,
    Select,
    Distinct,
    Group,
    Having,
    As,
    From,
    When,
    Then,
    Else,
    End,
    Case,
    If,
    Cast,

    // Logical operators (also keywords)
    And,
    Or,
    Not,
    Xor,
    Is,
    In,

    // Type keywords
    Any,
    All,
    Some,
    Exists,

    // Graph keywords
    Graph,
    Node,
    Edge,
    Path,
    Relationship,
    Walk,
    Trail,
    Acyclic,
    Simple,

    // Schema/catalog keywords
    Schema,
    Catalog,
    Drop,
    Alter,
    Property,
    Label,
    Type,
    Replace,
    Of,
    Like,
    Copy,

    // Session/Transaction keywords
    Session,
    Transaction,
    Start,
    Commit,
    Rollback,
    Reset,
    Close,
    Work,
    Zone,
    Characteristics,
    Read,
    Write,
    Only,
    Modifying,
    Current,
    Home,

    // Temporal keywords
    Date,
    Time,
    Timestamp,
    Duration,

    // Boolean literals
    True,
    False,

    // Null literals
    Null,
    Unknown,

    // Type names - Boolean
    Bool,
    Boolean,

    // Type names - String
    String,
    Char,
    Varchar,

    // Type names - Bytes
    Bytes,
    Binary,
    Varbinary,

    // Type names - Numeric (Signed)
    Int,
    Integer,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    Int256,
    Smallint,
    Bigint,
    Signed,

    // Type names - Numeric (Unsigned)
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,
    Uint256,
    Usmallint,
    Ubigint,
    Unsigned,

    // Type names - Numeric (Decimal/Float)
    Decimal,
    Dec,
    Float,
    Float16,
    Float32,
    Float64,
    Float128,
    Float256,
    Real,
    Double,
    Precision,

    // Type names - Temporal
    Zoned,
    Local,
    Datetime,
    Without,
    Year,
    Month,
    Day,
    Second,
    To,

    // Type names - Other
    Nothing,
    List,
    Array,
    Record,
    Vertex,

    // Additional expression and function keywords
    Value,
    Table,
    Binding,
    Variable,

    // Null ordering keywords
    Nulls,
    First,
    Last,

    // For statement keywords
    Ordinality,

    // Predicate keywords
    Typed,
    Normalized,
    Directed,
    Labeled,
    Source,
    Destination,

    // Built-in function keywords (common ones as tokens)
    // Numeric
    Abs,
    Mod,
    Floor,
    Ceil,
    Sqrt,
    Power,
    Exp,
    Ln,
    Log,

    // Trigonometric
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,

    // String functions
    Upper,
    Lower,
    Trim,
    Substring,
    Normalize,

    // Conditional
    Coalesce,
    Nullif,

    // Cardinality
    Cardinality,
    Size,

    // Graph
    Elements,
    Element,

    // Predicates
    AllDifferent,
    Same,
    PropertyExists,

    // Identifiers
    Identifier(SmolStr),
    DelimitedIdentifier(SmolStr),

    // Literals
    StringLiteral(SmolStr),
    ByteStringLiteral(SmolStr), // X'...'
    IntegerLiteral(SmolStr),
    FloatLiteral(SmolStr),

    // Parameters
    Parameter(SmolStr),          // $name or $1
    ReferenceParameter(SmolStr), // $$name (for catalog references)

    // Operators
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Caret,       // ^
    Eq,          // =
    NotEq,       // <>
    NotEqBang,   // !=
    Lt,          // <
    Gt,          // >
    LtEq,        // <=
    GtEq,        // >=
    Arrow,       // ->
    LeftArrow,   // <-
    Tilde,       // ~
    LeftTilde,   // <~
    RightTilde,  // ~>
    Pipe,        // |
    DoublePipe,  // ||
    Ampersand,   // &
    DoubleColon, // ::
    DotDot,      // ..

    // Punctuation
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Semicolon, // ;
    Dot,       // .
    Colon,     // :

    // Special
    Eof,
}

impl TokenKind {
    /// Returns true if this token kind is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Match
                | TokenKind::Where
                | TokenKind::Return
                | TokenKind::Create
                | TokenKind::Delete
                | TokenKind::Detach
                | TokenKind::Nodetach
                | TokenKind::Insert
                | TokenKind::Set
                | TokenKind::Remove
                | TokenKind::With
                | TokenKind::Call
                | TokenKind::Yield
                | TokenKind::Union
                | TokenKind::Intersect
                | TokenKind::Except
                | TokenKind::Otherwise
                | TokenKind::Optional
                | TokenKind::Use
                | TokenKind::At
                | TokenKind::Next
                | TokenKind::Finish
                | TokenKind::Let
                | TokenKind::For
                | TokenKind::Filter
                | TokenKind::Order
                | TokenKind::By
                | TokenKind::Asc
                | TokenKind::Ascending
                | TokenKind::Desc
                | TokenKind::Descending
                | TokenKind::Skip
                | TokenKind::Limit
                | TokenKind::Offset
                | TokenKind::Select
                | TokenKind::Distinct
                | TokenKind::Group
                | TokenKind::Having
                | TokenKind::As
                | TokenKind::From
                | TokenKind::When
                | TokenKind::Then
                | TokenKind::Else
                | TokenKind::End
                | TokenKind::Case
                | TokenKind::If
                | TokenKind::Cast
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::Xor
                | TokenKind::Is
                | TokenKind::In
                | TokenKind::Any
                | TokenKind::All
                | TokenKind::Some
                | TokenKind::Exists
                | TokenKind::Graph
                | TokenKind::Node
                | TokenKind::Edge
                | TokenKind::Path
                | TokenKind::Relationship
                | TokenKind::Walk
                | TokenKind::Trail
                | TokenKind::Acyclic
                | TokenKind::Simple
                | TokenKind::Schema
                | TokenKind::Catalog
                | TokenKind::Drop
                | TokenKind::Alter
                | TokenKind::Property
                | TokenKind::Label
                | TokenKind::Type
                | TokenKind::Replace
                | TokenKind::Of
                | TokenKind::Like
                | TokenKind::Copy
                | TokenKind::Session
                | TokenKind::Transaction
                | TokenKind::Start
                | TokenKind::Commit
                | TokenKind::Rollback
                | TokenKind::Reset
                | TokenKind::Close
                | TokenKind::Work
                | TokenKind::Zone
                | TokenKind::Characteristics
                | TokenKind::Read
                | TokenKind::Write
                | TokenKind::Only
                | TokenKind::Modifying
                | TokenKind::Current
                | TokenKind::Home
                | TokenKind::Date
                | TokenKind::Time
                | TokenKind::Timestamp
                | TokenKind::Duration
                // Type keywords
                | TokenKind::Bool
                | TokenKind::Boolean
                | TokenKind::String
                | TokenKind::Char
                | TokenKind::Varchar
                | TokenKind::Bytes
                | TokenKind::Binary
                | TokenKind::Varbinary
                | TokenKind::Int
                | TokenKind::Integer
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int32
                | TokenKind::Int64
                | TokenKind::Int128
                | TokenKind::Int256
                | TokenKind::Smallint
                | TokenKind::Bigint
                | TokenKind::Signed
                | TokenKind::Uint
                | TokenKind::Uint8
                | TokenKind::Uint16
                | TokenKind::Uint32
                | TokenKind::Uint64
                | TokenKind::Uint128
                | TokenKind::Uint256
                | TokenKind::Usmallint
                | TokenKind::Ubigint
                | TokenKind::Unsigned
                | TokenKind::Decimal
                | TokenKind::Dec
                | TokenKind::Float
                | TokenKind::Float16
                | TokenKind::Float32
                | TokenKind::Float64
                | TokenKind::Float128
                | TokenKind::Float256
                | TokenKind::Real
                | TokenKind::Double
                | TokenKind::Precision
                | TokenKind::Zoned
                | TokenKind::Local
                | TokenKind::Without
                | TokenKind::Year
                | TokenKind::Month
                | TokenKind::Day
                | TokenKind::Second
                | TokenKind::To
                | TokenKind::Nothing
                | TokenKind::List
                | TokenKind::Array
                | TokenKind::Record
                | TokenKind::Vertex
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Unknown
                | TokenKind::Value
                | TokenKind::Table
                | TokenKind::Binding
                | TokenKind::Variable
                | TokenKind::Datetime
                | TokenKind::Nulls
                | TokenKind::First
                | TokenKind::Last
                | TokenKind::Ordinality
                | TokenKind::Typed
                | TokenKind::Normalized
                | TokenKind::Directed
                | TokenKind::Labeled
                | TokenKind::Source
                | TokenKind::Destination
                | TokenKind::Abs
                | TokenKind::Mod
                | TokenKind::Floor
                | TokenKind::Ceil
                | TokenKind::Sqrt
                | TokenKind::Power
                | TokenKind::Exp
                | TokenKind::Ln
                | TokenKind::Log
                | TokenKind::Sin
                | TokenKind::Cos
                | TokenKind::Tan
                | TokenKind::Asin
                | TokenKind::Acos
                | TokenKind::Atan
                | TokenKind::Upper
                | TokenKind::Lower
                | TokenKind::Trim
                | TokenKind::Substring
                | TokenKind::Normalize
                | TokenKind::Coalesce
                | TokenKind::Nullif
                | TokenKind::Cardinality
                | TokenKind::Size
                | TokenKind::Elements
                | TokenKind::Element
                | TokenKind::AllDifferent
                | TokenKind::Same
                | TokenKind::PropertyExists
        )
    }

    /// Returns true if this token kind is a literal.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::StringLiteral(_)
                | TokenKind::ByteStringLiteral(_)
                | TokenKind::IntegerLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Unknown
        )
    }

    /// Returns true if this token kind is an operator.
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Caret
                | TokenKind::Eq
                | TokenKind::NotEq
                | TokenKind::NotEqBang
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::LtEq
                | TokenKind::GtEq
                | TokenKind::Arrow
                | TokenKind::LeftArrow
                | TokenKind::Tilde
                | TokenKind::LeftTilde
                | TokenKind::RightTilde
                | TokenKind::Pipe
                | TokenKind::DoublePipe
                | TokenKind::Ampersand
                | TokenKind::DoubleColon
                | TokenKind::DotDot
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Match => write!(f, "MATCH"),
            TokenKind::Where => write!(f, "WHERE"),
            TokenKind::Return => write!(f, "RETURN"),
            TokenKind::Create => write!(f, "CREATE"),
            TokenKind::Delete => write!(f, "DELETE"),
            TokenKind::Detach => write!(f, "DETACH"),
            TokenKind::Nodetach => write!(f, "NODETACH"),
            TokenKind::Insert => write!(f, "INSERT"),
            TokenKind::Set => write!(f, "SET"),
            TokenKind::Remove => write!(f, "REMOVE"),
            TokenKind::With => write!(f, "WITH"),
            TokenKind::Call => write!(f, "CALL"),
            TokenKind::Yield => write!(f, "YIELD"),
            TokenKind::Union => write!(f, "UNION"),
            TokenKind::Intersect => write!(f, "INTERSECT"),
            TokenKind::Except => write!(f, "EXCEPT"),
            TokenKind::Otherwise => write!(f, "OTHERWISE"),
            TokenKind::Optional => write!(f, "OPTIONAL"),
            TokenKind::Use => write!(f, "USE"),
            TokenKind::At => write!(f, "AT"),
            TokenKind::Next => write!(f, "NEXT"),
            TokenKind::Finish => write!(f, "FINISH"),
            TokenKind::Let => write!(f, "LET"),
            TokenKind::For => write!(f, "FOR"),
            TokenKind::Filter => write!(f, "FILTER"),
            TokenKind::Order => write!(f, "ORDER"),
            TokenKind::By => write!(f, "BY"),
            TokenKind::Asc => write!(f, "ASC"),
            TokenKind::Ascending => write!(f, "ASCENDING"),
            TokenKind::Desc => write!(f, "DESC"),
            TokenKind::Descending => write!(f, "DESCENDING"),
            TokenKind::Skip => write!(f, "SKIP"),
            TokenKind::Limit => write!(f, "LIMIT"),
            TokenKind::Offset => write!(f, "OFFSET"),
            TokenKind::Select => write!(f, "SELECT"),
            TokenKind::Distinct => write!(f, "DISTINCT"),
            TokenKind::Group => write!(f, "GROUP"),
            TokenKind::Having => write!(f, "HAVING"),
            TokenKind::As => write!(f, "AS"),
            TokenKind::From => write!(f, "FROM"),
            TokenKind::When => write!(f, "WHEN"),
            TokenKind::Then => write!(f, "THEN"),
            TokenKind::Else => write!(f, "ELSE"),
            TokenKind::End => write!(f, "END"),
            TokenKind::Case => write!(f, "CASE"),
            TokenKind::If => write!(f, "IF"),
            TokenKind::Cast => write!(f, "CAST"),
            TokenKind::And => write!(f, "AND"),
            TokenKind::Or => write!(f, "OR"),
            TokenKind::Not => write!(f, "NOT"),
            TokenKind::Xor => write!(f, "XOR"),
            TokenKind::Is => write!(f, "IS"),
            TokenKind::In => write!(f, "IN"),
            TokenKind::Any => write!(f, "ANY"),
            TokenKind::All => write!(f, "ALL"),
            TokenKind::Some => write!(f, "SOME"),
            TokenKind::Exists => write!(f, "EXISTS"),
            TokenKind::Graph => write!(f, "GRAPH"),
            TokenKind::Node => write!(f, "NODE"),
            TokenKind::Edge => write!(f, "EDGE"),
            TokenKind::Path => write!(f, "PATH"),
            TokenKind::Relationship => write!(f, "RELATIONSHIP"),
            TokenKind::Walk => write!(f, "WALK"),
            TokenKind::Trail => write!(f, "TRAIL"),
            TokenKind::Acyclic => write!(f, "ACYCLIC"),
            TokenKind::Simple => write!(f, "SIMPLE"),
            TokenKind::Schema => write!(f, "SCHEMA"),
            TokenKind::Catalog => write!(f, "CATALOG"),
            TokenKind::Drop => write!(f, "DROP"),
            TokenKind::Alter => write!(f, "ALTER"),
            TokenKind::Property => write!(f, "PROPERTY"),
            TokenKind::Label => write!(f, "LABEL"),
            TokenKind::Type => write!(f, "TYPE"),
            TokenKind::Replace => write!(f, "REPLACE"),
            TokenKind::Of => write!(f, "OF"),
            TokenKind::Like => write!(f, "LIKE"),
            TokenKind::Copy => write!(f, "COPY"),
            TokenKind::Session => write!(f, "SESSION"),
            TokenKind::Transaction => write!(f, "TRANSACTION"),
            TokenKind::Start => write!(f, "START"),
            TokenKind::Commit => write!(f, "COMMIT"),
            TokenKind::Rollback => write!(f, "ROLLBACK"),
            TokenKind::Reset => write!(f, "RESET"),
            TokenKind::Close => write!(f, "CLOSE"),
            TokenKind::Work => write!(f, "WORK"),
            TokenKind::Zone => write!(f, "ZONE"),
            TokenKind::Characteristics => write!(f, "CHARACTERISTICS"),
            TokenKind::Read => write!(f, "READ"),
            TokenKind::Write => write!(f, "WRITE"),
            TokenKind::Only => write!(f, "ONLY"),
            TokenKind::Modifying => write!(f, "MODIFYING"),
            TokenKind::Current => write!(f, "CURRENT"),
            TokenKind::Home => write!(f, "HOME"),
            TokenKind::Date => write!(f, "DATE"),
            TokenKind::Time => write!(f, "TIME"),
            TokenKind::Timestamp => write!(f, "TIMESTAMP"),
            TokenKind::Duration => write!(f, "DURATION"),
            TokenKind::True => write!(f, "TRUE"),
            TokenKind::False => write!(f, "FALSE"),
            TokenKind::Null => write!(f, "NULL"),
            TokenKind::Unknown => write!(f, "UNKNOWN"),
            // Type keywords
            TokenKind::Bool => write!(f, "BOOL"),
            TokenKind::Boolean => write!(f, "BOOLEAN"),
            TokenKind::String => write!(f, "STRING"),
            TokenKind::Char => write!(f, "CHAR"),
            TokenKind::Varchar => write!(f, "VARCHAR"),
            TokenKind::Bytes => write!(f, "BYTES"),
            TokenKind::Binary => write!(f, "BINARY"),
            TokenKind::Varbinary => write!(f, "VARBINARY"),
            TokenKind::Int => write!(f, "INT"),
            TokenKind::Integer => write!(f, "INTEGER"),
            TokenKind::Int8 => write!(f, "INT8"),
            TokenKind::Int16 => write!(f, "INT16"),
            TokenKind::Int32 => write!(f, "INT32"),
            TokenKind::Int64 => write!(f, "INT64"),
            TokenKind::Int128 => write!(f, "INT128"),
            TokenKind::Int256 => write!(f, "INT256"),
            TokenKind::Smallint => write!(f, "SMALLINT"),
            TokenKind::Bigint => write!(f, "BIGINT"),
            TokenKind::Signed => write!(f, "SIGNED"),
            TokenKind::Uint => write!(f, "UINT"),
            TokenKind::Uint8 => write!(f, "UINT8"),
            TokenKind::Uint16 => write!(f, "UINT16"),
            TokenKind::Uint32 => write!(f, "UINT32"),
            TokenKind::Uint64 => write!(f, "UINT64"),
            TokenKind::Uint128 => write!(f, "UINT128"),
            TokenKind::Uint256 => write!(f, "UINT256"),
            TokenKind::Usmallint => write!(f, "USMALLINT"),
            TokenKind::Ubigint => write!(f, "UBIGINT"),
            TokenKind::Unsigned => write!(f, "UNSIGNED"),
            TokenKind::Decimal => write!(f, "DECIMAL"),
            TokenKind::Dec => write!(f, "DEC"),
            TokenKind::Float => write!(f, "FLOAT"),
            TokenKind::Float16 => write!(f, "FLOAT16"),
            TokenKind::Float32 => write!(f, "FLOAT32"),
            TokenKind::Float64 => write!(f, "FLOAT64"),
            TokenKind::Float128 => write!(f, "FLOAT128"),
            TokenKind::Float256 => write!(f, "FLOAT256"),
            TokenKind::Real => write!(f, "REAL"),
            TokenKind::Double => write!(f, "DOUBLE"),
            TokenKind::Precision => write!(f, "PRECISION"),
            TokenKind::Zoned => write!(f, "ZONED"),
            TokenKind::Local => write!(f, "LOCAL"),
            TokenKind::Datetime => write!(f, "DATETIME"),
            TokenKind::Without => write!(f, "WITHOUT"),
            TokenKind::Year => write!(f, "YEAR"),
            TokenKind::Month => write!(f, "MONTH"),
            TokenKind::Day => write!(f, "DAY"),
            TokenKind::Second => write!(f, "SECOND"),
            TokenKind::To => write!(f, "TO"),
            TokenKind::Nothing => write!(f, "NOTHING"),
            TokenKind::List => write!(f, "LIST"),
            TokenKind::Array => write!(f, "ARRAY"),
            TokenKind::Record => write!(f, "RECORD"),
            TokenKind::Vertex => write!(f, "VERTEX"),
            TokenKind::Value => write!(f, "VALUE"),
            TokenKind::Table => write!(f, "TABLE"),
            TokenKind::Binding => write!(f, "BINDING"),
            TokenKind::Variable => write!(f, "VARIABLE"),
            TokenKind::Nulls => write!(f, "NULLS"),
            TokenKind::First => write!(f, "FIRST"),
            TokenKind::Last => write!(f, "LAST"),
            TokenKind::Ordinality => write!(f, "ORDINALITY"),
            TokenKind::Typed => write!(f, "TYPED"),
            TokenKind::Normalized => write!(f, "NORMALIZED"),
            TokenKind::Directed => write!(f, "DIRECTED"),
            TokenKind::Labeled => write!(f, "LABELED"),
            TokenKind::Source => write!(f, "SOURCE"),
            TokenKind::Destination => write!(f, "DESTINATION"),
            TokenKind::Abs => write!(f, "ABS"),
            TokenKind::Mod => write!(f, "MOD"),
            TokenKind::Floor => write!(f, "FLOOR"),
            TokenKind::Ceil => write!(f, "CEIL"),
            TokenKind::Sqrt => write!(f, "SQRT"),
            TokenKind::Power => write!(f, "POWER"),
            TokenKind::Exp => write!(f, "EXP"),
            TokenKind::Ln => write!(f, "LN"),
            TokenKind::Log => write!(f, "LOG"),
            TokenKind::Sin => write!(f, "SIN"),
            TokenKind::Cos => write!(f, "COS"),
            TokenKind::Tan => write!(f, "TAN"),
            TokenKind::Asin => write!(f, "ASIN"),
            TokenKind::Acos => write!(f, "ACOS"),
            TokenKind::Atan => write!(f, "ATAN"),
            TokenKind::Upper => write!(f, "UPPER"),
            TokenKind::Lower => write!(f, "LOWER"),
            TokenKind::Trim => write!(f, "TRIM"),
            TokenKind::Substring => write!(f, "SUBSTRING"),
            TokenKind::Normalize => write!(f, "NORMALIZE"),
            TokenKind::Coalesce => write!(f, "COALESCE"),
            TokenKind::Nullif => write!(f, "NULLIF"),
            TokenKind::Cardinality => write!(f, "CARDINALITY"),
            TokenKind::Size => write!(f, "SIZE"),
            TokenKind::Elements => write!(f, "ELEMENTS"),
            TokenKind::Element => write!(f, "ELEMENT"),
            TokenKind::AllDifferent => write!(f, "ALL_DIFFERENT"),
            TokenKind::Same => write!(f, "SAME"),
            TokenKind::PropertyExists => write!(f, "PROPERTY_EXISTS"),
            TokenKind::Identifier(name) => write!(f, "{name}"),
            TokenKind::DelimitedIdentifier(name) => write!(f, "`{name}`"),
            TokenKind::StringLiteral(s) => write!(f, "'{s}'"),
            TokenKind::ByteStringLiteral(s) => write!(f, "X'{s}'"),
            TokenKind::IntegerLiteral(n) => write!(f, "{n}"),
            TokenKind::FloatLiteral(n) => write!(f, "{n}"),
            TokenKind::Parameter(name) => write!(f, "${name}"),
            TokenKind::ReferenceParameter(name) => write!(f, "$${name}"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::NotEq => write!(f, "<>"),
            TokenKind::NotEqBang => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::LeftArrow => write!(f, "<-"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::LeftTilde => write!(f, "<~"),
            TokenKind::RightTilde => write!(f, "~>"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::DoublePipe => write!(f, "||"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::DoubleColon => write!(f, "::"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Eof => write!(f, "<EOF>"),
        }
    }
}

/// A lexical token with its kind and source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The kind of token.
    pub kind: TokenKind,
    /// The span in source text.
    pub span: Span,
}

impl Token {
    /// Creates a new token.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Returns the source slice covered by this token.
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.span.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_creation() {
        let token = Token::new(TokenKind::Match, 0..5);
        assert_eq!(token.kind, TokenKind::Match);
        assert_eq!(token.span, 0..5);
    }

    #[test]
    fn token_kind_is_keyword() {
        assert!(TokenKind::Match.is_keyword());
        assert!(TokenKind::Where.is_keyword());
        assert!(TokenKind::And.is_keyword());
        assert!(!TokenKind::Identifier("foo".into()).is_keyword());
        assert!(!TokenKind::Plus.is_keyword());
    }

    #[test]
    fn token_kind_is_literal() {
        assert!(TokenKind::StringLiteral("test".into()).is_literal());
        assert!(TokenKind::IntegerLiteral("42".into()).is_literal());
        assert!(TokenKind::True.is_literal());
        assert!(TokenKind::Null.is_literal());
        assert!(!TokenKind::Match.is_literal());
        assert!(!TokenKind::Plus.is_literal());
    }

    #[test]
    fn token_kind_is_operator() {
        assert!(TokenKind::Plus.is_operator());
        assert!(TokenKind::Arrow.is_operator());
        assert!(TokenKind::Eq.is_operator());
        assert!(!TokenKind::Match.is_operator());
        assert!(!TokenKind::LParen.is_operator());
    }

    #[test]
    fn token_kind_display() {
        assert_eq!(TokenKind::Match.to_string(), "MATCH");
        assert_eq!(TokenKind::Plus.to_string(), "+");
        assert_eq!(TokenKind::Arrow.to_string(), "->");
        assert_eq!(
            TokenKind::StringLiteral("hello".into()).to_string(),
            "'hello'"
        );
        assert_eq!(TokenKind::Identifier("foo".into()).to_string(), "foo");
    }
}
