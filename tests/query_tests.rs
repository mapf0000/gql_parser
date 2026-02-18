use gql_parser::parse;

fn diagnostics_text(diags: &[miette::Report]) -> String {
    diags
        .iter()
        .map(|diag| format!("{diag:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_ok(source: &str) {
    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics for `{source}`: {:?}",
        result.diagnostics
    );
}

#[test]
fn legacy_use_graph_syntax_is_rejected() {
    let result = parse("USE GRAPH g MATCH (n) RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = diagnostics_text(&result.diagnostics);
    assert!(
        diag_text.contains("Expected graph expression after USE")
            || diag_text.contains("unexpected trailing tokens after expression")
            || diag_text.contains("Expected query statement after USE clause")
            || diag_text.contains("Expected data-accessing statement after USE clause"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn use_clause_requires_graph_expression() {
    let result = parse("USE");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = diagnostics_text(&result.diagnostics);
    assert!(
        diag_text.contains("Expected query statement after USE clause")
            || diag_text.contains("Expected data-accessing statement after USE clause")
            || diag_text.contains("Expected graph expression after USE"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn return_alias_requires_identifier_after_as() {
    let result = parse("MATCH (n) RETURN n AS");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = diagnostics_text(&result.diagnostics);
    assert!(
        diag_text.contains("Expected alias after AS in RETURN item"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn select_alias_requires_identifier_after_as() {
    let result = parse("SELECT 1 AS");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = diagnostics_text(&result.diagnostics);
    assert!(
        diag_text.contains("Expected alias after AS in SELECT item"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn let_accepts_non_reserved_keyword_identifier() {
    parse_ok("LET GRAPH = 1 RETURN GRAPH");
}

#[test]
fn for_accepts_non_reserved_keyword_identifier() {
    parse_ok("FOR GRAPH IN items RETURN GRAPH");
}

#[test]
fn for_ordinality_accepts_non_reserved_keyword_identifier() {
    parse_ok("FOR n IN items WITH ORDINALITY GRAPH RETURN GRAPH");
}
