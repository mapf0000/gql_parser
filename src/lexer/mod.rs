//! Lexical analysis for GQL.
//!
//! This module implements a robust, error-tolerant lexer using `logos`.

pub mod keywords;
pub mod token;

use crate::diag::Diag;
use logos::{Lexer as LogosLexer, Logos, Skip};
use smol_str::SmolStr;
use token::{Token, TokenKind};

/// Result of lexical analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerResult {
    /// The tokens produced, including an EOF token at the end.
    pub tokens: Vec<Token>,
    /// Diagnostics (errors, warnings) encountered during lexing.
    pub diagnostics: Vec<Diag>,
}

/// A lexical analyzer for GQL source text.
pub struct Lexer<'a> {
    source: &'a str,
}

#[derive(Debug, Default)]
struct LexExtras {
    diagnostics: Vec<Diag>,
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(extras = LexExtras)]
enum RawToken {
    #[regex(r"[ \t\r\n\f]+", logos::skip)]
    Whitespace,

    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,

    #[token("/*", lex_nested_block_comment)]
    BlockComment,

    // Multi-character operators and punctuation
    #[token("->")]
    Arrow,
    #[token("<-")]
    LeftArrow,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<>")]
    NotEq,
    #[token("!=")]
    NotEqBang,
    #[token("<~")]
    LeftTilde,
    #[token("~>")]
    RightTilde,
    #[token("||")]
    DoublePipe,
    #[token("::")]
    DoubleColon,
    #[token("..")]
    DotDot,

    // Single-character operators and punctuation
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("=")]
    Eq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("~")]
    Tilde,
    #[token("|")]
    Pipe,
    #[token("&")]
    Ampersand,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,

    #[regex(r"\$\$[A-Za-z_][A-Za-z0-9_]*")]
    ReferenceParameter,

    #[regex(r"\$[A-Za-z_][A-Za-z0-9_]*|\$[0-9]+")]
    Parameter,

    // Closed and unclosed variants are handled in post-processing.
    #[regex(r"`(?:\\.|[^`\\])*`?")]
    DelimitedIdentifier,
    #[regex(r"'(?:\\.|[^'\\])*'?")]
    StringLiteral,

    #[regex(r"[0-9](?:[0-9_]*)(?:\.[0-9_]+(?:[eE][+-]?[0-9_]+)?|[eE][+-]?[0-9_]+)?")]
    Number,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    IdentifierOrKeyword,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given source text.
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Tokenizes the source text and returns the result.
    pub fn tokenize(self) -> LexerResult {
        let mut tokens = Vec::new();
        let mut diagnostics = Vec::new();

        let mut lexer = RawToken::lexer_with_extras(self.source, LexExtras::default());

        while let Some(item) = lexer.next() {
            let span = lexer.span();
            match item {
                Ok(raw) => {
                    if let Some(token) = self.map_raw_token(raw, span.clone(), &mut diagnostics) {
                        tokens.push(token);
                    }
                }
                Err(()) => {
                    let span = normalize_span(span, self.source.len());
                    let slice = &self.source[span.clone()];
                    let ch = slice.chars().next().unwrap_or('\0');
                    diagnostics.push(
                        Diag::error(format!("invalid character '{ch}'"))
                            .with_primary_label(span, "here")
                            .with_code("L001"),
                    );
                }
            }
        }

        diagnostics.extend(lexer.extras.diagnostics);

        let eof_pos = self.source.len();
        tokens.push(Token::new(TokenKind::Eof, eof_pos..eof_pos));

        LexerResult {
            tokens,
            diagnostics,
        }
    }

    fn map_raw_token(
        &self,
        raw: RawToken,
        span: std::ops::Range<usize>,
        diagnostics: &mut Vec<Diag>,
    ) -> Option<Token> {
        let token = match raw {
            RawToken::Whitespace | RawToken::LineComment | RawToken::BlockComment => return None,
            RawToken::Arrow => Token::new(TokenKind::Arrow, span),
            RawToken::LeftArrow => Token::new(TokenKind::LeftArrow, span),
            RawToken::LtEq => Token::new(TokenKind::LtEq, span),
            RawToken::GtEq => Token::new(TokenKind::GtEq, span),
            RawToken::NotEq => Token::new(TokenKind::NotEq, span),
            RawToken::NotEqBang => Token::new(TokenKind::NotEqBang, span),
            RawToken::LeftTilde => Token::new(TokenKind::LeftTilde, span),
            RawToken::RightTilde => Token::new(TokenKind::RightTilde, span),
            RawToken::DoublePipe => Token::new(TokenKind::DoublePipe, span),
            RawToken::DoubleColon => Token::new(TokenKind::DoubleColon, span),
            RawToken::DotDot => Token::new(TokenKind::DotDot, span),
            RawToken::Plus => Token::new(TokenKind::Plus, span),
            RawToken::Minus => Token::new(TokenKind::Minus, span),
            RawToken::Star => Token::new(TokenKind::Star, span),
            RawToken::Slash => Token::new(TokenKind::Slash, span),
            RawToken::Percent => Token::new(TokenKind::Percent, span),
            RawToken::Caret => Token::new(TokenKind::Caret, span),
            RawToken::Eq => Token::new(TokenKind::Eq, span),
            RawToken::Lt => Token::new(TokenKind::Lt, span),
            RawToken::Gt => Token::new(TokenKind::Gt, span),
            RawToken::Tilde => Token::new(TokenKind::Tilde, span),
            RawToken::Pipe => Token::new(TokenKind::Pipe, span),
            RawToken::Ampersand => Token::new(TokenKind::Ampersand, span),
            RawToken::LParen => Token::new(TokenKind::LParen, span),
            RawToken::RParen => Token::new(TokenKind::RParen, span),
            RawToken::LBracket => Token::new(TokenKind::LBracket, span),
            RawToken::RBracket => Token::new(TokenKind::RBracket, span),
            RawToken::LBrace => Token::new(TokenKind::LBrace, span),
            RawToken::RBrace => Token::new(TokenKind::RBrace, span),
            RawToken::Comma => Token::new(TokenKind::Comma, span),
            RawToken::Semicolon => Token::new(TokenKind::Semicolon, span),
            RawToken::Dot => Token::new(TokenKind::Dot, span),
            RawToken::Colon => Token::new(TokenKind::Colon, span),
            RawToken::ReferenceParameter => {
                let value = self.source[span.start + 2..span.end].into();
                Token::new(TokenKind::ReferenceParameter(value), span)
            }
            RawToken::Parameter => {
                let value = self.source[span.start + 1..span.end].into();
                Token::new(TokenKind::Parameter(value), span)
            }
            RawToken::DelimitedIdentifier => {
                let value = decode_delimited_identifier(
                    &self.source[span.clone()],
                    span.start,
                    diagnostics,
                );
                Token::new(TokenKind::DelimitedIdentifier(value), span)
            }
            RawToken::StringLiteral => {
                let value =
                    decode_string_literal(&self.source[span.clone()], span.start, diagnostics);
                Token::new(TokenKind::StringLiteral(value), span)
            }
            RawToken::Number => {
                let text = &self.source[span.clone()];
                if !is_valid_numeric_literal(text) {
                    diagnostics.push(
                        Diag::error(format!("malformed numeric literal '{text}'"))
                            .with_primary_label(span.clone(), "here")
                            .with_code("L002"),
                    );
                }

                let kind = if text.contains('.') || text.contains('e') || text.contains('E') {
                    TokenKind::FloatLiteral(text.into())
                } else {
                    TokenKind::IntegerLiteral(text.into())
                };
                Token::new(kind, span)
            }
            RawToken::IdentifierOrKeyword => {
                let text = &self.source[span.clone()];
                let kind = keywords::lookup_keyword(text)
                    .unwrap_or_else(|| TokenKind::Identifier(text.into()));
                Token::new(kind, span)
            }
        };

        Some(token)
    }
}

fn lex_nested_block_comment(lex: &mut LogosLexer<'_, RawToken>) -> Skip {
    let mut depth = 1usize;
    let remainder = lex.remainder();
    let mut iter = remainder.char_indices().peekable();

    while let Some((_, ch)) = iter.next() {
        if ch == '/' {
            if let Some((_, '*')) = iter.peek().copied() {
                depth += 1;
                iter.next();
            }
            continue;
        }

        if ch == '*'
            && let Some((next_idx, '/')) = iter.peek().copied()
        {
            depth -= 1;
            iter.next();
            if depth == 0 {
                let consumed = next_idx + '/'.len_utf8();
                lex.bump(consumed);
                return logos::Skip;
            }
        }
    }

    // Unterminated block comment: consume the rest and emit a diagnostic.
    lex.bump(remainder.len());
    let start = lex.span().start;
    let end = lex.source().len();
    lex.extras.diagnostics.push(
        Diag::error("unclosed block comment")
            .with_primary_label(start..end, "here")
            .with_code("L001"),
    );

    logos::Skip
}

fn decode_string_literal(raw: &str, span_start: usize, diagnostics: &mut Vec<Diag>) -> SmolStr {
    let closed = raw.len() >= 2 && raw.ends_with('\'');
    let content_end = if closed { raw.len() - 1 } else { raw.len() };

    if !closed {
        diagnostics.push(
            Diag::error("unclosed string literal")
                .with_primary_label(span_start..span_start + raw.len(), "here")
                .with_code("L001"),
        );
    }

    let content = if !raw.is_empty() {
        &raw[1..content_end]
    } else {
        ""
    };

    let mut out = String::new();
    let mut chars = content.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some((escape_idx, escaped)) = chars.next() else {
            break;
        };

        match escaped {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '\'' => out.push('\''),
            '\\' => out.push('\\'),
            'u' => {
                let mut hex = String::new();
                for _ in 0..4 {
                    if let Some((_, h)) = chars.next() {
                        if h.is_ascii_hexdigit() {
                            hex.push(h);
                        } else {
                            diagnostics.push(
                                Diag::error("invalid unicode escape sequence")
                                    .with_primary_label(
                                        span_start + 1 + escape_idx
                                            ..span_start + 1 + escape_idx + 1,
                                        "here",
                                    )
                                    .with_code("L001"),
                            );
                            break;
                        }
                    } else {
                        diagnostics.push(
                            Diag::error("invalid unicode escape sequence")
                                .with_primary_label(
                                    span_start + 1 + escape_idx..span_start + 1 + escape_idx + 1,
                                    "here",
                                )
                                .with_code("L001"),
                        );
                        break;
                    }
                }

                if hex.len() == 4
                    && let Ok(code) = u32::from_str_radix(&hex, 16)
                {
                    if let Some(codepoint) = char::from_u32(code) {
                        out.push(codepoint);
                    } else {
                        diagnostics.push(
                            Diag::error("invalid unicode code point")
                                .with_primary_label(
                                    span_start + 1 + escape_idx..span_start + 1 + escape_idx + 1,
                                    "here",
                                )
                                .with_code("L001"),
                        );
                    }
                }
            }
            other => {
                diagnostics.push(
                    Diag::error(format!("invalid escape sequence '\\{other}'"))
                        .with_primary_label(
                            span_start + 1 + escape_idx..span_start + 1 + escape_idx + 1,
                            "here",
                        )
                        .with_code("L001"),
                );
                out.push(other);
            }
        }
    }

    out.into()
}

fn decode_delimited_identifier(
    raw: &str,
    span_start: usize,
    diagnostics: &mut Vec<Diag>,
) -> SmolStr {
    let closed = raw.len() >= 2 && raw.ends_with('`');
    let content_end = if closed { raw.len() - 1 } else { raw.len() };

    if !closed {
        diagnostics.push(
            Diag::error("unclosed delimited identifier")
                .with_primary_label(span_start..span_start + raw.len(), "here")
                .with_code("L001"),
        );
    }

    let content = if !raw.is_empty() {
        &raw[1..content_end]
    } else {
        ""
    };

    let mut out = String::new();
    let mut chars = content.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some((escape_idx, escaped)) = chars.next() else {
            break;
        };

        match escaped {
            '`' => out.push('`'),
            '\\' => out.push('\\'),
            other => {
                diagnostics.push(
                    Diag::error(format!(
                        "invalid escape sequence '\\{other}' in delimited identifier"
                    ))
                    .with_primary_label(
                        span_start + 1 + escape_idx..span_start + 1 + escape_idx + 1,
                        "here",
                    )
                    .with_code("L001"),
                );
                out.push(other);
            }
        }
    }

    out.into()
}

fn normalize_span(span: std::ops::Range<usize>, len: usize) -> std::ops::Range<usize> {
    let start = span.start.min(len);
    let mut end = span.end.min(len);
    if end <= start {
        end = (start + 1).min(len);
    }
    start..end
}

fn is_valid_numeric_literal(text: &str) -> bool {
    let (mantissa, exponent) = match text.char_indices().find(|(_, ch)| matches!(ch, 'e' | 'E')) {
        Some((index, _)) => (&text[..index], Some(&text[index + 1..])),
        None => (text, None),
    };

    if !is_valid_mantissa(mantissa) {
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

        if exponent.is_empty() || !is_valid_digit_group(exponent) {
            return false;
        }
    }

    true
}

fn is_valid_mantissa(mantissa: &str) -> bool {
    if let Some((integer, fraction)) = mantissa.split_once('.') {
        !integer.is_empty()
            && !fraction.is_empty()
            && is_valid_digit_group(integer)
            && is_valid_digit_group(fraction)
    } else {
        is_valid_digit_group(mantissa)
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

/// Convenience function to tokenize a source string.
pub fn tokenize(source: &str) -> LexerResult {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let result = tokenize("");
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].kind, TokenKind::Eof);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn keyword_case_insensitive() {
        let result = tokenize("match Match MATCH MaTcH");
        assert_eq!(result.tokens.len(), 5);
        for token in result.tokens.iter().take(4) {
            assert_eq!(token.kind, TokenKind::Match);
        }
    }

    #[test]
    fn identifiers_and_keywords() {
        let result = tokenize("myVar MATCH _test");
        assert_eq!(result.tokens.len(), 4);
        assert_eq!(result.tokens[0].kind, TokenKind::Identifier("myVar".into()));
        assert_eq!(result.tokens[1].kind, TokenKind::Match);
        assert_eq!(result.tokens[2].kind, TokenKind::Identifier("_test".into()));
    }

    #[test]
    fn delimited_identifier_and_string() {
        let result = tokenize("`my var` 'hello\\nworld'");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::DelimitedIdentifier("my var".into())
        );
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::StringLiteral("hello\nworld".into())
        );
    }

    #[test]
    fn numeric_literals_and_validation() {
        let valid = tokenize("42 3.14 1e10 1_000_000");
        assert!(valid.diagnostics.is_empty());

        let invalid = tokenize("1e 1__2 1_ 1e1_");
        assert!(invalid.diagnostics.len() >= 3);
        assert!(
            invalid
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("malformed numeric literal"))
        );
    }

    #[test]
    fn parameters() {
        let result = tokenize("$name $1");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].kind, TokenKind::Parameter("name".into()));
        assert_eq!(result.tokens[1].kind, TokenKind::Parameter("1".into()));
    }

    #[test]
    fn comments_skipped_including_nested_block() {
        let result = tokenize("MATCH /* outer /* inner */ outer */ RETURN");
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.tokens[1].kind, TokenKind::Return);
    }

    #[test]
    fn unterminated_block_comment_reports_error() {
        let result = tokenize("MATCH /* comment");
        assert_eq!(result.tokens[0].kind, TokenKind::Match);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(
            result.diagnostics[0]
                .message
                .contains("unclosed block comment")
        );
    }

    #[test]
    fn unterminated_string_reports_error() {
        let result = tokenize("'unclosed");
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(
            result.tokens[0].kind,
            TokenKind::StringLiteral("unclosed".into())
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert!(result.diagnostics[0].message.contains("unclosed string"));
    }

    #[test]
    fn invalid_character_reports_error() {
        let result = tokenize("@ # £");
        assert_eq!(result.diagnostics.len(), 3);
    }

    #[test]
    fn temporal_literals_are_tokenized_structurally() {
        let result = tokenize("DATE '2024-01-15' TIME '14:30:00'");
        assert_eq!(result.tokens.len(), 5);
        assert_eq!(result.tokens[0].kind, TokenKind::Date);
        assert_eq!(
            result.tokens[1].kind,
            TokenKind::StringLiteral("2024-01-15".into())
        );
        assert_eq!(result.tokens[2].kind, TokenKind::Time);
        assert_eq!(
            result.tokens[3].kind,
            TokenKind::StringLiteral("14:30:00".into())
        );
    }
}
