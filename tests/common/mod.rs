//! Common test utilities
//!
//! This module contains shared test helpers, fixtures, and utilities
//! used across multiple test modules.
//!
//! # Diagnostic Helpers
//! - [`format_diagnostics`] - Format diagnostics for display in assertions
//! - [`assert_no_parse_errors`] - Assert that parsing produced no diagnostics
//! - [`assert_no_validation_errors`] - Assert that validation succeeded
//! - [`assert_has_error_containing`] - Assert that an error message contains specific text
//!
//! # Parsing Helpers
//! - [`assert_parses_cleanly`] - Assert that source parses without diagnostics
//! - [`parse_cleanly`] - Parse source and return AST, panicking on errors
//! - [`tokenize_cleanly`] - Tokenize source and return tokens, panicking on errors
//!
//! # Validation Helpers
//! - [`parse_and_validate`] - Parse source and run semantic validation
//! - [`parse_and_expect_failure`] - Parse and validate, expecting validation to fail

use gql_parser::{ast::Program, diag::{DiagSeverity, Diag}, parse, ParseResult, Token};
use gql_parser::lexer::Lexer;
use gql_parser::semantic::validator::SemanticValidator;
use gql_parser::ir::ValidationOutcome;

// ============================================================================
// Diagnostic Formatting and Assertion Helpers
// ============================================================================

/// Format diagnostics for display in assertion messages.
///
/// This is commonly used to show diagnostic details when tests fail.
///
/// # Example
/// ```no_run
/// let result = parse(source);
/// let diag_text = format_diagnostics(&result.diagnostics);
/// assert!(diag_text.contains("expected text"), "Diagnostics: {diag_text}");
/// ```
pub fn format_diagnostics(diags: &[miette::Report]) -> String {
    diags
        .iter()
        .map(|diag| format!("{diag:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format Diag diagnostics for display in assertion messages.
pub fn format_diag_diagnostics(diags: &[Diag]) -> String {
    diags
        .iter()
        .map(|diag| format!("{:?}", diag))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Assert that a parse result contains no diagnostics (no errors or warnings).
///
/// # Panics
/// Panics if any diagnostics are present, showing the source and diagnostics.
///
/// # Example
/// ```no_run
/// let result = parse(source);
/// assert_no_parse_errors(&result, source);
/// ```
pub fn assert_no_parse_errors(result: &ParseResult, source: &str) {
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics for `{source}`:\n{}",
        format_diagnostics(&result.diagnostics)
    );
}

/// Assert that a validation outcome contains no errors.
///
/// Note: This allows warnings but fails if any errors are present.
///
/// # Panics
/// Panics if the validation outcome contains any error-level diagnostics.
///
/// # Example
/// ```no_run
/// let outcome = validator.validate(&program);
/// assert_no_validation_errors(&outcome);
/// ```
pub fn assert_no_validation_errors(outcome: &ValidationOutcome) {
    let errors: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|d| d.severity == DiagSeverity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "validation should not have errors, but found {}:\n{}",
        errors.len(),
        format_diag_diagnostics(&outcome.diagnostics)
    );
}

/// Assert that a validation outcome contains an error with a message containing the specified text.
///
/// # Panics
/// Panics if no error containing the specified text is found.
///
/// # Example
/// ```no_run
/// let outcome = validator.validate(&program);
/// assert_has_error_containing(&outcome, "undefined variable");
/// ```
pub fn assert_has_error_containing(outcome: &ValidationOutcome, text: &str) {
    let has_matching_error = outcome.diagnostics.iter().any(|d| {
        d.severity == DiagSeverity::Error && format!("{:?}", d).contains(text)
    });

    assert!(
        has_matching_error,
        "expected error containing '{}', but found:\n{}",
        text,
        format_diag_diagnostics(&outcome.diagnostics)
    );
}

/// Assert that a validation outcome contains any error.
///
/// # Panics
/// Panics if no errors are present.
///
/// # Example
/// ```no_run
/// let outcome = validator.validate(&program);
/// assert_has_any_error(&outcome);
/// ```
pub fn assert_has_any_error(outcome: &ValidationOutcome) {
    let has_error = outcome
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagSeverity::Error);

    assert!(
        has_error,
        "expected at least one error, but validation succeeded"
    );
}

// ============================================================================
// Parsing Helpers
// ============================================================================

/// Assert that source code parses without any diagnostics.
///
/// This is useful for tests that just want to verify parsing succeeds
/// without needing to inspect the AST.
///
/// # Panics
/// Panics if parsing produces any diagnostics.
///
/// # Example
/// ```no_run
/// assert_parses_cleanly("MATCH (n) RETURN n");
/// ```
pub fn assert_parses_cleanly(source: &str) {
    let result = parse(source);
    assert_no_parse_errors(&result, source);
}

/// Parse source code and return the AST, panicking if any diagnostics occur.
///
/// This is useful when you need the AST but want to fail fast if parsing fails.
///
/// # Panics
/// Panics if parsing produces any diagnostics or if no AST is produced.
///
/// # Example
/// ```no_run
/// let program = parse_cleanly("MATCH (n) RETURN n");
/// assert_eq!(program.statements.len(), 1);
/// ```
pub fn parse_cleanly(source: &str) -> Program {
    let result = parse(source);
    assert_no_parse_errors(&result, source);
    result
        .ast
        .unwrap_or_else(|| panic!("expected AST for source: {source}"))
}

/// Tokenize source code and return tokens, panicking if any diagnostics occur.
///
/// This is useful for parser tests that work directly with tokens.
///
/// # Panics
/// Panics if tokenization produces any diagnostics.
///
/// # Example
/// ```no_run
/// let tokens = tokenize_cleanly("MATCH (n)");
/// assert_eq!(tokens.len(), 5); // MATCH, (, n, ), EOF
/// ```
pub fn tokenize_cleanly(source: &str) -> Vec<Token> {
    let lexed = Lexer::new(source).tokenize();
    assert!(
        lexed.diagnostics.is_empty(),
        "unexpected lexer diagnostics for `{source}`:\n{:?}",
        lexed.diagnostics
    );
    lexed.tokens
}

// ============================================================================
// Combined Parse + Validate Helpers
// ============================================================================

/// Parse source code and run semantic validation, returning the validation outcome.
///
/// This helper combines parsing and validation into a single step, which is
/// a very common pattern in semantic tests.
///
/// # Panics
/// Panics if parsing fails to produce an AST.
///
/// # Example
/// ```no_run
/// let outcome = parse_and_validate("MATCH (n:Person) RETURN n");
/// assert_no_validation_errors(&outcome);
/// ```
pub fn parse_and_validate(source: &str) -> ValidationOutcome {
    let parse_result = parse(source);
    let program = parse_result.ast.unwrap_or_else(|| {
        panic!(
            "parser should produce an AST for semantic validation.\nSource: {source}\nDiagnostics:\n{}",
            format_diagnostics(&parse_result.diagnostics)
        )
    });

    let validator = SemanticValidator::new();
    validator.validate(&program)
}

/// Parse source code and run semantic validation with a custom validator.
///
/// This allows tests to provide their own validator configuration or catalogs.
///
/// # Panics
/// Panics if parsing fails to produce an AST.
///
/// # Example
/// ```no_run
/// let validator = SemanticValidator::new().with_strict_mode(true);
/// let outcome = parse_and_validate_with("MATCH (n) RETURN n", &validator);
/// ```
pub fn parse_and_validate_with(source: &str, validator: &SemanticValidator) -> ValidationOutcome {
    let parse_result = parse(source);
    let program = parse_result.ast.unwrap_or_else(|| {
        panic!(
            "parser should produce an AST for semantic validation.\nSource: {source}\nDiagnostics:\n{}",
            format_diagnostics(&parse_result.diagnostics)
        )
    });

    validator.validate(&program)
}

/// Parse source code, validate it, and assert that validation fails with an error.
///
/// This is useful for negative tests that expect validation to fail.
///
/// # Panics
/// Panics if parsing fails or if validation succeeds.
///
/// # Example
/// ```no_run
/// parse_and_expect_failure("RETURN undefined_var");
/// ```
pub fn parse_and_expect_failure(source: &str) -> ValidationOutcome {
    let outcome = parse_and_validate(source);
    assert_has_any_error(&outcome);
    outcome
}

/// Parse source code, validate it, and assert that validation succeeds.
///
/// This is a convenience helper that combines parsing, validation, and assertion.
///
/// # Panics
/// Panics if parsing fails or if validation produces errors.
///
/// # Example
/// ```no_run
/// parse_and_expect_success("MATCH (n:Person) RETURN n");
/// ```
pub fn parse_and_expect_success(source: &str) {
    let outcome = parse_and_validate(source);
    assert_no_validation_errors(&outcome);
}
