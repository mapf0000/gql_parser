//! Keyword recognition and classification for GQL.
//!
//! GQL keywords are case-insensitive per the ISO standard.
//! This module provides efficient keyword lookup and classification.

use super::token::TokenKind;
use std::collections::HashMap;

/// Looks up a keyword by name (case-insensitive).
///
/// Returns the corresponding TokenKind if the input is a keyword,
/// or None if it's a regular identifier.
pub fn lookup_keyword(name: &str) -> Option<TokenKind> {
    // Convert to uppercase for case-insensitive comparison
    let upper = name.to_uppercase();
    KEYWORD_MAP.get(upper.as_str()).cloned()
}

/// Returns true if the given name is a keyword (case-insensitive).
pub fn is_keyword(name: &str) -> bool {
    lookup_keyword(name).is_some()
}

lazy_static::lazy_static! {
    static ref KEYWORD_MAP: HashMap<&'static str, TokenKind> = {
        let mut m = HashMap::new();

        // Reserved keywords
        m.insert("MATCH", TokenKind::Match);
        m.insert("WHERE", TokenKind::Where);
        m.insert("RETURN", TokenKind::Return);
        m.insert("CREATE", TokenKind::Create);
        m.insert("DELETE", TokenKind::Delete);
        m.insert("INSERT", TokenKind::Insert);
        m.insert("SET", TokenKind::Set);
        m.insert("REMOVE", TokenKind::Remove);
        m.insert("WITH", TokenKind::With);
        m.insert("CALL", TokenKind::Call);
        m.insert("YIELD", TokenKind::Yield);
        m.insert("UNION", TokenKind::Union);
        m.insert("INTERSECT", TokenKind::Intersect);
        m.insert("EXCEPT", TokenKind::Except);
        m.insert("OTHERWISE", TokenKind::Otherwise);
        m.insert("OPTIONAL", TokenKind::Optional);
        m.insert("USE", TokenKind::Use);
        m.insert("AT", TokenKind::At);
        m.insert("NEXT", TokenKind::Next);
        m.insert("FINISH", TokenKind::Finish);
        m.insert("LET", TokenKind::Let);
        m.insert("FOR", TokenKind::For);
        m.insert("FILTER", TokenKind::Filter);
        m.insert("ORDER", TokenKind::Order);
        m.insert("BY", TokenKind::By);
        m.insert("ASC", TokenKind::Asc);
        m.insert("ASCENDING", TokenKind::Ascending);
        m.insert("DESC", TokenKind::Desc);
        m.insert("DESCENDING", TokenKind::Descending);
        m.insert("SKIP", TokenKind::Skip);
        m.insert("LIMIT", TokenKind::Limit);
        m.insert("OFFSET", TokenKind::Offset);
        m.insert("SELECT", TokenKind::Select);
        m.insert("DISTINCT", TokenKind::Distinct);
        m.insert("GROUP", TokenKind::Group);
        m.insert("HAVING", TokenKind::Having);
        m.insert("AS", TokenKind::As);
        m.insert("FROM", TokenKind::From);
        m.insert("WHEN", TokenKind::When);
        m.insert("THEN", TokenKind::Then);
        m.insert("ELSE", TokenKind::Else);
        m.insert("END", TokenKind::End);
        m.insert("CASE", TokenKind::Case);
        m.insert("IF", TokenKind::If);
        m.insert("CAST", TokenKind::Cast);

        // Logical operators
        m.insert("AND", TokenKind::And);
        m.insert("OR", TokenKind::Or);
        m.insert("NOT", TokenKind::Not);
        m.insert("XOR", TokenKind::Xor);
        m.insert("IS", TokenKind::Is);
        m.insert("IN", TokenKind::In);

        // Quantifiers
        m.insert("ANY", TokenKind::Any);
        m.insert("ALL", TokenKind::All);
        m.insert("SOME", TokenKind::Some);
        m.insert("EXISTS", TokenKind::Exists);

        // Graph keywords
        m.insert("GRAPH", TokenKind::Graph);
        m.insert("NODE", TokenKind::Node);
        m.insert("EDGE", TokenKind::Edge);
        m.insert("PATH", TokenKind::Path);
        m.insert("RELATIONSHIP", TokenKind::Relationship);
        m.insert("WALK", TokenKind::Walk);
        m.insert("TRAIL", TokenKind::Trail);
        m.insert("ACYCLIC", TokenKind::Acyclic);
        m.insert("SIMPLE", TokenKind::Simple);

        // Schema/catalog keywords
        m.insert("SCHEMA", TokenKind::Schema);
        m.insert("CATALOG", TokenKind::Catalog);
        m.insert("DROP", TokenKind::Drop);
        m.insert("ALTER", TokenKind::Alter);
        m.insert("PROPERTY", TokenKind::Property);
        m.insert("LABEL", TokenKind::Label);

        // Temporal keywords
        m.insert("DATE", TokenKind::Date);
        m.insert("TIME", TokenKind::Time);
        m.insert("TIMESTAMP", TokenKind::Timestamp);
        m.insert("DURATION", TokenKind::Duration);

        // Boolean literals
        m.insert("TRUE", TokenKind::True);
        m.insert("FALSE", TokenKind::False);

        // Null literals
        m.insert("NULL", TokenKind::Null);
        m.insert("UNKNOWN", TokenKind::Unknown);

        // Type names
        m.insert("STRING", TokenKind::String);
        m.insert("INTEGER", TokenKind::Integer);
        m.insert("FLOAT", TokenKind::Float);
        m.insert("BOOLEAN", TokenKind::Boolean);
        m.insert("LIST", TokenKind::List);
        m.insert("RECORD", TokenKind::Record);

        // Standalone keywords
        m.insert("DETACH", TokenKind::Detach);
        m.insert("NODETACH", TokenKind::Nodetach);

        m
    };
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
