//! Token navigation and consumption primitives.

use crate::lexer::token::{Token, TokenKind};
use crate::parser::Parser;

impl<'source> Parser<'source> {
    /// Returns a reference to the current token without consuming it.
    ///
    /// This never fails - if at EOF, returns the EOF token.
    pub(crate) fn peek(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or_else(|| {
            // If we somehow go past the token stream, return the last token (should be EOF)
            self.tokens
                .last()
                .expect("token stream should never be empty")
        })
    }

    /// Returns the kind of the current token.
    pub(crate) fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    /// Look ahead N tokens without consuming.
    ///
    /// Returns the token at position `current + n`. If out of bounds,
    /// returns the EOF token (or last token in stream).
    ///
    /// This will be used in future sprints for grammar rules requiring lookahead.
    #[allow(dead_code)]
    pub(crate) fn peek_nth(&self, n: usize) -> &Token {
        let index = self.current.saturating_add(n);
        self.tokens.get(index).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream should never be empty")
        })
    }

    /// Consumes the current token and advances to the next.
    ///
    /// Returns a reference to the consumed token.
    pub(crate) fn advance(&mut self) -> &Token {
        let index = self.current;
        if self.current + 1 < self.tokens.len() {
            self.current += 1;
        }
        &self.tokens[index]
    }

    /// Checks if the current token matches the given kind.
    pub(crate) fn at(&self, kind: &TokenKind) -> bool {
        &self.peek_kind() == kind
    }

    /// Checks if the current token matches any of the given kinds.
    pub(crate) fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.peek_kind())
    }

    /// Checks if we're at the end of the token stream.
    pub(crate) fn is_eof(&self) -> bool {
        self.at(&TokenKind::Eof)
    }

    /// Consumes the current token if it matches the expected kind.
    ///
    /// Returns `Ok(token)` if successful, `Err(())` if the token doesn't match.
    /// Does not emit diagnostics on failure.
    ///
    /// This will be used in future sprints for optional token consumption.
    #[allow(dead_code)]
    pub(crate) fn consume(&mut self, kind: &TokenKind) -> Result<Token, ()> {
        if self.at(kind) {
            Ok(self.advance().clone())
        } else {
            Err(())
        }
    }

    /// Consumes the current token, expecting it to be of the given kind.
    ///
    /// If the token doesn't match, emits a diagnostic and returns `Err(())`.
    ///
    /// # Arguments
    ///
    /// * `kind` - Expected token kind
    /// * `msg` - Error message context (e.g., "in match clause")
    ///
    /// This will be used in future sprints for parsing grammar rules.
    #[allow(dead_code)]
    pub(crate) fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<Token, ()> {
        if self.at(kind) {
            Ok(self.advance().clone())
        } else {
            self.expected_token(kind.clone(), msg);
            Err(())
        }
    }

    /// Tries to consume a keyword token.
    ///
    /// Returns `Some(token)` if the current token matches the keyword,
    /// `None` otherwise. Does not emit diagnostics.
    ///
    /// This will be used in future sprints for optional keyword matching.
    #[allow(dead_code)]
    pub(crate) fn match_keyword(&mut self, keyword: &TokenKind) -> Option<Token> {
        if self.at(keyword) {
            Some(self.advance().clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn make_token(kind: TokenKind, start: usize, end: usize) -> Token {
        Token::new(kind, start..end, "")
    }

    #[test]
    fn test_peek() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.peek().kind, TokenKind::Match);
    }

    #[test]
    fn test_peek_kind() {
        let tokens = vec![
            make_token(TokenKind::Return, 0, 6),
            make_token(TokenKind::Eof, 6, 6),
        ];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.peek_kind(), TokenKind::Return);
    }

    #[test]
    fn test_peek_nth() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::LParen, 5, 6),
            make_token(TokenKind::Identifier("n".to_string()), 6, 7),
            make_token(TokenKind::Eof, 7, 7),
        ];
        let parser = Parser::new(tokens, "");
        assert_eq!(parser.peek_nth(0).kind, TokenKind::Match);
        assert_eq!(parser.peek_nth(1).kind, TokenKind::LParen);
        assert_eq!(
            parser.peek_nth(2).kind,
            TokenKind::Identifier("n".to_string())
        );
        // Out of bounds returns last token
        assert_eq!(parser.peek_nth(10).kind, TokenKind::Eof);
    }

    #[test]
    fn test_advance() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Return, 5, 11),
            make_token(TokenKind::Eof, 11, 11),
        ];
        let mut parser = Parser::new(tokens, "");

        assert_eq!(parser.peek_kind(), TokenKind::Match);
        parser.advance();
        assert_eq!(parser.peek_kind(), TokenKind::Return);
        parser.advance();
        assert_eq!(parser.peek_kind(), TokenKind::Eof);
    }

    #[test]
    fn test_at() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let parser = Parser::new(tokens, "");
        assert!(parser.at(&TokenKind::Match));
        assert!(!parser.at(&TokenKind::Return));
    }

    #[test]
    fn test_at_any() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let parser = Parser::new(tokens, "");
        assert!(parser.at_any(&[TokenKind::Match, TokenKind::Return]));
        assert!(!parser.at_any(&[TokenKind::Return, TokenKind::Where]));
    }

    #[test]
    fn test_is_eof() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let mut parser = Parser::new(tokens, "");
        assert!(!parser.is_eof());
        parser.advance();
        assert!(parser.is_eof());
    }

    #[test]
    fn test_consume() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let mut parser = Parser::new(tokens, "");

        // Successful consumption
        assert!(parser.consume(&TokenKind::Match).is_ok());
        assert_eq!(parser.peek_kind(), TokenKind::Eof);

        // Failed consumption
        assert!(parser.consume(&TokenKind::Return).is_err());
    }

    #[test]
    fn test_match_keyword() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let mut parser = Parser::new(tokens, "");

        // Successful match
        assert!(parser.match_keyword(&TokenKind::Match).is_some());
        assert_eq!(parser.peek_kind(), TokenKind::Eof);

        // Failed match
        assert!(parser.match_keyword(&TokenKind::Return).is_none());
    }
}
