use gql_parser::ast::{CatalogStatementKind, Statement};
use gql_parser::parse;

#[test]
fn create_procedure_parses() {
    let result = parse("CREATE PROCEDURE my_proc AS { RETURN 1 }");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);

    let Statement::Catalog(stmt) = &program.statements[0] else {
        panic!("expected catalog statement");
    };
    assert!(matches!(
        stmt.kind,
        CatalogStatementKind::CreateProcedure(_)
    ));
}

#[test]
fn create_or_replace_procedure_with_signature_parses() {
    let result = parse("CREATE OR REPLACE PROCEDURE my_proc(a, b) { RETURN 1 }");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn drop_procedure_parses() {
    let result = parse("DROP PROCEDURE IF EXISTS my_proc");
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let program = result.ast.expect("expected AST");
    assert_eq!(program.statements.len(), 1);

    let Statement::Catalog(stmt) = &program.statements[0] else {
        panic!("expected catalog statement");
    };
    assert!(matches!(stmt.kind, CatalogStatementKind::DropProcedure(_)));
}

#[test]
fn create_procedure_requires_body() {
    let result = parse("CREATE PROCEDURE my_proc");
    assert!(!result.diagnostics.is_empty(), "expected diagnostics");
}
