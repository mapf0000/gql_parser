//! Parser infrastructure for GQL syntax.
//!
//! The parser consumes a token stream produced by the lexer and constructs
//! an AST while preserving diagnostics and recovering at statement boundaries.

pub mod expression;
pub mod mutation;
pub mod patterns;
mod program;
pub mod query;
pub mod references;
pub mod types;

use crate::ast::Program;
use crate::diag::{Diag, DiagSeverity, SourceFile, convert_diagnostics_to_reports};
use crate::lexer::token::{Token, TokenKind};
use miette::Report;

/// Result of parsing a GQL program.
#[derive(Debug)]
pub struct ParseResult {
    /// The parsed program AST, or None if parsing failed completely.
    pub ast: Option<Program>,
    /// All collected diagnostics rendered as miette reports.
    pub diagnostics: Vec<Report>,
}

/// GQL parser with error recovery.
pub struct Parser<'source> {
    tokens: Vec<Token>,
    diagnostics: Vec<Diag>,
    source: &'source str,
}

impl<'source> Parser<'source> {
    /// Creates a new parser from a token stream.
    pub fn new(mut tokens: Vec<Token>, source: &'source str) -> Self {
        if tokens.is_empty() {
            tokens.push(Token::new(TokenKind::Eof, 0..0));
        } else if !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
            let eof_pos = tokens.last().map(|t| t.span.end).unwrap_or(0);
            tokens.push(Token::new(TokenKind::Eof, eof_pos..eof_pos));
        }

        Self {
            tokens,
            diagnostics: Vec::new(),
            source,
        }
    }

    /// Parses the token stream into a GQL program AST.
    pub fn parse(mut self) -> ParseResult {
        let (program, parser_diags) =
            program::parse_program_tokens(&self.tokens, self.source.len());
        self.diagnostics.extend(parser_diags);
        let has_error = self
            .diagnostics
            .iter()
            .any(|diag| diag.severity == DiagSeverity::Error);
        let ast = if has_error && program.statements.is_empty() {
            None
        } else {
            Some(program)
        };

        let source = SourceFile::new(self.source);
        let reports = convert_diagnostics_to_reports(&self.diagnostics, &source);

        ParseResult {
            ast,
            diagnostics: reports,
        }
    }

    /// Merges lexer diagnostics with parser diagnostics.
    pub fn with_lexer_diagnostics(mut self, lex_diags: Vec<Diag>) -> Self {
        let mut all_diags = lex_diags;
        all_diags.append(&mut self.diagnostics);
        self.diagnostics = all_diags;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::lexer::token::TokenKind;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn parser_creation_normalizes_missing_eof() {
        let tokens = vec![Token::new(TokenKind::Match, 0..5)];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.tokens.len(), 2);
        assert_eq!(parser.tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn parse_empty_program() {
        let tokens = vec![Token::new(TokenKind::Eof, 0..0)];
        let parser = Parser::new(tokens, "");
        let result = parser.parse();

        assert!(result.ast.is_some());
        assert!(result.ast.unwrap().statements.is_empty());
    }

    #[test]
    fn parse_returns_none_for_fatal_failure_without_statements() {
        let tokens = vec![
            Token::new(TokenKind::Identifier("invalid".into()), 0..7),
            Token::new(TokenKind::Eof, 7..7),
        ];
        let parser = Parser::new(tokens, "invalid");
        let result = parser.parse();

        assert!(result.ast.is_none());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn parse_keeps_partial_ast_when_recovery_produced_statements() {
        let source = "RETURN 1; invalid";
        let tokens = Lexer::new(source).tokenize().tokens;
        let parser = Parser::new(tokens, source);
        let result = parser.parse();

        assert!(result.ast.is_some());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn parser_never_panics_on_random_inputs() {
        fn random_token_kind(seed: &mut u64) -> TokenKind {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            match *seed % 10 {
                0 => TokenKind::Match,
                1 => TokenKind::Select,
                2 => TokenKind::From,
                3 => TokenKind::Insert,
                4 => TokenKind::Delete,
                5 => TokenKind::Create,
                6 => TokenKind::Drop,
                7 => TokenKind::Where,
                8 => TokenKind::Semicolon,
                _ => TokenKind::Identifier("x".into()),
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
                tokens.push(Token::new(kind, cursor..end));
                cursor = end;
            }

            let result = catch_unwind(AssertUnwindSafe(|| Parser::new(tokens, "").parse()));
            assert!(result.is_ok(), "parser panicked on randomized token stream");
        }
    }
}
