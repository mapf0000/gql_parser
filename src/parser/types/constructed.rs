//! Constructed type parsing for GQL.
//!
//! This module handles parsing of constructed types including:
//! - Path value types
//! - List value types (LIST<T>, ARRAY<T>)
//! - Record types with field type specifications

use crate::ast::{
    FieldType, FieldTypesSpecification, ListSyntaxForm, ListValueType, PathValueType, RecordType,
};
use crate::lexer::token::TokenKind;
use crate::parser::base::ParseResult;

use super::TypeParser;

impl<'a> TypeParser<'a> {
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
    pub(super) fn parse_path_value_type(&mut self) -> ParseResult<PathValueType> {
        let span = self.stream.expect(TokenKind::Path)?;
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
    pub(super) fn parse_list_value_type(&mut self) -> ParseResult<ListValueType> {
        let start = self.stream.current().span.start;

        let syntax_form = match &self.stream.current().kind {
            TokenKind::List => {
                self.stream.advance();
                ListSyntaxForm::List
            }
            TokenKind::Array => {
                self.stream.advance();
                ListSyntaxForm::Array
            }
            _ => {
                return Err(self.error_here("expected LIST or ARRAY"));
            }
        };

        self.stream.expect(TokenKind::Lt)?;
        let element_type = Box::new(self.parse_value_type()?);
        self.stream.expect(TokenKind::Gt)?;

        let end = self
            .stream
            .tokens()
            .get(self.stream.position().saturating_sub(1))
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
    pub(super) fn parse_record_type(&mut self) -> ParseResult<RecordType> {
        let start = self.stream.current().span.start;

        // Check for ANY RECORD
        if self.stream.check(&TokenKind::Any) {
            self.stream.advance();
            self.stream.expect(TokenKind::Record)?;
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            Ok(RecordType::AnyRecord { span: start..end })
        } else if self.stream.check(&TokenKind::Record) {
            self.stream.advance();
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
    pub(super) fn parse_field_types_specification(
        &mut self,
    ) -> ParseResult<FieldTypesSpecification> {
        let start = self.stream.current().span.start;
        self.stream.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();

        // Parse first field
        if !self.stream.check(&TokenKind::RBrace) {
            fields.push(self.parse_field_type()?);

            // Parse remaining fields
            while self.stream.consume(&TokenKind::Comma) {
                if self.stream.check(&TokenKind::RBrace) {
                    break; // Trailing comma
                }
                fields.push(self.parse_field_type()?);
            }
        }

        let end_span = self.stream.expect(TokenKind::RBrace)?;
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
        let start = self.stream.current().span.start;

        // Parse field name
        let field_name = match &self.stream.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.stream.advance();
                name
            }
            _ => {
                return Err(self.error_here("expected field name"));
            }
        };

        self.stream.expect(TokenKind::DoubleColon)?;
        let field_type = Box::new(self.parse_value_type()?);

        let end = field_type.span().end;

        Ok(FieldType {
            field_name,
            field_type,
            span: start..end,
        })
    }
}
