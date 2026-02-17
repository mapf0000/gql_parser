//! Error recovery strategies and synchronization.

use crate::ast::Span;
use crate::diag::Diag;
use crate::lexer::token::TokenKind;
use crate::parser::Parser;

/// Token kinds that mark the start of a statement.
const STATEMENT_START_TOKENS: &[TokenKind] = &[
    TokenKind::Match,
    TokenKind::Select,
    TokenKind::From,
    TokenKind::Insert,
    TokenKind::Delete,
    TokenKind::Set,
    TokenKind::Remove,
    TokenKind::Create,
    TokenKind::Drop,
    // Note: Session and transaction keywords will be added in Sprint 4
];

/// Token kinds that mark clause boundaries.
/// This will be used in future sprints for clause-level error recovery.
#[allow(dead_code)]
const CLAUSE_BOUNDARY_TOKENS: &[TokenKind] = &[
    TokenKind::Match,
    TokenKind::Where,
    TokenKind::Return,
    TokenKind::With,
    TokenKind::Order, // ORDER BY uses two keywords
    TokenKind::Limit,
    TokenKind::Offset,
    TokenKind::Union,
];

impl<'source> Parser<'source> {
    /// Returns true when `kind` can begin a top-level statement.
    pub(crate) fn is_statement_start_kind(kind: &TokenKind) -> bool {
        STATEMENT_START_TOKENS.contains(kind)
    }

    /// Returns true when the current token can begin a top-level statement.
    pub(crate) fn at_statement_start(&self) -> bool {
        Self::is_statement_start_kind(&self.peek_kind())
    }

    /// Recovers to the next statement boundary.
    ///
    /// Skips tokens until a statement-starting keyword or EOF is found.
    /// This is used for statement-level error recovery.
    pub(crate) fn synchronize_at_statement(&mut self) {
        while !self.is_eof() && !self.at_statement_start() {
            self.advance();
        }
    }

    /// Recovers to the next clause boundary.
    ///
    /// Skips tokens until a clause-starting keyword or EOF is found.
    /// This is used for clause-level error recovery within statements.
    ///
    /// This will be used in future sprints for parsing complex query clauses.
    #[allow(dead_code)]
    pub(crate) fn synchronize_at_clause(&mut self) {
        while !self.is_eof() && !self.at_any(CLAUSE_BOUNDARY_TOKENS) {
            self.advance();
        }
    }

    /// Recovers to any of the specified synchronization points.
    ///
    /// Skips tokens until one of the sync point tokens or EOF is found.
    ///
    /// # Arguments
    ///
    /// * `sync_points` - Array of token kinds to synchronize on
    ///
    /// This will be used in future sprints for custom synchronization strategies.
    #[allow(dead_code)]
    pub(crate) fn recover_to(&mut self, sync_points: &[TokenKind]) {
        while !self.is_eof() && !self.at_any(sync_points) {
            self.advance();
        }
    }

    /// Emits an "expected token" diagnostic.
    ///
    /// # Arguments
    ///
    /// * `expected` - The token kind that was expected
    /// * `context` - Context message (e.g., "in match clause")
    ///
    /// This will be used in future sprints when parsing grammar rules.
    #[allow(dead_code)]
    pub(crate) fn expected_token(&mut self, expected: TokenKind, context: &str) {
        let token = self.peek();
        let diag = Diag::error(format!(
            "expected {:?}, found {:?} {}",
            expected, token.kind, context
        ))
        .with_primary_label(token.span.clone(), format!("unexpected {:?}", token.kind))
        .with_help(format!("try inserting {:?} here", expected))
        .with_code("P002");

        self.diagnostics.push(diag);
    }

    /// Emits an "unexpected token" diagnostic.
    ///
    /// # Arguments
    ///
    /// * `context` - Context message (e.g., "in statement")
    pub(crate) fn unexpected_token(&mut self, context: &str) {
        let token = self.peek();
        let diag = Diag::error(format!("unexpected token in {}", context))
            .with_primary_label(token.span.clone(), format!("unexpected {:?}", token.kind))
            .with_code("P003");

        self.diagnostics.push(diag);
    }

    /// Emits a generic parser error.
    ///
    /// # Arguments
    ///
    /// * `span` - Span where the error occurred
    /// * `message` - Error message
    ///
    /// This will be used in future sprints for emitting custom error messages.
    #[allow(dead_code)]
    pub(crate) fn error(&mut self, span: Span, message: impl Into<String>) {
        let diag = Diag::error(message)
            .with_primary_label(span, "")
            .with_code("P001");

        self.diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::Token;

    fn make_token(kind: TokenKind, start: usize, end: usize) -> Token {
        Token::new(kind, start..end, "")
    }

    #[test]
    fn test_synchronize_at_statement() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::Identifier("x".to_string()), 1, 2),
            make_token(TokenKind::Match, 2, 7),
            make_token(TokenKind::Eof, 7, 7),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.synchronize_at_statement();
        assert_eq!(parser.peek_kind(), TokenKind::Match);
    }

    #[test]
    fn test_synchronize_at_clause() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::Identifier("x".to_string()), 1, 2),
            make_token(TokenKind::Where, 2, 7),
            make_token(TokenKind::Eof, 7, 7),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.synchronize_at_clause();
        assert_eq!(parser.peek_kind(), TokenKind::Where);
    }

    #[test]
    fn test_recover_to() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::Identifier("x".to_string()), 1, 2),
            make_token(TokenKind::RParen, 2, 3),
            make_token(TokenKind::Eof, 3, 3),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.recover_to(&[TokenKind::RParen]);
        assert_eq!(parser.peek_kind(), TokenKind::RParen);
    }

    #[test]
    fn test_synchronize_stops_at_eof() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::Identifier("x".to_string()), 1, 2),
            make_token(TokenKind::Eof, 2, 2),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.synchronize_at_statement();
        assert!(parser.is_eof());
    }

    #[test]
    fn test_synchronize_at_statement_stops_at_from() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::From, 1, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.synchronize_at_statement();
        assert_eq!(parser.peek_kind(), TokenKind::From);
    }

    #[test]
    fn test_error_diagnostics() {
        let tokens = vec![make_token(TokenKind::Eof, 0, 0)];
        let mut parser = Parser::new(tokens, "");

        parser.error(0..5, "test error");
        assert_eq!(parser.diagnostics.len(), 1);
        assert_eq!(
            parser.diagnostics[0].severity,
            crate::diag::DiagSeverity::Error
        );
    }

    #[test]
    fn test_expected_token_diagnostic() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 1),
            make_token(TokenKind::Eof, 1, 1),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.expected_token(TokenKind::LParen, "after identifier");
        assert_eq!(parser.diagnostics.len(), 1);
    }

    #[test]
    fn test_unexpected_token_diagnostic() {
        let tokens = vec![
            make_token(TokenKind::RBrace, 0, 1),
            make_token(TokenKind::Eof, 1, 1),
        ];
        let mut parser = Parser::new(tokens, "");

        parser.unexpected_token("statement");
        assert_eq!(parser.diagnostics.len(), 1);
    }
}
