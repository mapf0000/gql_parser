//! Behavior tests for graph pattern and path pattern parsing (Sprint 8).

use gql_parser::ast::Statement;
use gql_parser::ast::expression::{Expression, Literal};
use gql_parser::ast::query::{
    AbbreviatedEdgePattern, EdgeDirection, EdgePattern, ElementPattern, GraphPattern,
    GraphPatternQuantifier, LabelExpression, LinearQuery, MatchMode, MatchStatement,
    OptionalOperand, PathMode, PathPatternExpression, PathPatternPrefix, PathPrimary, PathSearch,
    PrimitiveQueryStatement, PrimitiveResultStatement, Query, SelectFromClause, ShortestPathSearch,
    SimplifiedPathPatternExpression,
};
use gql_parser::lexer::Lexer;
use gql_parser::lexer::token::TokenKind;
use gql_parser::parse;
use gql_parser::parser::patterns::{parse_graph_pattern, parse_graph_pattern_binding_table};
use crate::common::*;

fn parse_ambient_query(source: &str) -> gql_parser::ast::query::LinearQuery {
    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics for `{source}`: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1, "expected one statement");

    let Statement::Query(stmt) = &program.statements[0] else {
        panic!("expected query statement");
    };
    let Query::Linear(query) = &stmt.query else {
        panic!("expected linear query");
    };

    assert!(query.is_ambient(), "expected ambient query");
    query.clone()
}

fn parse_first_simple_match_pattern(source: &str) -> GraphPattern {
    let query = parse_ambient_query(source);
    let Some(PrimitiveQueryStatement::Match(MatchStatement::Simple(simple))) =
        query.primitive_statements.first()
    else {
        panic!("expected simple MATCH statement");
    };

    simple.pattern.clone()
}

fn first_factor_from_match(source: &str) -> gql_parser::ast::query::PathFactor {
    let pattern = parse_first_simple_match_pattern(source);
    let path = pattern
        .paths
        .patterns
        .first()
        .expect("expected at least one path pattern");

    let PathPatternExpression::Term(term) = &path.expression else {
        panic!("expected path expression to be a term");
    };

    term.factors
        .first()
        .cloned()
        .expect("expected at least one path factor")
}

fn expect_integer_literal(expr: &Expression, expected: &str) {
    let Expression::Literal(Literal::Integer(value), _) = expr else {
        panic!("expected integer literal expression");
    };
    assert_eq!(value.as_str(), expected);
}

fn extract_node_pattern(source: &str) -> gql_parser::ast::query::NodePattern {
    let factor = first_factor_from_match(source);
    let PathPrimary::ElementPattern(element) = factor.primary else {
        panic!("expected element pattern as first factor");
    };

    match element.as_ref() {
        ElementPattern::Node(node) => node.as_ref().clone(),
        ElementPattern::Edge(_) => panic!("expected node pattern, found edge"),
    }
}

fn extract_full_edge_direction(source: &str) -> EdgeDirection {
    let factor = first_factor_from_match(source);
    let PathPrimary::ElementPattern(element) = factor.primary else {
        panic!("expected element pattern as first factor");
    };

    match element.as_ref() {
        ElementPattern::Edge(EdgePattern::Full(full)) => full.direction,
        ElementPattern::Edge(EdgePattern::Abbreviated(_)) => {
            panic!("expected full edge pattern")
        }
        ElementPattern::Node(_) => panic!("expected edge pattern"),
    }
}

fn extract_abbreviated_edge(source: &str) -> AbbreviatedEdgePattern {
    let factor = first_factor_from_match(source);
    let PathPrimary::ElementPattern(element) = factor.primary else {
        panic!("expected element pattern as first factor");
    };

    match element.as_ref() {
        ElementPattern::Edge(EdgePattern::Abbreviated(abbrev)) => abbrev.clone(),
        ElementPattern::Edge(EdgePattern::Full(_)) => panic!("expected abbreviated edge pattern"),
        ElementPattern::Node(_) => panic!("expected edge pattern"),
    }
}

fn extract_simplified_inner(source: &str) -> (EdgeDirection, SimplifiedPathPatternExpression) {
    let factor = first_factor_from_match(source);
    let PathPrimary::SimplifiedExpression(expr) = factor.primary else {
        panic!("expected simplified expression as first factor");
    };

    let SimplifiedPathPatternExpression::DirectionOverride(override_expr) = expr.as_ref() else {
        panic!("expected top-level simplified direction override");
    };

    (
        override_expr.direction,
        override_expr.pattern.as_ref().clone(),
    )
}

#[test]
fn parse_match_query_contains_match_and_return() {
    let query = parse_ambient_query("MATCH (n) RETURN n");

    assert_eq!(query.primitive_statements.len(), 1);
    assert!(matches!(
        query.primitive_statements.first(),
        Some(PrimitiveQueryStatement::Match(MatchStatement::Simple(_)))
    ));
    assert!(matches!(
        query.result_statement.as_deref(),
        Some(PrimitiveResultStatement::Return(_))
    ));
}

#[test]
fn parse_match_mode_variants() {
    let repeatable = parse_first_simple_match_pattern("MATCH REPEATABLE ELEMENTS (n) RETURN n");
    assert_eq!(repeatable.match_mode, Some(MatchMode::RepeatableElements));

    let different = parse_first_simple_match_pattern("MATCH DIFFERENT EDGES (n) RETURN n");
    assert_eq!(different.match_mode, Some(MatchMode::DifferentEdges));
}

#[test]
fn parse_path_variable_declaration_and_pattern_list() {
    let pattern = parse_first_simple_match_pattern("MATCH p = (a), q = (b) RETURN p");
    assert_eq!(pattern.paths.patterns.len(), 2);

    assert_eq!(
        pattern.paths.patterns[0]
            .variable_declaration
            .as_ref()
            .map(|decl| decl.variable.as_str()),
        Some("p")
    );
    assert_eq!(
        pattern.paths.patterns[1]
            .variable_declaration
            .as_ref()
            .map(|decl| decl.variable.as_str()),
        Some("q")
    );
}

#[test]
fn parse_keep_where_and_yield_clauses() {
    let pattern = parse_first_simple_match_pattern(
        "MATCH p = WALK PATH (n) KEEP TRAIL WHERE n.age > 10 YIELD n AS out, n.age RETURN out",
    );

    assert!(matches!(
        pattern.paths.patterns[0].prefix,
        Some(PathPatternPrefix::PathMode(PathMode::Walk))
    ));
    assert!(matches!(
        pattern.keep_clause.as_ref().map(|keep| &keep.prefix),
        Some(PathPatternPrefix::PathMode(PathMode::Trail))
    ));
    assert!(pattern.where_clause.is_some());

    let yield_clause = pattern
        .yield_clause
        .as_ref()
        .expect("expected YIELD clause");
    assert_eq!(yield_clause.items.len(), 2);
    assert_eq!(yield_clause.items[0].alias.as_deref(), Some("out"));
    assert!(yield_clause.items[1].alias.is_none());
}

#[test]
fn parse_optional_match_operand_pattern() {
    let query = parse_ambient_query("OPTIONAL MATCH (n) RETURN n");

    let Some(PrimitiveQueryStatement::Match(MatchStatement::Optional(optional))) =
        query.primitive_statements.first()
    else {
        panic!("expected optional match statement");
    };

    let OptionalOperand::Match { pattern } = &optional.operand else {
        panic!("expected OPTIONAL MATCH operand");
    };
    assert!(!pattern.paths.patterns.is_empty());
}

#[test]
fn parse_path_mode_prefix_variants() {
    let cases = [
        ("MATCH WALK PATH (n) RETURN n", PathMode::Walk),
        ("MATCH TRAIL PATH (n) RETURN n", PathMode::Trail),
        ("MATCH SIMPLE PATH (n) RETURN n", PathMode::Simple),
        ("MATCH ACYCLIC PATH (n) RETURN n", PathMode::Acyclic),
    ];

    for (source, expected_mode) in cases {
        let pattern = parse_first_simple_match_pattern(source);
        let prefix = pattern.paths.patterns[0]
            .prefix
            .as_ref()
            .expect("expected prefix");

        match prefix {
            PathPatternPrefix::PathMode(mode) => assert_eq!(*mode, expected_mode),
            PathPatternPrefix::PathSearch(_) => panic!("expected path-mode prefix"),
        }
    }
}

#[test]
fn parse_path_search_prefix_variants() {
    let all = parse_first_simple_match_pattern("MATCH ALL PATHS (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::All(all_search))) =
        all.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected ALL path search");
    };
    assert_eq!(all_search.mode, None);
    assert!(all_search.use_paths_keyword);

    let any = parse_first_simple_match_pattern("MATCH ANY TRAIL PATH (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::Any(any_search))) =
        any.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected ANY path search");
    };
    assert_eq!(any_search.mode, Some(PathMode::Trail));

    let all_shortest =
        parse_first_simple_match_pattern("MATCH ALL SHORTEST SIMPLE PATH (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::Shortest(
        ShortestPathSearch::AllShortest { mode, .. },
    ))) = all_shortest.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected ALL SHORTEST search");
    };
    assert_eq!(*mode, Some(PathMode::Simple));

    let any_shortest =
        parse_first_simple_match_pattern("MATCH ANY SHORTEST ACYCLIC PATH (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::Shortest(
        ShortestPathSearch::AnyShortest { mode, .. },
    ))) = any_shortest.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected ANY SHORTEST search");
    };
    assert_eq!(*mode, Some(PathMode::Acyclic));

    let counted = parse_first_simple_match_pattern("MATCH SHORTEST 3 WALK PATHS (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::Shortest(
        ShortestPathSearch::CountedShortest {
            count,
            mode,
            use_paths_keyword,
            ..
        },
    ))) = counted.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected counted SHORTEST search");
    };
    expect_integer_literal(count, "3");
    assert_eq!(*mode, Some(PathMode::Walk));
    assert!(*use_paths_keyword);

    let groups = parse_first_simple_match_pattern("MATCH SHORTEST 2 GROUPS (n) RETURN n");
    let Some(PathPatternPrefix::PathSearch(PathSearch::Shortest(
        ShortestPathSearch::CountedShortestGroups { count, mode, .. },
    ))) = groups.paths.patterns[0].prefix.as_ref()
    else {
        panic!("expected counted SHORTEST GROUPS search");
    };
    expect_integer_literal(count, "2");
    assert_eq!(*mode, None);
}

#[test]
fn parse_path_expression_union_and_multiset_alternation() {
    let union = parse_first_simple_match_pattern("MATCH (a)|(b) RETURN a");
    assert!(matches!(
        union.paths.patterns[0].expression,
        PathPatternExpression::Union { .. }
    ));

    let alternation = parse_first_simple_match_pattern("MATCH (a)|+|(b)|+|(c) RETURN a");
    let PathPatternExpression::Alternation { alternatives, .. } =
        &alternation.paths.patterns[0].expression
    else {
        panic!("expected |+| alternation");
    };
    assert_eq!(alternatives.len(), 3);
}

#[test]
fn parse_path_expression_mixed_union_and_multiset_alternation_has_stable_precedence() {
    let mixed = parse_first_simple_match_pattern("MATCH (a)|(b)|+|(c)|(d) RETURN a");

    let PathPatternExpression::Alternation { alternatives, .. } =
        &mixed.paths.patterns[0].expression
    else {
        panic!("expected top-level alternation");
    };

    assert_eq!(alternatives.len(), 2);
    assert!(matches!(
        alternatives[0],
        PathPatternExpression::Union { .. }
    ));
    assert!(matches!(
        alternatives[1],
        PathPatternExpression::Union { .. }
    ));
}

#[test]
fn parse_node_pattern_with_property_predicate() {
    let node = extract_node_pattern("MATCH (n:Person {age: 42}) RETURN n");

    assert_eq!(
        node.variable.as_ref().map(|var| var.variable.as_str()),
        Some("n")
    );

    let Some(label_expr) = node.label_expression.as_ref() else {
        panic!("expected node label expression");
    };
    assert!(matches!(
        label_expr,
        LabelExpression::LabelName { name, .. } if name == "Person"
    ));

    let props = node
        .properties
        .as_ref()
        .expect("expected node property specification");
    assert_eq!(props.properties.len(), 1);
    assert_eq!(props.properties[0].key.as_str(), "age");

    assert!(node.where_clause.is_none());
}

#[test]
fn parse_node_pattern_with_where_predicate() {
    let node = extract_node_pattern("MATCH (n:Person WHERE n.age > 10) RETURN n");

    assert_eq!(
        node.variable.as_ref().map(|var| var.variable.as_str()),
        Some("n")
    );
    assert!(node.properties.is_none());
    assert!(node.where_clause.is_some());
}

#[test]
fn parse_node_pattern_rejects_mixed_property_and_where_predicates() {
    let result = parse("MATCH (n:Person {age: 42} WHERE n.age > 10) RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text
            .contains("Element pattern can have either property specification or WHERE predicate"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parse_label_expression_operations_and_label_set_phrase() {
    let complex = extract_node_pattern("MATCH (n:!(Person|Company)&%) RETURN n");
    let Some(label_expr) = complex.label_expression.as_ref() else {
        panic!("expected complex label expression");
    };

    let LabelExpression::Conjunction { left, right, .. } = label_expr else {
        panic!("expected conjunction at label-expression root");
    };
    assert!(matches!(right.as_ref(), LabelExpression::Wildcard { .. }));
    assert!(matches!(left.as_ref(), LabelExpression::Negation { .. }));

    let label_set = extract_node_pattern("MATCH (n:LABELS Person&Employee) RETURN n");
    let Some(label_expr) = label_set.label_expression.as_ref() else {
        panic!("expected label-set expression");
    };
    assert!(matches!(label_expr, LabelExpression::Conjunction { .. }));
}

#[test]
fn parse_full_edge_pattern_direction_variants() {
    let cases = [
        ("MATCH <-[e]- RETURN 1", EdgeDirection::PointingLeft),
        ("MATCH -[e]-> RETURN 1", EdgeDirection::PointingRight),
        ("MATCH ~[e]~ RETURN 1", EdgeDirection::Undirected),
        ("MATCH <-[e]-> RETURN 1", EdgeDirection::AnyDirected),
        ("MATCH <~[e]~ RETURN 1", EdgeDirection::LeftOrUndirected),
        ("MATCH -[e]- RETURN 1", EdgeDirection::AnyDirection),
        ("MATCH ~[e]~> RETURN 1", EdgeDirection::RightOrUndirected),
    ];

    for (source, expected) in cases {
        let direction = extract_full_edge_direction(source);
        assert_eq!(direction, expected, "unexpected direction for `{source}`");
    }
}

#[test]
fn parse_abbreviated_edge_pattern_variants() {
    let left = extract_abbreviated_edge("MATCH <- RETURN 1");
    assert!(matches!(left, AbbreviatedEdgePattern::LeftArrow { .. }));

    let right = extract_abbreviated_edge("MATCH -> RETURN 1");
    assert!(matches!(right, AbbreviatedEdgePattern::RightArrow { .. }));

    let undirected = extract_abbreviated_edge("MATCH ~ RETURN 1");
    assert!(matches!(
        undirected,
        AbbreviatedEdgePattern::Undirected { .. }
    ));

    let any = extract_abbreviated_edge("MATCH - RETURN 1");
    assert!(matches!(any, AbbreviatedEdgePattern::AnyDirection { .. }));
}

#[test]
fn parse_quantifier_variants() {
    let star = first_factor_from_match("MATCH (n)* RETURN n");
    assert!(matches!(
        star.quantifier,
        Some(GraphPatternQuantifier::Star { .. })
    ));

    let plus = first_factor_from_match("MATCH (n)+ RETURN n");
    assert!(matches!(
        plus.quantifier,
        Some(GraphPatternQuantifier::Plus { .. })
    ));

    let question = first_factor_from_match("MATCH (n)? RETURN n");
    assert!(matches!(
        question.quantifier,
        Some(GraphPatternQuantifier::QuestionMark { .. })
    ));

    let fixed = first_factor_from_match("MATCH (n){3} RETURN n");
    assert!(matches!(
        fixed.quantifier,
        Some(GraphPatternQuantifier::Fixed { count: 3, .. })
    ));

    let bounded = first_factor_from_match("MATCH (n){2,5} RETURN n");
    assert!(matches!(
        bounded.quantifier,
        Some(GraphPatternQuantifier::General {
            min: Some(2),
            max: Some(5),
            ..
        })
    ));

    let open_upper = first_factor_from_match("MATCH (n){2,} RETURN n");
    assert!(matches!(
        open_upper.quantifier,
        Some(GraphPatternQuantifier::General {
            min: Some(2),
            max: None,
            ..
        })
    ));

    let open_lower = first_factor_from_match("MATCH (n){,5} RETURN n");
    assert!(matches!(
        open_lower.quantifier,
        Some(GraphPatternQuantifier::General {
            min: None,
            max: Some(5),
            ..
        })
    ));
}

#[test]
fn parse_simplified_path_variants() {
    let (direction, inner) = extract_simplified_inner("MATCH -/a/- RETURN 1");
    assert_eq!(direction, EdgeDirection::AnyDirection);
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Contents(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/a|b/- RETURN 1");
    assert!(matches!(inner, SimplifiedPathPatternExpression::Union(..)));

    let (_, inner) = extract_simplified_inner("MATCH -/a|+|b/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::MultisetAlternation(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/a&b/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Conjunction(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/a b/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Concatenation(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/a?/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Questioned(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/a*/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Quantified(..)
    ));

    let (_, inner) = extract_simplified_inner("MATCH -/!a/- RETURN 1");
    assert!(matches!(
        inner,
        SimplifiedPathPatternExpression::Negation(..)
    ));
}

#[test]
fn parse_simplified_mixed_union_and_multiset_alternation_has_stable_precedence() {
    let (_, inner) = extract_simplified_inner("MATCH -/a|b|+|c|d/- RETURN 1");
    let SimplifiedPathPatternExpression::MultisetAlternation(alternation) = inner else {
        panic!("expected top-level simplified multiset alternation");
    };

    assert_eq!(alternation.alternatives.len(), 2);
    assert!(matches!(
        alternation.alternatives[0],
        SimplifiedPathPatternExpression::Union(..)
    ));
    assert!(matches!(
        alternation.alternatives[1],
        SimplifiedPathPatternExpression::Union(..)
    ));
}

#[test]
fn parse_deeply_nested_quantifiers_on_parenthesized_patterns() {
    let factor = first_factor_from_match("MATCH (((n){1,2}){2,3}){3,4} RETURN n");
    assert!(matches!(
        factor.quantifier,
        Some(GraphPatternQuantifier::General {
            min: Some(3),
            max: Some(4),
            ..
        })
    ));
}

#[test]
fn parse_reports_single_chained_quantifier_diagnostic() {
    let result = parse("MATCH (n)?? RETURN n");
    assert!(result.ast.is_some(), "expected recoverable AST");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text.contains("Chained path quantifiers are not allowed"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parse_select_from_match_list_contains_patterns() {
    let result = parse("SELECT * FROM MATCH (n) RETURN n");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    let Statement::Query(stmt) = &program.statements[0] else {
        panic!("expected query statement");
    };
    let Query::Linear(linear_query) = &stmt.query else {
        panic!("expected linear query");
    };
    if linear_query.use_graph.is_some() {
        panic!("expected ambient linear query");
    }
    let Some(PrimitiveQueryStatement::Select(select)) = linear_query.primitive_statements.first()
    else {
        panic!("expected SELECT primitive statement");
    };
    let Some(SelectFromClause::GraphMatchList { matches }) = &select.from_clause else {
        panic!("expected FROM graph match list");
    };

    assert_eq!(matches.len(), 1);
    assert!(
        matches
            .iter()
            .all(|pattern| !pattern.paths.patterns.is_empty())
    );
}

#[test]
fn parse_select_from_missing_payload_reports_diagnostic() {
    let result = parse("SELECT * FROM");
    assert!(
        result.ast.is_some(),
        "SELECT should still produce a recoverable AST"
    );
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn parse_graph_pattern_binding_table_captures_yield_clause() {
    let lexer_result = Lexer::new("(n) YIELD n AS out RETURN out").tokenize();
    let mut pos = 0;
    let (table_opt, diags) = parse_graph_pattern_binding_table(&lexer_result.tokens, &mut pos);

    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let table = table_opt.expect("expected graph pattern binding table");

    assert!(matches!(lexer_result.tokens[pos].kind, TokenKind::Return));

    let yield_clause = table
        .yield_clause
        .as_ref()
        .expect("expected binding-table yield clause");
    assert_eq!(yield_clause.items.len(), 1);
    assert_eq!(yield_clause.items[0].alias.as_deref(), Some("out"));

    assert!(table.pattern.yield_clause.is_some());
}

#[test]
fn parse_graph_pattern_consumes_until_return_boundary() {
    let lexer_result = Lexer::new("(n) RETURN n").tokenize();
    let mut pos = 0;
    let (pattern_opt, diags) = parse_graph_pattern(&lexer_result.tokens, &mut pos);

    assert!(pattern_opt.is_some());
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert!(pos > 0);
    assert!(matches!(lexer_result.tokens[pos].kind, TokenKind::Return));
}

#[test]
fn parse_graph_pattern_reports_missing_keep_prefix() {
    let lexer_result = Lexer::new("(n) KEEP RETURN n").tokenize();
    let mut pos = 0;
    let (pattern_opt, diags) = parse_graph_pattern(&lexer_result.tokens, &mut pos);

    assert!(pattern_opt.is_some());
    assert!(!diags.is_empty());
    assert!(matches!(lexer_result.tokens[pos].kind, TokenKind::Return));
}

#[test]
fn parse_graph_pattern_rejects_empty_pattern() {
    let lexer_result = Lexer::new("RETURN n").tokenize();
    let mut pos = 0;
    let (pattern_opt, diags) = parse_graph_pattern(&lexer_result.tokens, &mut pos);

    assert!(pattern_opt.is_none());
    assert!(!diags.is_empty());
}

#[test]
fn parse_path_variable_accepts_non_reserved_keyword_identifier() {
    let pattern = parse_first_simple_match_pattern("MATCH GRAPH = (n) RETURN GRAPH");
    let variable = pattern.paths.patterns[0]
        .variable_declaration
        .as_ref()
        .map(|decl| decl.variable.as_str());
    assert_eq!(variable, Some("GRAPH"));
}

#[test]
fn parse_path_variable_rejects_delimited_identifier() {
    let result = parse("MATCH `p` = (n) RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text.contains("Path variable declaration requires a regular identifier"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parse_element_variable_rejects_delimited_identifier() {
    let result = parse("MATCH (`n`:Person) RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text.contains("Element variable declaration requires a regular identifier"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parse_property_map_rejects_reserved_unquoted_property_name() {
    let result = parse("MATCH (n {RETURN: 1}) RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text.contains("Expected property name in property specification"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parser_returns_none_ast_for_fatal_single_token_input() {
    let result = parse("x");
    assert!(result.ast.is_none());
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn parser_recovers_after_invalid_token_and_keeps_query_statement() {
    let result = parse("x MATCH (n) RETURN n");
    assert!(result.ast.is_some());
    assert!(!result.diagnostics.is_empty());

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Query(_)));
}
