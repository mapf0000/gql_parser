//! Shared parser infrastructure for token stream navigation and error handling.
//!
//! This module provides common functionality used by all parser modules to avoid
//! code duplication. All parsers use composition with `TokenStream` rather than
//! reimplementing these methods.

use crate::ast::Span;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};

/// Common error type for parsing operations.
pub type ParseError = Box<Diag>;

/// Common result type for parsing operations.
pub type ParseResult<T> = Result<T, ParseError>;

/// Token stream navigator providing common operations for all parsers.
///
/// This struct encapsulates token navigation, lookahead, and basic matching
/// operations. All parser modules should use this via composition to avoid
/// duplicating these methods.
pub struct TokenStream<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> TokenStream<'a> {
    /// Creates a new token stream from a token slice.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Returns the current token.
    ///
    /// If the position is past the end, returns the last token (which should be EOF).
    pub fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream must be non-empty"))
    }

    /// Returns the next token without consuming the current one.
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    /// Advances to the next token.
    ///
    /// This can advance past the last token to indicate all tokens have been consumed.
    ///
    /// Note: If already at or past the EOF token, this is a no-op. Parse functions should
    /// check `is_at_end()` if they need to ensure progress was made, or use `try_advance()`
    /// to get feedback about whether the advance succeeded.
    pub fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Attempts to advance to the next token, returning whether progress was made.
    ///
    /// Returns `true` if the position changed, `false` if already past the last real token.
    /// Use this when you need to ensure an advance actually made progress.
    ///
    /// Note: This can advance from the EOF token position to one past EOF, but will
    /// return false if already past EOF.
    ///
    /// # Example
    /// ```ignore
    /// if !stream.try_advance() {
    ///     // Can't advance further
    ///     return None;
    /// }
    /// ```
    pub fn try_advance(&mut self) -> bool {
        if self.pos < self.tokens.len() {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Returns true if positioned at or past the EOF token.
    ///
    /// Use this to check if further parsing is possible. Parse functions that do error
    /// recovery should avoid returning Some(result) when at EOF, as this can cause
    /// infinite loops in greedy parsing loops.
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len().saturating_sub(1)
            && self.tokens.last().map(|t| t.kind == TokenKind::Eof).unwrap_or(true)
    }

    /// Checks if the current token matches the given kind.
    pub fn check(&self, kind: &TokenKind) -> bool {
        if self.pos >= self.tokens.len() {
            return matches!(kind, TokenKind::Eof);
        }

        &self.tokens[self.pos].kind == kind
    }

    /// Consumes the current token if it matches the given kind.
    ///
    /// Returns `true` if the token was consumed, `false` otherwise.
    pub fn consume(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expects a specific token kind and returns its span.
    ///
    /// If the current token doesn't match, returns an error.
    pub fn expect(&mut self, kind: TokenKind) -> ParseResult<Span> {
        if self.check(&kind) {
            let span = self.current().span.clone();
            self.advance();
            Ok(span)
        } else {
            Err(self.error_here(format!("expected {kind}, found {}", self.current().kind)))
        }
    }

    /// Creates an error at the current token position.
    pub fn error_here(&self, message: impl Into<String>) -> ParseError {
        Box::new(
            Diag::error(message.into()).with_primary_label(self.current().span.clone(), "here"),
        )
    }

    /// Creates an error at the current token position with a specific error code.
    pub fn error_here_with_code(&self, message: impl Into<String>, code: &str) -> ParseError {
        Box::new(
            Diag::error(message.into())
                .with_primary_label(self.current().span.clone(), "here")
                .with_code(code),
        )
    }

    /// Returns the current position in the token stream.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Sets the position in the token stream (used for backtracking).
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos.min(self.tokens.len());
    }

    /// Returns a reference to the underlying token slice.
    pub fn tokens(&self) -> &'a [Token] {
        self.tokens
    }

    /// Returns the span of the previous token (useful after consuming a token).
    pub fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span.clone()
        } else {
            self.current().span.clone()
        }
    }
}

/// Merges two spans into a single span covering both.
pub fn merge_spans(start: &Span, end: &Span) -> Span {
    start.start..end.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::TokenKind;

    fn make_tokens() -> Vec<Token> {
        vec![
            Token::new(TokenKind::Match, 0..5),
            Token::new(TokenKind::LParen, 5..6),
            Token::new(TokenKind::Identifier("n".into()), 6..7),
            Token::new(TokenKind::RParen, 7..8),
            Token::new(TokenKind::Eof, 8..8),
        ]
    }

    #[test]
    fn token_stream_navigation() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        assert_eq!(stream.current().kind, TokenKind::Match);
        assert_eq!(stream.peek().map(|t| &t.kind), Some(&TokenKind::LParen));

        stream.advance();
        assert_eq!(stream.current().kind, TokenKind::LParen);

        stream.advance();
        assert_eq!(stream.current().kind, TokenKind::Identifier("n".into()));
    }

    #[test]
    fn token_stream_check_and_consume() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        assert!(stream.check(&TokenKind::Match));
        assert!(!stream.check(&TokenKind::Select));

        assert!(stream.consume(&TokenKind::Match));
        assert_eq!(stream.current().kind, TokenKind::LParen);

        assert!(!stream.consume(&TokenKind::Match));
        assert_eq!(stream.current().kind, TokenKind::LParen);
    }

    #[test]
    fn token_stream_expect_success() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        let span = stream.expect(TokenKind::Match).expect("Match token should be present");
        assert_eq!(span, 0..5);
        assert_eq!(stream.current().kind, TokenKind::LParen);
    }

    #[test]
    fn token_stream_expect_failure() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        let result = stream.expect(TokenKind::Select);
        assert!(result.is_err());
        assert_eq!(stream.current().kind, TokenKind::Match); // Position unchanged
    }

    #[test]
    fn token_stream_at_eof() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        // Advance to EOF
        while stream.current().kind != TokenKind::Eof {
            stream.advance();
        }

        // Should stay at EOF
        stream.advance();
        assert_eq!(stream.current().kind, TokenKind::Eof);
    }

    #[test]
    fn token_stream_try_advance_success() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        assert!(stream.try_advance()); // Match -> LParen
        assert_eq!(stream.current().kind, TokenKind::LParen);

        assert!(stream.try_advance()); // LParen -> Identifier
        assert_eq!(stream.current().kind, TokenKind::Identifier("n".into()));
    }

    #[test]
    fn token_stream_try_advance_at_eof() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        // Advance to EOF token
        while stream.current().kind != TokenKind::Eof {
            stream.advance();
        }

        // We're AT the EOF token (position = tokens.len() - 1)
        assert_eq!(stream.current().kind, TokenKind::Eof);

        // try_advance should succeed once more (to go past EOF)
        assert!(stream.try_advance());

        // Now we're past EOF (position = tokens.len())
        // current() still returns EOF (last token)
        assert_eq!(stream.current().kind, TokenKind::Eof);

        // But try_advance should now return false
        assert!(!stream.try_advance());
        assert_eq!(stream.current().kind, TokenKind::Eof);
    }

    #[test]
    fn token_stream_is_at_end() {
        let tokens = make_tokens();
        let mut stream = TokenStream::new(&tokens);

        assert!(!stream.is_at_end()); // At Match

        // Advance to just before EOF
        while stream.peek().map(|t| &t.kind) != Some(&TokenKind::Eof) {
            stream.advance();
        }
        assert!(!stream.is_at_end()); // At RParen

        // Advance to EOF
        stream.advance();
        assert!(stream.is_at_end()); // At EOF token
    }
}
