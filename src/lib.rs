//! GQL parser with rich diagnostics.
//!
//! This library provides a GQL (Graph Query Language) parser with comprehensive
//! error reporting built on miette for beautiful diagnostic messages.
//!
//! # Example
//!
//! ```
//! use gql_parser::parse;
//!
//! let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n";
//! let parse_result = parse(source);
//!
//! // Check that we got an AST
//! assert!(parse_result.ast.is_some());
//! ```

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;

// Re-export syntax span primitives.
pub use ast::{Span, Spanned};

// Re-export lexer types for convenience.
pub use diag::{Diag, DiagLabel, DiagSeverity, LabelRole};
pub use lexer::token::{Token, TokenKind};
pub use lexer::{Lexer, LexerResult, tokenize};

// Re-export parser types for convenience.
pub use parser::{ParseResult, Parser};

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
}
