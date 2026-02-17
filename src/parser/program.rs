//! Program structure and statement parsing.

use crate::ast::{CatalogStatement, MutationStatement, Program, QueryStatement, Statement};
use crate::lexer::token::TokenKind;
use crate::parser::Parser;

impl<'source> Parser<'source> {
    /// Parses a complete GQL program.
    ///
    /// A program consists of zero or more statements. Errors in individual
    /// statements are recovered at statement boundaries, allowing parsing
    /// to continue.
    pub(crate) fn parse_program(&mut self) -> Program {
        let start = self.peek().span.start;
        let mut statements = Vec::new();

        while !self.is_eof() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(()) => {
                    // Error already recorded, synchronize to next statement
                    self.synchronize_at_statement();
                }
            }
        }

        let end = if self.current > 0 {
            self.tokens
                .get(self.current - 1)
                .map(|t| t.span.end)
                .unwrap_or(start)
        } else {
            start
        };

        Program {
            statements,
            span: start..end,
        }
    }

    /// Parses a single statement.
    ///
    /// Dispatches to the appropriate statement parser based on the
    /// leading keyword. Returns `Err(())` if the statement cannot be parsed.
    fn parse_statement(&mut self) -> Result<Statement, ()> {
        match self.peek_kind() {
            TokenKind::Semicolon => {
                let span = self.advance().span.clone();
                Ok(Statement::Empty(span))
            }
            // Query statements
            TokenKind::Match | TokenKind::Select | TokenKind::From => self.parse_query_statement(),
            // Mutation statements
            TokenKind::Insert | TokenKind::Delete | TokenKind::Set | TokenKind::Remove => {
                self.parse_mutation_statement()
            }
            // Catalog statements
            TokenKind::Create | TokenKind::Drop => self.parse_catalog_statement(),
            // Note: Session and Transaction statements will be added in Sprint 4
            // when the appropriate keywords are added to the lexer
            TokenKind::Eof => {
                // Gracefully handle EOF
                Err(())
            }
            _ => {
                self.unexpected_token("statement");
                Err(())
            }
        }
    }

    /// Parses a query statement (stub for Sprint 7).
    fn parse_query_statement(&mut self) -> Result<Statement, ()> {
        let start = self.peek().span.start;
        // For now, just consume the leading keyword
        self.advance();

        // Stub: consume tokens until we hit a statement boundary or EOF
        // In Sprint 7, this will be replaced with proper query parsing
        while !self.is_eof() && !self.at_statement_start() && !self.at(&TokenKind::Semicolon) {
            self.advance();
        }

        let end = if self.current > 0 {
            self.tokens
                .get(self.current - 1)
                .map(|t| t.span.end)
                .unwrap_or(start)
        } else {
            start
        };

        Ok(Statement::Query(Box::new(QueryStatement {
            span: start..end,
        })))
    }

    /// Parses a mutation statement (stub for Sprint 10).
    fn parse_mutation_statement(&mut self) -> Result<Statement, ()> {
        let start = self.peek().span.start;
        self.advance();

        // Stub: consume tokens until statement boundary
        while !self.is_eof() && !self.at_statement_start() && !self.at(&TokenKind::Semicolon) {
            self.advance();
        }

        let end = if self.current > 0 {
            self.tokens
                .get(self.current - 1)
                .map(|t| t.span.end)
                .unwrap_or(start)
        } else {
            start
        };

        Ok(Statement::Mutation(Box::new(MutationStatement {
            span: start..end,
        })))
    }

    /// Parses a catalog statement (stub for Sprint 4).
    fn parse_catalog_statement(&mut self) -> Result<Statement, ()> {
        let start = self.peek().span.start;
        self.advance();

        // Stub: consume tokens until statement boundary
        while !self.is_eof() && !self.at_statement_start() && !self.at(&TokenKind::Semicolon) {
            self.advance();
        }

        let end = if self.current > 0 {
            self.tokens
                .get(self.current - 1)
                .map(|t| t.span.end)
                .unwrap_or(start)
        } else {
            start
        };

        Ok(Statement::Catalog(Box::new(CatalogStatement {
            span: start..end,
        })))
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
    fn test_parse_empty_program() {
        let tokens = vec![make_token(TokenKind::Eof, 0, 0)];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 0);
        assert_eq!(program.span, 0..0);
    }

    #[test]
    fn test_parse_single_query_statement() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Eof, 5, 5),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Query(_)));
    }

    #[test]
    fn test_parse_multiple_statements() {
        let tokens = vec![
            make_token(TokenKind::Match, 0, 5),
            make_token(TokenKind::Insert, 5, 11),
            make_token(TokenKind::Create, 11, 17),
            make_token(TokenKind::Eof, 17, 17),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 3);
        assert!(matches!(program.statements[0], Statement::Query(_)));
        assert!(matches!(program.statements[1], Statement::Mutation(_)));
        assert!(matches!(program.statements[2], Statement::Catalog(_)));
    }

    #[test]
    fn test_parse_with_invalid_token() {
        let tokens = vec![
            make_token(TokenKind::Identifier("x".to_string()), 0, 3),
            make_token(TokenKind::Match, 3, 8),
            make_token(TokenKind::Eof, 8, 8),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        // First invalid token produces error, then Match is parsed
        assert_eq!(program.statements.len(), 1);
        assert!(!parser.diagnostics.is_empty());
    }

    #[test]
    fn test_parse_mutation_statement() {
        let tokens = vec![
            make_token(TokenKind::Insert, 0, 6),
            make_token(TokenKind::Eof, 6, 6),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Mutation(_)));
    }

    #[test]
    fn test_parse_catalog_statement() {
        let tokens = vec![
            make_token(TokenKind::Create, 0, 6),
            make_token(TokenKind::Eof, 6, 6),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Catalog(_)));
    }

    #[test]
    fn test_parse_from_as_query_statement() {
        let tokens = vec![
            make_token(TokenKind::From, 0, 4),
            make_token(TokenKind::Eof, 4, 4),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Query(_)));
    }

    #[test]
    fn test_parse_empty_statement_semicolon() {
        let tokens = vec![
            make_token(TokenKind::Semicolon, 0, 1),
            make_token(TokenKind::Eof, 1, 1),
        ];
        let mut parser = Parser::new(tokens, "");
        let program = parser.parse_program();

        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Empty(_)));
    }
}
