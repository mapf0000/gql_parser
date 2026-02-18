use gql_parser::parse;

fn diagnostics_text(diags: &[miette::Report]) -> String {
    diags
        .iter()
        .map(|diag| format!("{diag:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn top_level_call_with_yield_parses() {
    let result = parse("CALL my_proc() YIELD x");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn top_level_optional_call_parses_without_optional_match_errors() {
    let result = parse("OPTIONAL CALL my_proc()");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn query_pipeline_call_parses() {
    let result = parse("MATCH (n) CALL my_proc(n) RETURN n");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn named_call_requires_parentheses() {
    let result = parse("CALL my_proc");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");
}

#[test]
fn inline_procedure_body_requires_statement_block() {
    let result = parse("CALL () { }");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");
}

#[test]
fn variable_definition_requires_initializer() {
    let result = parse("CALL () { VALUE x RETURN x }");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");
}

#[test]
fn yield_disallows_arbitrary_expression_items() {
    let result = parse("CALL my_proc() YIELD 1 + 2");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");
    let text = diagnostics_text(&result.diagnostics);
    assert!(
        text.contains("yield") || text.contains("Expected"),
        "unexpected diagnostics: {text}"
    );
}

#[test]
fn at_schema_clause_stops_before_call_statement() {
    let result = parse("CALL () { AT my_schema CALL my_proc() }");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn at_schema_clause_stops_before_variable_definitions() {
    let result = parse("CALL () { AT my_schema VALUE x = 1 RETURN x }");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}
