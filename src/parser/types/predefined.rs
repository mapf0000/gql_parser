//! Predefined type parsing for GQL.
//!
//! This module handles parsing of predefined types including:
//! - Boolean types (BOOL, BOOLEAN)
//! - Character string types (STRING, CHAR, VARCHAR)
//! - Byte string types (BYTES, BINARY, VARBINARY)
//! - Numeric types (signed/unsigned integers, decimals, floats)
//! - Temporal types (dates, times, durations)
//! - Immaterial types (NULL, NOTHING)

use crate::ast::{
    ApproximateNumericType, BooleanType, ByteStringType, CharacterStringType,
    DecimalExactNumericType, DecimalKind, ExactNumericType, ImmaterialValueType, NumericType,
    SignedBinaryExactNumericType, TemporalDurationType, TemporalInstantType, TemporalType,
    UnsignedBinaryExactNumericType,
};
use crate::lexer::token::TokenKind;
use crate::parser::base::ParseResult;

use super::TypeParser;

impl<'a> TypeParser<'a> {
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
    pub(super) fn parse_boolean_type(&mut self) -> ParseResult<BooleanType> {
        let bool_type = match &self.stream.current().kind {
            TokenKind::Bool => BooleanType::Bool,
            TokenKind::Boolean => BooleanType::Boolean,
            _ => return Err(self.error_here("expected BOOL or BOOLEAN")),
        };
        self.stream.advance();
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
    pub(super) fn parse_character_string_type(&mut self) -> ParseResult<CharacterStringType> {
        let char_type = match &self.stream.current().kind {
            TokenKind::String => {
                self.stream.advance();
                CharacterStringType::String
            }
            TokenKind::Char => {
                self.stream.advance();
                let length = self.parse_optional_length_param()?;
                CharacterStringType::Char(length)
            }
            TokenKind::Varchar => {
                self.stream.advance();
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
    pub(super) fn parse_byte_string_type(&mut self) -> ParseResult<ByteStringType> {
        let byte_type = match &self.stream.current().kind {
            TokenKind::Bytes => {
                self.stream.advance();
                ByteStringType::Bytes
            }
            TokenKind::Binary => {
                self.stream.advance();
                let length = self.parse_optional_length_param()?;
                ByteStringType::Binary(length)
            }
            TokenKind::Varbinary => {
                self.stream.advance();
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
    pub(super) fn parse_numeric_type(&mut self) -> ParseResult<NumericType> {
        match &self.stream.current().kind {
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
        match &self.stream.current().kind {
            // Decimal types
            TokenKind::Decimal | TokenKind::Dec => {
                let decimal_type = self.parse_decimal_exact_numeric_type()?;
                Ok(ExactNumericType::Decimal(decimal_type))
            }

            // Unsigned types
            TokenKind::Unsigned => {
                self.stream.advance();
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
                self.stream.advance();
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
        let signed_type = match &self.stream.current().kind {
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
        self.stream.advance();
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
        let unsigned_type = match &self.stream.current().kind {
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
        self.stream.advance();
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
        let start = self.stream.current().span.start;
        let kind = match &self.stream.current().kind {
            TokenKind::Decimal => DecimalKind::Decimal,
            TokenKind::Dec => DecimalKind::Dec,
            _ => return Err(self.error_here("expected DECIMAL or DEC")),
        };
        self.stream.advance();

        let (precision, scale) = self.parse_optional_precision_scale()?;
        let end = self
            .stream
            .tokens()
            .get(self.stream.position().saturating_sub(1))
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
        let approx_type = match &self.stream.current().kind {
            TokenKind::Float16 => {
                self.stream.advance();
                ApproximateNumericType::Float16
            }
            TokenKind::Float32 => {
                self.stream.advance();
                ApproximateNumericType::Float32
            }
            TokenKind::Float64 => {
                self.stream.advance();
                ApproximateNumericType::Float64
            }
            TokenKind::Float128 => {
                self.stream.advance();
                ApproximateNumericType::Float128
            }
            TokenKind::Float256 => {
                self.stream.advance();
                ApproximateNumericType::Float256
            }
            TokenKind::Float => {
                self.stream.advance();
                let precision = self.parse_optional_length_param()?;
                ApproximateNumericType::Float(precision)
            }
            TokenKind::Real => {
                self.stream.advance();
                ApproximateNumericType::Real
            }
            TokenKind::Double => {
                self.stream.advance();
                self.stream.expect(TokenKind::Precision)?;
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
    pub(super) fn parse_temporal_type(&mut self) -> ParseResult<TemporalType> {
        match &self.stream.current().kind {
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
        let instant_type = match &self.stream.current().kind {
            TokenKind::Zoned => {
                self.stream.advance();
                match &self.stream.current().kind {
                    TokenKind::Datetime => {
                        self.stream.advance();
                        TemporalInstantType::ZonedDatetime
                    }
                    TokenKind::Time => {
                        self.stream.advance();
                        TemporalInstantType::ZonedTime
                    }
                    _ => {
                        return Err(self.error_here("expected DATETIME or TIME after ZONED"));
                    }
                }
            }
            TokenKind::Local => {
                self.stream.advance();
                match &self.stream.current().kind {
                    TokenKind::Datetime => {
                        self.stream.advance();
                        TemporalInstantType::LocalDatetime
                    }
                    TokenKind::Time => {
                        self.stream.advance();
                        TemporalInstantType::LocalTime
                    }
                    _ => {
                        return Err(self.error_here("expected DATETIME or TIME after LOCAL"));
                    }
                }
            }
            TokenKind::Date => {
                self.stream.advance();
                TemporalInstantType::Date
            }
            TokenKind::Time => {
                self.stream.advance();
                // Check for optional WITHOUT TIME ZONE
                if self.stream.check(&TokenKind::Without) {
                    self.stream.advance();
                    self.stream.expect(TokenKind::Time)?;
                    self.stream.expect(TokenKind::Zone)?;
                    TemporalInstantType::LocalTime
                } else if self.stream.check(&TokenKind::With) {
                    self.stream.advance();
                    self.stream.expect(TokenKind::Time)?;
                    self.stream.expect(TokenKind::Zone)?;
                    TemporalInstantType::ZonedTime
                } else {
                    TemporalInstantType::LocalTime
                }
            }
            TokenKind::Timestamp => {
                self.stream.advance();
                // Check for optional WITH/WITHOUT TIME ZONE
                if self.stream.check(&TokenKind::With) {
                    self.stream.advance();
                    self.stream.expect(TokenKind::Time)?;
                    self.stream.expect(TokenKind::Zone)?;
                    TemporalInstantType::ZonedDatetime
                } else if self.stream.check(&TokenKind::Without) {
                    self.stream.advance();
                    self.stream.expect(TokenKind::Time)?;
                    self.stream.expect(TokenKind::Zone)?;
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
        self.stream.expect(TokenKind::Duration)?;

        let duration_type = if self.stream.check(&TokenKind::Year) {
            self.stream.advance();
            self.stream.expect(TokenKind::To)?;
            self.stream.expect(TokenKind::Month)?;
            TemporalDurationType::DurationYearToMonth
        } else if self.stream.check(&TokenKind::Day) {
            self.stream.advance();
            self.stream.expect(TokenKind::To)?;
            self.stream.expect(TokenKind::Second)?;
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
    pub(super) fn parse_immaterial_value_type(&mut self) -> ParseResult<ImmaterialValueType> {
        let imm_type = match &self.stream.current().kind {
            TokenKind::Null => {
                self.stream.advance();
                // Check for NOT NULL (paradoxical type)
                if self.stream.check(&TokenKind::Not) {
                    if let Some(next) = self.stream.peek() {
                        if matches!(next.kind, TokenKind::Null) {
                            self.stream.advance(); // consume NOT
                            self.stream.advance(); // consume NULL
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
                self.stream.advance();
                ImmaterialValueType::Nothing
            }
            _ => {
                return Err(self.error_here("expected NULL or NOTHING"));
            }
        };
        Ok(imm_type)
    }
}
