//! Demo of Sprint 11 Procedure Parsing Capabilities
//!
//! This example demonstrates the procedural composition features implemented
//! in Sprint 11, including CALL statements, variable definitions, and YIELD clauses.

use gql_parser::ast::ProcedureCall;
use gql_parser::lexer::Lexer;
use gql_parser::parser::procedure::*;

fn main() {
    println!("=== Sprint 11: Procedures, Nested Specs, and Execution Flow ===\n");

    // Example 1: Simple named procedure call
    demo_simple_call();

    // Example 2: OPTIONAL procedure call
    demo_optional_call();

    // Example 3: Procedure call with arguments and YIELD
    demo_call_with_yield();

    // Example 4: Inline procedure call with variable scope
    demo_inline_call();

    // Example 5: Variable definitions
    demo_variable_definitions();

    // Example 6: AT schema clause
    demo_at_schema();

    println!("\n=== All Sprint 11 features demonstrated successfully! ===");
}

fn demo_simple_call() {
    println!("--- Example 1: Simple Named Procedure Call ---");
    let source = "CALL my_procedure()";
    println!("Source: {}", source);

    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    match parse_call_procedure_statement(&tokens, &mut pos) {
        (Some(stmt), diags) if diags.is_empty() => {
            println!("✓ Parsed successfully");
            println!("  Optional: {}", stmt.optional);
            if let ProcedureCall::Named(named) = &stmt.call {
                println!("  Procedure: {:?}", named.procedure);
                println!("  Has arguments: {}", named.arguments.is_some());
                println!("  Has yield: {}", named.yield_clause.is_some());
            }
        }
        (_, diags) => {
            println!("✗ Parse failed with {} diagnostics", diags.len());
        }
    }
    println!();
}

fn demo_optional_call() {
    println!("--- Example 2: OPTIONAL Procedure Call ---");
    let source = "OPTIONAL CALL risky_operation()";
    println!("Source: {}", source);

    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    match parse_call_procedure_statement(&tokens, &mut pos) {
        (Some(stmt), diags) if diags.is_empty() => {
            println!("✓ Parsed successfully");
            println!("  Optional: {} (continues on failure)", stmt.optional);
        }
        (_, diags) => {
            println!("✗ Parse failed with {} diagnostics", diags.len());
        }
    }
    println!();
}

fn demo_call_with_yield() {
    println!("--- Example 3: Procedure Call with Arguments and YIELD ---");
    let source = "CALL process_data(input1, input2) YIELD result AS output, count";
    println!("Source: {}", source);

    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    match parse_call_procedure_statement(&tokens, &mut pos) {
        (Some(stmt), _diags) => {
            println!("✓ Parsed successfully");
            if let ProcedureCall::Named(named) = &stmt.call {
                if let Some(args) = &named.arguments {
                    println!("  Arguments: {} items", args.arguments.len());
                }
                if let Some(yield_clause) = &named.yield_clause {
                    println!("  Yield items: {}", yield_clause.items.items.len());
                    for (i, item) in yield_clause.items.items.iter().enumerate() {
                        if let Some(alias) = &item.alias {
                            println!("    Item {}: with alias '{}'", i + 1, alias.name);
                        } else {
                            println!("    Item {}: no alias", i + 1);
                        }
                    }
                }
            }
        }
        (_, diags) => {
            println!("✗ Parse failed with {} diagnostics", diags.len());
        }
    }
    println!();
}

fn demo_inline_call() {
    println!("--- Example 4: Inline Procedure Call with Variable Scope ---");
    let source = "CALL (x, y) { RETURN x }";
    println!("Source: {}", source);

    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    match parse_call_procedure_statement(&tokens, &mut pos) {
        (Some(stmt), _diags) => {
            println!("✓ Parsed successfully");
            if let ProcedureCall::Inline(inline) = &stmt.call
                && let Some(scope) = &inline.variable_scope
            {
                println!("  Variable scope: {} variables", scope.variables.len());
                for var in &scope.variables {
                    println!("    - {}", var.name);
                }
            }
        }
        (_, diags) => {
            println!("✗ Parse failed with {} diagnostics", diags.len());
        }
    }
    println!();
}

fn demo_variable_definitions() {
    println!("--- Example 5: Variable Definitions ---");

    // Graph variable
    let source1 = "PROPERTY GRAPH my_graph";
    println!("Source: {}", source1);
    let tokens = Lexer::new(source1).tokenize().tokens;
    let mut pos = 0;
    match parse_graph_variable_definition(&tokens, &mut pos) {
        (Some(def), diags) if diags.is_empty() => {
            println!("✓ Graph variable parsed");
            println!("  Is property: {}", def.is_property);
            println!("  Variable: {}", def.variable.name);
        }
        _ => println!("✗ Parse failed"),
    }

    // Value variable
    let source2 = "VALUE counter = 42";
    println!("\nSource: {}", source2);
    let tokens = Lexer::new(source2).tokenize().tokens;
    let mut pos = 0;
    match parse_value_variable_definition(&tokens, &mut pos) {
        (Some(def), _diags) => {
            println!("✓ Value variable parsed");
            println!("  Variable: {}", def.variable.name);
            println!("  Has initializer: {}", def.initializer.is_some());
        }
        _ => println!("✗ Parse failed"),
    }

    // Binding table variable
    let source3 = "BINDING TABLE results";
    println!("\nSource: {}", source3);
    let tokens = Lexer::new(source3).tokenize().tokens;
    let mut pos = 0;
    match parse_binding_table_variable_definition(&tokens, &mut pos) {
        (Some(def), diags) if diags.is_empty() => {
            println!("✓ Binding table variable parsed");
            println!("  Is binding: {}", def.is_binding);
            println!("  Variable: {}", def.variable.name);
        }
        _ => println!("✗ Parse failed"),
    }
    println!();
}

fn demo_at_schema() {
    println!("--- Example 6: AT Schema Clause ---");
    let source = "AT my_schema";
    println!("Source: {}", source);

    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    match parse_at_schema_clause(&tokens, &mut pos) {
        (Some(_clause), _diags) => {
            println!("✓ AT schema clause parsed successfully");
        }
        (_, diags) => {
            println!("✗ Parse failed with {} diagnostics", diags.len());
        }
    }
    println!();
}
