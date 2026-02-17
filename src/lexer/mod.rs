//! Lexical analysis for GQL.
//!
//! This module implements a robust, error-tolerant lexer that converts GQL source text
//! into a stream of tokens. The lexer integrates with the diagnostic infrastructure
//! from Sprint 1 to provide rich error reporting.

pub mod keywords;
pub mod token;

use crate::diag::Diag;
use token::{Token, TokenKind};

/// Result of lexical analysis.
///
/// Contains both the tokens produced and any diagnostics encountered during scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerResult {
    /// The tokens produced, including an EOF token at the end.
    pub tokens: Vec<Token>,
    /// Diagnostics (errors, warnings) encountered during lexing.
    pub diagnostics: Vec<Diag>,
}

/// A lexical analyzer for GQL source text.
///
/// The lexer scans source text character by character and produces tokens.
/// It continues scanning after errors to provide comprehensive diagnostics.
pub struct Lexer<'a> {
    /// The source text being lexed.
    source: &'a str,
    /// Current byte position in source.
    pos: usize,
    /// Accumulated tokens.
    tokens: Vec<Token>,
    /// Accumulated diagnostics.
    diagnostics: Vec<Diag>,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Tokenizes the source text and returns the result.
    ///
    /// This consumes the lexer and returns both tokens and diagnostics.
    pub fn tokenize(mut self) -> LexerResult {
        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }
            self.scan_token();
        }

        // Always add EOF token
        let eof_pos = self.source.len();
        self.tokens
            .push(Token::new(TokenKind::Eof, eof_pos..eof_pos, ""));

        LexerResult {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    /// Scans a single token.
    fn scan_token(&mut self) {
        let start = self.pos;
        let ch = self.advance();

        match ch {
            // Single-character tokens
            '(' => self.add_token(TokenKind::LParen, start),
            ')' => self.add_token(TokenKind::RParen, start),
            '[' => self.add_token(TokenKind::LBracket, start),
            ']' => self.add_token(TokenKind::RBracket, start),
            '{' => self.add_token(TokenKind::LBrace, start),
            '}' => self.add_token(TokenKind::RBrace, start),
            ',' => self.add_token(TokenKind::Comma, start),
            ';' => self.add_token(TokenKind::Semicolon, start),
            '+' => self.add_token(TokenKind::Plus, start),
            '*' => self.add_token(TokenKind::Star, start),
            '/' => self.add_token(TokenKind::Slash, start),
            '%' => self.add_token(TokenKind::Percent, start),
            '^' => self.add_token(TokenKind::Caret, start),
            '&' => self.add_token(TokenKind::Ampersand, start),

            // Multi-character operators
            '-' => {
                if self.match_char('>') {
                    self.add_token(TokenKind::Arrow, start);
                } else {
                    self.add_token(TokenKind::Minus, start);
                }
            }
            '<' => {
                if self.match_char('-') {
                    self.add_token(TokenKind::LeftArrow, start);
                } else if self.match_char('=') {
                    self.add_token(TokenKind::LtEq, start);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::NotEq, start);
                } else if self.match_char('~') {
                    self.add_token(TokenKind::LeftTilde, start);
                } else {
                    self.add_token(TokenKind::Lt, start);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::GtEq, start);
                } else {
                    self.add_token(TokenKind::Gt, start);
                }
            }
            '=' => self.add_token(TokenKind::Eq, start),
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::NotEqBang, start);
                } else {
                    self.error(start, "unexpected character '!'");
                    // Error recovery: skip this character
                }
            }
            '~' => {
                if self.match_char('>') {
                    self.add_token(TokenKind::RightTilde, start);
                } else {
                    self.add_token(TokenKind::Tilde, start);
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenKind::DoublePipe, start);
                } else {
                    self.add_token(TokenKind::Pipe, start);
                }
            }
            ':' => {
                if self.match_char(':') {
                    self.add_token(TokenKind::DoubleColon, start);
                } else {
                    self.add_token(TokenKind::Colon, start);
                }
            }
            '.' => {
                if self.match_char('.') {
                    self.add_token(TokenKind::DotDot, start);
                } else {
                    self.add_token(TokenKind::Dot, start);
                }
            }

            // String literals
            '\'' => self.scan_string_literal(start),

            // Parameter tokens
            '$' => self.scan_parameter(start),

            // Delimited identifiers
            '`' => self.scan_delimited_identifier(start),

            // Numbers
            '0'..='9' => self.scan_number(start),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier_or_keyword(start),

            // Invalid character
            _ => {
                self.error(start, &format!("invalid character '{}'", ch));
                // Error recovery: skip this character and continue
            }
        }
    }

    /// Scans an identifier or keyword.
    fn scan_identifier_or_keyword(&mut self, start: usize) {
        while self.is_identifier_continue(self.peek()) {
            self.advance();
        }

        let text = &self.source[start..self.pos];

        // Check for temporal literals (keyword + whitespace + string)
        if self.try_scan_temporal_literal(text, start).is_some() {
            // Temporal literal was scanned
            return;
        }

        // Check if it's a keyword
        if let Some(kind) = keywords::lookup_keyword(text) {
            self.add_token(kind, start);
        } else {
            // Regular identifier
            self.add_token(TokenKind::Identifier(text.to_string()), start);
        }
    }

    /// Tries to scan a temporal literal (DATE 'yyyy-mm-dd', etc.).
    /// Returns Some(kind) if successful, None otherwise.
    fn try_scan_temporal_literal(&mut self, keyword: &str, keyword_start: usize) -> Option<()> {
        let upper = keyword.to_uppercase();
        if !matches!(upper.as_str(), "DATE" | "TIME" | "TIMESTAMP" | "DURATION") {
            return None;
        }

        // Save position in case this isn't a temporal literal
        let saved_pos = self.pos;

        // Skip whitespace
        while self.peek() == ' ' || self.peek() == '\t' {
            self.advance();
        }

        // Must be followed by a string literal
        if self.peek() == '\'' {
            self.advance(); // consume opening quote

            // Scan the string content
            let mut value = String::new();
            let mut valid = true;

            while self.peek() != '\'' && !self.is_at_end() {
                if self.peek() == '\n' {
                    // Temporal literals shouldn't span lines
                    valid = false;
                    break;
                }
                value.push(self.advance());
            }

            if !self.is_at_end() && self.peek() == '\'' {
                self.advance(); // consume closing quote

                if valid {
                    let kind = match upper.as_str() {
                        "DATE" => TokenKind::DateLiteral(value),
                        "TIME" => TokenKind::TimeLiteral(value),
                        "TIMESTAMP" => TokenKind::TimestampLiteral(value),
                        "DURATION" => TokenKind::DurationLiteral(value),
                        _ => unreachable!(),
                    };
                    self.add_token(kind, keyword_start);
                    return Some(());
                }
            }
        }

        // Not a temporal literal, restore position
        self.pos = saved_pos;
        None
    }

    /// Scans a string literal.
    fn scan_string_literal(&mut self, start: usize) {
        let mut value = String::new();
        let mut valid = true;

        while self.peek() != '\'' && !self.is_at_end() {
            if self.peek() == '\n' {
                // Allow multiline strings
            }

            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    break;
                }
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\'' => value.push('\''),
                    '\\' => value.push('\\'),
                    'u' => {
                        // Unicode escape sequence: \uXXXX
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if self.peek().is_ascii_hexdigit() {
                                hex.push(self.advance());
                            } else {
                                self.error(self.pos - 1, "invalid unicode escape sequence");
                                valid = false;
                                break;
                            }
                        }
                        if valid && let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                value.push(ch);
                            } else {
                                self.error(start, "invalid unicode code point");
                            }
                        }
                    }
                    _ => {
                        self.error(
                            self.pos - 1,
                            &format!("invalid escape sequence '\\{}'", escaped),
                        );
                        value.push(escaped);
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            self.error(start, "unclosed string literal");
            // Error recovery: synthesize closing quote
        } else {
            self.advance(); // consume closing quote
        }

        self.add_token(TokenKind::StringLiteral(value), start);
    }

    /// Scans a delimited identifier (backtick-quoted).
    fn scan_delimited_identifier(&mut self, start: usize) {
        let mut value = String::new();

        while self.peek() != '`' && !self.is_at_end() {
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    break;
                }
                let escaped = self.advance();
                match escaped {
                    '`' => value.push('`'),
                    '\\' => value.push('\\'),
                    _ => {
                        self.error(
                            self.pos - 1,
                            &format!(
                                "invalid escape sequence '\\{}' in delimited identifier",
                                escaped
                            ),
                        );
                        value.push(escaped);
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            self.error(start, "unclosed delimited identifier");
            // Error recovery: synthesize closing backtick
        } else {
            self.advance(); // consume closing backtick
        }

        self.add_token(TokenKind::DelimitedIdentifier(value), start);
    }

    /// Scans a number (integer or float).
    fn scan_number(&mut self, start: usize) {
        // Scan integer part
        while self.peek().is_ascii_digit() || self.peek() == '_' {
            self.advance();
        }

        // Check for float
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'

            // Scan fractional part
            while self.peek().is_ascii_digit() || self.peek() == '_' {
                self.advance();
            }

            // Check for exponent
            if matches!(self.peek(), 'e' | 'E') {
                self.advance();
                if matches!(self.peek(), '+' | '-') {
                    self.advance();
                }
                while self.peek().is_ascii_digit() || self.peek() == '_' {
                    self.advance();
                }
            }

            let text = &self.source[start..self.pos];
            self.add_token(TokenKind::FloatLiteral(text.to_string()), start);
        } else if matches!(self.peek(), 'e' | 'E') {
            // Integer with exponent is a float
            self.advance();
            if matches!(self.peek(), '+' | '-') {
                self.advance();
            }
            while self.peek().is_ascii_digit() || self.peek() == '_' {
                self.advance();
            }
            let text = &self.source[start..self.pos];
            self.add_token(TokenKind::FloatLiteral(text.to_string()), start);
        } else {
            let text = &self.source[start..self.pos];
            self.add_token(TokenKind::IntegerLiteral(text.to_string()), start);
        }

        let text = &self.source[start..self.pos];
        if !Self::is_valid_numeric_literal(text) {
            self.error_span(
                start..self.pos,
                &format!("malformed numeric literal '{}'", text),
                "L002",
            );
        }
    }

    /// Scans a parameter token ($name or $123).
    fn scan_parameter(&mut self, start: usize) {
        if self.is_at_end() {
            self.error(start, "unexpected end of input after '$'");
            return;
        }

        // Parameter can be $name or $123
        if self.peek().is_ascii_digit() {
            // Positional parameter
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        } else if self.is_identifier_start(self.peek()) {
            // Named parameter
            while self.is_identifier_continue(self.peek()) {
                self.advance();
            }
        } else {
            self.error(start, "expected identifier or number after '$'");
            return;
        }

        let text = &self.source[start + 1..self.pos]; // Skip the '$'
        self.add_token(TokenKind::Parameter(text.to_string()), start);
    }

    /// Skips whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        // Single-line comment
                        self.advance(); // consume first '/'
                        self.advance(); // consume second '/'
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else if self.peek_next() == '*' {
                        // Block comment
                        let comment_start = self.pos;
                        self.advance(); // consume '/'
                        self.advance(); // consume '*'

                        let mut depth = 1;
                        while depth > 0 && !self.is_at_end() {
                            if self.peek() == '/' && self.peek_next() == '*' {
                                // Nested block comment
                                self.advance();
                                self.advance();
                                depth += 1;
                            } else if self.peek() == '*' && self.peek_next() == '/' {
                                // End of block comment
                                self.advance();
                                self.advance();
                                depth -= 1;
                            } else {
                                self.advance();
                            }
                        }

                        if depth > 0 {
                            self.error(comment_start, "unclosed block comment");
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    /// Returns true if the character can start an identifier.
    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    /// Returns true if the character can continue an identifier.
    fn is_identifier_continue(&self, ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    /// Adds a token to the token stream.
    fn add_token(&mut self, kind: TokenKind, start: usize) {
        let text = self.source[start..self.pos].to_string();
        let span = start..self.pos;
        self.tokens.push(Token::new(kind, span, text));
    }

    /// Adds an error diagnostic.
    fn error(&mut self, pos: usize, message: &str) {
        let span = pos..pos.saturating_add(1).min(self.source.len());
        self.error_span(span, message, "L001");
    }

    /// Adds an error diagnostic with an explicit span and code.
    fn error_span(&mut self, span: std::ops::Range<usize>, message: &str, code: &str) {
        self.diagnostics.push(
            Diag::error(message)
                .with_primary_label(span, "here")
                .with_code(code),
        );
    }

    /// Returns true if a scanned numeric literal has valid GQL-style separators/exponent.
    fn is_valid_numeric_literal(text: &str) -> bool {
        let (mantissa, exponent) = match text.char_indices().find(|(_, ch)| matches!(ch, 'e' | 'E'))
        {
            Some((index, _)) => (&text[..index], Some(&text[index + 1..])),
            None => (text, None),
        };

        if !Self::is_valid_mantissa(mantissa) {
            return false;
        }

        if let Some(exponent) = exponent {
            let exponent = if let Some(stripped) = exponent.strip_prefix('+') {
                stripped
            } else if let Some(stripped) = exponent.strip_prefix('-') {
                stripped
            } else {
                exponent
            };

            if exponent.is_empty() || !Self::is_valid_digit_group(exponent) {
                return false;
            }
        }

        true
    }

    fn is_valid_mantissa(mantissa: &str) -> bool {
        if let Some((integer, fraction)) = mantissa.split_once('.') {
            !integer.is_empty()
                && !fraction.is_empty()
                && Self::is_valid_digit_group(integer)
                && Self::is_valid_digit_group(fraction)
        } else {
            Self::is_valid_digit_group(mantissa)
        }
    }

    fn is_valid_digit_group(group: &str) -> bool {
        if group.is_empty() {
            return false;
        }

        let mut prev_was_underscore = false;
        let mut saw_digit = false;

        for ch in group.chars() {
            match ch {
                '0'..='9' => {
                    saw_digit = true;
                    prev_was_underscore = false;
                }
                '_' => {
                    if !saw_digit || prev_was_underscore {
                        return false;
                    }
                    prev_was_underscore = true;
                }
                _ => return false,
            }
        }

        saw_digit && !prev_was_underscore
    }

    /// Returns the current character without advancing.
    fn peek(&self) -> char {
        self.source[self.pos..].chars().next().unwrap_or('\0')
    }

    /// Returns the next character without advancing.
    fn peek_next(&self) -> char {
        let mut chars = self.source[self.pos..].chars();
        chars.next();
        chars.next().unwrap_or('\0')
    }

    /// Advances and returns the current character.
    fn advance(&mut self) -> char {
        let ch = self.peek();
        if ch != '\0' {
            self.pos += ch.len_utf8();
        }
        ch
    }

    /// Matches and consumes a character if it matches the expected one.
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Returns true if at end of input.
    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }
}

/// Convenience function to tokenize a source string.
///
/// This is the main entry point for lexical analysis.
pub fn tokenize(source: &str) -> LexerResult {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let result = tokenize("");
        assert_eq!(result.tokens.len(), 1); // Just EOF
        assert_eq!(result.tokens[0].kind, TokenKind::Eof);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let result = tokenize("   \t\n  ");
        assert_eq!(result.tokens.len(), 1); // Just EOF
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn single_keyword() {
        let result = tokenize("MATCH");
        assert_eq!(result.tokens.len(), 2); // MATCH + EOF
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.tokens[0].text, "MATCH");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn keyword_case_insensitive() {
        let result = tokenize("match Match MATCH MaTcH");
        assert_eq!(result.tokens.len(), 5); // 4 keywords + EOF
        for i in 0..4 {
            assert_eq!(result.tokens[i].kind, TokenKind::Match);
        }
    }

    #[test]
    fn detach_keywords_are_standalone_tokens() {
        let result = tokenize("DETACH DELETE NODETACH DELETE");
        assert_eq!(result.tokens.len(), 5);
        assert_eq!(result.tokens[0].kind, TokenKind::Detach);
        assert_eq!(result.tokens[1].kind, TokenKind::Delete);
        assert_eq!(result.tokens[2].kind, TokenKind::Nodetach);
        assert_eq!(result.tokens[3].kind, TokenKind::Delete);
    }

    #[test]
    fn identifier() {
        let result = tokenize("myVar _test foo123");
        assert_eq!(result.tokens.len(), 4); // 3 identifiers + EOF
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::Identifier("myVar".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::Identifier("_test".to_string())
        );
        assert_eq!(
            result.tokens[2].kind,
            TokenKind::Identifier("foo123".to_string())
        );
    }

    #[test]
    fn delimited_identifier() {
        let result = tokenize("`my var` `test\\`escaped`");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::DelimitedIdentifier("my var".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::DelimitedIdentifier("test`escaped".to_string())
        );
    }

    #[test]
    fn string_literal() {
        let result = tokenize("'hello' 'world'");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::StringLiteral("hello".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::StringLiteral("world".to_string())
        );
    }

    #[test]
    fn string_with_escapes() {
        let result = tokenize(r"'hello\nworld' 'test\'quote'");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::StringLiteral("hello\nworld".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::StringLiteral("test'quote".to_string())
        );
    }

    #[test]
    fn integer_literals() {
        let result = tokenize("42 0 1000 1_000_000");
        assert_eq!(result.tokens.len(), 5);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::IntegerLiteral("42".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::IntegerLiteral("0".to_string())
        );
        assert_eq!(
            result.tokens[2].kind,
            TokenKind::IntegerLiteral("1000".to_string())
        );
        assert_eq!(
            result.tokens[3].kind,
            TokenKind::IntegerLiteral("1_000_000".to_string())
        );
    }

    #[test]
    fn float_literals() {
        let result = tokenize("3.14 1.0e10 2.5E-3");
        assert_eq!(result.tokens.len(), 4);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::FloatLiteral("3.14".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::FloatLiteral("1.0e10".to_string())
        );
        assert_eq!(
            result.tokens[2].kind,
            TokenKind::FloatLiteral("2.5E-3".to_string())
        );
    }

    #[test]
    fn operators() {
        let result = tokenize("+ - * / % ^");
        assert_eq!(result.tokens.len(), 7);
        assert_eq!(result.tokens[0].kind, TokenKind::Plus);
        assert_eq!(result.tokens[1].kind, TokenKind::Minus);
        assert_eq!(result.tokens[2].kind, TokenKind::Star);
        assert_eq!(result.tokens[3].kind, TokenKind::Slash);
        assert_eq!(result.tokens[4].kind, TokenKind::Percent);
        assert_eq!(result.tokens[5].kind, TokenKind::Caret);
    }

    #[test]
    fn comparison_operators() {
        let result = tokenize("= <> != < > <= >=");
        assert_eq!(result.tokens.len(), 8);
        assert_eq!(result.tokens[0].kind, TokenKind::Eq);
        assert_eq!(result.tokens[1].kind, TokenKind::NotEq);
        assert_eq!(result.tokens[2].kind, TokenKind::NotEqBang);
        assert_eq!(result.tokens[3].kind, TokenKind::Lt);
        assert_eq!(result.tokens[4].kind, TokenKind::Gt);
        assert_eq!(result.tokens[5].kind, TokenKind::LtEq);
        assert_eq!(result.tokens[6].kind, TokenKind::GtEq);
    }

    #[test]
    fn arrow_operators() {
        let result = tokenize("-> <- ~ <~ ~>");
        assert_eq!(result.tokens.len(), 6);
        assert_eq!(result.tokens[0].kind, TokenKind::Arrow);
        assert_eq!(result.tokens[1].kind, TokenKind::LeftArrow);
        assert_eq!(result.tokens[2].kind, TokenKind::Tilde);
        assert_eq!(result.tokens[3].kind, TokenKind::LeftTilde);
        assert_eq!(result.tokens[4].kind, TokenKind::RightTilde);
    }

    #[test]
    fn punctuation() {
        let result = tokenize("( ) [ ] { } , ; . :");
        assert_eq!(result.tokens.len(), 11);
        assert_eq!(result.tokens[0].kind, TokenKind::LParen);
        assert_eq!(result.tokens[1].kind, TokenKind::RParen);
        assert_eq!(result.tokens[2].kind, TokenKind::LBracket);
        assert_eq!(result.tokens[3].kind, TokenKind::RBracket);
        assert_eq!(result.tokens[4].kind, TokenKind::LBrace);
        assert_eq!(result.tokens[5].kind, TokenKind::RBrace);
        assert_eq!(result.tokens[6].kind, TokenKind::Comma);
        assert_eq!(result.tokens[7].kind, TokenKind::Semicolon);
        assert_eq!(result.tokens[8].kind, TokenKind::Dot);
        assert_eq!(result.tokens[9].kind, TokenKind::Colon);
    }

    #[test]
    fn parameters() {
        let result = tokenize("$name $1 $param_123");
        assert_eq!(result.tokens.len(), 4);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::Parameter("name".to_string())
        );
        assert_eq!(result.tokens[1].kind, TokenKind::Parameter("1".to_string()));
        assert_eq!(
            result.tokens[2].kind,
            TokenKind::Parameter("param_123".to_string())
        );
    }

    #[test]
    fn boolean_literals() {
        let result = tokenize("TRUE FALSE true false");
        assert_eq!(result.tokens.len(), 5);
        assert_eq!(result.tokens[0].kind, TokenKind::True);
        assert_eq!(result.tokens[1].kind, TokenKind::False);
        assert_eq!(result.tokens[2].kind, TokenKind::True);
        assert_eq!(result.tokens[3].kind, TokenKind::False);
    }

    #[test]
    fn null_literals() {
        let result = tokenize("NULL UNKNOWN null");
        assert_eq!(result.tokens.len(), 4);
        assert_eq!(result.tokens[0].kind, TokenKind::Null);
        assert_eq!(result.tokens[1].kind, TokenKind::Unknown);
        assert_eq!(result.tokens[2].kind, TokenKind::Null);
    }

    #[test]
    fn temporal_literals() {
        let result = tokenize("DATE '2024-01-15' TIME '14:30:00' TIMESTAMP '2024-01-15T14:30:00'");
        assert_eq!(result.tokens.len(), 4);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::DateLiteral("2024-01-15".to_string())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::TimeLiteral("14:30:00".to_string())
        );
        assert_eq!(
            result.tokens[2].kind,
            TokenKind::TimestampLiteral("2024-01-15T14:30:00".to_string())
        );
    }

    #[test]
    fn single_line_comment() {
        let result = tokenize("MATCH // this is a comment\nRETURN");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.tokens[1].kind, TokenKind::Return);
    }

    #[test]
    fn block_comment() {
        let result = tokenize("MATCH /* comment */ RETURN");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.tokens[1].kind, TokenKind::Return);
    }

    #[test]
    fn nested_block_comment() {
        let result = tokenize("MATCH /* outer /* inner */ outer */ RETURN");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.tokens[1].kind, TokenKind::Return);
    }

    #[test]
    fn error_unclosed_string() {
        let result = tokenize("'unclosed");
        assert_eq!(result.tokens.len(), 2); // String token + EOF
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::StringLiteral("unclosed".to_string())
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert!(result.diagnostics[0].message.contains("unclosed string"));
    }

    #[test]
    fn error_invalid_character() {
        let result = tokenize("@ # £");
        assert_eq!(result.diagnostics.len(), 3);
    }

    #[test]
    fn error_invalid_escape() {
        let result = tokenize(r"'test\x'");
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(result.diagnostics[0].message.contains("invalid escape"));
    }

    #[test]
    fn error_malformed_numbers() {
        let result = tokenize("1e 1e+ 1__2 1_ 1_.2 1e1_");
        assert_eq!(result.diagnostics.len(), 6);
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("malformed numeric literal"))
        );
    }

    #[test]
    fn complex_query() {
        let result = tokenize("MATCH (n:Person {name: 'Alice'}) RETURN n.age");
        assert!(result.tokens.len() > 10);
        assert!(result.diagnostics.is_empty());
    }
}
