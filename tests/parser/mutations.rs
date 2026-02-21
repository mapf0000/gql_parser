use crate::common::*;
use gql_parser::ast::{
    DetachOption, PrimitiveDataModifyingStatement, PrimitiveResultStatement,
    SimpleDataAccessingStatement, SimpleDataModifyingStatement, Statement,
};
use gql_parser::parse;

#[test]
fn parse_set_statement_is_mutation_start() {
    let program = parse_cleanly("SET n.age = 1");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_remove_statement_is_mutation_start() {
    let program = parse_cleanly("REMOVE n:OldLabel");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_detach_delete_statement_is_mutation_start() {
    let program = parse_cleanly("DETACH DELETE n");
    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };

    assert!(
        stmt.statement.is_ambient(),
        "expected ambient linear mutation"
    );

    let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Primitive(
        PrimitiveDataModifyingStatement::Delete(delete_stmt),
    ))) = stmt.statement.statements.first()
    else {
        panic!("expected DELETE primitive");
    };

    assert_eq!(delete_stmt.detach_option, DetachOption::Detach);
}

#[test]
fn parse_focused_use_graph_mutation() {
    let program = parse_cleanly("USE myGraph INSERT (n) RETURN n");

    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };

    assert!(
        stmt.statement.is_focused(),
        "expected focused mutation statement"
    );

    assert_eq!(stmt.statement.statements.len(), 1);
    assert!(matches!(
        stmt.statement.primitive_result_statement,
        Some(PrimitiveResultStatement::Return(_))
    ));
}

#[test]
fn parse_mutation_chain_with_query_step_stays_single_statement() {
    let program = parse_cleanly("INSERT (n) MATCH (n) RETURN n");
    assert_eq!(program.statements.len(), 1);

    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!("expected mutation statement");
    };
    assert!(
        stmt.statement.is_ambient(),
        "expected ambient mutation statement"
    );

    assert_eq!(stmt.statement.statements.len(), 2);
    assert!(matches!(
        stmt.statement.statements[0],
        SimpleDataAccessingStatement::Modifying(_)
    ));
    assert!(matches!(
        stmt.statement.statements[1],
        SimpleDataAccessingStatement::Query(_)
    ));
}

#[test]
fn parse_mutation_and_catalog_without_semicolon_split_correctly() {
    let program = parse_cleanly("SET n.age = 1 CREATE SCHEMA /foo");
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
    assert!(matches!(program.statements[1], Statement::Catalog(_)));
}

#[test]
fn parse_insert_with_optional_fillers() {
    let program = parse_cleanly("INSERT ()-[]->() , ()~[]~()");
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
    let program = parse_cleanly("INSERT (n) CALL { RETURN n }");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_inline_call_with_scope_is_mutation_start() {
    let program = parse_cleanly("INSERT (n) CALL (n, m) { RETURN n }");
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn inline_call_reports_unclosed_procedure_specification() {
    let result = parse("INSERT (n) CALL { RETURN n");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");

    let diag_text = format_diagnostics(&result.diagnostics);
    assert!(
        diag_text.contains("Expected closing brace in nested procedure specification")
            || diag_text.contains("expected '}'"),
        "unexpected diagnostics: {diag_text}"
    );
}

#[test]
fn parse_match_followed_by_set_is_single_mutation_statement() {
    // MATCH followed by SET/DELETE/REMOVE should be parsed as a single mutation statement,
    // not as separate Query and Mutation statements
    let program = parse_cleanly("MATCH (n) SET n.age = 30");

    // Should be parsed as a single mutation statement
    assert_eq!(
        program.statements.len(),
        1,
        "MATCH...SET should be parsed as one statement, got {} statements",
        program.statements.len()
    );

    let Statement::Mutation(stmt) = &program.statements[0] else {
        panic!(
            "expected mutation statement, got {:?}",
            program.statements[0]
        );
    };

    assert!(
        stmt.statement.is_ambient(),
        "expected ambient mutation statement"
    );

    // Should have MATCH as query statement and SET as modifying statement
    assert_eq!(
        stmt.statement.statements.len(),
        2,
        "expected MATCH and SET as two sub-statements"
    );
    assert!(
        matches!(
            stmt.statement.statements[0],
            SimpleDataAccessingStatement::Query(_)
        ),
        "first sub-statement should be Query (MATCH)"
    );
    assert!(
        matches!(
            stmt.statement.statements[1],
            SimpleDataAccessingStatement::Modifying(_)
        ),
        "second sub-statement should be Modifying (SET)"
    );
}

#[test]
fn parse_match_followed_by_delete_is_single_mutation_statement() {
    let program = parse_cleanly("MATCH (n) DELETE n");

    assert_eq!(
        program.statements.len(),
        1,
        "MATCH...DELETE should be parsed as one statement"
    );
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}

#[test]
fn parse_match_followed_by_remove_is_single_mutation_statement() {
    let program = parse_cleanly("MATCH (n) REMOVE n:Label");

    assert_eq!(
        program.statements.len(),
        1,
        "MATCH...REMOVE should be parsed as one statement"
    );
    assert!(matches!(program.statements[0], Statement::Mutation(_)));
}
