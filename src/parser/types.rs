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
    ApproximateNumericType, BindingTableReferenceValueType, BooleanType, ByteStringType,
    CharacterStringType, DecimalExactNumericType, DecimalKind, EdgeReferenceValueType,
    EdgeTypeSpecification, ExactNumericType, FieldType, FieldTypesSpecification,
    GraphReferenceValueType, ImmaterialValueType, ListSyntaxForm, ListValueType,
    NestedGraphTypeSpecification, NodeReferenceValueType, NodeTypeSpecification, NumericType,
    PathValueType, PredefinedType, RecordType, ReferenceValueType, SignedBinaryExactNumericType,
    Span, TemporalDurationType, TemporalInstantType, TemporalType, UnsignedBinaryExactNumericType,
    ValueType,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};

type ParseError = Box<Diag>;
type ParseResult<T> = Result<T, ParseError>;

/// Parser for type specifications.
pub struct TypeParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> TypeParser<'a> {
    /// Creates a new type parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Returns the current token.
    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream must be non-empty"))
    }

    /// Returns the next token without consuming current.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    /// Advances to the next token.
    fn advance(&mut self) {
        if self.pos < self.tokens.len().saturating_sub(1) {
            self.pos += 1;
        }
    }

    /// Checks if the current token matches the given kind.
    fn check(&self, kind: &TokenKind) -> bool {
        &self.current().kind == kind
    }

    /// Consumes the current token if it matches the given kind.
    fn consume(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expects a specific token kind and returns its span.
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Span> {
        if self.check(&kind) {
            let span = self.current().span.clone();
            self.advance();
            Ok(span)
        } else {
            Err(self.error_here(format!("expected {kind}, found {}", self.current().kind)))
        }
    }

    /// Creates an error at the current position.
    fn error_here(&self, message: impl Into<String>) -> ParseError {
        Box::new(
            Diag::error(message.into())
                .with_primary_label(self.current().span.clone(), "here")
                .with_code("P_TYPE"),
        )
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
        while matches!(self.current().kind, TokenKind::List | TokenKind::Array) {
            let syntax_form = match self.current().kind {
                TokenKind::List => ListSyntaxForm::PostfixList,
                TokenKind::Array => ListSyntaxForm::PostfixArray,
                _ => unreachable!("guarded by matches! above"),
            };
            let start = value_type.span().start;
            self.advance();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
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
        let start = self.current().span.start;

        match &self.current().kind {
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
                if let Some(next) = self.peek() {
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

            _ => Err(self.error_here(format!("expected type, found {}", self.current().kind))),
        }
    }

    // ========================================================================
    // Predefined Types - Boolean
    // ========================================================================

    /// Parses a boolean type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// boolean_type ::= BOOL | BOOLEAN
    /// ```
    pub fn parse_boolean_type(&mut self) -> ParseResult<BooleanType> {
        let bool_type = match &self.current().kind {
            TokenKind::Bool => BooleanType::Bool,
            TokenKind::Boolean => BooleanType::Boolean,
            _ => return Err(self.error_here("expected BOOL or BOOLEAN")),
        };
        self.advance();
        Ok(bool_type)
    }

    // ========================================================================
    // Predefined Types - Character String
    // ========================================================================

    /// Parses a character string type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// character_string_type ::=
    ///     | STRING
    ///     | CHAR [ ( n ) ]
    ///     | VARCHAR [ ( n ) ]
    /// ```
    pub fn parse_character_string_type(&mut self) -> ParseResult<CharacterStringType> {
        let char_type = match &self.current().kind {
            TokenKind::String => {
                self.advance();
                CharacterStringType::String
            }
            TokenKind::Char => {
                self.advance();
                let length = self.parse_optional_length_param()?;
                CharacterStringType::Char(length)
            }
            TokenKind::Varchar => {
                self.advance();
                let length = self.parse_optional_length_param()?;
                CharacterStringType::VarChar(length)
            }
            _ => return Err(self.error_here("expected STRING, CHAR, or VARCHAR")),
        };
        Ok(char_type)
    }

    // ========================================================================
    // Predefined Types - Byte String
    // ========================================================================

    /// Parses a byte string type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// byte_string_type ::=
    ///     | BYTES
    ///     | BINARY [ ( n ) ]
    ///     | VARBINARY [ ( n ) ]
    /// ```
    pub fn parse_byte_string_type(&mut self) -> ParseResult<ByteStringType> {
        let byte_type = match &self.current().kind {
            TokenKind::Bytes => {
                self.advance();
                ByteStringType::Bytes
            }
            TokenKind::Binary => {
                self.advance();
                let length = self.parse_optional_length_param()?;
                ByteStringType::Binary(length)
            }
            TokenKind::Varbinary => {
                self.advance();
                let length = self.parse_optional_length_param()?;
                ByteStringType::VarBinary(length)
            }
            _ => return Err(self.error_here("expected BYTES, BINARY, or VARBINARY")),
        };
        Ok(byte_type)
    }

    // ========================================================================
    // Predefined Types - Numeric
    // ========================================================================

    /// Parses a numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// numeric_type ::= exact_numeric_type | approximate_numeric_type
    /// ```
    pub fn parse_numeric_type(&mut self) -> ParseResult<NumericType> {
        match &self.current().kind {
            // Approximate numeric types
            TokenKind::Float
            | TokenKind::Float16
            | TokenKind::Float32
            | TokenKind::Float64
            | TokenKind::Float128
            | TokenKind::Float256
            | TokenKind::Real
            | TokenKind::Double => {
                let approx_type = self.parse_approximate_numeric_type()?;
                Ok(NumericType::Approximate(approx_type))
            }

            // Exact numeric types
            _ => {
                let exact_type = self.parse_exact_numeric_type()?;
                Ok(NumericType::Exact(exact_type))
            }
        }
    }

    /// Parses an exact numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// exact_numeric_type ::=
    ///     | signed_binary_exact_numeric_type
    ///     | unsigned_binary_exact_numeric_type
    ///     | decimal_exact_numeric_type
    /// ```
    fn parse_exact_numeric_type(&mut self) -> ParseResult<ExactNumericType> {
        match &self.current().kind {
            // Decimal types
            TokenKind::Decimal | TokenKind::Dec => {
                let decimal_type = self.parse_decimal_exact_numeric_type()?;
                Ok(ExactNumericType::Decimal(decimal_type))
            }

            // Unsigned types
            TokenKind::Unsigned => {
                self.advance();
                let unsigned_type = self.parse_unsigned_binary_exact_numeric_type()?;
                Ok(ExactNumericType::UnsignedBinary(unsigned_type))
            }
            TokenKind::Uint
            | TokenKind::Uint8
            | TokenKind::Uint16
            | TokenKind::Uint32
            | TokenKind::Uint64
            | TokenKind::Uint128
            | TokenKind::Uint256
            | TokenKind::Usmallint
            | TokenKind::Ubigint => {
                let unsigned_type = self.parse_unsigned_binary_exact_numeric_type()?;
                Ok(ExactNumericType::UnsignedBinary(unsigned_type))
            }

            // Signed types (SIGNED keyword is optional)
            TokenKind::Signed => {
                self.advance();
                let signed_type = self.parse_signed_binary_exact_numeric_type()?;
                Ok(ExactNumericType::SignedBinary(signed_type))
            }
            _ => {
                let signed_type = self.parse_signed_binary_exact_numeric_type()?;
                Ok(ExactNumericType::SignedBinary(signed_type))
            }
        }
    }

    /// Parses a signed binary exact numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// signed_binary_exact_numeric_type ::=
    ///     | [SIGNED] INT8
    ///     | [SIGNED] INT16
    ///     | [SIGNED] INT32
    ///     | [SIGNED] INT64
    ///     | [SIGNED] INT128
    ///     | [SIGNED] INT256
    ///     | [SIGNED] SMALLINT
    ///     | [SIGNED] INT
    ///     | [SIGNED] INTEGER
    ///     | [SIGNED] BIGINT
    /// ```
    fn parse_signed_binary_exact_numeric_type(
        &mut self,
    ) -> ParseResult<SignedBinaryExactNumericType> {
        let signed_type = match &self.current().kind {
            TokenKind::Int8 => SignedBinaryExactNumericType::Int8,
            TokenKind::Int16 => SignedBinaryExactNumericType::Int16,
            TokenKind::Int32 => SignedBinaryExactNumericType::Int32,
            TokenKind::Int64 => SignedBinaryExactNumericType::Int64,
            TokenKind::Int128 => SignedBinaryExactNumericType::Int128,
            TokenKind::Int256 => SignedBinaryExactNumericType::Int256,
            TokenKind::Smallint => SignedBinaryExactNumericType::SmallInt,
            TokenKind::Int => SignedBinaryExactNumericType::Int,
            TokenKind::Integer => SignedBinaryExactNumericType::Integer,
            TokenKind::Bigint => SignedBinaryExactNumericType::BigInt,
            _ => {
                return Err(self.error_here("expected signed integer type"));
            }
        };
        self.advance();
        Ok(signed_type)
    }

    /// Parses an unsigned binary exact numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// unsigned_binary_exact_numeric_type ::=
    ///     | [UNSIGNED] UINT8
    ///     | [UNSIGNED] UINT16
    ///     | [UNSIGNED] UINT32
    ///     | [UNSIGNED] UINT64
    ///     | [UNSIGNED] UINT128
    ///     | [UNSIGNED] UINT256
    ///     | [UNSIGNED] USMALLINT
    ///     | [UNSIGNED] UINT
    ///     | [UNSIGNED] UBIGINT
    ///     | UNSIGNED INT8
    ///     | UNSIGNED INT16
    ///     | UNSIGNED INT32
    ///     | UNSIGNED INT64
    ///     | UNSIGNED INT128
    ///     | UNSIGNED INT256
    ///     | UNSIGNED SMALLINT
    ///     | UNSIGNED INT
    ///     | UNSIGNED BIGINT
    /// ```
    fn parse_unsigned_binary_exact_numeric_type(
        &mut self,
    ) -> ParseResult<UnsignedBinaryExactNumericType> {
        let unsigned_type = match &self.current().kind {
            TokenKind::Uint8 => UnsignedBinaryExactNumericType::UInt8,
            TokenKind::Uint16 => UnsignedBinaryExactNumericType::UInt16,
            TokenKind::Uint32 => UnsignedBinaryExactNumericType::UInt32,
            TokenKind::Uint64 => UnsignedBinaryExactNumericType::UInt64,
            TokenKind::Uint128 => UnsignedBinaryExactNumericType::UInt128,
            TokenKind::Uint256 => UnsignedBinaryExactNumericType::UInt256,
            TokenKind::Usmallint => UnsignedBinaryExactNumericType::USmallInt,
            TokenKind::Uint => UnsignedBinaryExactNumericType::UInt,
            TokenKind::Ubigint => UnsignedBinaryExactNumericType::UBigInt,
            // Handle UNSIGNED INT8, UNSIGNED INT16, etc.
            TokenKind::Int8 => UnsignedBinaryExactNumericType::UInt8,
            TokenKind::Int16 => UnsignedBinaryExactNumericType::UInt16,
            TokenKind::Int32 => UnsignedBinaryExactNumericType::UInt32,
            TokenKind::Int64 => UnsignedBinaryExactNumericType::UInt64,
            TokenKind::Int128 => UnsignedBinaryExactNumericType::UInt128,
            TokenKind::Int256 => UnsignedBinaryExactNumericType::UInt256,
            TokenKind::Smallint => UnsignedBinaryExactNumericType::USmallInt,
            TokenKind::Int | TokenKind::Integer => UnsignedBinaryExactNumericType::UInt,
            TokenKind::Bigint => UnsignedBinaryExactNumericType::UBigInt,
            _ => {
                return Err(self.error_here("expected unsigned integer type"));
            }
        };
        self.advance();
        Ok(unsigned_type)
    }

    /// Parses a decimal exact numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// decimal_exact_numeric_type ::=
    ///     | DECIMAL [ ( precision [ , scale ] ) ]
    ///     | DEC [ ( precision [ , scale ] ) ]
    /// ```
    fn parse_decimal_exact_numeric_type(&mut self) -> ParseResult<DecimalExactNumericType> {
        let start = self.current().span.start;
        let kind = match &self.current().kind {
            TokenKind::Decimal => DecimalKind::Decimal,
            TokenKind::Dec => DecimalKind::Dec,
            _ => return Err(self.error_here("expected DECIMAL or DEC")),
        };
        self.advance();

        let (precision, scale) = self.parse_optional_precision_scale()?;
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(DecimalExactNumericType {
            kind,
            precision,
            scale,
            span: start..end,
        })
    }

    /// Parses an approximate numeric type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// approximate_numeric_type ::=
    ///     | FLOAT16
    ///     | FLOAT32
    ///     | FLOAT64
    ///     | FLOAT128
    ///     | FLOAT256
    ///     | FLOAT [ ( precision ) ]
    ///     | REAL
    ///     | DOUBLE PRECISION
    /// ```
    fn parse_approximate_numeric_type(&mut self) -> ParseResult<ApproximateNumericType> {
        let approx_type = match &self.current().kind {
            TokenKind::Float16 => {
                self.advance();
                ApproximateNumericType::Float16
            }
            TokenKind::Float32 => {
                self.advance();
                ApproximateNumericType::Float32
            }
            TokenKind::Float64 => {
                self.advance();
                ApproximateNumericType::Float64
            }
            TokenKind::Float128 => {
                self.advance();
                ApproximateNumericType::Float128
            }
            TokenKind::Float256 => {
                self.advance();
                ApproximateNumericType::Float256
            }
            TokenKind::Float => {
                self.advance();
                let precision = self.parse_optional_length_param()?;
                ApproximateNumericType::Float(precision)
            }
            TokenKind::Real => {
                self.advance();
                ApproximateNumericType::Real
            }
            TokenKind::Double => {
                self.advance();
                self.expect(TokenKind::Precision)?;
                ApproximateNumericType::DoublePrecision
            }
            _ => {
                return Err(self.error_here("expected approximate numeric type"));
            }
        };
        Ok(approx_type)
    }

    // ========================================================================
    // Predefined Types - Temporal
    // ========================================================================

    /// Parses a temporal type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// temporal_type ::= temporal_instant_type | temporal_duration_type
    /// ```
    pub fn parse_temporal_type(&mut self) -> ParseResult<TemporalType> {
        match &self.current().kind {
            TokenKind::Duration => {
                let duration_type = self.parse_temporal_duration_type()?;
                Ok(TemporalType::Duration(duration_type))
            }
            _ => {
                let instant_type = self.parse_temporal_instant_type()?;
                Ok(TemporalType::Instant(instant_type))
            }
        }
    }

    /// Parses a temporal instant type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// temporal_instant_type ::=
    ///     | ZONED DATETIME
    ///     | ZONED TIME
    ///     | LOCAL DATETIME
    ///     | LOCAL TIME
    ///     | DATE
    ///     | TIME [ WITHOUT TIME ZONE ]
    ///     | TIMESTAMP [ WITH[OUT] TIME ZONE ]
    /// ```
    fn parse_temporal_instant_type(&mut self) -> ParseResult<TemporalInstantType> {
        let instant_type = match &self.current().kind {
            TokenKind::Zoned => {
                self.advance();
                match &self.current().kind {
                    TokenKind::Datetime => {
                        self.advance();
                        TemporalInstantType::ZonedDatetime
                    }
                    TokenKind::Time => {
                        self.advance();
                        TemporalInstantType::ZonedTime
                    }
                    _ => {
                        return Err(self.error_here("expected DATETIME or TIME after ZONED"));
                    }
                }
            }
            TokenKind::Local => {
                self.advance();
                match &self.current().kind {
                    TokenKind::Datetime => {
                        self.advance();
                        TemporalInstantType::LocalDatetime
                    }
                    TokenKind::Time => {
                        self.advance();
                        TemporalInstantType::LocalTime
                    }
                    _ => {
                        return Err(self.error_here("expected DATETIME or TIME after LOCAL"));
                    }
                }
            }
            TokenKind::Date => {
                self.advance();
                TemporalInstantType::Date
            }
            TokenKind::Time => {
                self.advance();
                // Check for optional WITHOUT TIME ZONE
                if self.check(&TokenKind::Without) {
                    self.advance();
                    self.expect(TokenKind::Time)?;
                    self.expect(TokenKind::Zone)?;
                    TemporalInstantType::LocalTime
                } else if self.check(&TokenKind::With) {
                    self.advance();
                    self.expect(TokenKind::Time)?;
                    self.expect(TokenKind::Zone)?;
                    TemporalInstantType::ZonedTime
                } else {
                    TemporalInstantType::LocalTime
                }
            }
            TokenKind::Timestamp => {
                self.advance();
                // Check for optional WITH/WITHOUT TIME ZONE
                if self.check(&TokenKind::With) {
                    self.advance();
                    self.expect(TokenKind::Time)?;
                    self.expect(TokenKind::Zone)?;
                    TemporalInstantType::ZonedDatetime
                } else if self.check(&TokenKind::Without) {
                    self.advance();
                    self.expect(TokenKind::Time)?;
                    self.expect(TokenKind::Zone)?;
                    TemporalInstantType::LocalDatetime
                } else {
                    TemporalInstantType::LocalDatetime
                }
            }
            _ => {
                return Err(self.error_here("expected temporal instant type"));
            }
        };
        Ok(instant_type)
    }

    /// Parses a temporal duration type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// temporal_duration_type ::=
    ///     | DURATION
    ///     | DURATION YEAR TO MONTH
    ///     | DURATION DAY TO SECOND
    /// ```
    fn parse_temporal_duration_type(&mut self) -> ParseResult<TemporalDurationType> {
        self.expect(TokenKind::Duration)?;

        let duration_type = if self.check(&TokenKind::Year) {
            self.advance();
            self.expect(TokenKind::To)?;
            self.expect(TokenKind::Month)?;
            TemporalDurationType::DurationYearToMonth
        } else if self.check(&TokenKind::Day) {
            self.advance();
            self.expect(TokenKind::To)?;
            self.expect(TokenKind::Second)?;
            TemporalDurationType::DurationDayToSecond
        } else {
            TemporalDurationType::Duration
        };

        Ok(duration_type)
    }

    // ========================================================================
    // Predefined Types - Immaterial
    // ========================================================================

    /// Parses an immaterial value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// immaterial_value_type ::=
    ///     | NULL [ NOT NULL ]
    ///     | NOTHING
    /// ```
    pub fn parse_immaterial_value_type(&mut self) -> ParseResult<ImmaterialValueType> {
        let imm_type = match &self.current().kind {
            TokenKind::Null => {
                self.advance();
                // Check for NOT NULL (paradoxical type)
                if self.check(&TokenKind::Not) {
                    if let Some(next) = self.peek() {
                        if matches!(next.kind, TokenKind::Null) {
                            self.advance(); // consume NOT
                            self.advance(); // consume NULL
                            ImmaterialValueType::NullNotNull
                        } else {
                            ImmaterialValueType::Null
                        }
                    } else {
                        ImmaterialValueType::Null
                    }
                } else {
                    ImmaterialValueType::Null
                }
            }
            TokenKind::Nothing => {
                self.advance();
                ImmaterialValueType::Nothing
            }
            _ => {
                return Err(self.error_here("expected NULL or NOTHING"));
            }
        };
        Ok(imm_type)
    }

    // ========================================================================
    // Reference Value Types
    // ========================================================================

    /// Parses a graph reference value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// graph_reference_value_type ::=
    ///     | ANY [ PROPERTY ] GRAPH [ NOT NULL ]
    ///     | PROPERTY GRAPH <nested_spec> [ NOT NULL ]
    /// ```
    pub fn parse_graph_reference_value_type(&mut self) -> ParseResult<GraphReferenceValueType> {
        let start = self.current().span.start;

        // Check for ANY [PROPERTY] GRAPH
        if self.check(&TokenKind::Any) {
            self.advance();
            // Optional PROPERTY keyword
            self.consume(&TokenKind::Property);
            self.expect(TokenKind::Graph)?;

            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(GraphReferenceValueType::AnyPropertyGraph {
                not_null,
                span: start..end,
            })
        } else if self.check(&TokenKind::Property) || self.check(&TokenKind::Graph) {
            // PROPERTY GRAPH <nested_spec> [NOT NULL]
            self.consume(&TokenKind::Property);
            self.expect(TokenKind::Graph)?;

            let spec_span = self.parse_placeholder_spec_span("nested graph type specification")?;
            let spec = Box::new(NestedGraphTypeSpecification { span: spec_span });

            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(GraphReferenceValueType::PropertyGraph {
                spec,
                not_null,
                span: start..end,
            })
        } else {
            Err(self.error_here("expected graph reference type"))
        }
    }

    /// Parses a binding table reference value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// binding_table_reference_value_type ::=
    ///     BINDING TABLE [ <field_types_spec> ] [ NOT NULL ]
    /// ```
    pub fn parse_binding_table_reference_value_type(
        &mut self,
    ) -> ParseResult<BindingTableReferenceValueType> {
        let start = self.current().span.start;
        if self.check(&TokenKind::Binding) {
            self.advance();
            self.expect(TokenKind::Table)?;
        } else {
            self.expect(TokenKind::Table)?;
        }

        // Optional field types specification
        let field_types = if self.check(&TokenKind::LBrace) {
            Some(self.parse_field_types_specification()?)
        } else {
            None
        };

        let not_null = self.check_not_null();
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(BindingTableReferenceValueType {
            field_types,
            not_null,
            span: start..end,
        })
    }

    /// Parses a node reference value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// node_reference_value_type ::=
    ///     | [ ANY ] NODE [ NOT NULL ]
    ///     | [ ANY ] VERTEX [ NOT NULL ]
    ///     | <node_type_spec> [ NOT NULL ]
    /// ```
    pub fn parse_node_reference_value_type(&mut self) -> ParseResult<NodeReferenceValueType> {
        let start = self.current().span.start;
        let had_any = self.consume(&TokenKind::Any);

        if self.check(&TokenKind::Vertex) || self.check(&TokenKind::Node) {
            let use_vertex = if self.check(&TokenKind::Vertex) {
                self.advance();
                true
            } else {
                self.advance();
                false
            };

            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(NodeReferenceValueType::Any {
                use_vertex,
                not_null,
                span: start..end,
            })
        } else if had_any {
            Err(self.error_here("expected NODE or VERTEX after ANY"))
        } else {
            let spec_span = self.parse_placeholder_spec_span("node type specification")?;
            let spec = Box::new(NodeTypeSpecification { span: spec_span });
            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            Ok(NodeReferenceValueType::Typed {
                spec,
                not_null,
                span: start..end,
            })
        }
    }

    /// Parses an edge reference value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// edge_reference_value_type ::=
    ///     | [ ANY ] EDGE [ NOT NULL ]
    ///     | [ ANY ] RELATIONSHIP [ NOT NULL ]
    ///     | <edge_type_spec> [ NOT NULL ]
    /// ```
    pub fn parse_edge_reference_value_type(&mut self) -> ParseResult<EdgeReferenceValueType> {
        let start = self.current().span.start;
        let had_any = self.consume(&TokenKind::Any);

        if self.check(&TokenKind::Relationship) || self.check(&TokenKind::Edge) {
            let use_relationship = if self.check(&TokenKind::Relationship) {
                self.advance();
                true
            } else {
                self.advance();
                false
            };

            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(EdgeReferenceValueType::Any {
                use_relationship,
                not_null,
                span: start..end,
            })
        } else if had_any {
            Err(self.error_here("expected EDGE or RELATIONSHIP after ANY"))
        } else {
            let spec_span = self.parse_placeholder_spec_span("edge type specification")?;
            let spec = Box::new(EdgeTypeSpecification { span: spec_span });
            let not_null = self.check_not_null();
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            Ok(EdgeReferenceValueType::Typed {
                spec,
                not_null,
                span: start..end,
            })
        }
    }

    // ========================================================================
    // Constructed Types - Path
    // ========================================================================

    /// Parses a path value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// path_value_type ::= PATH
    /// ```
    pub fn parse_path_value_type(&mut self) -> ParseResult<PathValueType> {
        let span = self.expect(TokenKind::Path)?;
        Ok(PathValueType { span })
    }

    // ========================================================================
    // Constructed Types - List
    // ========================================================================

    /// Parses a list value type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// list_value_type ::=
    ///     | LIST < value_type >
    ///     | ARRAY < value_type >
    ///     | value_type LIST
    ///     | value_type ARRAY
    /// ```
    pub fn parse_list_value_type(&mut self) -> ParseResult<ListValueType> {
        let start = self.current().span.start;

        let syntax_form = match &self.current().kind {
            TokenKind::List => {
                self.advance();
                ListSyntaxForm::List
            }
            TokenKind::Array => {
                self.advance();
                ListSyntaxForm::Array
            }
            _ => {
                return Err(self.error_here("expected LIST or ARRAY"));
            }
        };

        self.expect(TokenKind::Lt)?;
        let element_type = Box::new(self.parse_value_type()?);
        self.expect(TokenKind::Gt)?;

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(ListValueType {
            element_type,
            syntax_form,
            span: start..end,
        })
    }

    // ========================================================================
    // Constructed Types - Record
    // ========================================================================

    /// Parses a record type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// record_type ::=
    ///     | ANY RECORD
    ///     | RECORD <field_types_spec>
    /// ```
    pub fn parse_record_type(&mut self) -> ParseResult<RecordType> {
        let start = self.current().span.start;

        // Check for ANY RECORD
        if self.check(&TokenKind::Any) {
            self.advance();
            self.expect(TokenKind::Record)?;
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            Ok(RecordType::AnyRecord { span: start..end })
        } else if self.check(&TokenKind::Record) {
            self.advance();
            let field_types = self.parse_field_types_specification()?;
            let end = field_types.span.end;
            Ok(RecordType::Record {
                field_types,
                span: start..end,
            })
        } else {
            Err(self.error_here("expected RECORD"))
        }
    }

    /// Parses a field types specification.
    ///
    /// # Grammar
    ///
    /// ```text
    /// field_types_specification ::= { field_type [ , field_type ]* }
    /// ```
    fn parse_field_types_specification(&mut self) -> ParseResult<FieldTypesSpecification> {
        let start = self.current().span.start;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();

        // Parse first field
        if !self.check(&TokenKind::RBrace) {
            fields.push(self.parse_field_type()?);

            // Parse remaining fields
            while self.consume(&TokenKind::Comma) {
                if self.check(&TokenKind::RBrace) {
                    break; // Trailing comma
                }
                fields.push(self.parse_field_type()?);
            }
        }

        let end_span = self.expect(TokenKind::RBrace)?;
        let end = end_span.end;

        Ok(FieldTypesSpecification {
            fields,
            span: start..end,
        })
    }

    /// Parses a field type.
    ///
    /// # Grammar
    ///
    /// ```text
    /// field_type ::= field_name :: value_type
    /// ```
    fn parse_field_type(&mut self) -> ParseResult<FieldType> {
        let start = self.current().span.start;

        // Parse field name
        let field_name = match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return Err(self.error_here("expected field name"));
            }
        };

        self.expect(TokenKind::DoubleColon)?;
        let field_type = Box::new(self.parse_value_type()?);

        let end = field_type.span().end;

        Ok(FieldType {
            field_name,
            field_type,
            span: start..end,
        })
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Parses a placeholder specification span for Sprint 12-delayed type specs.
    ///
    /// This consumes at least one token and supports balanced delimiters when
    /// the placeholder starts with `{` or `(`.
    fn parse_placeholder_spec_span(&mut self, expected: &str) -> ParseResult<Span> {
        let start = self.current().span.start;

        if matches!(
            self.current().kind,
            TokenKind::Not
                | TokenKind::Comma
                | TokenKind::RParen
                | TokenKind::RBrace
                | TokenKind::Gt
                | TokenKind::Eof
        ) {
            return Err(self.error_here(format!("expected {expected}")));
        }

        if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut depth = 1usize;
            let mut end = self.tokens[self.pos.saturating_sub(1)].span.end;
            while depth > 0 {
                if self.check(&TokenKind::Eof) {
                    return Err(self
                        .error_here(format!("unterminated placeholder while parsing {expected}")));
                }
                match self.current().kind {
                    TokenKind::LBrace => depth += 1,
                    TokenKind::RBrace => depth = depth.saturating_sub(1),
                    _ => {}
                }
                end = self.current().span.end;
                self.advance();
            }
            return Ok(start..end);
        }

        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut depth = 1usize;
            let mut end = self.tokens[self.pos.saturating_sub(1)].span.end;
            while depth > 0 {
                if self.check(&TokenKind::Eof) {
                    return Err(self
                        .error_here(format!("unterminated placeholder while parsing {expected}")));
                }
                match self.current().kind {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => depth = depth.saturating_sub(1),
                    _ => {}
                }
                end = self.current().span.end;
                self.advance();
            }
            return Ok(start..end);
        }

        let end = self.current().span.end;
        self.advance();
        Ok(start..end)
    }

    /// Parses an optional length parameter: ( n )
    fn parse_optional_length_param(&mut self) -> ParseResult<Option<u32>> {
        if self.check(&TokenKind::LParen) {
            self.advance();
            let length = self.parse_unsigned_integer()?;
            self.expect(TokenKind::RParen)?;
            Ok(Some(length))
        } else {
            Ok(None)
        }
    }

    /// Parses optional precision and scale: ( p [, s] )
    fn parse_optional_precision_scale(&mut self) -> ParseResult<(Option<u32>, Option<u32>)> {
        if self.check(&TokenKind::LParen) {
            self.advance();
            let precision = self.parse_unsigned_integer()?;

            let scale = if self.consume(&TokenKind::Comma) {
                Some(self.parse_unsigned_integer()?)
            } else {
                None
            };

            self.expect(TokenKind::RParen)?;
            Ok((Some(precision), scale))
        } else {
            Ok((None, None))
        }
    }

    /// Parses an unsigned integer from an integer literal.
    fn parse_unsigned_integer(&mut self) -> ParseResult<u32> {
        match &self.current().kind {
            TokenKind::IntegerLiteral(s) => {
                let value = s
                    .parse::<u32>()
                    .map_err(|_| self.error_here(format!("invalid integer: {s}")))?;
                self.advance();
                Ok(value)
            }
            _ => Err(self.error_here("expected integer literal")),
        }
    }

    /// Checks for and consumes optional NOT NULL constraint.
    fn check_not_null(&mut self) -> bool {
        if self.check(&TokenKind::Not)
            && self
                .peek()
                .is_some_and(|next| matches!(next.kind, TokenKind::Null))
        {
            self.advance(); // consume NOT
            self.advance(); // consume NULL
            return true;
        }
        false
    }

    fn wrap_predefined(&self, predefined: PredefinedType, start: usize) -> ValueType {
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        ValueType::Predefined(predefined, start..end)
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
/// ```ignore
/// let tokens = vec![
///     Token::new(TokenKind::Int, 0..3),
/// ];
/// let value_type = parse_value_type(&tokens)?;
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
    Ok((value_type, parser.pos))
}

fn parse_with_full_consumption<T>(
    tokens: &[Token],
    parse: impl FnOnce(&mut TypeParser<'_>) -> ParseResult<T>,
) -> ParseResult<T> {
    let normalized = normalize_tokens(tokens);
    let mut parser = TypeParser::new(&normalized);
    let parsed = parse(&mut parser)?;

    if !matches!(parser.current().kind, TokenKind::Eof) {
        return Err(Box::new(
            Diag::error("unexpected trailing tokens after type")
                .with_primary_label(parser.current().span.clone(), "unexpected token")
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
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_signed_binary_exact_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SignedBinaryExactNumericType::Int);

        let tokens = vec![make_token(TokenKind::Bigint, 0, 6)];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_signed_binary_exact_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SignedBinaryExactNumericType::BigInt);
    }

    #[test]
    fn test_parse_unsigned_integer_types() {
        let tokens = vec![make_token(TokenKind::Uint, 0, 4)];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_unsigned_binary_exact_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UnsignedBinaryExactNumericType::UInt);

        let tokens = vec![make_token(TokenKind::Uint64, 0, 6)];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_unsigned_binary_exact_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), UnsignedBinaryExactNumericType::UInt64);
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
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_decimal_exact_numeric_type();
        assert!(result.is_ok());
        let decimal = result.unwrap();
        assert_eq!(decimal.kind, DecimalKind::Decimal);
        assert_eq!(decimal.precision, Some(10));
        assert_eq!(decimal.scale, Some(2));
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
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_approximate_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApproximateNumericType::Float32);

        // DOUBLE PRECISION
        let tokens = vec![
            make_token(TokenKind::Double, 0, 6),
            make_token(TokenKind::Precision, 7, 16),
        ];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_approximate_numeric_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApproximateNumericType::DoublePrecision);
    }

    #[test]
    fn test_parse_temporal_instant_types() {
        // ZONED DATETIME
        let tokens = vec![
            make_token(TokenKind::Zoned, 0, 5),
            make_token(TokenKind::Datetime, 6, 14),
        ];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_temporal_instant_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TemporalInstantType::ZonedDatetime);

        // DATE
        let tokens = vec![make_token(TokenKind::Date, 0, 4)];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_temporal_instant_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TemporalInstantType::Date);
    }

    #[test]
    fn test_parse_temporal_duration_types() {
        // DURATION
        let tokens = vec![make_token(TokenKind::Duration, 0, 8)];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_temporal_duration_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TemporalDurationType::Duration);

        // DURATION YEAR TO MONTH
        let tokens = vec![
            make_token(TokenKind::Duration, 0, 8),
            make_token(TokenKind::Year, 9, 13),
            make_token(TokenKind::To, 14, 16),
            make_token(TokenKind::Month, 17, 22),
        ];
        let mut parser = TypeParser::new(&tokens);
        let result = parser.parse_temporal_duration_type();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TemporalDurationType::DurationYearToMonth);
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
