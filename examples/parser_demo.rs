use gql_parser::parse;

fn main() {
    // Test basic parsing
    test_parse("MATCH (n:Person) RETURN n; INVALID; FROM users");

    // Test Sprint 4: Session statements
    test_parse("SESSION SET SCHEMA myschema");

    // Test Sprint 4: Transaction statements
    test_parse("START TRANSACTION; COMMIT; ROLLBACK");

    // Test Sprint 4: Catalog statements
    test_parse("CREATE SCHEMA myschema; DROP GRAPH mygraph");

    // Test mixed Sprint 4 features
    test_parse(
        "SESSION SET SCHEMA myschema; START TRANSACTION; MATCH (n) RETURN n; COMMIT; SESSION CLOSE",
    );
}

fn test_parse(source: &str) {
    println!("\n=== Testing: {} ===", source);
    let result = parse(source);

    match &result.ast {
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
            println!("✗ No AST produced");
        }
    }

    if result.diagnostics.is_empty() {
        println!("✓ No diagnostics");
    } else {
        println!("⚠ Diagnostics: {}", result.diagnostics.len());
        for diag in &result.diagnostics {
            println!("  - {}", diag);
        }
    }
}
