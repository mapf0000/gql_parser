//! Case-insensitive keyword testing for GQL parser.
//!
//! Per ISO GQL specification (GQL.g4 line 3: `options { caseInsensitive = true; }`),
//! all keywords must be case-insensitive. This test suite validates that every
//! keyword works correctly in UPPERCASE, lowercase, and MiXeDcAsE forms.

use gql_parser::lexer::keywords::lookup_keyword;
use gql_parser::{KeywordClassification, TokenKind, classify_keyword, parse};

#[test]
fn all_reserved_keywords_case_insensitive() {
    // Query keywords
    let query_keywords = vec![
        "SELECT", "MATCH", "WHERE", "RETURN", "WITH", "FILTER", "ORDER", "BY", "GROUP", "HAVING",
        "LIMIT", "OFFSET", "SKIP", "DISTINCT", "ALL", "ANY", "EXISTS", "CASE", "WHEN", "THEN",
        "ELSE", "END", "IF", "AS", "FROM", "FOR", "LET", "FINISH", "NEXT",
    ];

    for keyword in query_keywords {
        // Test UPPERCASE
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} should be reserved (uppercase)",
            keyword
        );

        // Test lowercase
        let lower = keyword.to_lowercase();
        assert_eq!(
            classify_keyword(&lower),
            Some(KeywordClassification::Reserved),
            "{} should be reserved (lowercase)",
            lower
        );

        // Test MiXeDcAsE
        let mixed: String = keyword
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_lowercase().next().expect("char should have lowercase variant")
                } else {
                    c.to_uppercase().next().expect("char should have uppercase variant")
                }
            })
            .collect();
        assert_eq!(
            classify_keyword(&mixed),
            Some(KeywordClassification::Reserved),
            "{} should be reserved (mixed case)",
            mixed
        );
    }
}

#[test]
fn data_modification_keywords_case_insensitive() {
    let dm_keywords = vec![
        "INSERT", "DELETE", "SET", "REMOVE", "DETACH", "NODETACH", "CREATE", "DROP", "COPY",
        "REPLACE",
    ];

    for keyword in dm_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::Reserved)
        );
    }
}

#[test]
fn type_keywords_case_insensitive() {
    let type_keywords = vec![
        "INT",
        "INTEGER",
        "BIGINT",
        "SMALLINT",
        "FLOAT",
        "DOUBLE",
        "REAL",
        "DECIMAL",
        "DEC",
        "BOOL",
        "BOOLEAN",
        "STRING",
        "BYTES",
        "DATE",
        "TIME",
        "TIMESTAMP",
        "DATETIME",
        "DURATION",
        "INT8",
        "INT16",
        "INT32",
        "INT64",
        "INT128",
        "INT256",
        "UINT",
        "UINT8",
        "UINT16",
        "UINT32",
        "UINT64",
        "UINT128",
        "UINT256",
        "FLOAT16",
        "FLOAT32",
        "FLOAT64",
        "FLOAT128",
        "FLOAT256",
    ];

    for keyword in type_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }
}

#[test]
fn operator_keywords_case_insensitive() {
    let operator_keywords = vec![
        "AND", "OR", "NOT", "XOR", "IS", "IN", "LIKE", "CAST", "NULLIF", "COALESCE",
    ];

    for keyword in operator_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::Reserved)
        );
    }
}

#[test]
fn aggregate_function_keywords_case_insensitive() {
    let agg_keywords = vec![
        "COUNT",
        "SUM",
        "AVG",
        "MAX",
        "MIN",
        "COLLECT_LIST",
        "STDDEV_SAMP",
        "STDDEV_POP",
        "PERCENTILE_CONT",
        "PERCENTILE_DISC",
        "CARDINALITY",
        "SIZE",
    ];

    for keyword in agg_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }
}

#[test]
fn set_operator_keywords_case_insensitive() {
    let set_keywords = vec!["UNION", "EXCEPT", "INTERSECT", "OTHERWISE"];

    for keyword in set_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::Reserved)
        );
    }
}

#[test]
fn pre_reserved_keywords_case_insensitive() {
    let pre_reserved = vec![
        "ABSTRACT",
        "AGGREGATE",
        "AGGREGATES",
        "ALTER",
        "CATALOG",
        "CONSTRAINT",
        "FUNCTION",
        "PROCEDURE",
        "QUERY",
        "SUBSTRING",
        "GRANT",
        "REVOKE",
        "RENAME",
    ];

    for keyword in pre_reserved {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::PreReserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::PreReserved),
            "{} lowercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::PreReserved),
            "{} capitalized",
            keyword
        );
    }
}

#[test]
fn non_reserved_keywords_case_insensitive() {
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
        "ACYCLIC",
        "SHORTEST",
        "BINDING",
        "TABLE",
        "TYPE",
        "SOURCE",
        "DESTINATION",
        "FIRST",
        "LAST",
    ];

    for keyword in non_reserved {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::NonReserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::NonReserved),
            "{} lowercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::NonReserved),
            "{} capitalized",
            keyword
        );
    }
}

#[test]
fn multi_word_keywords_case_insensitive() {
    // Multi-word keywords with underscores
    let multi_word = vec![
        "COLLECT_LIST",
        "STDDEV_SAMP",
        "STDDEV_POP",
        "PERCENTILE_CONT",
        "PERCENTILE_DISC",
        "ALL_DIFFERENT",
        "PROPERTY_EXISTS",
        "CURRENT_DATE",
        "CURRENT_TIME",
        "CURRENT_TIMESTAMP",
    ];

    for keyword in multi_word {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );

        // Test mixed case variations
        let parts: Vec<&str> = keyword.split('_').collect();
        let mixed = parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if i % 2 == 0 {
                    part.to_lowercase()
                } else {
                    part.to_uppercase()
                }
            })
            .collect::<Vec<_>>()
            .join("_");
        assert_eq!(
            classify_keyword(&mixed),
            Some(KeywordClassification::Reserved),
            "{} mixed case",
            mixed
        );
    }
}

#[test]
fn query_parsing_with_mixed_case_keywords() {
    // Test that actual queries parse correctly with mixed case keywords
    let queries = vec![
        "match (n) return n",
        "MATCH (n) RETURN n",
        "Match (n) Return n",
        "mAtCh (n) ReTuRn n",
        "select n from match (n)",
        "SELECT N FROM MATCH (N)",
        "SeLeCt n FrOm MaTcH (n)",
    ];

    for query in queries {
        let result = parse(query);
        // The query should parse successfully - ast should not be None
        assert!(
            result.ast.is_some(),
            "Query '{}' should parse successfully, diagnostics count: {}",
            query,
            result.diagnostics.len()
        );
    }
}

#[test]
fn boolean_literals_case_insensitive() {
    let boolean_literals = vec![
        ("TRUE", TokenKind::True),
        ("FALSE", TokenKind::False),
        ("UNKNOWN", TokenKind::Unknown),
    ];

    for (literal, expected) in boolean_literals {
        // ISO classifies these as BOOLEAN_LITERAL alternatives, not reserved keywords.
        assert_eq!(
            classify_keyword(literal),
            None,
            "{} should not be classified as a reserved/pre/non-reserved keyword",
            literal
        );
        assert_eq!(
            classify_keyword(&literal.to_lowercase()),
            None,
            "{} lowercase should not be keyword-classified",
            literal
        );
        assert_eq!(
            classify_keyword(&capitalize_first(literal)),
            None,
            "{} mixed case should not be keyword-classified",
            literal
        );

        assert_eq!(lookup_keyword(literal), Some(expected.clone()));
        assert_eq!(
            lookup_keyword(&literal.to_lowercase()),
            Some(expected.clone())
        );
        assert_eq!(lookup_keyword(&capitalize_first(literal)), Some(expected));
    }
}

#[test]
fn null_keywords_case_insensitive() {
    let null_keywords = vec!["NULL", "NOTHING", "NULLS", "NULLIF"];

    for keyword in null_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }
}

#[test]
fn temporal_keywords_case_insensitive() {
    let temporal_keywords = vec![
        "DATE",
        "TIME",
        "TIMESTAMP",
        "DATETIME",
        "DURATION",
        "YEAR",
        "MONTH",
        "DAY",
        "SECOND",
        "ZONED",
        "LOCAL",
    ];

    for keyword in temporal_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }
}

#[test]
fn sorting_keywords_case_insensitive() {
    let sorting_keywords = vec!["ASC", "ASCENDING", "DESC", "DESCENDING"];

    for keyword in sorting_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved)
        );
        assert_eq!(
            classify_keyword(&capitalize_first(keyword)),
            Some(KeywordClassification::Reserved)
        );
    }

    for keyword in ["FIRST", "LAST"] {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::NonReserved)
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::NonReserved)
        );
    }
}

#[test]
fn graph_specific_keywords_case_insensitive() {
    let graph_keywords = vec!["USE", "AT", "SAME", "ALL_DIFFERENT"];

    for keyword in graph_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }
}

#[test]
fn built_in_function_keywords_case_insensitive() {
    let function_keywords = vec![
        "ABS",
        "ACOS",
        "ASIN",
        "ATAN",
        "CEIL",
        "COS",
        "EXP",
        "FLOOR",
        "LN",
        "LOG",
        "MOD",
        "POWER",
        "SIN",
        "SQRT",
        "TAN",
        "UPPER",
        "LOWER",
        "TRIM",
        "TRIM_LIST",
        "NORMALIZE",
    ];

    for keyword in function_keywords {
        assert_eq!(
            classify_keyword(keyword),
            Some(KeywordClassification::Reserved),
            "{} uppercase",
            keyword
        );
        assert_eq!(
            classify_keyword(&keyword.to_lowercase()),
            Some(KeywordClassification::Reserved),
            "{} lowercase",
            keyword
        );
    }

    assert_eq!(
        classify_keyword("SUBSTRING"),
        Some(KeywordClassification::PreReserved)
    );
    assert_eq!(
        classify_keyword("substring"),
        Some(KeywordClassification::PreReserved)
    );
}

#[test]
fn comprehensive_case_variation_test() {
    // Test a representative sample of each keyword category with all case variations
    let test_cases = vec![
        ("MATCH", KeywordClassification::Reserved),
        ("SELECT", KeywordClassification::Reserved),
        ("AND", KeywordClassification::Reserved),
        ("ABSTRACT", KeywordClassification::PreReserved),
        ("CONSTRAINT", KeywordClassification::PreReserved),
        ("GRAPH", KeywordClassification::NonReserved),
        ("NODE", KeywordClassification::NonReserved),
        ("DIRECTED", KeywordClassification::NonReserved),
    ];

    for (keyword, expected_class) in test_cases {
        // UPPERCASE
        assert_eq!(
            classify_keyword(keyword),
            Some(expected_class),
            "{} UPPERCASE",
            keyword
        );

        // lowercase
        let lower = keyword.to_lowercase();
        assert_eq!(
            classify_keyword(&lower),
            Some(expected_class),
            "{} lowercase",
            lower
        );

        // Capitalized
        let cap = capitalize_first(keyword);
        assert_eq!(
            classify_keyword(&cap),
            Some(expected_class),
            "{} Capitalized",
            cap
        );

        // mIxEd
        let mixed1: String = keyword
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_lowercase().next().expect("char should have lowercase variant")
                } else {
                    c.to_uppercase().next().expect("char should have uppercase variant")
                }
            })
            .collect();
        assert_eq!(
            classify_keyword(&mixed1),
            Some(expected_class),
            "{} mIxEd",
            mixed1
        );

        // MiXeD (opposite pattern)
        let mixed2: String = keyword
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 1 {
                    c.to_lowercase().next().expect("char should have lowercase variant")
                } else {
                    c.to_uppercase().next().expect("char should have uppercase variant")
                }
            })
            .collect();
        assert_eq!(
            classify_keyword(&mixed2),
            Some(expected_class),
            "{} MiXeD",
            mixed2
        );
    }
}

// Helper function to capitalize first letter
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.as_str().to_lowercase().chars())
            .collect(),
    }
}
