//! Keyword recognition and classification for GQL.
//!
//! GQL keywords are case-insensitive per the ISO standard.

use super::token::TokenKind;

/// Looks up a keyword by name (case-insensitive).
pub fn lookup_keyword(name: &str) -> Option<TokenKind> {
    match name.to_ascii_uppercase().as_str() {
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
        "LABELED" => Some(TokenKind::Labeled),
        "SOURCE" => Some(TokenKind::Source),
        "DESTINATION" => Some(TokenKind::Destination),

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

        _ => None,
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
}
