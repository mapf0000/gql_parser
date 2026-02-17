use gql_parser::parse;

fn main() {
    let source = "MATCH (n:Person) RETURN n; INVALID; FROM users";
    let result = parse(source);

    match &result.ast {
        Some(program) => {
            println!("Parsed {} statement(s)", program.statements.len());
        }
        None => {
            println!("No AST produced");
        }
    }

    if result.diagnostics.is_empty() {
        println!("No diagnostics");
    } else {
        println!("Diagnostics: {}", result.diagnostics.len());
        for diag in &result.diagnostics {
            println!("- {}", diag.message);
        }
    }
}
