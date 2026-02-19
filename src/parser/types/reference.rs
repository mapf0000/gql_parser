//! Reference value type parsing for GQL.
//!
//! This module handles parsing of reference types including:
//! - Graph reference types (ANY PROPERTY GRAPH, PROPERTY GRAPH)
//! - Binding table reference types (BINDING TABLE, TABLE)
//! - Node reference types (NODE, VERTEX)
//! - Edge reference types (EDGE, RELATIONSHIP)

use crate::ast::{
    BindingTableReferenceValueType, EdgeReferenceValueType, EdgeTypeSpecification,
    GraphReferenceValueType, NestedGraphTypeSpecification, NodeReferenceValueType,
    NodeTypeSpecification,
};
use crate::lexer::token::TokenKind;
use crate::parser::base::ParseResult;

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

            let spec_span = self.parse_placeholder_spec_span("nested graph type specification")?;

            // Create a minimal placeholder nested graph type spec
            let body = crate::ast::graph_type::GraphTypeSpecificationBody {
                element_types: crate::ast::graph_type::ElementTypeList {
                    types: Vec::new(),
                    span: spec_span.clone(),
                },
                span: spec_span.clone(),
            };
            let spec = Box::new(NestedGraphTypeSpecification {
                body,
                span: spec_span,
            });

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

        if self.stream.check(&TokenKind::Vertex) || self.stream.check(&TokenKind::Node) {
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
            let spec_span = self.parse_placeholder_spec_span("node type specification")?;

            // Create a minimal placeholder node type spec
            let pattern = crate::ast::graph_type::NodeTypePattern {
                phrase: crate::ast::graph_type::NodeTypePhrase {
                    filler: None,
                    alias: None,
                    span: spec_span.clone(),
                },
                span: spec_span.clone(),
            };
            let spec = Box::new(NodeTypeSpecification {
                pattern,
                span: spec_span,
            });

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

        if self.stream.check(&TokenKind::Relationship) || self.stream.check(&TokenKind::Edge) {
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
            let spec_span = self.parse_placeholder_spec_span("edge type specification")?;

            // Create a minimal placeholder edge type spec
            let left_endpoint = crate::ast::graph_type::NodeTypePattern {
                phrase: crate::ast::graph_type::NodeTypePhrase {
                    filler: None,
                    alias: None,
                    span: spec_span.clone(),
                },
                span: spec_span.clone(),
            };
            let right_endpoint = left_endpoint.clone();
            let arc = crate::ast::graph_type::DirectedArcType::PointingRight(
                crate::ast::graph_type::ArcTypePointingRight {
                    filler: None,
                    span: spec_span.clone(),
                },
            );
            let pattern = crate::ast::graph_type::EdgeTypePattern::Directed(
                crate::ast::graph_type::EdgeTypePatternDirected {
                    left_endpoint,
                    arc,
                    right_endpoint,
                    span: spec_span.clone(),
                },
            );
            let spec = Box::new(EdgeTypeSpecification {
                pattern,
                span: spec_span,
            });

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
}
