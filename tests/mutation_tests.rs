use gql_parser::ast::{
    DetachOption, LinearDataModifyingStatement, PrimitiveDataModifyingStatement,
    PrimitiveResultStatement, SimpleDataAccessingStatement, SimpleDataModifyingStatement,
    Statement,
};
use gql_parser::parse;

fn parse_ok(source: &str) -> gql_parser::ast::Program {
    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics for `{source}`: {:?}",
        result.diagnostics
    );
    result.ast.expect("expected AST")
}

fn diagnostics_text(diags: &[miette::Report]) -> String {
    diags
        .iter()
        .map(|diag| format!("{diag:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn parse_set_statement_is_mutation_start() {
    let program = parse_ok("SET n.age = 1");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_remove_statement_is_mutation_start() {
    let program = parse_ok("REMOVE n:OldLabel");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_detach_delete_statement_is_mutation_start() {
    let program = parse_ok("DETACH DELETE n");
    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };

    let LinearDataModifyingStatement::Ambient(ambient) = &stmt.statement else {
        panic!("expected ambient linear mutation");
    };
    let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Primitive(
        PrimitiveDataModifyingStatement::Delete(delete_stmt),
    ))) = ambient.statements.first()
    else {
        panic!("expected DELETE primitive");
    };

    assert_eq!(delete_stmt.detach_option, DetachOption::Detach);
}

#[test]
fn parse_focused_use_graph_mutation() {
    let program = parse_ok("USE myGraph INSERT (n) RETURN n");

    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };

    let LinearDataModifyingStatement::Focused(focused) = &stmt.statement else {
        panic!("expected focused mutation statement");
    };

    assert_eq!(focused.statements.len(), 1);
    assert!(matches!(
        focused.primitive_result_statement,
        Some(PrimitiveResultStatement::Return(_))
    ));
}

#[test]
fn parse_mutation_chain_with_query_step_stays_single_statement() {
    let program = parse_ok("INSERT (n) MATCH (n) RETURN n");
    assert_eq!(program.statements.len(), 1);

    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };
    let LinearDataModifyingStatement::Ambient(ambient) = &stmt.statement else {
        panic!("expected ambient mutation statement");
    };

    assert_eq!(ambient.statements.len(), 2);
    assert!(matches!(
        ambient.statements[0],
        SimpleDataAccessingStatement::Modifying(_)
    ));
    assert!(matches!(
        ambient.statements[1],
        SimpleDataAccessingStatement::Query(_)
    ));
}

#[test]
fn parse_mutation_and_catalog_without_semicolon_split_correctly() {
    let program = parse_ok("SET n.age = 1 CREATE SCHEMA /foo");
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
    assert!(matches!(program.statements[1], Statement::Catalog(_)));
}

#[test]
fn parse_insert_with_optional_fillers() {
    let program = parse_ok("INSERT ()-[]->() , ()~[]~()");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn insert_rejects_empty_property_map_in_insert_filler() {
    let result = parse("INSERT (n {})");
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn set_all_properties_accepts_empty_map() {
    let result = parse("SET n = {}");
    assert!(result.diagnostics.is_empty(), "unexpected diagnostics");
}

#[test]
fn parse_inline_call_statement_is_mutation_start() {
    let program = parse_ok("INSERT (n) CALL { RETURN n }");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_inline_call_with_scope_is_mutation_start() {
    let program = parse_ok("INSERT (n) CALL (n, m) { RETURN n }");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn inline_call_reports_unclosed_procedure_specification() {
    let result = parse("INSERT (n) CALL { RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = diagnostics_text(&result.diagnostics);
    assert!(
        diag_text.contains("Expected closing brace in nested procedure specification")
            || diag_text.contains("expected '}'"),
        "unexpected diagnostics: {diag_text}"
    );
}
