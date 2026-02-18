//! Graph type specification parsing for GQL.
//!
//! This module implements parsing of the complete graph type specification system,
//! enabling comprehensive schema definition for property graphs including node types,
//! edge types, property types, label sets, and connectivity constraints.

use crate::ast::{
    ArcTypePointingLeft, ArcTypePointingRight, ArcTypeUndirected, DirectedArcType, EdgeKind,
    EdgeTypePattern, EdgeTypePatternDirected, EdgeTypePatternUndirected, EdgeTypeSpec,
    EdgeTypeFiller, EdgeTypeLabelSet, EdgeTypePhrase, EdgeTypePhraseContent, EdgeTypePropertyTypes,
    ElementTypeList, ElementTypeSpecification, EndpointPair, EndpointPairPhrase,
    GraphTypeSpecificationBody, LabelName, LabelSetPhrase, LabelSetSpecification,
    LocalNodeTypeAlias, NestedGraphTypeSpec, NodeTypeFiller,
    NodeTypeKeyLabelSet, NodeTypeLabelSet, NodeTypePattern, NodeTypePhrase,
    NodeTypePropertyTypes, NodeTypeReference, NodeTypeSpec, PropertyName, PropertyType,
    PropertyTypeList, PropertyTypesSpecification, PropertyValueType, Span,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::types::TypeParser;

type ParseError = Box<Diag>;
type ParseResult<T> = Result<T, ParseError>;

/// Parser for graph type specifications.
pub struct GraphTypeParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> GraphTypeParser<'a> {
    /// Creates a new graph type parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Returns the current token.
    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream must be non-empty"))
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
            Err(self.error_here(format!("expected {:?}, found {:?}", kind, self.current().kind)))
        }
    }

    /// Creates an error at the current position.
    fn error_here(&self, message: String) -> Box<Diag> {
        Box::new(
            Diag::error(message)
                .with_primary_label(self.current().span.clone(), "here")
                .with_code("P_GRAPH_TYPE"),
        )
    }

    /// Merges two spans into a single span covering both.
    fn merge_spans(&self, start: &Span, end: &Span) -> Span {
        start.start..end.end
    }

    // ========================================================================
    // Nested Graph Type Specification (Top-level)
    // ========================================================================

    /// Parses a nested graph type specification.
    ///
    /// Syntax: `{ graph_type_specification_body }`
    pub fn parse_nested_graph_type_specification(&mut self) -> ParseResult<NestedGraphTypeSpec> {
        let start_span = self.expect(TokenKind::LBrace)?;
        let body = self.parse_graph_type_specification_body()?;
        let end_span = self.expect(TokenKind::RBrace)?;
        let span = self.merge_spans(&start_span, &end_span);

        Ok(NestedGraphTypeSpec { body, span })
    }

    /// Parses a graph type specification body.
    ///
    /// Contains element type list (nodes and edges).
    fn parse_graph_type_specification_body(&mut self) -> ParseResult<GraphTypeSpecificationBody> {
        let start_span = self.current().span.clone();

        // Empty body is allowed
        if self.check(&TokenKind::RBrace) {
            let element_types = ElementTypeList {
                types: Vec::new(),
                span: start_span.clone(),
            };
            return Ok(GraphTypeSpecificationBody {
                element_types,
                span: start_span,
            });
        }

        let element_types = self.parse_element_type_list()?;
        let span = element_types.span.clone();

        Ok(GraphTypeSpecificationBody {
            element_types,
            span,
        })
    }

    /// Parses an element type list (comma-separated).
    fn parse_element_type_list(&mut self) -> ParseResult<ElementTypeList> {
        let start_span = self.current().span.clone();
        let mut types = Vec::new();

        // Parse first element type
        types.push(self.parse_element_type_specification()?);

        // Parse remaining comma-separated element types
        while self.consume(&TokenKind::Comma) {
            // Allow trailing comma
            if self.check(&TokenKind::RBrace) {
                break;
            }
            types.push(self.parse_element_type_specification()?);
        }

        let end_span = types.last().map(|t| match t {
            ElementTypeSpecification::Node(n) => n.span.clone(),
            ElementTypeSpecification::Edge(e) => e.span.clone(),
        }).unwrap_or_else(|| start_span.clone());

        Ok(ElementTypeList {
            types,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses an element type specification (node or edge).
    ///
    /// Dispatches to node type or edge type parser based on lookahead.
    fn parse_element_type_specification(&mut self) -> ParseResult<ElementTypeSpecification> {
        // Check for node type keywords
        if self.check(&TokenKind::Node) || self.check(&TokenKind::Vertex) {
            let node_type = self.parse_node_type_specification()?;
            return Ok(ElementTypeSpecification::Node(Box::new(node_type)));
        }

        // Check for edge type keywords
        if self.check(&TokenKind::Edge)
            || self.check(&TokenKind::Relationship)
            || self.check(&TokenKind::Directed)
            || self.check(&TokenKind::Undirected)
        {
            let edge_type = self.parse_edge_type_specification()?;
            return Ok(ElementTypeSpecification::Edge(Box::new(edge_type)));
        }

        if self.check(&TokenKind::LParen) {
            if self.is_edge_pattern_after_left_endpoint() {
                let edge_type = self.parse_edge_type_specification()?;
                return Ok(ElementTypeSpecification::Edge(Box::new(edge_type)));
            }
            let node_type = self.parse_node_type_specification()?;
            return Ok(ElementTypeSpecification::Node(Box::new(node_type)));
        }

        Err(self.error_here(
            "expected NODE, EDGE, or edge type pattern".to_string()
        ))
    }

    // ========================================================================
    // Node Type Specifications
    // ========================================================================

    /// Parses a node type specification.
    ///
    /// Syntax: `node_type_pattern`
    pub fn parse_node_type_specification(&mut self) -> ParseResult<NodeTypeSpec> {
        let saved = self.pos;
        if let Ok(pattern) = self.parse_node_type_pattern() {
            let span = pattern.span.clone();
            return Ok(NodeTypeSpec { pattern, span });
        }

        self.pos = saved;
        let phrase = self.parse_node_type_phrase()?;
        let span = phrase.span.clone();
        Ok(NodeTypeSpec {
            pattern: NodeTypePattern {
                phrase,
                span: span.clone(),
            },
            span,
        })
    }

    /// Parses a node type pattern.
    fn parse_node_type_pattern(&mut self) -> ParseResult<NodeTypePattern> {
        let start_span = self.current().span.clone();

        // Optional leading node synonym/type/name prefix.
        if self.consume(&TokenKind::Node) || self.consume(&TokenKind::Vertex) {
            self.consume(&TokenKind::Type);
            if self.is_regular_identifier_start() {
                let _ = self.parse_regular_identifier("node type name")?;
            }
        }

        self.expect(TokenKind::LParen)?;
        let alias = if self.is_regular_identifier_start() {
            let (name, span) = self.parse_regular_identifier("node type alias")?;
            Some(LocalNodeTypeAlias { name, span })
        } else {
            None
        };
        let filler = if self.is_node_type_filler_start() {
            Some(self.parse_node_type_filler()?)
        } else {
            None
        };
        let end_span = self.expect(TokenKind::RParen)?;

        let phrase_end = filler
            .as_ref()
            .map(|f| f.span.clone())
            .or_else(|| alias.as_ref().map(|a| a.span.clone()))
            .unwrap_or_else(|| end_span.clone());
        let phrase = NodeTypePhrase {
            filler,
            alias,
            span: self.merge_spans(&start_span, &phrase_end),
        };

        Ok(NodeTypePattern {
            span: self.merge_spans(&start_span, &end_span),
            phrase,
        })
    }

    /// Parses a node type phrase.
    ///
    /// Syntax: `[NODE [TYPE]] [node_type_filler] [AS alias]`
    fn parse_node_type_phrase(&mut self) -> ParseResult<NodeTypePhrase> {
        let start_span = self.current().span.clone();

        let has_node_keyword = self.consume(&TokenKind::Node) || self.consume(&TokenKind::Vertex);
        if !has_node_keyword {
            return Err(self.error_here("expected NODE or VERTEX".to_string()));
        }
        self.consume(&TokenKind::Type);

        let mut has_phrase_name = false;
        if self.is_regular_identifier_start() {
            let _ = self.parse_regular_identifier("node type name")?;
            has_phrase_name = true;
        }
        let filler = if self.is_node_type_filler_start() {
            Some(self.parse_node_type_filler()?)
        } else {
            None
        };
        if !has_phrase_name && filler.is_none() {
            return Err(self.error_here("expected node type name or node type filler".to_string()));
        }

        // Optional AS alias
        let alias = if self.consume(&TokenKind::As) {
            Some(self.parse_local_node_type_alias()?)
        } else {
            None
        };

        let end_span = alias.as_ref().map(|a| a.span.clone())
            .or_else(|| filler.as_ref().map(|f| f.span.clone()))
            .unwrap_or_else(|| start_span.clone());

        Ok(NodeTypePhrase {
            filler,
            alias,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Checks if current position starts a node type filler.
    fn is_node_type_filler_start(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Label | TokenKind::Labels | TokenKind::Is |
            TokenKind::Colon | TokenKind::LBrace | TokenKind::Key
        )
    }

    /// Parses a local node type alias.
    fn parse_local_node_type_alias(&mut self) -> ParseResult<LocalNodeTypeAlias> {
        let (name, span) = self.parse_regular_identifier("identifier for node type alias")?;
        Ok(LocalNodeTypeAlias { name, span })
    }

    // ========================================================================
    // Node Type Filler
    // ========================================================================

    /// Parses a node type filler.
    ///
    /// Contains labels, properties, keys, and implied content.
    fn parse_node_type_filler(&mut self) -> ParseResult<NodeTypeFiller> {
        let start_span = self.current().span.clone();
        let mut label_set = None;
        let mut property_types = None;
        let mut key_label_set = None;
        let implied_content = None;

        // Parse optional label set
        if self.is_label_set_phrase_start() {
            label_set = Some(self.parse_node_type_label_set()?);
        }

        // Parse optional property types
        if self.check(&TokenKind::LBrace) {
            property_types = Some(self.parse_node_type_property_types()?);
        }

        // Parse optional key label set
        if self.consume(&TokenKind::Key) {
            key_label_set = Some(self.parse_node_type_key_label_set()?);
        }

        // implied_content is left as None - this is an optional future enhancement
        // that is not currently part of the GQL grammar specification

        let end_span = key_label_set.as_ref().map(|k| k.span.clone())
            .or_else(|| property_types.as_ref().map(|p| p.span.clone()))
            .or_else(|| label_set.as_ref().map(|l| l.span.clone()))
            .unwrap_or_else(|| start_span.clone());

        Ok(NodeTypeFiller {
            label_set,
            property_types,
            key_label_set,
            implied_content,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a node type label set.
    fn parse_node_type_label_set(&mut self) -> ParseResult<NodeTypeLabelSet> {
        let label_set_phrase = self.parse_label_set_phrase()?;
        let span = label_set_phrase_span(&label_set_phrase);

        Ok(NodeTypeLabelSet {
            label_set_phrase,
            span,
        })
    }

    /// Parses node type property types.
    fn parse_node_type_property_types(&mut self) -> ParseResult<NodeTypePropertyTypes> {
        let specification = self.parse_property_types_specification()?;
        let span = specification.span.clone();

        Ok(NodeTypePropertyTypes {
            specification,
            span,
        })
    }

    /// Parses a node type key label set.
    ///
    /// Syntax: `KEY label_set_specification` (KEY keyword already consumed)
    fn parse_node_type_key_label_set(&mut self) -> ParseResult<NodeTypeKeyLabelSet> {
        let label_set = self.parse_label_set_specification()?;
        let span = label_set.span.clone();

        Ok(NodeTypeKeyLabelSet { label_set, span })
    }

    // ========================================================================
    // Edge Type Specifications
    // ========================================================================

    /// Parses an edge type specification.
    ///
    /// Can be directed or undirected edge pattern.
    pub fn parse_edge_type_specification(&mut self) -> ParseResult<EdgeTypeSpec> {
        let pattern = self.parse_edge_type_pattern()?;
        let span = match &pattern {
            EdgeTypePattern::Directed(d) => d.span.clone(),
            EdgeTypePattern::Undirected(u) => u.span.clone(),
        };

        Ok(EdgeTypeSpec { pattern, span })
    }

    /// Parses an edge type pattern (directed or undirected).
    fn parse_edge_type_pattern(&mut self) -> ParseResult<EdgeTypePattern> {
        // Check for edge type phrase (keywords before pattern)
        if self.check(&TokenKind::Directed) || self.check(&TokenKind::Undirected)
            || self.check(&TokenKind::Edge) || self.check(&TokenKind::Relationship) {
            return self.parse_edge_type_phrase_pattern();
        }

        // Otherwise, parse visual edge pattern
        self.parse_edge_type_visual_pattern()
    }

    /// Parses edge type pattern from phrase keywords.
    fn parse_edge_type_phrase_pattern(&mut self) -> ParseResult<EdgeTypePattern> {
        let start_span = self.current().span.clone();

        // Parse edge kind
        let edge_kind = if self.consume(&TokenKind::Directed) {
            EdgeKind::Directed
        } else if self.consume(&TokenKind::Undirected) {
            EdgeKind::Undirected
        } else {
            EdgeKind::Inferred
        };

        // Expect EDGE or RELATIONSHIP keyword
        if !self.consume(&TokenKind::Edge) && !self.consume(&TokenKind::Relationship) {
            return Err(self.error_here("expected EDGE or RELATIONSHIP keyword".to_string()));
        }

        // Optional TYPE keyword
        self.consume(&TokenKind::Type);

        // Optional edge type name
        if self.is_regular_identifier_start() {
            let _ = self.parse_regular_identifier("edge type name")?;
        }

        // Parse optional phrase content (labels and properties)
        let filler_content = if self.is_label_set_phrase_start() || self.check(&TokenKind::LBrace) {
            Some(self.parse_edge_type_phrase_content()?)
        } else {
            None
        };

        // Expect CONNECTING keyword
        self.expect(TokenKind::Connecting)?;

        // Parse endpoint pair
        let endpoint_pair_phrase = self.parse_endpoint_pair_phrase()?;
        let end_span = endpoint_pair_phrase.span.clone();

        let phrase = EdgeTypePhrase {
            edge_kind,
            filler_content,
            endpoint_pair_phrase,
            span: self.merge_spans(&start_span, &end_span),
        };

        let filler = EdgeTypeFiller {
            span: phrase.span.clone(),
            phrase,
        };

        // Create a simple directed pattern (this is a simplification)
        // In a full implementation, we'd need to construct proper endpoint patterns
        let left_endpoint = NodeTypePattern {
            phrase: NodeTypePhrase {
                filler: None,
                alias: None,
                span: start_span.clone(),
            },
            span: start_span.clone(),
        };
        let right_endpoint = left_endpoint.clone();

        let arc = DirectedArcType::PointingRight(ArcTypePointingRight {
            filler: Some(filler),
            span: start_span.clone(),
        });

        Ok(EdgeTypePattern::Directed(EdgeTypePatternDirected {
            left_endpoint,
            arc,
            right_endpoint,
            span: self.merge_spans(&start_span, &end_span),
        }))
    }

    /// Parses visual edge type pattern: `(node)-[edge]->(node)` or `(node)~[edge]~(node)`
    fn parse_edge_type_visual_pattern(&mut self) -> ParseResult<EdgeTypePattern> {
        let start_span = self.current().span.clone();

        // Parse left endpoint: node_type_pattern
        let left_endpoint = self.parse_node_type_pattern()?;

        // Check for directed or undirected arc
        let is_directed = self.check(&TokenKind::Minus)
            || self.check(&TokenKind::Lt)
            || self.check(&TokenKind::LeftArrow);
        let is_undirected = self.check(&TokenKind::Tilde);

        if is_directed {
            let is_pointing_left = self.check(&TokenKind::Lt) || self.check(&TokenKind::LeftArrow);

            if is_pointing_left {
                // <-[edge]-
                if self.consume(&TokenKind::LeftArrow) {
                    // already consumed "<-"
                } else {
                    self.expect(TokenKind::Lt)?;
                    self.expect(TokenKind::Minus)?;
                }
                let filler = self.parse_edge_arc_filler()?;
                self.expect(TokenKind::Minus)?;

                let arc = DirectedArcType::PointingLeft(ArcTypePointingLeft {
                    filler,
                    span: start_span.clone(),
                });

                // Parse right endpoint
                let right_endpoint = self.parse_node_type_pattern()?;
                let end_span = right_endpoint.span.clone();

                Ok(EdgeTypePattern::Directed(EdgeTypePatternDirected {
                    left_endpoint,
                    arc,
                    right_endpoint,
                    span: self.merge_spans(&start_span, &end_span),
                }))
            } else {
                // -[edge]->
                self.expect(TokenKind::Minus)?;
                let filler = self.parse_edge_arc_filler()?;
                if !self.consume(&TokenKind::Arrow) {
                    self.expect(TokenKind::Minus)?;
                    self.expect(TokenKind::Gt)?;
                }

                let arc = DirectedArcType::PointingRight(ArcTypePointingRight {
                    filler,
                    span: start_span.clone(),
                });

                // Parse right endpoint
                let right_endpoint = self.parse_node_type_pattern()?;
                let end_span = right_endpoint.span.clone();

                Ok(EdgeTypePattern::Directed(EdgeTypePatternDirected {
                    left_endpoint,
                    arc,
                    right_endpoint,
                    span: self.merge_spans(&start_span, &end_span),
                }))
            }
        } else if is_undirected {
            // ~[edge]~
            self.expect(TokenKind::Tilde)?;
            let filler = self.parse_edge_arc_filler()?;
            self.expect(TokenKind::Tilde)?;

            let arc = ArcTypeUndirected {
                filler,
                span: start_span.clone(),
            };

            // Parse right endpoint
            let right_endpoint = self.parse_node_type_pattern()?;
            let end_span = right_endpoint.span.clone();

            Ok(EdgeTypePattern::Undirected(EdgeTypePatternUndirected {
                left_endpoint,
                arc,
                right_endpoint,
                span: self.merge_spans(&start_span, &end_span),
            }))
        } else {
            Err(self.error_here("expected edge pattern: -, <-, or ~".to_string()))
        }
    }

    /// Parses edge arc filler within brackets: `[edge_type_filler?]`
    fn parse_edge_arc_filler(&mut self) -> ParseResult<Option<EdgeTypeFiller>> {
        self.expect(TokenKind::LBracket)?;

        // Empty brackets allowed
        if self.check(&TokenKind::RBracket) {
            self.advance();
            return Ok(None);
        }

        // Parse edge type filler if present
        let filler = if self.is_edge_type_filler_start() {
            Some(self.parse_edge_type_filler()?)
        } else {
            None
        };

        self.expect(TokenKind::RBracket)?;
        Ok(filler)
    }

    /// Checks if current position starts edge type filler.
    fn is_edge_type_filler_start(&self) -> bool {
        self.is_label_set_phrase_start() || self.check(&TokenKind::LBrace)
            || matches!(self.current().kind, TokenKind::Identifier(_))
    }

    /// Parses edge type filler.
    fn parse_edge_type_filler(&mut self) -> ParseResult<EdgeTypeFiller> {
        let start_span = self.current().span.clone();

        // For now, we'll create a simple phrase with minimal content
        // A full implementation would parse edge labels/properties more completely
        let filler_content = if self.is_label_set_phrase_start() || self.check(&TokenKind::LBrace) {
            Some(self.parse_edge_type_phrase_content()?)
        } else {
            None
        };

        // Create a minimal endpoint pair for now
        let endpoint_pair = EndpointPair {
            source: NodeTypeReference {
                node_type: NodeTypePattern {
                    phrase: NodeTypePhrase {
                        filler: None,
                        alias: None,
                        span: start_span.clone(),
                    },
                    span: start_span.clone(),
                },
                span: start_span.clone(),
            },
            destination: NodeTypeReference {
                node_type: NodeTypePattern {
                    phrase: NodeTypePhrase {
                        filler: None,
                        alias: None,
                        span: start_span.clone(),
                    },
                    span: start_span.clone(),
                },
                span: start_span.clone(),
            },
            span: start_span.clone(),
        };

        let phrase = EdgeTypePhrase {
            edge_kind: EdgeKind::Inferred,
            filler_content,
            endpoint_pair_phrase: EndpointPairPhrase {
                endpoint_pair,
                span: start_span.clone(),
            },
            span: start_span.clone(),
        };

        Ok(EdgeTypeFiller {
            phrase,
            span: start_span,
        })
    }

    /// Parses edge type phrase content (labels and properties).
    fn parse_edge_type_phrase_content(&mut self) -> ParseResult<EdgeTypePhraseContent> {
        let start_span = self.current().span.clone();
        let mut label_set = None;
        let mut property_types = None;

        // Parse optional label set
        if self.is_label_set_phrase_start() {
            label_set = Some(self.parse_edge_type_label_set()?);
        }

        // Parse optional property types
        if self.check(&TokenKind::LBrace) {
            property_types = Some(self.parse_edge_type_property_types()?);
        }

        let end_span = property_types.as_ref().map(|p| p.span.clone())
            .or_else(|| label_set.as_ref().map(|l| l.span.clone()))
            .unwrap_or_else(|| start_span.clone());

        Ok(EdgeTypePhraseContent {
            label_set,
            property_types,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses edge type label set.
    fn parse_edge_type_label_set(&mut self) -> ParseResult<EdgeTypeLabelSet> {
        let label_set_phrase = self.parse_label_set_phrase()?;
        let span = label_set_phrase_span(&label_set_phrase);

        Ok(EdgeTypeLabelSet {
            label_set_phrase,
            span,
        })
    }

    /// Parses edge type property types.
    fn parse_edge_type_property_types(&mut self) -> ParseResult<EdgeTypePropertyTypes> {
        let specification = self.parse_property_types_specification()?;
        let span = specification.span.clone();

        Ok(EdgeTypePropertyTypes {
            specification,
            span,
        })
    }

    // ========================================================================
    // Endpoint Pairs
    // ========================================================================

    /// Parses an endpoint pair phrase.
    ///
    /// Syntax: `CONNECTING (endpoint_pair)` (CONNECTING already consumed)
    fn parse_endpoint_pair_phrase(&mut self) -> ParseResult<EndpointPairPhrase> {
        self.expect(TokenKind::LParen)?;
        let endpoint_pair = self.parse_endpoint_pair()?;
        let end_span = self.expect(TokenKind::RParen)?;
        let span = self.merge_spans(&endpoint_pair.span, &end_span);

        Ok(EndpointPairPhrase {
            endpoint_pair,
            span,
        })
    }

    /// Parses an endpoint pair.
    ///
    /// Syntax: `source_node_type TO destination_node_type`
    fn parse_endpoint_pair(&mut self) -> ParseResult<EndpointPair> {
        let start_span = self.current().span.clone();

        // Parse source node type
        let source = self.parse_node_type_reference()?;

        // Expect TO keyword
        self.expect(TokenKind::To)?;

        // Parse destination node type
        let destination = self.parse_node_type_reference()?;
        let end_span = destination.span.clone();

        Ok(EndpointPair {
            source,
            destination,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a node type reference.
    fn parse_node_type_reference(&mut self) -> ParseResult<NodeTypeReference> {
        let node_type = if self.is_regular_identifier_start() {
            let (name, span) = self.parse_regular_identifier("node type alias")?;
            NodeTypePattern {
                phrase: NodeTypePhrase {
                    filler: None,
                    alias: Some(LocalNodeTypeAlias {
                        name,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                span,
            }
        } else {
            self.parse_node_type_pattern()?
        };
        let span = node_type.span.clone();

        Ok(NodeTypeReference { node_type, span })
    }

    // ========================================================================
    // Property Types Specification
    // ========================================================================

    /// Parses a property types specification.
    ///
    /// Syntax: `{ property_type_list? }`
    pub fn parse_property_types_specification(&mut self) -> ParseResult<PropertyTypesSpecification> {
        let start_span = self.expect(TokenKind::LBrace)?;

        // Empty braces allowed
        if self.check(&TokenKind::RBrace) {
            let end_span = self.expect(TokenKind::RBrace)?;
            return Ok(PropertyTypesSpecification {
                property_types: None,
                span: self.merge_spans(&start_span, &end_span),
            });
        }

        // Parse property type list
        let property_types = Some(self.parse_property_type_list()?);
        let end_span = self.expect(TokenKind::RBrace)?;

        Ok(PropertyTypesSpecification {
            property_types,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a property type list (comma-separated).
    fn parse_property_type_list(&mut self) -> ParseResult<PropertyTypeList> {
        let start_span = self.current().span.clone();
        let mut types = Vec::new();

        // Parse first property type
        types.push(self.parse_property_type()?);

        // Parse remaining comma-separated property types
        while self.consume(&TokenKind::Comma) {
            // Allow trailing comma
            if self.check(&TokenKind::RBrace) {
                break;
            }
            types.push(self.parse_property_type()?);
        }

        let end_span = types.last().map(|t| t.span.clone())
            .unwrap_or_else(|| start_span.clone());

        Ok(PropertyTypeList {
            types,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a property type.
    ///
    /// Syntax: `property_name [:: | TYPED] value_type [NOT NULL]`
    fn parse_property_type(&mut self) -> ParseResult<PropertyType> {
        let name = self.parse_property_name()?;
        let name_span = name.span.clone();

        // Typed marker is optional per grammar.
        let _typed = self.consume(&TokenKind::DoubleColon) || self.consume(&TokenKind::Typed);

        // Parse value type using TypeParser
        let value_type = self.parse_property_value_type()?;

        // Check for NOT NULL constraint
        let not_null = if self.consume(&TokenKind::Not) {
            self.expect(TokenKind::Null)?;
            true
        } else {
            false
        };

        let end_span = if not_null {
            self.tokens[self.pos.saturating_sub(1)].span.clone()
        } else {
            value_type.span.clone()
        };

        Ok(PropertyType {
            name,
            value_type,
            not_null,
            span: self.merge_spans(&name_span, &end_span),
        })
    }

    /// Parses a property name.
    fn parse_property_name(&mut self) -> ParseResult<PropertyName> {
        let (name, span) = self.parse_regular_identifier("property name identifier")?;
        Ok(PropertyName { name, span })
    }

    /// Parses a property value type.
    fn parse_property_value_type(&mut self) -> ParseResult<PropertyValueType> {
        // Use TypeParser to parse the value type
        let mut type_parser = TypeParser::new(&self.tokens[self.pos..]);
        let value_type = type_parser.parse_value_type()
            .map_err(|_e| self.error_here("failed to parse property value type".to_string()))?;

        // Advance our position by how much the type parser consumed
        self.pos += type_parser.current_position();

        let span = value_type.span();

        Ok(PropertyValueType { value_type, span })
    }

    // ========================================================================
    // Label Set Phrases and Specifications
    // ========================================================================

    /// Checks if current position starts a label set phrase.
    fn is_label_set_phrase_start(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Label | TokenKind::Labels | TokenKind::Is | TokenKind::Colon
        )
    }

    /// Parses a label set phrase.
    ///
    /// Syntax variants:
    /// - `LABEL label_name`
    /// - `LABELS label_set_specification`
    /// - `IS label_set_specification`
    /// - `: label_set_specification`
    pub fn parse_label_set_phrase(&mut self) -> ParseResult<LabelSetPhrase> {
        if self.consume(&TokenKind::Label) {
            // LABEL <label_name>
            let label = self.parse_label_name()?;
            Ok(LabelSetPhrase::Label(label))
        } else if self.consume(&TokenKind::Labels) {
            // LABELS <label_set_specification>
            let label_set = self.parse_label_set_specification()?;
            Ok(LabelSetPhrase::Labels(label_set))
        } else if self.consume(&TokenKind::Is) || self.consume(&TokenKind::Colon) {
            // IS or : <label_set_specification>
            let label_set = self.parse_label_set_specification()?;
            Ok(LabelSetPhrase::IsLabelSet(label_set))
        } else {
            Err(self.error_here("expected LABEL, LABELS, IS, or :".to_string()))
        }
    }

    /// Parses a label set specification (ampersand-separated labels).
    ///
    /// Syntax: `label1 & label2 & label3 & ...`
    pub fn parse_label_set_specification(&mut self) -> ParseResult<LabelSetSpecification> {
        let start_span = self.current().span.clone();
        let mut labels = Vec::new();

        // Parse first label
        labels.push(self.parse_label_name()?);

        // Parse remaining ampersand-separated labels
        while self.consume(&TokenKind::Ampersand) {
            labels.push(self.parse_label_name()?);
        }

        let end_span = labels.last().map(|l| l.span.clone())
            .unwrap_or_else(|| start_span.clone());

        Ok(LabelSetSpecification {
            labels,
            span: self.merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a label name.
    fn parse_label_name(&mut self) -> ParseResult<LabelName> {
        let (name, span) = self.parse_regular_identifier("label name identifier")?;
        Ok(LabelName { name, span })
    }

    fn parse_regular_identifier(&mut self, expected: &str) -> ParseResult<(smol_str::SmolStr, Span)> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok((name.clone(), token.span))
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                self.advance();
                Ok((smol_str::SmolStr::new(kind.to_string()), token.span))
            }
            _ => Err(self.error_here(format!("expected {expected}"))),
        }
    }

    fn is_regular_identifier_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
            || self.current().kind.is_non_reserved_identifier_keyword()
    }

    fn is_edge_pattern_after_left_endpoint(&self) -> bool {
        if !self.check(&TokenKind::LParen) {
            return false;
        }
        let mut depth = 0usize;
        let mut cursor = self.pos;
        while cursor < self.tokens.len() {
            match self.tokens[cursor].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        cursor += 1;
                        break;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            cursor += 1;
        }
        matches!(
            self.tokens.get(cursor).map(|t| &t.kind),
            Some(TokenKind::Minus | TokenKind::Lt | TokenKind::LeftArrow | TokenKind::Tilde)
        )
    }

    /// Returns the current parser position (for integration with TypeParser).
    pub fn current_position(&self) -> usize {
        self.pos
    }
}

/// Helper function to extract span from LabelSetPhrase
fn label_set_phrase_span(phrase: &LabelSetPhrase) -> Span {
    match phrase {
        LabelSetPhrase::Label(l) => l.span.clone(),
        LabelSetPhrase::Labels(ls) => ls.span.clone(),
        LabelSetPhrase::IsLabelSet(ls) => ls.span.clone(),
    }
}
