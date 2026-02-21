//! Integration tests for procedure parsing.
//!
//! This test file validates Sprint 11 implementation:
//! - CALL procedure statements
//! - Inline and named procedure calls
//! - Variable scoping
//! - YIELD clauses
//! - Variable definitions
//! - AT schema clauses

use gql_parser::ast::ProcedureCall;
use gql_parser::lexer::Lexer;
use gql_parser::parser::base::TokenStream;
use gql_parser::parser::procedure::*;

#[test]
fn test_simple_named_procedure_call() {
    let source = "CALL my_procedure()";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(stmt_opt.is_some(), "Failed to parse simple procedure call");
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let stmt = stmt_opt.unwrap();
    assert!(!stmt.optional);
    if let ProcedureCall::Named(named) = &stmt.call {
        // Arguments may be Some with empty list or None
        if let Some(args) = &named.arguments {
            assert_eq!(args.arguments.len(), 0, "Expected empty arguments");
        }
        assert!(named.yield_clause.is_none());
    } else {
        panic!("Expected named procedure call");
    }
}

#[test]
fn test_procedure_call_with_arguments() {
    let source = "CALL my_procedure(1, 2, 3)";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, _diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse procedure call with arguments"
    );

    let stmt = stmt_opt.unwrap();
    if let ProcedureCall::Named(named) = &stmt.call {
        assert!(named.arguments.is_some());
        let args = named.arguments.as_ref().unwrap();
        assert_eq!(args.arguments.len(), 3);
    } else {
        panic!("Expected named procedure call");
    }
}

#[test]
fn test_procedure_call_with_yield() {
    let source = "CALL my_procedure() YIELD result1, result2";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse procedure call with yield"
    );
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let stmt = stmt_opt.unwrap();
    if let ProcedureCall::Named(named) = &stmt.call {
        assert!(named.yield_clause.is_some());
        let yield_clause = named.yield_clause.as_ref().unwrap();
        assert_eq!(yield_clause.items.items.len(), 2);
    } else {
        panic!("Expected named procedure call");
    }
}

#[test]
fn test_procedure_call_with_yield_aliases() {
    let source = "CALL my_procedure() YIELD result1 AS r1, result2 AS r2";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse procedure call with yield aliases"
    );
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let stmt = stmt_opt.unwrap();
    if let ProcedureCall::Named(named) = &stmt.call {
        assert!(named.yield_clause.is_some());
        let yield_clause = named.yield_clause.as_ref().unwrap();
        assert_eq!(yield_clause.items.items.len(), 2);
        assert!(yield_clause.items.items[0].alias.is_some());
        assert!(yield_clause.items.items[1].alias.is_some());
    } else {
        panic!("Expected named procedure call");
    }
}

#[test]
fn test_optional_procedure_call() {
    let source = "OPTIONAL CALL risky_procedure()";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse optional procedure call"
    );
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let stmt = stmt_opt.unwrap();
    assert!(stmt.optional, "Expected OPTIONAL flag to be set");
}

#[test]
fn test_inline_procedure_call_with_empty_scope() {
    let source = "CALL () { RETURN x }";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, _diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse inline procedure call with empty scope"
    );

    let stmt = stmt_opt.unwrap();
    if let ProcedureCall::Inline(inline) = &stmt.call {
        assert!(inline.variable_scope.is_some());
        let scope = inline.variable_scope.as_ref().unwrap();
        assert!(scope.variables.is_empty());
    } else {
        panic!("Expected inline procedure call");
    }
}

#[test]
fn test_inline_procedure_call_with_variables() {
    let source = "CALL (x, y) { RETURN x }";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, _diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_some(),
        "Failed to parse inline procedure call with variables"
    );

    let stmt = stmt_opt.unwrap();
    if let ProcedureCall::Inline(inline) = &stmt.call {
        assert!(inline.variable_scope.is_some());
        let scope = inline.variable_scope.as_ref().unwrap();
        assert_eq!(scope.variables.len(), 2);
        assert_eq!(scope.variables[0].name.as_str(), "x");
        assert_eq!(scope.variables[1].name.as_str(), "y");
    } else {
        panic!("Expected inline procedure call");
    }
}

#[test]
fn test_value_variable_definition_simple() {
    let source = "VALUE counter";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, diags) = parse_value_variable_definition(&mut stream);
    assert!(def_opt.is_none(), "Expected missing initializer to fail");
    assert!(
        !diags.is_empty(),
        "Expected diagnostics for missing initializer"
    );
}

#[test]
fn test_value_variable_definition_with_initializer() {
    let source = "VALUE counter = 42";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, _diags) = parse_value_variable_definition(&mut stream);
    assert!(
        def_opt.is_some(),
        "Failed to parse value variable with initializer"
    );

    let def = def_opt.unwrap();
    assert_eq!(def.variable.name.as_str(), "counter");
    assert!(def.initializer.is_some());
}

#[test]
fn test_graph_variable_definition() {
    let source = "GRAPH g";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, diags) = parse_graph_variable_definition(&mut stream);
    assert!(def_opt.is_none(), "Expected missing initializer to fail");
    assert!(
        !diags.is_empty(),
        "Expected diagnostics for missing initializer"
    );
}

#[test]
fn test_property_graph_variable_definition() {
    let source = "PROPERTY GRAPH pg";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, diags) = parse_graph_variable_definition(&mut stream);
    assert!(def_opt.is_none(), "Expected missing initializer to fail");
    assert!(
        !diags.is_empty(),
        "Expected diagnostics for missing initializer"
    );
}

#[test]
fn test_binding_table_variable_definition() {
    let source = "TABLE results";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, diags) = parse_binding_table_variable_definition(&mut stream);
    assert!(def_opt.is_none(), "Expected missing initializer to fail");
    assert!(
        !diags.is_empty(),
        "Expected diagnostics for missing initializer"
    );
}

#[test]
fn test_binding_binding_table_variable_definition() {
    let source = "BINDING TABLE results";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (def_opt, diags) = parse_binding_table_variable_definition(&mut stream);
    assert!(def_opt.is_none(), "Expected missing initializer to fail");
    assert!(
        !diags.is_empty(),
        "Expected diagnostics for missing initializer"
    );
}

#[test]
fn test_at_schema_clause() {
    let source = "AT my_schema";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (clause_opt, _diags) = parse_at_schema_clause(&mut stream);
    assert!(clause_opt.is_some(), "Failed to parse AT schema clause");
}

#[test]
fn test_multiple_variable_definitions() {
    let source = "VALUE x = 1";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (block_opt, _diags) = parse_binding_variable_definition_block(&mut stream);
    assert!(
        block_opt.is_some(),
        "Failed to parse variable definition block"
    );

    let block = block_opt.unwrap();
    assert_eq!(block.definitions.len(), 1);
}

#[test]
fn test_procedure_argument_list_empty() {
    let source = "()";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (args_opt, diags) = parse_procedure_argument_list(&mut stream);
    assert!(args_opt.is_some(), "Failed to parse empty argument list");
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let args = args_opt.unwrap();
    assert_eq!(args.arguments.len(), 0);
}

#[test]
fn test_procedure_argument_list_multiple_args() {
    let source = "(1, 2, 3)";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (args_opt, _diags) = parse_procedure_argument_list(&mut stream);
    assert!(args_opt.is_some(), "Failed to parse argument list");

    let args = args_opt.unwrap();
    assert_eq!(args.arguments.len(), 3);
}

#[test]
fn test_yield_item_list_simple() {
    let source = "YIELD x, y";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (yield_opt, diags) = parse_yield_clause(&mut stream);
    assert!(yield_opt.is_some(), "Failed to parse yield clause");
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let yield_clause = yield_opt.unwrap();
    assert_eq!(yield_clause.items.items.len(), 2);
}

#[test]
fn test_yield_item_with_alias() {
    let source = "YIELD x AS result";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (yield_opt, diags) = parse_yield_clause(&mut stream);
    assert!(
        yield_opt.is_some(),
        "Failed to parse yield clause with alias"
    );
    assert!(diags.is_empty(), "Unexpected diagnostics: {diags:?}");

    let yield_clause = yield_opt.unwrap();
    assert_eq!(yield_clause.items.items.len(), 1);
    assert!(yield_clause.items.items[0].alias.is_some());
    assert_eq!(
        yield_clause.items.items[0]
            .alias
            .as_ref()
            .unwrap()
            .name
            .as_str(),
        "result"
    );
}

#[test]
fn test_named_call_requires_parentheses() {
    let source = "CALL my_procedure";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(stmt_opt.is_none(), "Expected missing parentheses to fail");
    assert!(!diags.is_empty(), "Expected diagnostics");
}

#[test]
fn test_yield_disallows_arbitrary_expression_items() {
    let source = "CALL my_procedure() YIELD 1 + 2";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut pos = 0;

    let (stmt_opt, diags) = parse_call_procedure_statement(&tokens, &mut pos);
    assert!(
        stmt_opt.is_none() || !diags.is_empty(),
        "Expected expression yield item to be rejected"
    );
}

#[test]
fn test_argument_list_disallows_trailing_comma() {
    let source = "(1,)";
    let tokens = Lexer::new(source).tokenize().tokens;
    let mut stream = TokenStream::new(&tokens);

    let (_args_opt, diags) = parse_procedure_argument_list(&mut stream);
    assert!(!diags.is_empty(), "Expected trailing comma diagnostic");
}
