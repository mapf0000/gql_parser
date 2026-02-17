//! Parser infrastructure for GQL syntax.
//!
//! The parser consumes a token stream produced by the lexer and constructs
//! an Abstract Syntax Tree (AST) with comprehensive error recovery.
//!
//! # Architecture
//!
//! - **Parser**: Core parser struct with token stream navigation
//! - **Error Recovery**: Panic-mode recovery at natural synchronization points
//! - **Partial AST**: Returns partial results with diagnostics for malformed input
//! - **Diagnostics**: Integrates with the `diag` module for rich error messages
//!
//! # Example
//!
//! ```
//! use gql_parser::parse;
//!
//! let source = "MATCH (n) RETURN n";
//! let parse_result = parse(source);
//!
//! assert!(parse_result.ast.is_some());
//! ```
//!
//! For manual control, build a [`Parser`] from lexer output and merge lexer
//! diagnostics via [`Parser::with_lexer_diagnostics`].

mod primitives;
mod program;
mod recovery;

use crate::ast::Program;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};

/// Result of parsing a GQL program.
///
/// Contains the parsed AST (if any) and accumulated diagnostics.
///
/// Parser diagnostics are always present when emitted. Lexer diagnostics are
/// included when provided via [`Parser::with_lexer_diagnostics`] or when using
/// the crate-level [`crate::parse`] helper.
#[derive(Debug)]
pub struct ParseResult {
    /// The parsed program AST, or None if parsing failed completely.
    pub ast: Option<Program>,
    /// All collected diagnostics.
    pub diagnostics: Vec<Diag>,
}

/// GQL parser with error recovery.
///
/// The parser consumes a token stream and produces an AST with diagnostics.
/// It implements panic-mode error recovery at statement and clause boundaries,
/// ensuring partial results are returned even for malformed input.
pub struct Parser<'source> {
    /// Token stream from the lexer.
    tokens: Vec<Token>,
    /// Current position in the token stream.
    current: usize,
    /// Accumulated parser diagnostics.
    diagnostics: Vec<Diag>,
    /// Source text for diagnostic messages (will be used in future sprints).
    #[allow(dead_code)]
    source: &'source str,
}

impl<'source> Parser<'source> {
    /// Creates a new parser from a token stream.
    ///
    /// # Arguments
    ///
    /// * `tokens` - Token stream from the lexer
    /// * `source` - Original source text for diagnostics
    ///
    /// The parser is designed to consume lexer output, which includes a
    /// trailing EOF token. For robustness, this constructor normalizes the
    /// stream by synthesizing an EOF token when missing.
    pub fn new(mut tokens: Vec<Token>, source: &'source str) -> Self {
        if tokens.is_empty() {
            tokens.push(Token::new(TokenKind::Eof, 0..0, ""));
        } else if !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
            let eof_pos = tokens.last().map(|t| t.span.end).unwrap_or(0);
            tokens.push(Token::new(TokenKind::Eof, eof_pos..eof_pos, ""));
        }

        Self {
            tokens,
            current: 0,
            diagnostics: Vec::new(),
            source,
        }
    }

    /// Parses the token stream into a GQL program AST.
    ///
    /// This is the main entry point for parsing. It consumes the parser
    /// and returns a `ParseResult` containing the AST and diagnostics.
    ///
    /// # Example
    ///
    /// ```
    /// use gql_parser::{tokenize, Parser};
    ///
    /// let source = "MATCH (n) RETURN n";
    /// let lex_result = tokenize(source);
    /// let parser = Parser::new(lex_result.tokens, source)
    ///     .with_lexer_diagnostics(lex_result.diagnostics);
    /// let result = parser.parse();
    ///
    /// assert!(result.ast.is_some());
    /// ```
    pub fn parse(mut self) -> ParseResult {
        let ast = self.parse_program();
        ParseResult {
            ast: Some(ast),
            diagnostics: self.diagnostics,
        }
    }

    /// Merges lexer diagnostics with parser diagnostics.
    ///
    /// Use this when you have lexer diagnostics that should be included
    /// in the final parse result.
    pub fn with_lexer_diagnostics(mut self, lex_diags: Vec<Diag>) -> Self {
        // Prepend lexer diagnostics (they come first chronologically)
        let mut all_diags = lex_diags;
        all_diags.append(&mut self.diagnostics);
        self.diagnostics = all_diags;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::TokenKind;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn test_parser_creation() {
        let tokens = vec![Token::new(TokenKind::Eof, 0..0, "")];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.current, 0);
        assert_eq!(parser.diagnostics.len(), 0);
    }

    #[test]
    fn test_parser_creation_normalizes_missing_eof() {
        let tokens = vec![Token::new(TokenKind::Match, 0..5, "MATCH")];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.tokens.len(), 2);
        assert_eq!(parser.tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_parser_creation_normalizes_empty_stream() {
        let parser = Parser::new(Vec::new(), "");
        assert_eq!(parser.tokens.len(), 1);
        assert_eq!(parser.tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_parse_empty_program() {
        let tokens = vec![Token::new(TokenKind::Eof, 0..0, "")];
        let parser = Parser::new(tokens, "");
        let result = parser.parse();

        assert!(result.ast.is_some());
        assert_eq!(result.ast.unwrap().statements.len(), 0);
    }

    #[test]
    fn test_parser_never_panics_on_random_inputs() {
        fn random_token_kind(seed: &mut u64) -> TokenKind {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            match *seed % 12 {
                0 => TokenKind::Match,
                1 => TokenKind::Select,
                2 => TokenKind::From,
                3 => TokenKind::Insert,
                4 => TokenKind::Delete,
                5 => TokenKind::Create,
                6 => TokenKind::Drop,
                7 => TokenKind::Where,
                8 => TokenKind::Semicolon,
                9 => TokenKind::LParen,
                10 => TokenKind::RParen,
                _ => TokenKind::Identifier("x".to_string()),
            }
        }

        let mut seed = 0xC0FFEE_u64;
        for _ in 0..10_000 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let token_count = (seed % 32) as usize;

            let mut tokens = Vec::with_capacity(token_count + 1);
            let mut cursor = 0usize;

            for _ in 0..token_count {
                let kind = random_token_kind(&mut seed);
                let end = cursor + 1;
                tokens.push(Token::new(kind, cursor..end, ""));
                cursor = end;
            }

            let result = catch_unwind(AssertUnwindSafe(|| Parser::new(tokens, "").parse()));
            assert!(result.is_ok(), "parser panicked on randomized token stream");
        }
    }
}
