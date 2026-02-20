//! Reference value type parsing for GQL.
//!
//! This module handles parsing of reference types including:
//! - Graph reference types (ANY PROPERTY GRAPH, PROPERTY GRAPH)
//! - Binding table reference types (BINDING TABLE, TABLE)
//! - Node reference types (NODE, VERTEX)
//! - Edge reference types (EDGE, RELATIONSHIP)

use crate::ast::{
    BindingTableReferenceValueType, EdgeReferenceValueType, GraphReferenceValueType,
    NodeReferenceValueType,
};
use crate::lexer::token::TokenKind;
use crate::parser::base::ParseResult;
use crate::parser::graph_type::GraphTypeParser;

use super::TypeParser;

impl<'a> TypeParser<'a> {
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
    pub(super) fn parse_graph_reference_value_type(
        &mut self,
    ) -> ParseResult<GraphReferenceValueType> {
        let start = self.stream.current().span.start;

        // Check for ANY [PROPERTY] GRAPH
        if self.stream.check(&TokenKind::Any) {
            self.stream.advance();
            // Optional PROPERTY keyword
            self.stream.consume(&TokenKind::Property);
            self.stream.expect(TokenKind::Graph)?;

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(GraphReferenceValueType::AnyPropertyGraph {
                not_null,
                span: start..end,
            })
        } else if self.stream.check(&TokenKind::Property) || self.stream.check(&TokenKind::Graph) {
            // PROPERTY GRAPH <nested_spec> [NOT NULL]
            self.stream.consume(&TokenKind::Property);
            self.stream.expect(TokenKind::Graph)?;

            let start = self.stream.position();
            let mut graph_type_parser = GraphTypeParser::new(&self.stream.tokens()[start..]);
            let spec = Box::new(graph_type_parser.parse_nested_graph_type_specification()?);
            let consumed = graph_type_parser.current_position();
            if consumed == 0 {
                return Err(self.error_here("expected nested graph type specification"));
            }
            for _ in 0..consumed {
                self.stream.advance();
            }

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
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
    pub(super) fn parse_binding_table_reference_value_type(
        &mut self,
    ) -> ParseResult<BindingTableReferenceValueType> {
        let start = self.stream.current().span.start;
        if self.stream.check(&TokenKind::Binding) {
            self.stream.advance();
            self.stream.expect(TokenKind::Table)?;
        } else {
            self.stream.expect(TokenKind::Table)?;
        }

        // Optional field types specification
        let field_types = if self.stream.check(&TokenKind::LBrace) {
            Some(self.parse_field_types_specification()?)
        } else {
            None
        };

        let not_null = self.check_not_null();
        let end = self
            .stream
            .tokens()
            .get(self.stream.position().saturating_sub(1))
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
    pub(super) fn parse_node_reference_value_type(
        &mut self,
    ) -> ParseResult<NodeReferenceValueType> {
        let start = self.stream.current().span.start;
        let had_any = self.stream.consume(&TokenKind::Any);
        let starts_with_node_keyword =
            self.stream.check(&TokenKind::Vertex) || self.stream.check(&TokenKind::Node);
        let parse_keyword_as_any = starts_with_node_keyword
            && (had_any || !self.looks_like_typed_node_spec_after_keyword());

        if parse_keyword_as_any {
            let use_vertex = if self.stream.check(&TokenKind::Vertex) {
                self.stream.advance();
                true
            } else {
                self.stream.advance();
                false
            };

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
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
            let start = self.stream.position();
            let mut graph_type_parser = GraphTypeParser::new(&self.stream.tokens()[start..]);
            let spec = Box::new(graph_type_parser.parse_node_type_specification()?);
            let consumed = graph_type_parser.current_position();
            if consumed == 0 {
                return Err(self.error_here("expected node type specification"));
            }
            for _ in 0..consumed {
                self.stream.advance();
            }

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
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
    pub(super) fn parse_edge_reference_value_type(
        &mut self,
    ) -> ParseResult<EdgeReferenceValueType> {
        let start = self.stream.current().span.start;
        let had_any = self.stream.consume(&TokenKind::Any);
        let starts_with_edge_keyword =
            self.stream.check(&TokenKind::Relationship) || self.stream.check(&TokenKind::Edge);
        let parse_keyword_as_any = starts_with_edge_keyword
            && (had_any || !self.looks_like_typed_edge_spec_after_keyword());

        if parse_keyword_as_any {
            let use_relationship = if self.stream.check(&TokenKind::Relationship) {
                self.stream.advance();
                true
            } else {
                self.stream.advance();
                false
            };

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
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
            let start = self.stream.position();
            let mut graph_type_parser = GraphTypeParser::new(&self.stream.tokens()[start..]);
            let spec = Box::new(graph_type_parser.parse_edge_type_specification()?);
            let consumed = graph_type_parser.current_position();
            if consumed == 0 {
                return Err(self.error_here("expected edge type specification"));
            }
            for _ in 0..consumed {
                self.stream.advance();
            }

            let not_null = self.check_not_null();
            let end = self
                .stream
                .tokens()
                .get(self.stream.position().saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);
            Ok(EdgeReferenceValueType::Typed {
                spec,
                not_null,
                span: start..end,
            })
        }
    }

    fn looks_like_typed_node_spec_after_keyword(&self) -> bool {
        let Some(next) = self.stream.peek() else {
            return false;
        };
        matches!(
            next.kind,
            TokenKind::Type
                | TokenKind::LParen
                | TokenKind::Label
                | TokenKind::Labels
                | TokenKind::Is
                | TokenKind::Colon
                | TokenKind::LBrace
                | TokenKind::Key
                | TokenKind::As
                | TokenKind::Identifier(_)
        ) || next.kind.is_non_reserved_identifier_keyword()
    }

    fn looks_like_typed_edge_spec_after_keyword(&self) -> bool {
        let Some(next) = self.stream.peek() else {
            return false;
        };
        matches!(
            next.kind,
            TokenKind::Type
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::Label
                | TokenKind::Labels
                | TokenKind::Is
                | TokenKind::Colon
                | TokenKind::LBrace
                | TokenKind::Connecting
                | TokenKind::Directed
                | TokenKind::Undirected
                | TokenKind::Identifier(_)
        ) || next.kind.is_non_reserved_identifier_keyword()
    }
}
