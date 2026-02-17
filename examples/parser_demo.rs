use gql_parser::parse;

fn main() {
    let cases = [
        DemoCase {
            name: "basic_query",
            source: "MATCH (n:Person) RETURN n",
            expect_ast: true,
            expected_diags: 0,
        },
        DemoCase {
            name: "session_schema",
            source: "SESSION SET SCHEMA myschema",
            expect_ast: true,
            expected_diags: 0,
        },
        DemoCase {
            name: "transaction_block",
            source: "START TRANSACTION; COMMIT; ROLLBACK",
            expect_ast: true,
            expected_diags: 0,
        },
        DemoCase {
            name: "catalog_block",
            source: "CREATE SCHEMA myschema; DROP GRAPH mygraph",
            expect_ast: true,
            expected_diags: 0,
        },
        DemoCase {
            name: "mixed_statements",
            source: "SESSION SET SCHEMA myschema; START TRANSACTION; MATCH (n) RETURN n; COMMIT; SESSION CLOSE",
            expect_ast: true,
            expected_diags: 0,
        },
        DemoCase {
            name: "recoverable_invalid_prefix",
            source: "x MATCH (n) RETURN n",
            expect_ast: true,
            expected_diags: 1,
        },
        DemoCase {
            name: "fatal_invalid_statement",
            source: "x",
            expect_ast: false,
            expected_diags: 1,
        },
    ];

    let mut failures = 0usize;
    for case in cases {
        if !test_parse(case) {
            failures += 1;
        }
    }

    if failures > 0 {
        panic!("parser_demo detected {failures} failing case(s)");
    }
}

struct DemoCase {
    name: &'static str,
    source: &'static str,
    expect_ast: bool,
    expected_diags: usize,
}

fn test_parse(case: DemoCase) -> bool {
    println!("\n=== {} ===", case.name);
    println!("source: {}", case.source);
    let result = parse(case.source);

    match result.ast.as_ref() {
        Some(program) => {
            println!("✓ Parsed {} statement(s)", program.statements.len());
            for (i, stmt) in program.statements.iter().enumerate() {
                let stmt_type = match stmt {
                    gql_parser::ast::Statement::Query(_) => "Query",
                    gql_parser::ast::Statement::Mutation(_) => "Mutation",
                    gql_parser::ast::Statement::Session(_) => "Session",
                    gql_parser::ast::Statement::Transaction(_) => "Transaction",
                    gql_parser::ast::Statement::Catalog(_) => "Catalog",
                    gql_parser::ast::Statement::Empty(_) => "Empty",
                };
                println!("  Statement {}: {}", i + 1, stmt_type);
            }
        }
        None => {
            println!("(no AST produced)");
        }
    }

    println!("diagnostics: {}", result.diagnostics.len());
    for diag in &result.diagnostics {
        println!("  - {}", diag);
    }

    let ast_ok = result.ast.is_some() == case.expect_ast;
    let diag_ok = result.diagnostics.len() == case.expected_diags;
    let passed = ast_ok && diag_ok;

    if passed {
        println!("PASS");
    } else {
        println!(
            "FAIL (expected ast={}, diags={}; got ast={}, diags={})",
            case.expect_ast,
            case.expected_diags,
            result.ast.is_some(),
            result.diagnostics.len()
        );
    }

    passed
}
