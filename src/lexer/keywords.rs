//! Keyword recognition and classification for GQL.
//!
//! GQL keywords are case-insensitive per the ISO standard.
//!
//! ## Keyword Classification
//!
//! Per ISO GQL specification, keywords are classified into three categories:
//!
//! 1. **Reserved Words** (~200 keywords): Cannot be used as regular identifiers.
//!    Must be delimited with quotes (`"CREATE"`) or backticks (`` `SELECT` ``) to use as identifiers.
//!    Grammar reference: GQL.g4 lines 3277-3494
//!
//! 2. **Pre-Reserved Words** (~40 keywords): Future-proofed for potential use in later GQL versions.
//!    Currently allowed as identifiers for forward compatibility.
//!    Grammar reference: GQL.g4 lines 3497-3535
//!
//! 3. **Non-Reserved Words** (~50 keywords): Context-sensitive keywords that can act as both
//!    keywords and identifiers depending on parsing context.
//!    Grammar reference: GQL.g4 lines 3538-3584

use super::token::TokenKind;

/// Classification of GQL keywords per ISO specification.
///
/// This enum distinguishes between reserved, pre-reserved, and non-reserved words,
/// which affects identifier parsing and validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordClassification {
    /// Reserved words that cannot be used as undelimited identifiers.
    /// Examples: SELECT, MATCH, CREATE, INSERT, DELETE
    Reserved,

    /// Pre-reserved words for future GQL versions, currently allowed as identifiers.
    /// Examples: ABSTRACT, CONSTRAINT, FUNCTION, AGGREGATE
    PreReserved,

    /// Non-reserved words that are context-sensitive.
    /// Can be keywords in some contexts and identifiers in others.
    /// Examples: GRAPH, NODE, EDGE, TYPE, DIRECTED
    NonReserved,
}

const RESERVED_WORDS: &[&str] = &[
    "ABS",
    "ACOS",
    "ALL",
    "ALL_DIFFERENT",
    "AND",
    "ANY",
    "ARRAY",
    "AS",
    "ASC",
    "ASCENDING",
    "ASIN",
    "AT",
    "ATAN",
    "AVG",
    "BIG",
    "BIGINT",
    "BINARY",
    "BOOL",
    "BOOLEAN",
    "BOTH",
    "BTRIM",
    "BY",
    "BYTES",
    "BYTE_LENGTH",
    "CALL",
    "CARDINALITY",
    "CASE",
    "CAST",
    "CEIL",
    "CEILING",
    "CHAR",
    "CHARACTERISTICS",
    "CHARACTER_LENGTH",
    "CHAR_LENGTH",
    "CLOSE",
    "COALESCE",
    "COLLECT_LIST",
    "COMMIT",
    "COPY",
    "COS",
    "COSH",
    "COT",
    "COUNT",
    "CREATE",
    "CURRENT_DATE",
    "CURRENT_GRAPH",
    "CURRENT_PROPERTY_GRAPH",
    "CURRENT_SCHEMA",
    "CURRENT_TIME",
    "CURRENT_TIMESTAMP",
    "DATE",
    "DATETIME",
    "DAY",
    "DEC",
    "DECIMAL",
    "DEGREES",
    "DELETE",
    "DESC",
    "DESCENDING",
    "DETACH",
    "DISTINCT",
    "DOUBLE",
    "DROP",
    "DURATION",
    "DURATION_BETWEEN",
    "ELEMENT_ID",
    "ELSE",
    "END",
    "EXCEPT",
    "EXISTS",
    "EXP",
    "FILTER",
    "FINISH",
    "FLOAT",
    "FLOAT128",
    "FLOAT16",
    "FLOAT256",
    "FLOAT32",
    "FLOAT64",
    "FLOOR",
    "FOR",
    "FROM",
    "GROUP",
    "HAVING",
    "HOME_GRAPH",
    "HOME_PROPERTY_GRAPH",
    "HOME_SCHEMA",
    "HOUR",
    "IF",
    "IN",
    "INSERT",
    "INT",
    "INT128",
    "INT16",
    "INT256",
    "INT32",
    "INT64",
    "INT8",
    "INTEGER",
    "INTEGER128",
    "INTEGER16",
    "INTEGER256",
    "INTEGER32",
    "INTEGER64",
    "INTEGER8",
    "INTERSECT",
    "INTERVAL",
    "IS",
    "LEADING",
    "LEFT",
    "LET",
    "LIKE",
    "LIMIT",
    "LIST",
    "LN",
    "LOCAL",
    "LOCAL_DATETIME",
    "LOCAL_TIME",
    "LOCAL_TIMESTAMP",
    "LOG",
    "LOG10",
    "LOWER",
    "LTRIM",
    "MATCH",
    "MAX",
    "MIN",
    "MINUTE",
    "MOD",
    "MONTH",
    "NEXT",
    "NODETACH",
    "NORMALIZE",
    "NOT",
    "NOTHING",
    "NULL",
    "NULLIF",
    "NULLS",
    "OCTET_LENGTH",
    "OF",
    "OFFSET",
    "OPTIONAL",
    "OR",
    "ORDER",
    "OTHERWISE",
    "PARAMETER",
    "PARAMETERS",
    "PATH",
    "PATHS",
    "PATH_LENGTH",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "POWER",
    "PRECISION",
    "PROPERTY_EXISTS",
    "RADIANS",
    "REAL",
    "RECORD",
    "REMOVE",
    "REPLACE",
    "RESET",
    "RETURN",
    "RIGHT",
    "ROLLBACK",
    "RTRIM",
    "SAME",
    "SCHEMA",
    "SECOND",
    "SELECT",
    "SESSION",
    "SESSION_USER",
    "SET",
    "SIGNED",
    "SIN",
    "SINH",
    "SIZE",
    "SKIP",
    "SMALL",
    "SMALLINT",
    "SQRT",
    "START",
    "STDDEV_POP",
    "STDDEV_SAMP",
    "STRING",
    "SUM",
    "TAN",
    "TANH",
    "THEN",
    "TIME",
    "TIMESTAMP",
    "TRAILING",
    "TRIM",
    "TYPED",
    "UBIGINT",
    "UINT",
    "UINT128",
    "UINT16",
    "UINT256",
    "UINT32",
    "UINT64",
    "UINT8",
    "UNION",
    "UNSIGNED",
    "UPPER",
    "USE",
    "USMALLINT",
    "VALUE",
    "VARBINARY",
    "VARCHAR",
    "VARIABLE",
    "WHEN",
    "WHERE",
    "WITH",
    "XOR",
    "YEAR",
    "YIELD",
    "ZONED",
    "ZONED_DATETIME",
    "ZONED_TIME",
];

const PRE_RESERVED_WORDS: &[&str] = &[
    "ABSTRACT",
    "AGGREGATE",
    "AGGREGATES",
    "ALTER",
    "CATALOG",
    "CLEAR",
    "CLONE",
    "CONSTRAINT",
    "CURRENT_ROLE",
    "CURRENT_USER",
    "DATA",
    "DIRECTORY",
    "DRYRUN",
    "EXACT",
    "EXISTING",
    "FUNCTION",
    "GQLSTATUS",
    "GRANT",
    "INFINITY",
    "INSTANT",
    "NUMBER",
    "NUMERIC",
    "ON",
    "OPEN",
    "PARTITION",
    "PROCEDURE",
    "PRODUCT",
    "PROJECT",
    "QUERY",
    "RECORDS",
    "REFERENCE",
    "RENAME",
    "REVOKE",
    "SUBSTRING",
    "SYSTEM_USER",
    "TEMPORAL",
    "UNIQUE",
    "UNIT",
    "VALUES",
];

const NON_RESERVED_WORDS: &[&str] = &[
    "ACYCLIC",
    "BINDING",
    "BINDINGS",
    "CONNECTING",
    "DESTINATION",
    "DIFFERENT",
    "DIRECTED",
    "EDGE",
    "EDGES",
    "ELEMENT",
    "ELEMENTS",
    "FIRST",
    "GRAPH",
    "GROUPS",
    "KEEP",
    "LABEL",
    "LABELED",
    "LABELS",
    "LAST",
    "NFC",
    "NFD",
    "NFKC",
    "NFKD",
    "NO",
    "NODE",
    "NORMALIZED",
    "ONLY",
    "ORDINALITY",
    "PROPERTY",
    "READ",
    "RELATIONSHIP",
    "RELATIONSHIPS",
    "REPEATABLE",
    "SHORTEST",
    "SIMPLE",
    "SOURCE",
    "TABLE",
    "TO",
    "TRAIL",
    "TRANSACTION",
    "TYPE",
    "UNDIRECTED",
    "VERTEX",
    "WALK",
    "WITHOUT",
    "WRITE",
    "ZONE",
];

fn contains_keyword(list: &[&str], upper: &str) -> bool {
    list.binary_search(&upper).is_ok()
}

/// Classifies a keyword by its type (reserved, pre-reserved, or non-reserved).
///
/// Returns `None` if the name is not a recognized keyword.
///
/// # Examples
///
/// ```rust
/// use gql_parser::{classify_keyword, KeywordClassification};
///
/// assert_eq!(classify_keyword("MATCH"), Some(KeywordClassification::Reserved));
/// assert_eq!(classify_keyword("ABSTRACT"), Some(KeywordClassification::PreReserved));
/// assert_eq!(classify_keyword("GRAPH"), Some(KeywordClassification::NonReserved));
/// assert_eq!(classify_keyword("myVariable"), None);
/// ```
pub fn classify_keyword(name: &str) -> Option<KeywordClassification> {
    let upper = name.to_ascii_uppercase();
    if contains_keyword(RESERVED_WORDS, &upper) {
        Some(KeywordClassification::Reserved)
    } else if contains_keyword(PRE_RESERVED_WORDS, &upper) {
        Some(KeywordClassification::PreReserved)
    } else if contains_keyword(NON_RESERVED_WORDS, &upper) {
        Some(KeywordClassification::NonReserved)
    } else {
        None
    }
}

/// Returns true if the given name is a reserved word.
///
/// # Examples
///
/// ```rust
/// use gql_parser::is_reserved_word;
///
/// assert!(is_reserved_word("MATCH"));
/// assert!(is_reserved_word("match"));
/// assert!(!is_reserved_word("GRAPH")); // Non-reserved
/// assert!(!is_reserved_word("ABSTRACT")); // Pre-reserved
/// assert!(!is_reserved_word("myVariable")); // Not a keyword
/// ```
pub fn is_reserved_word(name: &str) -> bool {
    matches!(
        classify_keyword(name),
        Some(KeywordClassification::Reserved)
    )
}

/// Returns true if the given name is a pre-reserved word.
pub fn is_pre_reserved_word(name: &str) -> bool {
    matches!(
        classify_keyword(name),
        Some(KeywordClassification::PreReserved)
    )
}

/// Returns true if the given name is a non-reserved word.
pub fn is_non_reserved_word(name: &str) -> bool {
    matches!(
        classify_keyword(name),
        Some(KeywordClassification::NonReserved)
    )
}

/// Looks up a keyword by name (case-insensitive).
pub fn lookup_keyword(name: &str) -> Option<TokenKind> {
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        // Reserved keywords
        "MATCH" => Some(TokenKind::Match),
        "WHERE" => Some(TokenKind::Where),
        "RETURN" => Some(TokenKind::Return),
        "CREATE" => Some(TokenKind::Create),
        "DELETE" => Some(TokenKind::Delete),
        "INSERT" => Some(TokenKind::Insert),
        "SET" => Some(TokenKind::Set),
        "REMOVE" => Some(TokenKind::Remove),
        "WITH" => Some(TokenKind::With),
        "CALL" => Some(TokenKind::Call),
        "YIELD" => Some(TokenKind::Yield),
        "UNION" => Some(TokenKind::Union),
        "INTERSECT" => Some(TokenKind::Intersect),
        "EXCEPT" => Some(TokenKind::Except),
        "OTHERWISE" => Some(TokenKind::Otherwise),
        "OPTIONAL" => Some(TokenKind::Optional),
        "USE" => Some(TokenKind::Use),
        "AT" => Some(TokenKind::At),
        "NEXT" => Some(TokenKind::Next),
        "FINISH" => Some(TokenKind::Finish),
        "LET" => Some(TokenKind::Let),
        "FOR" => Some(TokenKind::For),
        "FILTER" => Some(TokenKind::Filter),
        "ORDER" => Some(TokenKind::Order),
        "BY" => Some(TokenKind::By),
        "ASC" => Some(TokenKind::Asc),
        "ASCENDING" => Some(TokenKind::Ascending),
        "DESC" => Some(TokenKind::Desc),
        "DESCENDING" => Some(TokenKind::Descending),
        "SKIP" => Some(TokenKind::Skip),
        "LIMIT" => Some(TokenKind::Limit),
        "OFFSET" => Some(TokenKind::Offset),
        "SELECT" => Some(TokenKind::Select),
        "DISTINCT" => Some(TokenKind::Distinct),
        "GROUP" => Some(TokenKind::Group),
        "HAVING" => Some(TokenKind::Having),
        "AS" => Some(TokenKind::As),
        "FROM" => Some(TokenKind::From),
        "WHEN" => Some(TokenKind::When),
        "THEN" => Some(TokenKind::Then),
        "ELSE" => Some(TokenKind::Else),
        "END" => Some(TokenKind::End),
        "CASE" => Some(TokenKind::Case),
        "IF" => Some(TokenKind::If),
        "CAST" => Some(TokenKind::Cast),

        // Logical operators
        "AND" => Some(TokenKind::And),
        "OR" => Some(TokenKind::Or),
        "NOT" => Some(TokenKind::Not),
        "XOR" => Some(TokenKind::Xor),
        "IS" => Some(TokenKind::Is),
        "IN" => Some(TokenKind::In),

        // Quantifiers
        "ANY" => Some(TokenKind::Any),
        "ALL" => Some(TokenKind::All),
        "SOME" => Some(TokenKind::Some),
        "EXISTS" => Some(TokenKind::Exists),

        // Graph keywords
        "GRAPH" => Some(TokenKind::Graph),
        "NODE" => Some(TokenKind::Node),
        "EDGE" => Some(TokenKind::Edge),
        "PATH" => Some(TokenKind::Path),
        "RELATIONSHIP" => Some(TokenKind::Relationship),
        "WALK" => Some(TokenKind::Walk),
        "TRAIL" => Some(TokenKind::Trail),
        "ACYCLIC" => Some(TokenKind::Acyclic),
        "SIMPLE" => Some(TokenKind::Simple),

        // Pattern matching keywords
        "REPEATABLE" => Some(TokenKind::Repeatable),
        "DIFFERENT" => Some(TokenKind::Different),
        "KEEP" => Some(TokenKind::Keep),
        "SHORTEST" => Some(TokenKind::Shortest),
        "PATHS" => Some(TokenKind::Paths),
        "GROUPS" => Some(TokenKind::Groups),
        "LABELS" => Some(TokenKind::Labels),

        // Aggregate function keywords
        "AVG" => Some(TokenKind::Avg),
        "COUNT" => Some(TokenKind::Count),
        "MAX" => Some(TokenKind::Max),
        "MIN" => Some(TokenKind::Min),
        "SUM" => Some(TokenKind::Sum),
        "COLLECT_LIST" => Some(TokenKind::CollectList),
        "STDDEV_SAMP" => Some(TokenKind::StddevSamp),
        "STDDEV_POP" => Some(TokenKind::StddevPop),
        "PERCENTILE_CONT" => Some(TokenKind::PercentileCont),
        "PERCENTILE_DISC" => Some(TokenKind::PercentileDisc),

        // Schema/catalog keywords
        "SCHEMA" => Some(TokenKind::Schema),
        "CATALOG" => Some(TokenKind::Catalog),
        "DROP" => Some(TokenKind::Drop),
        "ALTER" => Some(TokenKind::Alter),
        "PROPERTY" => Some(TokenKind::Property),
        "LABEL" => Some(TokenKind::Label),
        "TYPE" => Some(TokenKind::Type),
        "REPLACE" => Some(TokenKind::Replace),
        "OF" => Some(TokenKind::Of),
        "LIKE" => Some(TokenKind::Like),
        "COPY" => Some(TokenKind::Copy),

        // Session/Transaction keywords
        "SESSION" => Some(TokenKind::Session),
        "TRANSACTION" => Some(TokenKind::Transaction),
        "START" => Some(TokenKind::Start),
        "COMMIT" => Some(TokenKind::Commit),
        "ROLLBACK" => Some(TokenKind::Rollback),
        "RESET" => Some(TokenKind::Reset),
        "CLOSE" => Some(TokenKind::Close),
        "WORK" => Some(TokenKind::Work),
        "ZONE" => Some(TokenKind::Zone),
        "CHARACTERISTICS" => Some(TokenKind::Characteristics),
        "READ" => Some(TokenKind::Read),
        "WRITE" => Some(TokenKind::Write),
        "ONLY" => Some(TokenKind::Only),
        "MODIFYING" => Some(TokenKind::Modifying),
        "CURRENT" => Some(TokenKind::Current),
        "HOME" => Some(TokenKind::Home),

        // Temporal keywords
        "DATE" => Some(TokenKind::Date),
        "TIME" => Some(TokenKind::Time),
        "TIMESTAMP" => Some(TokenKind::Timestamp),
        "DURATION" => Some(TokenKind::Duration),

        // Boolean literals
        "TRUE" => Some(TokenKind::True),
        "FALSE" => Some(TokenKind::False),

        // Null literals
        "NULL" => Some(TokenKind::Null),
        "UNKNOWN" => Some(TokenKind::Unknown),

        // Type names - Boolean
        "BOOL" => Some(TokenKind::Bool),
        "BOOLEAN" => Some(TokenKind::Boolean),

        // Type names - String
        "STRING" => Some(TokenKind::String),
        "CHAR" => Some(TokenKind::Char),
        "VARCHAR" => Some(TokenKind::Varchar),

        // Type names - Bytes
        "BYTES" => Some(TokenKind::Bytes),
        "BINARY" => Some(TokenKind::Binary),
        "VARBINARY" => Some(TokenKind::Varbinary),

        // Type names - Numeric (Signed)
        "INT" => Some(TokenKind::Int),
        "INTEGER" => Some(TokenKind::Integer),
        "INT8" => Some(TokenKind::Int8),
        "INT16" => Some(TokenKind::Int16),
        "INT32" => Some(TokenKind::Int32),
        "INT64" => Some(TokenKind::Int64),
        "INT128" => Some(TokenKind::Int128),
        "INT256" => Some(TokenKind::Int256),
        "SMALLINT" => Some(TokenKind::Smallint),
        "BIGINT" => Some(TokenKind::Bigint),
        "SIGNED" => Some(TokenKind::Signed),

        // Type names - Numeric (Unsigned)
        "UINT" => Some(TokenKind::Uint),
        "UINT8" => Some(TokenKind::Uint8),
        "UINT16" => Some(TokenKind::Uint16),
        "UINT32" => Some(TokenKind::Uint32),
        "UINT64" => Some(TokenKind::Uint64),
        "UINT128" => Some(TokenKind::Uint128),
        "UINT256" => Some(TokenKind::Uint256),
        "USMALLINT" => Some(TokenKind::Usmallint),
        "UBIGINT" => Some(TokenKind::Ubigint),
        "UNSIGNED" => Some(TokenKind::Unsigned),

        // Type names - Numeric (Decimal/Float)
        "DECIMAL" => Some(TokenKind::Decimal),
        "DEC" => Some(TokenKind::Dec),
        "FLOAT" => Some(TokenKind::Float),
        "FLOAT16" => Some(TokenKind::Float16),
        "FLOAT32" => Some(TokenKind::Float32),
        "FLOAT64" => Some(TokenKind::Float64),
        "FLOAT128" => Some(TokenKind::Float128),
        "FLOAT256" => Some(TokenKind::Float256),
        "REAL" => Some(TokenKind::Real),
        "DOUBLE" => Some(TokenKind::Double),
        "PRECISION" => Some(TokenKind::Precision),

        // Type names - Temporal
        "ZONED" => Some(TokenKind::Zoned),
        "LOCAL" => Some(TokenKind::Local),
        "DATETIME" => Some(TokenKind::Datetime),
        "WITHOUT" => Some(TokenKind::Without),
        "YEAR" => Some(TokenKind::Year),
        "MONTH" => Some(TokenKind::Month),
        "DAY" => Some(TokenKind::Day),
        "SECOND" => Some(TokenKind::Second),
        "TO" => Some(TokenKind::To),

        // Type names - Other
        "NOTHING" => Some(TokenKind::Nothing),
        "LIST" => Some(TokenKind::List),
        "ARRAY" => Some(TokenKind::Array),
        "RECORD" => Some(TokenKind::Record),
        "VERTEX" => Some(TokenKind::Vertex),

        // Additional expression and function keywords
        "VALUE" => Some(TokenKind::Value),
        "TABLE" => Some(TokenKind::Table),
        "BINDING" => Some(TokenKind::Binding),
        "VARIABLE" => Some(TokenKind::Variable),

        // Standalone keywords
        "DETACH" => Some(TokenKind::Detach),
        "NODETACH" => Some(TokenKind::Nodetach),

        // Null ordering keywords
        "NULLS" => Some(TokenKind::Nulls),
        "FIRST" => Some(TokenKind::First),
        "LAST" => Some(TokenKind::Last),

        // For statement keywords
        "ORDINALITY" => Some(TokenKind::Ordinality),

        // Predicate keywords
        "TYPED" => Some(TokenKind::Typed),
        "NORMALIZED" => Some(TokenKind::Normalized),
        "DIRECTED" => Some(TokenKind::Directed),
        "UNDIRECTED" => Some(TokenKind::Undirected),
        "LABELED" => Some(TokenKind::Labeled),
        "SOURCE" => Some(TokenKind::Source),
        "DESTINATION" => Some(TokenKind::Destination),
        "CONNECTING" => Some(TokenKind::Connecting),
        "KEY" => Some(TokenKind::Key),

        // Built-in function keywords - Numeric
        "ABS" => Some(TokenKind::Abs),
        "MOD" => Some(TokenKind::Mod),
        "FLOOR" => Some(TokenKind::Floor),
        "CEIL" => Some(TokenKind::Ceil),
        "SQRT" => Some(TokenKind::Sqrt),
        "POWER" => Some(TokenKind::Power),
        "EXP" => Some(TokenKind::Exp),
        "LN" => Some(TokenKind::Ln),
        "LOG" => Some(TokenKind::Log),

        // Built-in function keywords - Trigonometric
        "SIN" => Some(TokenKind::Sin),
        "COS" => Some(TokenKind::Cos),
        "TAN" => Some(TokenKind::Tan),
        "ASIN" => Some(TokenKind::Asin),
        "ACOS" => Some(TokenKind::Acos),
        "ATAN" => Some(TokenKind::Atan),

        // Built-in function keywords - String functions
        "UPPER" => Some(TokenKind::Upper),
        "LOWER" => Some(TokenKind::Lower),
        "TRIM" => Some(TokenKind::Trim),
        "SUBSTRING" => Some(TokenKind::Substring),
        "NORMALIZE" => Some(TokenKind::Normalize),

        // Built-in function keywords - Conditional
        "COALESCE" => Some(TokenKind::Coalesce),
        "NULLIF" => Some(TokenKind::Nullif),

        // Built-in function keywords - Cardinality
        "CARDINALITY" => Some(TokenKind::Cardinality),
        "SIZE" => Some(TokenKind::Size),

        // Built-in function keywords - Graph
        "ELEMENTS" => Some(TokenKind::Elements),
        "ELEMENT" => Some(TokenKind::Element),

        // Built-in function keywords - Predicates
        "ALL_DIFFERENT" => Some(TokenKind::AllDifferent),
        "SAME" => Some(TokenKind::Same),
        "PROPERTY_EXISTS" => Some(TokenKind::PropertyExists),

        _ => classify_keyword(&upper).map(|classification| match classification {
            KeywordClassification::Reserved => TokenKind::ReservedKeyword(upper.into()),
            KeywordClassification::PreReserved => TokenKind::PreReservedKeyword(upper.into()),
            KeywordClassification::NonReserved => TokenKind::NonReservedKeyword(upper.into()),
        }),
    }
}

/// Returns true if the given name is a keyword (case-insensitive).
pub fn is_keyword(name: &str) -> bool {
    lookup_keyword(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_reserved_keyword() {
        assert_eq!(lookup_keyword("MATCH"), Some(TokenKind::Match));
        assert_eq!(lookup_keyword("match"), Some(TokenKind::Match));
        assert_eq!(lookup_keyword("Match"), Some(TokenKind::Match));
        assert_eq!(lookup_keyword("MaTcH"), Some(TokenKind::Match));
    }

    #[test]
    fn lookup_logical_keyword() {
        assert_eq!(lookup_keyword("AND"), Some(TokenKind::And));
        assert_eq!(lookup_keyword("and"), Some(TokenKind::And));
        assert_eq!(lookup_keyword("OR"), Some(TokenKind::Or));
        assert_eq!(lookup_keyword("NOT"), Some(TokenKind::Not));
    }

    #[test]
    fn lookup_boolean_literal() {
        assert_eq!(lookup_keyword("TRUE"), Some(TokenKind::True));
        assert_eq!(lookup_keyword("true"), Some(TokenKind::True));
        assert_eq!(lookup_keyword("FALSE"), Some(TokenKind::False));
        assert_eq!(lookup_keyword("false"), Some(TokenKind::False));
    }

    #[test]
    fn lookup_null_literal() {
        assert_eq!(lookup_keyword("NULL"), Some(TokenKind::Null));
        assert_eq!(lookup_keyword("null"), Some(TokenKind::Null));
        assert_eq!(lookup_keyword("UNKNOWN"), Some(TokenKind::Unknown));
    }

    #[test]
    fn lookup_type_keyword() {
        assert_eq!(lookup_keyword("STRING"), Some(TokenKind::String));
        assert_eq!(lookup_keyword("INTEGER"), Some(TokenKind::Integer));
        assert_eq!(lookup_keyword("FLOAT"), Some(TokenKind::Float));
    }

    #[test]
    fn lookup_non_keyword() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("bar123"), None);
        assert_eq!(lookup_keyword("_test"), None);
    }

    #[test]
    fn is_keyword_check() {
        assert!(is_keyword("MATCH"));
        assert!(is_keyword("match"));
        assert!(is_keyword("WHERE"));
        assert!(!is_keyword("myIdentifier"));
        assert!(!is_keyword("test123"));
    }

    #[test]
    fn temporal_keywords() {
        assert_eq!(lookup_keyword("DATE"), Some(TokenKind::Date));
        assert_eq!(lookup_keyword("TIME"), Some(TokenKind::Time));
        assert_eq!(lookup_keyword("TIMESTAMP"), Some(TokenKind::Timestamp));
        assert_eq!(lookup_keyword("DURATION"), Some(TokenKind::Duration));
    }

    #[test]
    fn graph_keywords() {
        assert_eq!(lookup_keyword("GRAPH"), Some(TokenKind::Graph));
        assert_eq!(lookup_keyword("NODE"), Some(TokenKind::Node));
        assert_eq!(lookup_keyword("EDGE"), Some(TokenKind::Edge));
        assert_eq!(lookup_keyword("PATH"), Some(TokenKind::Path));
    }

    #[test]
    fn detach_keywords() {
        assert_eq!(lookup_keyword("DETACH"), Some(TokenKind::Detach));
        assert_eq!(lookup_keyword("NODETACH"), Some(TokenKind::Nodetach));
    }

    // ===== Keyword Classification Tests =====

    #[test]
    fn classify_reserved_words() {
        // Query keywords
        assert_eq!(
            classify_keyword("MATCH"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("SELECT"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("WHERE"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("RETURN"),
            Some(KeywordClassification::Reserved)
        );

        // Data modification keywords
        assert_eq!(
            classify_keyword("INSERT"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("DELETE"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("CREATE"),
            Some(KeywordClassification::Reserved)
        );

        // Type keywords
        assert_eq!(
            classify_keyword("STRING"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("INTEGER"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("BOOLEAN"),
            Some(KeywordClassification::Reserved)
        );

        // Operator keywords
        assert_eq!(
            classify_keyword("AND"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("OR"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("NOT"),
            Some(KeywordClassification::Reserved)
        );
    }

    #[test]
    fn classify_pre_reserved_words() {
        assert_eq!(
            classify_keyword("ABSTRACT"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("CONSTRAINT"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("FUNCTION"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("AGGREGATE"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("GRANT"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("REVOKE"),
            Some(KeywordClassification::PreReserved)
        );
    }

    #[test]
    fn classify_non_reserved_words() {
        // Graph element keywords
        assert_eq!(
            classify_keyword("GRAPH"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("NODE"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("EDGE"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("PROPERTY"),
            Some(KeywordClassification::NonReserved)
        );

        // Path mode keywords
        assert_eq!(
            classify_keyword("WALK"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("TRAIL"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("SIMPLE"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("SHORTEST"),
            Some(KeywordClassification::NonReserved)
        );

        // Directionality keywords
        assert_eq!(
            classify_keyword("DIRECTED"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("UNDIRECTED"),
            Some(KeywordClassification::NonReserved)
        );

        // Context keywords
        assert_eq!(
            classify_keyword("BINDING"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("TABLE"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("TYPE"),
            Some(KeywordClassification::NonReserved)
        );
    }

    #[test]
    fn classify_non_keyword() {
        assert_eq!(classify_keyword("myVariable"), None);
        assert_eq!(classify_keyword("foo"), None);
        assert_eq!(classify_keyword("test123"), None);
        assert_eq!(classify_keyword("_private"), None);
    }

    #[test]
    fn classify_case_insensitive() {
        // Reserved words case variations
        assert_eq!(
            classify_keyword("match"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("Match"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("MATCH"),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword("MaTcH"),
            Some(KeywordClassification::Reserved)
        );

        // Pre-reserved words case variations
        assert_eq!(
            classify_keyword("abstract"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("Abstract"),
            Some(KeywordClassification::PreReserved)
        );
        assert_eq!(
            classify_keyword("ABSTRACT"),
            Some(KeywordClassification::PreReserved)
        );

        // Non-reserved words case variations
        assert_eq!(
            classify_keyword("graph"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("Graph"),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword("GRAPH"),
            Some(KeywordClassification::NonReserved)
        );
    }

    #[test]
    fn is_reserved_word_check() {
        // Reserved words
        assert!(is_reserved_word("MATCH"));
        assert!(is_reserved_word("match"));
        assert!(is_reserved_word("SELECT"));
        assert!(is_reserved_word("INSERT"));
        assert!(is_reserved_word("AND"));
        assert!(is_reserved_word("OR"));

        // Pre-reserved words (not reserved)
        assert!(!is_reserved_word("ABSTRACT"));
        assert!(!is_reserved_word("CONSTRAINT"));

        // Non-reserved words (not reserved)
        assert!(!is_reserved_word("GRAPH"));
        assert!(!is_reserved_word("NODE"));

        // Non-keywords
        assert!(!is_reserved_word("myVariable"));
        assert!(!is_reserved_word("foo"));
    }

    #[test]
    fn is_pre_reserved_word_check() {
        // Pre-reserved words
        assert!(is_pre_reserved_word("ABSTRACT"));
        assert!(is_pre_reserved_word("abstract"));
        assert!(is_pre_reserved_word("CONSTRAINT"));
        assert!(is_pre_reserved_word("FUNCTION"));

        // Reserved words (not pre-reserved)
        assert!(!is_pre_reserved_word("MATCH"));
        assert!(!is_pre_reserved_word("SELECT"));

        // Non-reserved words (not pre-reserved)
        assert!(!is_pre_reserved_word("GRAPH"));

        // Non-keywords
        assert!(!is_pre_reserved_word("myVariable"));
    }

    #[test]
    fn is_non_reserved_word_check() {
        // Non-reserved words
        assert!(is_non_reserved_word("GRAPH"));
        assert!(is_non_reserved_word("graph"));
        assert!(is_non_reserved_word("NODE"));
        assert!(is_non_reserved_word("EDGE"));
        assert!(is_non_reserved_word("DIRECTED"));
        assert!(is_non_reserved_word("BINDING"));

        // Reserved words (not non-reserved)
        assert!(!is_non_reserved_word("MATCH"));
        assert!(!is_non_reserved_word("SELECT"));

        // Pre-reserved words (not non-reserved)
        assert!(!is_non_reserved_word("ABSTRACT"));

        // Non-keywords
        assert!(!is_non_reserved_word("myVariable"));
    }

    #[test]
    fn comprehensive_keyword_coverage() {
        // Test sample of all categories to ensure comprehensive coverage

        // Reserved - Query
        let reserved_query = vec!["MATCH", "SELECT", "WHERE", "RETURN", "WITH", "ORDER", "BY"];
        for kw in reserved_query {
            assert!(is_reserved_word(kw), "{} should be reserved", kw);
        }

        // Reserved - Data modification
        let reserved_dm = vec!["INSERT", "DELETE", "CREATE", "DROP", "SET", "REMOVE"];
        for kw in reserved_dm {
            assert!(is_reserved_word(kw), "{} should be reserved", kw);
        }

        // Reserved - Types
        let reserved_types = vec![
            "INT", "STRING", "BOOLEAN", "DATE", "TIME", "TIMESTAMP", "FLOAT", "DOUBLE",
        ];
        for kw in reserved_types {
            assert!(is_reserved_word(kw), "{} should be reserved", kw);
        }

        // Pre-reserved
        let pre_reserved = vec![
            "ABSTRACT",
            "AGGREGATE",
            "CONSTRAINT",
            "FUNCTION",
            "GRANT",
            "REVOKE",
        ];
        for kw in pre_reserved {
            assert!(is_pre_reserved_word(kw), "{} should be pre-reserved", kw);
        }

        // Non-reserved
        let non_reserved = vec![
            "GRAPH",
            "NODE",
            "EDGE",
            "PROPERTY",
            "DIRECTED",
            "UNDIRECTED",
            "WALK",
            "TRAIL",
            "SIMPLE",
            "SHORTEST",
            "BINDING",
            "TABLE",
            "TYPE",
        ];
        for kw in non_reserved {
            assert!(is_non_reserved_word(kw), "{} should be non-reserved", kw);
        }
    }
}
