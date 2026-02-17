//! GQL parser with rich diagnostics.
//!
//! This library provides a GQL (Graph Query Language) parser with comprehensive
//! error reporting built on miette for beautiful diagnostic messages.
//!
//! # Example
//!
//! ```
//! use gql_parser::{tokenize, TokenKind};
//!
//! let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n";
//! let result = tokenize(source);
//!
//! // Check that we got tokens
//! assert!(result.tokens.len() > 0);
//! assert_eq!(result.tokens[0].kind, TokenKind::Match);
//!
//! // Check for any lexical errors
//! assert!(result.diagnostics.is_empty());
//! ```

pub mod ast;
pub mod diag;
pub mod lexer;

// Re-export syntax span primitives.
pub use ast::{Span, Spanned};

// Re-export lexer types for convenience.
pub use diag::{Diag, DiagLabel, DiagSeverity, LabelRole};
pub use lexer::token::{Token, TokenKind};
pub use lexer::{Lexer, LexerResult, tokenize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_accessible() {
        // Verify that syntax span primitives are accessible through the public API.
        let _span: Span = 0..5;
        let _spanned = Spanned::new(42, 0..5);
    }
}
