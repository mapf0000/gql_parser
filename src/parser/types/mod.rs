//! Type parsing for GQL.
//!
//! This module implements comprehensive parsing of the GQL type system,
//! including predefined types (boolean, numeric, string, temporal),
//! reference types (graph, node, edge, binding table), and constructed
//! types (path, list, record).
//!
//! # Type Grammar Overview
//!
//! ```text
//! value_type ::=
//!     | predefined_type
//!     | path_value_type
//!     | list_value_type
//!     | record_type
//!
//! predefined_type ::=
//!     | boolean_type
//!     | character_string_type
//!     | byte_string_type
//!     | numeric_type
//!     | temporal_type
//!     | reference_value_type
//!     | immaterial_value_type
//! ```

use crate::ast::{
    BindingTableReferenceValueType, BooleanType, ByteStringType, CharacterStringType,
    EdgeReferenceValueType, GraphReferenceValueType, ImmaterialValueType, ListSyntaxForm,
    ListValueType, NodeReferenceValueType, NumericType, PathValueType, PredefinedType, RecordType,
    ReferenceValueType, Span, TemporalType, ValueType,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::base::{ParseError, ParseResult, TokenStream};

mod constructed;
mod predefined;
mod reference;

/// Parser for type specifications.
pub struct TypeParser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> TypeParser<'a> {
    /// Creates a new type parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            stream: TokenStream::new(tokens),
        }
    }

    /// Creates an error at the current position with the P_TYPE code.
    fn error_here(&self, message: impl Into<String>) -> ParseError {
        self.stream.error_here_with_code(message, "P_TYPE")
    }

    /// Parses a value type (entry point).
    ///
    /// # Grammar
    ///
    /// ```text
    /// value_type ::=
    ///     | predefined_type
    ///     | path_value_type
    ///     | list_value_type
    ///     | record_type
    /// ```
    pub fn parse_value_type(&mut self) -> ParseResult<ValueType> {
        let mut value_type = self.parse_base_value_type()?;

        // Postfix list forms:
        //   value_type LIST
        //   value_type ARRAY
        while matches!(
            self.stream.current().kind,
            TokenKind::List | TokenKind::Array
        ) {
            let syntax_form = match self.stream.current().kind {
                TokenKind::List => ListSyntaxForm::PostfixList,
                TokenKind::Array => ListSyntaxForm::PostfixArray,
                _ => unreachable!("guarded by matches! above"),
            };
            let start = value_type.span().start;
            self.stream.advance();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            value_type = ValueType::List(ListValueType {
                element_type: Box::new(value_type),
                syntax_form,
                span: start..end,
            });
        }

        Ok(value_type)
    }

    /// Parses a value type without consuming postfix LIST/ARRAY suffixes.
    fn parse_base_value_type(&mut self) -> ParseResult<ValueType> {
        let start = self.stream.current().span.start;

        match &self.stream.current().kind {
            // Boolean types
            TokenKind::Bool | TokenKind::Boolean => {
                let bool_type = self.parse_boolean_type()?;
                Ok(self.wrap_predefined(PredefinedType::Boolean(bool_type), start))
            }

            // Character string types
            TokenKind::String | TokenKind::Char | TokenKind::Varchar => {
                let char_type = self.parse_character_string_type()?;
                Ok(self.wrap_predefined(PredefinedType::CharacterString(char_type), start))
            }

            // Byte string types
            TokenKind::Bytes | TokenKind::Binary | TokenKind::Varbinary => {
                let byte_type = self.parse_byte_string_type()?;
                Ok(self.wrap_predefined(PredefinedType::ByteString(byte_type), start))
            }

            // Numeric types
            TokenKind::Int
            | TokenKind::Integer
            | TokenKind::Int8
            | TokenKind::Int16
            | TokenKind::Int32
            | TokenKind::Int64
            | TokenKind::Int128
            | TokenKind::Int256
            | TokenKind::Smallint
            | TokenKind::Bigint
            | TokenKind::Signed
            | TokenKind::Uint
            | TokenKind::Uint8
            | TokenKind::Uint16
            | TokenKind::Uint32
            | TokenKind::Uint64
            | TokenKind::Uint128
            | TokenKind::Uint256
            | TokenKind::Usmallint
            | TokenKind::Ubigint
            | TokenKind::Unsigned
            | TokenKind::Decimal
            | TokenKind::Dec
            | TokenKind::Float
            | TokenKind::Float16
            | TokenKind::Float32
            | TokenKind::Float64
            | TokenKind::Float128
            | TokenKind::Float256
            | TokenKind::Real
            | TokenKind::Double => {
                let num_type = self.parse_numeric_type()?;
                Ok(self.wrap_predefined(PredefinedType::Numeric(num_type), start))
            }

            // Temporal types
            TokenKind::Date
            | TokenKind::Time
            | TokenKind::Timestamp
            | TokenKind::Duration
            | TokenKind::Zoned
            | TokenKind::Local => {
                let temp_type = self.parse_temporal_type()?;
                Ok(self.wrap_predefined(PredefinedType::Temporal(temp_type), start))
            }

            // Immaterial types
            TokenKind::Null | TokenKind::Nothing => {
                let imm_type = self.parse_immaterial_value_type()?;
                Ok(self.wrap_predefined(PredefinedType::Immaterial(imm_type), start))
            }

            // Reference value types
            TokenKind::Property | TokenKind::Graph => {
                let ref_type = self.parse_graph_reference_value_type()?;
                Ok(self.wrap_predefined(
                    PredefinedType::ReferenceValue(ReferenceValueType::Graph(ref_type)),
                    start,
                ))
            }

            TokenKind::Binding | TokenKind::Table => {
                let ref_type = self.parse_binding_table_reference_value_type()?;
                Ok(self.wrap_predefined(
                    PredefinedType::ReferenceValue(ReferenceValueType::BindingTable(ref_type)),
                    start,
                ))
            }

            TokenKind::Node | TokenKind::Vertex => {
                let ref_type = self.parse_node_reference_value_type()?;
                Ok(self.wrap_predefined(
                    PredefinedType::ReferenceValue(ReferenceValueType::Node(ref_type)),
                    start,
                ))
            }

            TokenKind::Edge | TokenKind::Relationship => {
                let ref_type = self.parse_edge_reference_value_type()?;
                Ok(self.wrap_predefined(
                    PredefinedType::ReferenceValue(ReferenceValueType::Edge(ref_type)),
                    start,
                ))
            }

            // Path type
            TokenKind::Path => Ok(ValueType::Path(self.parse_path_value_type()?)),

            // List types (prefix form: LIST<T>, ARRAY<T>)
            TokenKind::List | TokenKind::Array => {
                Ok(ValueType::List(self.parse_list_value_type()?))
            }

            // Record types
            TokenKind::Record => Ok(ValueType::Record(self.parse_record_type()?)),

            // ANY could be "ANY PROPERTY GRAPH", "ANY NODE", "ANY EDGE", or "ANY RECORD"
            TokenKind::Any => {
                if let Some(next) = self.stream.peek() {
                    match &next.kind {
                        TokenKind::Property | TokenKind::Graph => {
                            let ref_type = self.parse_graph_reference_value_type()?;
                            Ok(self.wrap_predefined(
                                PredefinedType::ReferenceValue(ReferenceValueType::Graph(ref_type)),
                                start,
                            ))
                        }
                        TokenKind::Node | TokenKind::Vertex => {
                            let ref_type = self.parse_node_reference_value_type()?;
                            Ok(self.wrap_predefined(
                                PredefinedType::ReferenceValue(ReferenceValueType::Node(ref_type)),
                                start,
                            ))
                        }
                        TokenKind::Edge | TokenKind::Relationship => {
                            let ref_type = self.parse_edge_reference_value_type()?;
                            Ok(self.wrap_predefined(
                                PredefinedType::ReferenceValue(ReferenceValueType::Edge(ref_type)),
                                start,
                            ))
                        }
                        TokenKind::Record => Ok(ValueType::Record(self.parse_record_type()?)),
                        _ => Err(self.error_here("unexpected type after ANY")),
                    }
                } else {
                    Err(self.error_here("expected type after ANY"))
                }
            }

            _ => Err(self.error_here(format!(
                "expected type, found {}",
                self.stream.current().kind
            ))),
        }
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Parses a placeholder specification span for Sprint 12-delayed type specs.
    ///
    /// This consumes at least one token and supports balanced delimiters when
    /// the placeholder starts with `{` or `(`.
    fn parse_placeholder_spec_span(&mut self, expected: &str) -> ParseResult<Span> {
        let start = self.stream.current().span.start;

        if matches!(
            self.stream.current().kind,
            TokenKind::Not
                | TokenKind::Comma
                | TokenKind::RParen
                | TokenKind::RBrace
                | TokenKind::Gt
                | TokenKind::Eof
        ) {
            return Err(self.error_here(format!("expected {expected}")));
        }

        if self.stream.check(&TokenKind::LBrace) {
            self.stream.advance();
            let mut depth = 1usize;
            let mut end = self.stream.tokens()[self.stream.position().saturating_sub(1)]
                .span
                .end;
            while depth > 0 {
                if self.stream.check(&TokenKind::Eof) {
                    return Err(self
                        .error_here(format!("unterminated placeholder while parsing {expected}")));
                }
                match self.stream.current().kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth = depth.saturating_sub(1),
                    _ => {}
                }
                end = self.stream.current().span.end;
                self.stream.advance();
            }
            return Ok(start..end);
        }

        if self.stream.check(&TokenKind::LParen) {
            self.stream.advance();
            let mut depth = 1usize;
            let mut end = self.stream.tokens()[self.stream.position().saturating_sub(1)]
                .span
                .end;
            while depth > 0 {
                if self.stream.check(&TokenKind::Eof) {
                    return Err(self
                        .error_here(format!("unterminated placeholder while parsing {expected}")));
                }
                match self.stream.current().kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth = depth.saturating_sub(1),
                    _ => {}
                }
                end = self.stream.current().span.end;
                self.stream.advance();
            }
            return Ok(start..end);
        }

        let end = self.stream.current().span.end;
        self.stream.advance();
        Ok(start..end)
    }

    /// Parses an optional length parameter: ( n )
    fn parse_optional_length_param(&mut self) -> ParseResult<Option<u32>> {
        if self.stream.check(&TokenKind::LParen) {
            self.stream.advance();
            let length = self.parse_unsigned_integer()?;
            self.stream.expect(TokenKind::RParen)?;
            Ok(Some(length))
        } else {
            Ok(None)
        }
    }

    /// Parses optional precision and scale: ( p [, s] )
    fn parse_optional_precision_scale(&mut self) -> ParseResult<(Option<u32>, Option<u32>)> {
        if self.stream.check(&TokenKind::LParen) {
            self.stream.advance();
            let precision = self.parse_unsigned_integer()?;

            let scale = if self.stream.consume(&TokenKind::Comma) {
                Some(self.parse_unsigned_integer()?)
            } else {
                None
            };

            self.stream.expect(TokenKind::RParen)?;
            Ok((Some(precision), scale))
        } else {
            Ok((None, None))
        }
    }

    /// Parses an unsigned integer from an integer literal.
    fn parse_unsigned_integer(&mut self) -> ParseResult<u32> {
        match &self.stream.current().kind {
            TokenKind::IntegerLiteral(s) => {
                let value = s
                    .parse::<u32>()
                    .map_err(|_| self.error_here(format!("invalid integer: {s}")))?;
                self.stream.advance();
                Ok(value)
            }
            _ => Err(self.error_here("expected integer literal")),
        }
    }

    /// Checks for and consumes optional NOT NULL constraint.
    fn check_not_null(&mut self) -> bool {
        if self.stream.check(&TokenKind::Not)
            && self
                .stream
                .peek()
                .is_some_and(|next| matches!(next.kind, TokenKind::Null))
        {
            self.stream.advance(); // consume NOT
            self.stream.advance(); // consume NULL
            return true;
        }
        false
    }

    fn wrap_predefined(&self, predefined: PredefinedType, start: usize) -> ValueType {
        let end = self
            .stream
            .tokens()
            .get(self.stream.position().saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        ValueType::Predefined(predefined, start..end)
    }

    /// Returns the current parser position.
    ///
    /// This is useful for integrating with other parsers that need to know
    /// how many tokens were consumed.
    pub fn current_position(&self) -> usize {
        self.stream.position()
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Parses a value type from a token slice.
///
/// This is the main entry point for parsing types in the GQL parser.
///
/// # Examples
///
/// ```rust
/// use gql_parser::parser::types::parse_value_type;
/// use gql_parser::{Token, TokenKind};
///
/// let tokens = vec![
///     Token::new(TokenKind::Int, 0..3),
/// ];
/// let value_type = parse_value_type(&tokens).unwrap();
/// assert!(matches!(value_type, gql_parser::ast::ValueType::Predefined(_, _)));
/// ```
pub fn parse_value_type(tokens: &[Token]) -> ParseResult<ValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_value_type())
}

/// Parses a boolean type from a token slice.
pub fn parse_boolean_type(tokens: &[Token]) -> ParseResult<BooleanType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_boolean_type())
}

/// Parses a character string type from a token slice.
pub fn parse_character_string_type(tokens: &[Token]) -> ParseResult<CharacterStringType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_character_string_type())
}

/// Parses a byte string type from a token slice.
pub fn parse_byte_string_type(tokens: &[Token]) -> ParseResult<ByteStringType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_byte_string_type())
}

/// Parses a numeric type from a token slice.
pub fn parse_numeric_type(tokens: &[Token]) -> ParseResult<NumericType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_numeric_type())
}

/// Parses a temporal type from a token slice.
pub fn parse_temporal_type(tokens: &[Token]) -> ParseResult<TemporalType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_temporal_type())
}

/// Parses an immaterial value type from a token slice.
pub fn parse_immaterial_value_type(tokens: &[Token]) -> ParseResult<ImmaterialValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_immaterial_value_type())
}

/// Parses a graph reference value type from a token slice.
pub fn parse_graph_reference_value_type(tokens: &[Token]) -> ParseResult<GraphReferenceValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_graph_reference_value_type())
}

/// Parses a binding table reference value type from a token slice.
pub fn parse_binding_table_reference_value_type(
    tokens: &[Token],
) -> ParseResult<BindingTableReferenceValueType> {
    parse_with_full_consumption(tokens, |parser| {
        parser.parse_binding_table_reference_value_type()
    })
}

/// Parses a node reference value type from a token slice.
pub fn parse_node_reference_value_type(tokens: &[Token]) -> ParseResult<NodeReferenceValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_node_reference_value_type())
}

/// Parses an edge reference value type from a token slice.
pub fn parse_edge_reference_value_type(tokens: &[Token]) -> ParseResult<EdgeReferenceValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_edge_reference_value_type())
}

/// Parses a path value type from a token slice.
pub fn parse_path_value_type(tokens: &[Token]) -> ParseResult<PathValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_path_value_type())
}

/// Parses a list value type from a token slice.
pub fn parse_list_value_type(tokens: &[Token]) -> ParseResult<ListValueType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_list_value_type())
}

/// Parses a record type from a token slice.
pub fn parse_record_type(tokens: &[Token]) -> ParseResult<RecordType> {
    parse_with_full_consumption(tokens, |parser| parser.parse_record_type())
}

/// Parses a value type and returns the number of consumed tokens.
///
/// This is used by parent parsers (for example expression parsing) that need a
/// prefix parse instead of strict full-slice consumption.
pub(crate) fn parse_value_type_prefix(tokens: &[Token]) -> ParseResult<(ValueType, usize)> {
    let normalized = normalize_tokens(tokens);
    let mut parser = TypeParser::new(&normalized);
    let value_type = parser.parse_value_type()?;
    Ok((value_type, parser.stream.position()))
}

fn parse_with_full_consumption<T>(
    tokens: &[Token],
    parse: impl FnOnce(&mut TypeParser<'_>) -> ParseResult<T>,
) -> ParseResult<T> {
    let normalized = normalize_tokens(tokens);
    let mut parser = TypeParser::new(&normalized);
    let parsed = parse(&mut parser)?;

    if !matches!(parser.stream.current().kind, TokenKind::Eof) {
        return Err(Box::new(
            Diag::error("unexpected trailing tokens after type")
                .with_primary_label(parser.stream.current().span.clone(), "unexpected token")
                .with_code("P_TYPE"),
        ));
    }

    Ok(parsed)
}

fn normalize_tokens(tokens: &[Token]) -> Vec<Token> {
    let mut normalized = tokens.to_vec();
    if normalized.is_empty() {
        normalized.push(Token::new(TokenKind::Eof, 0..0));
    } else if !matches!(normalized.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
        let eof_pos = normalized.last().map_or(0, |token| token.span.end);
        normalized.push(Token::new(TokenKind::Eof, eof_pos..eof_pos));
    }
    normalized
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        ApproximateNumericType, DecimalKind, ExactNumericType, SignedBinaryExactNumericType,
        TemporalDurationType, TemporalInstantType, UnsignedBinaryExactNumericType,
    };

    fn make_token(kind: TokenKind, start: usize, end: usize) -> Token {
        Token::new(kind, start..end)
    }

    #[test]
    fn test_parse_boolean_type() {
        let tokens = vec![make_token(TokenKind::Bool, 0, 4)];
        let result = parse_boolean_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BooleanType::Bool);

        let tokens = vec![make_token(TokenKind::Boolean, 0, 7)];
        let result = parse_boolean_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BooleanType::Boolean);
    }

    #[test]
    fn test_parse_character_string_type() {
        // STRING
        let tokens = vec![make_token(TokenKind::String, 0, 6)];
        let result = parse_character_string_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CharacterStringType::String);

        // CHAR(10)
        let tokens = vec![
            make_token(TokenKind::Char, 0, 4),
            make_token(TokenKind::LParen, 4, 5),
            make_token(TokenKind::IntegerLiteral("10".into()), 5, 7),
            make_token(TokenKind::RParen, 7, 8),
        ];
        let result = parse_character_string_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CharacterStringType::Char(Some(10)));
    }

    #[test]
    fn test_parse_signed_integer_types() {
        let tokens = vec![make_token(TokenKind::Int, 0, 3)];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Exact(ExactNumericType::SignedBinary(signed_type)) = result.unwrap() {
            assert_eq!(signed_type, SignedBinaryExactNumericType::Int);
        } else {
            panic!("Expected SignedBinaryExactNumericType");
        }

        let tokens = vec![make_token(TokenKind::Bigint, 0, 6)];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Exact(ExactNumericType::SignedBinary(signed_type)) = result.unwrap() {
            assert_eq!(signed_type, SignedBinaryExactNumericType::BigInt);
        } else {
            panic!("Expected SignedBinaryExactNumericType");
        }
    }

    #[test]
    fn test_parse_unsigned_integer_types() {
        let tokens = vec![make_token(TokenKind::Uint, 0, 4)];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Exact(ExactNumericType::UnsignedBinary(unsigned_type)) =
            result.unwrap()
        {
            assert_eq!(unsigned_type, UnsignedBinaryExactNumericType::UInt);
        } else {
            panic!("Expected UnsignedBinaryExactNumericType");
        }

        let tokens = vec![make_token(TokenKind::Uint64, 0, 6)];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Exact(ExactNumericType::UnsignedBinary(unsigned_type)) =
            result.unwrap()
        {
            assert_eq!(unsigned_type, UnsignedBinaryExactNumericType::UInt64);
        } else {
            panic!("Expected UnsignedBinaryExactNumericType");
        }
    }

    #[test]
    fn test_parse_decimal_type() {
        // DECIMAL(10, 2)
        let tokens = vec![
            make_token(TokenKind::Decimal, 0, 7),
            make_token(TokenKind::LParen, 7, 8),
            make_token(TokenKind::IntegerLiteral("10".into()), 8, 10),
            make_token(TokenKind::Comma, 10, 11),
            make_token(TokenKind::IntegerLiteral("2".into()), 11, 12),
            make_token(TokenKind::RParen, 12, 13),
        ];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Exact(ExactNumericType::Decimal(decimal)) = result.unwrap() {
            assert_eq!(decimal.kind, DecimalKind::Decimal);
            assert_eq!(decimal.precision, Some(10));
            assert_eq!(decimal.scale, Some(2));
        } else {
            panic!("Expected DecimalExactNumericType");
        }
    }

    #[test]
    fn test_predefined_type_span_covers_full_extent() {
        let tokens = vec![
            make_token(TokenKind::Decimal, 0, 7),
            make_token(TokenKind::LParen, 7, 8),
            make_token(TokenKind::IntegerLiteral("10".into()), 8, 10),
            make_token(TokenKind::Comma, 10, 11),
            make_token(TokenKind::IntegerLiteral("2".into()), 11, 12),
            make_token(TokenKind::RParen, 12, 13),
        ];
        let result = parse_value_type(&tokens).expect("type should parse");
        assert_eq!(result.span(), 0..13);
    }

    #[test]
    fn test_parse_float_types() {
        let tokens = vec![make_token(TokenKind::Float32, 0, 7)];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Approximate(approx_type) = result.unwrap() {
            assert_eq!(approx_type, ApproximateNumericType::Float32);
        } else {
            panic!("Expected ApproximateNumericType");
        }

        // DOUBLE PRECISION
        let tokens = vec![
            make_token(TokenKind::Double, 0, 6),
            make_token(TokenKind::Precision, 7, 16),
        ];
        let result = parse_numeric_type(&tokens);
        assert!(result.is_ok());
        if let NumericType::Approximate(approx_type) = result.unwrap() {
            assert_eq!(approx_type, ApproximateNumericType::DoublePrecision);
        } else {
            panic!("Expected ApproximateNumericType");
        }
    }

    #[test]
    fn test_parse_temporal_instant_types() {
        // ZONED DATETIME
        let tokens = vec![
            make_token(TokenKind::Zoned, 0, 5),
            make_token(TokenKind::Datetime, 6, 14),
        ];
        let result = parse_temporal_type(&tokens);
        assert!(result.is_ok());
        if let TemporalType::Instant(instant_type) = result.unwrap() {
            assert_eq!(instant_type, TemporalInstantType::ZonedDatetime);
        } else {
            panic!("Expected TemporalInstantType");
        }

        // DATE
        let tokens = vec![make_token(TokenKind::Date, 0, 4)];
        let result = parse_temporal_type(&tokens);
        assert!(result.is_ok());
        if let TemporalType::Instant(instant_type) = result.unwrap() {
            assert_eq!(instant_type, TemporalInstantType::Date);
        } else {
            panic!("Expected TemporalInstantType");
        }
    }

    #[test]
    fn test_parse_temporal_duration_types() {
        // DURATION
        let tokens = vec![make_token(TokenKind::Duration, 0, 8)];
        let result = parse_temporal_type(&tokens);
        assert!(result.is_ok());
        if let TemporalType::Duration(duration_type) = result.unwrap() {
            assert_eq!(duration_type, TemporalDurationType::Duration);
        } else {
            panic!("Expected TemporalDurationType");
        }

        // DURATION YEAR TO MONTH
        let tokens = vec![
            make_token(TokenKind::Duration, 0, 8),
            make_token(TokenKind::Year, 9, 13),
            make_token(TokenKind::To, 14, 16),
            make_token(TokenKind::Month, 17, 22),
        ];
        let result = parse_temporal_type(&tokens);
        assert!(result.is_ok());
        if let TemporalType::Duration(duration_type) = result.unwrap() {
            assert_eq!(duration_type, TemporalDurationType::DurationYearToMonth);
        } else {
            panic!("Expected TemporalDurationType");
        }
    }

    #[test]
    fn test_parse_immaterial_types() {
        let tokens = vec![make_token(TokenKind::Null, 0, 4)];
        let result = parse_immaterial_value_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImmaterialValueType::Null);

        let tokens = vec![make_token(TokenKind::Nothing, 0, 7)];
        let result = parse_immaterial_value_type(&tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImmaterialValueType::Nothing);
    }

    #[test]
    fn test_parse_path_type() {
        let tokens = vec![make_token(TokenKind::Path, 0, 4)];
        let result = parse_path_value_type(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_list_type() {
        // LIST<INT>
        let tokens = vec![
            make_token(TokenKind::List, 0, 4),
            make_token(TokenKind::Lt, 4, 5),
            make_token(TokenKind::Int, 5, 8),
            make_token(TokenKind::Gt, 8, 9),
        ];
        let result = parse_list_value_type(&tokens);
        assert!(result.is_ok());
        let list_type = result.unwrap();
        assert_eq!(list_type.syntax_form, ListSyntaxForm::List);
    }

    #[test]
    fn test_parse_postfix_list_type() {
        // INT LIST
        let tokens = vec![
            make_token(TokenKind::Int, 0, 3),
            make_token(TokenKind::List, 4, 8),
        ];
        let result = parse_value_type(&tokens);
        assert!(result.is_ok());
        let value_type = result.unwrap();
        assert!(matches!(
            value_type,
            ValueType::List(ListValueType {
                syntax_form: ListSyntaxForm::PostfixList,
                ..
            })
        ));
    }

    #[test]
    fn test_parse_record_type() {
        // ANY RECORD
        let tokens = vec![
            make_token(TokenKind::Any, 0, 3),
            make_token(TokenKind::Record, 4, 10),
        ];
        let result = parse_record_type(&tokens);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RecordType::AnyRecord { .. }));
    }

    #[test]
    fn test_parse_graph_reference_type() {
        // ANY PROPERTY GRAPH
        let tokens = vec![
            make_token(TokenKind::Any, 0, 3),
            make_token(TokenKind::Property, 4, 12),
            make_token(TokenKind::Graph, 13, 18),
        ];
        let result = parse_graph_reference_value_type(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphReferenceValueType::AnyPropertyGraph { .. }
        ));
    }

    #[test]
    fn test_parse_binding_table_reference_type_table_alias() {
        // TABLE { x :: INT }
        let tokens = vec![
            make_token(TokenKind::Table, 0, 5),
            make_token(TokenKind::LBrace, 6, 7),
            make_token(TokenKind::Identifier("x".into()), 8, 9),
            make_token(TokenKind::DoubleColon, 10, 12),
            make_token(TokenKind::Int, 13, 16),
            make_token(TokenKind::RBrace, 17, 18),
        ];
        let result = parse_value_type(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_node_reference_type() {
        // ANY NODE
        let tokens = vec![
            make_token(TokenKind::Any, 0, 3),
            make_token(TokenKind::Node, 4, 8),
        ];
        let result = parse_node_reference_value_type(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            NodeReferenceValueType::Any { .. }
        ));
    }

    #[test]
    fn test_parse_edge_reference_type() {
        // EDGE
        let tokens = vec![make_token(TokenKind::Edge, 0, 4)];
        let result = parse_edge_reference_value_type(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            EdgeReferenceValueType::Any { .. }
        ));
    }

    #[test]
    fn test_parse_value_type_rejects_trailing_tokens() {
        let tokens = vec![
            make_token(TokenKind::Int, 0, 3),
            make_token(TokenKind::Identifier("x".into()), 4, 5),
        ];
        let result = parse_value_type(&tokens);
        assert!(result.is_err());
    }
}
