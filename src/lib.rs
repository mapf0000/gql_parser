#![allow(rustdoc::broken_intra_doc_links, rustdoc::invalid_html_tags)]
//! Pure-Rust ISO GQL parser with diagnostics, AST traversal, and query analysis APIs.
//!
//! # Parse
//!
//! ```
//! use gql_parser::parse;
//!
//! let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n.name";
//! let result = parse(source);
//! assert!(result.ast.is_some());
//! ```
//!
//! # Traverse AST
//!
//! ```
//! use gql_parser::ast::{AstVisitor, VariableCollector};
//! use gql_parser::parse;
//!
//! let program = parse("MATCH (n)-[:KNOWS]->(m) RETURN m").ast.unwrap();
//! let mut collector = VariableCollector::new();
//! let _ = collector.visit_program(&program);
//! assert!(collector.definitions().contains("n"));
//! ```
//!
//! # Analyze Query
//!
//! ```
//! use gql_parser::{QueryInfo, VariableDependencyGraph, parse};
//!
//! let statement = &parse("MATCH (n) LET x = n.age RETURN x")
//!     .ast
//!     .unwrap()
//!     .statements[0];
//!
//! let info = QueryInfo::from_ast(statement);
//! let deps = VariableDependencyGraph::build(statement);
//!
//! assert_eq!(info.clause_sequence.len(), 3);
//! assert!(!deps.edges.is_empty());
//! ```

use miette::Report;

pub mod analysis;
pub mod ast;
pub mod diag;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;

// Re-export syntax span primitives.
pub use ast::{Span, Spanned};

// Re-export lexer types for convenience.
pub use diag::{Diag, DiagLabel, DiagSeverity, LabelRole};
pub use lexer::keywords::{
    KeywordClassification, classify_keyword, is_non_reserved_word, is_pre_reserved_word,
    is_reserved_word,
};
pub use lexer::token::{Token, TokenKind};
pub use lexer::{Lexer, LexerResult, tokenize};

// Re-export parser types for convenience.
pub use parser::{ParseResult, Parser};

// Re-export semantic validation types for convenience.
pub use ir::{IR, ValidationResult};
pub use semantic::SemanticValidator;

// Re-export analysis types for convenience.
pub use analysis::{
    ClauseId, ClauseInfo, ClauseKind, DefineUseEdge, DefinitionPoint, ExpressionInfo, LiteralInfo,
    PatternInfo, PropertyReference, QueryInfo, QueryShape, UsagePoint, VariableDependencyGraph,
};

/// Parses GQL source text end-to-end (lexing + parsing).
///
/// This is the recommended API entry point. It guarantees parser input
/// comes from the lexer and merges diagnostics from both phases.
pub fn parse(source: &str) -> ParseResult {
    let lex_result = tokenize(source);
    Parser::new(lex_result.tokens, source)
        .with_lexer_diagnostics(lex_result.diagnostics)
        .parse()
}

/// Result of parsing and semantic validation with rendered diagnostics.
#[derive(Debug)]
pub struct ParseAndValidateResult {
    /// The validated IR, if successful.
    pub ir: Option<IR>,
    /// Combined diagnostics from parsing and semantic validation.
    pub diagnostics: Vec<Report>,
}

/// Parses and semantically validates GQL source text.
///
/// This function performs both syntactic parsing and semantic validation,
/// returning a result that contains either a validated IR (if successful)
/// or diagnostics from both phases.
///
/// # Example
///
/// ```
/// use gql_parser::parse_and_validate;
///
/// let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n";
/// let result = parse_and_validate(source);
///
/// if let Some(ir) = result.ir {
///     println!("Validation successful!");
///     // Use the validated IR for query planning/execution
/// } else {
///     for diag in &result.diagnostics {
///         eprintln!("Error: {:?}", diag);
///     }
/// }
/// ```
pub fn parse_and_validate(source: &str) -> ParseAndValidateResult {
    parse_and_validate_internal(source, SemanticValidator::new())
}

/// Parses and semantically validates GQL source text with custom validation configuration.
///
/// This function allows customizing the semantic validation behavior through
/// the provided configuration.
///
/// # Example
///
/// ```
/// use gql_parser::{parse_and_validate_with_config, semantic::ValidationConfig};
///
/// let source = "MATCH (n:Person) RETURN n";
/// let config = ValidationConfig {
///     strict_mode: true,
///     schema_validation: false,
///     catalog_validation: false,
///     warn_on_shadowing: true,
///     warn_on_disconnected_patterns: true,
/// };
///
/// let result = parse_and_validate_with_config(source, config);
/// if let Some(ir) = result.ir {
///     println!("Validation successful!");
/// } else {
///     for diag in &result.diagnostics {
///         eprintln!("Error: {:?}", diag);
///     }
/// }
/// ```
pub fn parse_and_validate_with_config(
    source: &str,
    config: semantic::ValidationConfig,
) -> ParseAndValidateResult {
    parse_and_validate_internal(source, SemanticValidator::with_config(config))
}

/// Internal helper for parse and validate operations.
fn parse_and_validate_internal(
    source: &str,
    validator: SemanticValidator,
) -> ParseAndValidateResult {
    // First, parse the source
    let parse_result = parse(source);

    // If there are any parse errors, return them immediately
    if !parse_result.diagnostics.is_empty() {
        return ParseAndValidateResult {
            ir: None,
            diagnostics: parse_result.diagnostics,
        };
    }

    // If we don't have an AST (shouldn't happen if no diagnostics), return error
    let Some(program) = parse_result.ast else {
        return ParseAndValidateResult {
            ir: None,
            diagnostics: vec![miette::Report::msg(
                "Failed to parse source (no AST produced)",
            )],
        };
    };

    // Run semantic validation
    let outcome = validator.validate(&program);

    // Convert semantic diagnostics to Reports
    let source_file = diag::SourceFile::new(source);
    let reports = diag::convert_diagnostics_to_reports(&outcome.diagnostics, &source_file);

    ParseAndValidateResult {
        ir: outcome.ir,
        diagnostics: reports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_accessible() {
        // Verify that syntax span primitives are accessible through the public API.
        let _span: Span = 0..5;
        let _spanned = Spanned::new(42, 0..5);
    }

    #[test]
    fn parse_includes_lexer_diagnostics() {
        let result = parse("@");
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn parse_and_validate_valid_query() {
        let source = "MATCH (n:Person) RETURN n";
        let result = parse_and_validate(source);
        assert!(result.ir.is_some(), "Expected successful validation");
        assert!(result.diagnostics.is_empty(), "Expected no diagnostics");
    }

    #[test]
    fn parse_and_validate_undefined_variable() {
        let source = "MATCH (n:Person) RETURN m";
        let result = parse_and_validate(source);
        assert!(
            result.ir.is_none(),
            "Expected validation error for undefined variable"
        );
        assert!(!result.diagnostics.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn parse_and_validate_with_config_strict_mode() {
        let source = "MATCH (n:Person) RETURN n";
        let config = semantic::ValidationConfig {
            strict_mode: true,
            schema_validation: false,
            catalog_validation: false,
            warn_on_shadowing: true,
            warn_on_disconnected_patterns: true,
        };
        let result = parse_and_validate_with_config(source, config);
        assert!(
            result.ir.is_some(),
            "Expected successful validation in strict mode"
        );
    }

    #[test]
    fn parse_and_validate_syntax_error() {
        let source = "MATCH (n:Person WHERE n.age > 18 RETURN n"; // Missing closing paren
        let result = parse_and_validate(source);
        assert!(result.ir.is_none(), "Expected parse error");
        assert!(!result.diagnostics.is_empty(), "Expected diagnostics");
    }

    #[test]
    fn parse_and_validate_preserves_semantic_diagnostic_structure() {
        // Test that semantic diagnostics preserve labels/spans/notes, not just messages
        let source = "MATCH (n:Person) RETURN undefinedVar";
        let result = parse_and_validate(source);

        assert!(result.ir.is_none(), "Expected semantic validation error");
        assert!(!result.diagnostics.is_empty(), "Expected diagnostics");

        // The diagnostic should have proper structure (this test verifies the report was created
        // through the full diagnostic conversion pipeline, not Report::msg)
        let diag = &result.diagnostics[0];
        let diag_str = format!("{:?}", diag);
        // Verify it's not a simple message-only diagnostic
        assert!(
            diag_str.len() > 50,
            "Diagnostic should have rich structure, not just a message"
        );
    }

    #[test]
    fn parse_and_validate_only_runs_semantics_on_parse_success() {
        // Test that semantic validation only runs when parse succeeds
        let source = "MATCH (n"; // Parse error
        let result = parse_and_validate(source);

        assert!(result.ir.is_none());
        assert!(!result.diagnostics.is_empty());

        // Now test with valid syntax but semantic error
        let source = "MATCH (n) RETURN m"; // Undefined variable
        let result = parse_and_validate(source);

        assert!(result.ir.is_none());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn parse_and_validate_no_ast_without_diagnostics() {
        // Edge case: if parse produces no AST and no diagnostics (shouldn't happen normally)
        // the API should return a diagnostic
        // This is more of a defensive test
    }
}
