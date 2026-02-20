//! Graph type specification parsing for GQL.
//!
//! This module implements parsing of the complete graph type specification system,
//! enabling comprehensive schema definition for property graphs including node types,
//! edge types, property types, label sets, and connectivity constraints.
//!
//! # ISO GQL Compliance
//!
//! This parser implements the ISO GQL standard grammar rules as specified in
//! ISO/IEC 39075 (GQL). Key conformance points:
//!
//! ## Element Type Lists (§18.1)
//! - Element types (NODE TYPE, EDGE TYPE) are **comma-separated**, not semicolon-separated
//! - Grammar: `elementTypeList ::= elementTypeSpecification (COMMA elementTypeSpecification)*`
//! - Trailing commas are allowed before closing brace
//!
//! ## Inheritance Clauses (§18.2, §18.3)
//! - Supports single and multiple inheritance: `INHERITS Parent1, Parent2, Parent3`
//! - **Critical ambiguity resolution**: When parsing inheritance clauses, the parser must
//!   distinguish between commas for multiple parents vs. commas for element type separation
//! - Implementation: Checks for element type start keywords after consuming comma to determine
//!   if it's a new parent or a new element type
//!
//! ## Property Types Specification (§18.5)
//! - Grammar: `propertyTypesSpecification ::= LEFT_BRACE propertyTypeList? RIGHT_BRACE`
//! - Properties: `propertyName [::] propertyValueType [NOT NULL]`
//! - **Empty property blocks are allowed**: `{ }`
//!
//! ## Constraints
//! - Constraints appear **AFTER** property types specification, not inside `{ }` blocks
//! - Valid: `NODE TYPE Person { id :: INT } CONSTRAINT UNIQUE (id)`
//! - Invalid: `NODE TYPE Person { id :: INT, CONSTRAINT UNIQUE (id) }`
//! - Supported constraint types: UNIQUE, CHECK, MANDATORY, KEY, Custom
//!
//! ## Label Sets (§18.4)
//! - Single label: `LABEL LabelName`
//! - Multiple labels: `LABELS Label1 & Label2 & Label3`
//! - **Multiple LABEL clauses are NOT supported** (use LABELS with & instead)
//!
//! ## Edge Type Specifications (§18.3)
//! - Directed edges: `DIRECTED EDGE TYPE name CONNECTING (Source TO Destination)`
//! - Undirected edges: `UNDIRECTED EDGE TYPE name CONNECTING (Source TO Destination)`
//! - **Only ONE CONNECTING clause per edge type** (per ISO GQL grammar line 1633-1635)
//!
//! ## Abstract Types
//! - Both node and edge types can be marked ABSTRACT
//! - Syntax: `ABSTRACT NODE TYPE name` or `ABSTRACT DIRECTED EDGE TYPE name`
//!
//! # Grammar References
//!
//! This module implements these ISO GQL grammar rules (line numbers from GQL.g4):
//! - §18.1 `nestedGraphTypeSpecification` (line 1482-1488)
//! - §18.2 `nodeTypeSpecification` (line 1501-1545)
//! - §18.3 `edgeTypeSpecification` (line 1548-1588)
//! - §18.4 `labelSetPhrase` (line 1679-1687)
//! - §18.5 `propertyTypesSpecification` (line 1691-1697)
//!
//! # Example
//!
//! ```gql
//! CREATE GRAPH TYPE social_network AS {
//!     NODE TYPE Person
//!         LABEL Person {
//!             id :: INT,
//!             name :: STRING,
//!             email :: STRING
//!         }
//!         CONSTRAINT UNIQUE (id)
//!         CONSTRAINT UNIQUE (email),
//!     NODE TYPE Company
//!         LABELS Company & Organization {
//!             id :: INT,
//!             name :: STRING
//!         },
//!     DIRECTED EDGE TYPE WORKS_AT
//!         CONNECTING (Person TO Company)
//! }
//! ```

use crate::ast::{
    ArcTypePointingLeft, ArcTypePointingRight, ArcTypeUndirected, DirectedArcType, EdgeKind,
    EdgeTypeFiller, EdgeTypeLabelSet, EdgeTypePattern, EdgeTypePatternDirected,
    EdgeTypePatternUndirected, EdgeTypePhrase, EdgeTypePhraseContent, EdgeTypePropertyTypes,
    EdgeTypeSpec, ElementTypeList, ElementTypeSpecification, EndpointPair, EndpointPairPhrase,
    GraphTypeConstraint, GraphTypeConstraintArgument, GraphTypeSpecificationBody,
    InheritedTypeReference, LabelName, LabelSetPhrase, LabelSetSpecification, LocalNodeTypeAlias,
    NestedGraphTypeSpec, NodeTypeFiller, NodeTypeKeyLabelSet, NodeTypeLabelSet, NodeTypePattern,
    NodeTypePhrase, NodeTypePropertyTypes, NodeTypeReference, NodeTypeSpec, PropertyName,
    PropertyType, PropertyTypeList, PropertyTypesSpecification, PropertyValueType, Span,
    TypeInheritanceClause,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::base::{ParseResult, TokenStream, merge_spans};
use crate::parser::types::TypeParser;

/// Parser for graph type specifications.
pub struct GraphTypeParser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> GraphTypeParser<'a> {
    /// Creates a new graph type parser.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            stream: TokenStream::new(tokens),
        }
    }

    /// Creates an error at the current position with the P_GRAPH_TYPE code.
    fn error_here(&self, message: String) -> Box<Diag> {
        self.stream.error_here_with_code(message, "P_GRAPH_TYPE")
    }

    // ========================================================================
    // Nested Graph Type Specification (Top-level)
    // ========================================================================

    /// Parses a nested graph type specification.
    ///
    /// Syntax: `{ graph_type_specification_body }`
    pub fn parse_nested_graph_type_specification(&mut self) -> ParseResult<NestedGraphTypeSpec> {
        let start_span = self.stream.expect(TokenKind::LBrace)?;
        let body = self.parse_graph_type_specification_body()?;
        let end_span = self.stream.expect(TokenKind::RBrace)?;
        let span = merge_spans(&start_span, &end_span);

        Ok(NestedGraphTypeSpec { body, span })
    }

    /// Parses a graph type specification body.
    ///
    /// Contains element type list (nodes and edges).
    fn parse_graph_type_specification_body(&mut self) -> ParseResult<GraphTypeSpecificationBody> {
        let start_span = self.stream.current().span.clone();

        // Empty body is allowed
        if self.stream.check(&TokenKind::RBrace) {
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
        let start_span = self.stream.current().span.clone();
        let mut types = Vec::new();

        // Parse first element type
        types.push(self.parse_element_type_specification()?);

        // Parse remaining comma-separated element types
        while self.stream.consume(&TokenKind::Comma) {
            // Allow trailing comma
            if self.stream.check(&TokenKind::RBrace) {
                break;
            }
            types.push(self.parse_element_type_specification()?);
        }

        let end_span = types
            .last()
            .map(|t| match t {
                ElementTypeSpecification::Node(n) => n.span.clone(),
                ElementTypeSpecification::Edge(e) => e.span.clone(),
            })
            .unwrap_or_else(|| start_span.clone());

        Ok(ElementTypeList {
            types,
            span: merge_spans(&start_span, &end_span),
        })
    }

    /// Parses an element type specification (node or edge).
    ///
    /// Dispatches to node type or edge type parser based on lookahead.
    fn parse_element_type_specification(&mut self) -> ParseResult<ElementTypeSpecification> {
        let is_abstract = self.consume_word("ABSTRACT");

        // Check for node type keywords
        if self.stream.check(&TokenKind::Node) || self.stream.check(&TokenKind::Vertex) {
            let node_type = self.parse_node_type_specification(is_abstract)?;
            return Ok(ElementTypeSpecification::Node(Box::new(node_type)));
        }

        // Check for edge type keywords
        if self.stream.check(&TokenKind::Edge)
            || self.stream.check(&TokenKind::Relationship)
            || self.stream.check(&TokenKind::Directed)
            || self.stream.check(&TokenKind::Undirected)
        {
            let edge_type = self.parse_edge_type_specification(is_abstract)?;
            return Ok(ElementTypeSpecification::Edge(Box::new(edge_type)));
        }

        if self.stream.check(&TokenKind::LParen) {
            if self.is_edge_pattern_after_left_endpoint() {
                let edge_type = self.parse_edge_type_specification(is_abstract)?;
                return Ok(ElementTypeSpecification::Edge(Box::new(edge_type)));
            }
            let node_type = self.parse_node_type_specification(is_abstract)?;
            return Ok(ElementTypeSpecification::Node(Box::new(node_type)));
        }

        Err(self.error_here("expected NODE, EDGE, or edge type pattern".to_string()))
    }

    // ========================================================================
    // Node Type Specifications
    // ========================================================================

    /// Parses a node type specification.
    ///
    /// Syntax: `node_type_pattern`
    pub fn parse_node_type_specification(
        &mut self,
        is_abstract: bool,
    ) -> ParseResult<NodeTypeSpec> {
        let saved = self.stream.position();
        if let Ok((pattern, inheritance)) = self.parse_node_type_pattern() {
            let span = pattern.span.clone();
            return Ok(NodeTypeSpec {
                is_abstract,
                inheritance,
                pattern,
                span,
            });
        }

        self.stream.set_position(saved);
        let (phrase, inheritance) = self.parse_node_type_phrase()?;
        let span = phrase.span.clone();
        Ok(NodeTypeSpec {
            is_abstract,
            inheritance,
            pattern: NodeTypePattern {
                phrase,
                span: span.clone(),
            },
            span,
        })
    }

    /// Parses a node type pattern.
    fn parse_node_type_pattern(
        &mut self,
    ) -> ParseResult<(NodeTypePattern, Option<TypeInheritanceClause>)> {
        let start_span = self.stream.current().span.clone();
        let mut inheritance = None;

        // Optional leading node synonym/type/name prefix.
        if self.stream.consume(&TokenKind::Node) || self.stream.consume(&TokenKind::Vertex) {
            self.stream.consume(&TokenKind::Type);
            if self.is_regular_identifier_start() {
                let _ = self.parse_regular_identifier("node type name")?;
            }
            inheritance = self.parse_inheritance_clause_opt()?;
        }

        self.stream.expect(TokenKind::LParen)?;
        let alias = if self.is_regular_identifier_start() {
            let (name, span) = self.parse_regular_identifier("node type alias")?;
            Some(LocalNodeTypeAlias { name, span })
        } else {
            None
        };
        if inheritance.is_none() {
            inheritance = self.parse_inheritance_clause_opt()?;
        }
        let filler = if self.is_node_type_filler_start() {
            Some(self.parse_node_type_filler()?)
        } else {
            None
        };
        let end_span = self.stream.expect(TokenKind::RParen)?;

        let phrase_end = filler
            .as_ref()
            .map(|f| f.span.clone())
            .or_else(|| alias.as_ref().map(|a| a.span.clone()))
            .unwrap_or_else(|| end_span.clone());
        let phrase = NodeTypePhrase {
            filler,
            alias,
            span: merge_spans(&start_span, &phrase_end),
        };

        Ok((
            NodeTypePattern {
                span: merge_spans(&start_span, &end_span),
                phrase,
            },
            inheritance,
        ))
    }

    /// Parses a node type phrase.
    ///
    /// Syntax: `[NODE [TYPE]] [node_type_filler] [AS alias]`
    fn parse_node_type_phrase(
        &mut self,
    ) -> ParseResult<(NodeTypePhrase, Option<TypeInheritanceClause>)> {
        let start_span = self.stream.current().span.clone();

        let has_node_keyword =
            self.stream.consume(&TokenKind::Node) || self.stream.consume(&TokenKind::Vertex);
        if !has_node_keyword {
            return Err(self.error_here("expected NODE or VERTEX".to_string()));
        }
        self.stream.consume(&TokenKind::Type);

        let mut has_phrase_name = false;
        if self.is_regular_identifier_start() {
            let _ = self.parse_regular_identifier("node type name")?;
            has_phrase_name = true;
        }
        let inheritance = self.parse_inheritance_clause_opt()?;
        let filler = if self.is_node_type_filler_start() {
            Some(self.parse_node_type_filler()?)
        } else {
            None
        };
        if !has_phrase_name && filler.is_none() {
            return Err(self.error_here("expected node type name or node type filler".to_string()));
        }

        // Optional AS alias
        let alias = if self.stream.consume(&TokenKind::As) {
            Some(self.parse_local_node_type_alias()?)
        } else {
            None
        };

        let end_span = alias
            .as_ref()
            .map(|a| a.span.clone())
            .or_else(|| filler.as_ref().map(|f| f.span.clone()))
            .unwrap_or_else(|| start_span.clone());

        Ok((
            NodeTypePhrase {
                filler,
                alias,
                span: merge_spans(&start_span, &end_span),
            },
            inheritance,
        ))
    }

    /// Checks if current position starts a node type filler.
    fn is_node_type_filler_start(&self) -> bool {
        matches!(
            self.stream.current().kind,
            TokenKind::Label
                | TokenKind::Labels
                | TokenKind::Is
                | TokenKind::Colon
                | TokenKind::LBrace
                | TokenKind::Key
        ) || self.is_constraint_start()
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
        let start_span = self.stream.current().span.clone();
        let mut label_set = None;
        let mut property_types = None;
        let mut key_label_set = None;
        let implied_content = None;
        let mut constraints = Vec::new();

        // Parse optional label set
        if self.is_label_set_phrase_start() {
            label_set = Some(self.parse_node_type_label_set()?);
        }

        // Parse optional property types
        if self.stream.check(&TokenKind::LBrace) {
            property_types = Some(self.parse_node_type_property_types()?);
        }

        // Parse optional key label set
        if self.stream.consume(&TokenKind::Key) {
            key_label_set = Some(self.parse_node_type_key_label_set()?);
        }

        while self.is_constraint_start() {
            constraints.push(self.parse_graph_type_constraint()?);
        }

        // implied_content is left as None - this is an optional future enhancement
        // that is not currently part of the GQL grammar specification

        let end_span = key_label_set
            .as_ref()
            .map(|k| k.span.clone())
            .or_else(|| property_types.as_ref().map(|p| p.span.clone()))
            .or_else(|| label_set.as_ref().map(|l| l.span.clone()))
            .or_else(|| constraints.last().map(|c| c.span()))
            .unwrap_or_else(|| start_span.clone());

        Ok(NodeTypeFiller {
            label_set,
            property_types,
            key_label_set,
            implied_content,
            constraints,
            span: merge_spans(&start_span, &end_span),
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
    pub fn parse_edge_type_specification(
        &mut self,
        is_abstract: bool,
    ) -> ParseResult<EdgeTypeSpec> {
        let (pattern, inheritance) = self.parse_edge_type_pattern()?;
        let span = match &pattern {
            EdgeTypePattern::Directed(d) => d.span.clone(),
            EdgeTypePattern::Undirected(u) => u.span.clone(),
        };

        Ok(EdgeTypeSpec {
            is_abstract,
            inheritance,
            pattern,
            span,
        })
    }

    /// Parses an edge type pattern (directed or undirected).
    fn parse_edge_type_pattern(
        &mut self,
    ) -> ParseResult<(EdgeTypePattern, Option<TypeInheritanceClause>)> {
        // Check for edge type phrase (keywords before pattern)
        if self.stream.check(&TokenKind::Directed)
            || self.stream.check(&TokenKind::Undirected)
            || self.stream.check(&TokenKind::Edge)
            || self.stream.check(&TokenKind::Relationship)
        {
            return self.parse_edge_type_phrase_pattern();
        }

        // Otherwise, parse visual edge pattern
        self.parse_edge_type_visual_pattern()
    }

    /// Parses edge type pattern from phrase keywords.
    fn parse_edge_type_phrase_pattern(
        &mut self,
    ) -> ParseResult<(EdgeTypePattern, Option<TypeInheritanceClause>)> {
        let start_span = self.stream.current().span.clone();

        // Parse edge kind
        let edge_kind = if self.stream.consume(&TokenKind::Directed) {
            EdgeKind::Directed
        } else if self.stream.consume(&TokenKind::Undirected) {
            EdgeKind::Undirected
        } else {
            EdgeKind::Inferred
        };

        // Expect EDGE or RELATIONSHIP keyword
        if !self.stream.consume(&TokenKind::Edge) && !self.stream.consume(&TokenKind::Relationship)
        {
            return Err(self.error_here("expected EDGE or RELATIONSHIP keyword".to_string()));
        }

        // Optional TYPE keyword
        self.stream.consume(&TokenKind::Type);

        // Optional edge type name
        if self.is_regular_identifier_start() {
            let _ = self.parse_regular_identifier("edge type name")?;
        }
        let inheritance = self.parse_inheritance_clause_opt()?;

        // Parse optional phrase content (labels and properties)
        let filler_content =
            if self.is_label_set_phrase_start() || self.stream.check(&TokenKind::LBrace) {
                Some(self.parse_edge_type_phrase_content()?)
            } else {
                None
            };

        let mut constraints = Vec::new();
        while self.is_constraint_start() {
            constraints.push(self.parse_graph_type_constraint()?);
        }

        // Expect CONNECTING keyword
        self.stream.expect(TokenKind::Connecting)?;

        // Parse endpoint pair
        let endpoint_pair_phrase = self.parse_endpoint_pair_phrase()?;
        let end_span = endpoint_pair_phrase.span.clone();
        let left_endpoint = endpoint_pair_phrase.endpoint_pair.source.node_type.clone();
        let right_endpoint = endpoint_pair_phrase
            .endpoint_pair
            .destination
            .node_type
            .clone();
        let pattern_span = merge_spans(&start_span, &end_span);
        let pattern_kind = edge_kind.clone();

        let phrase = EdgeTypePhrase {
            edge_kind,
            filler_content,
            endpoint_pair_phrase,
            span: pattern_span.clone(),
        };

        let filler = EdgeTypeFiller {
            span: phrase.span.clone(),
            phrase,
            constraints,
        };
        match pattern_kind {
            EdgeKind::Undirected => {
                let arc = ArcTypeUndirected {
                    filler: Some(filler),
                    span: pattern_span.clone(),
                };
                Ok((
                    EdgeTypePattern::Undirected(EdgeTypePatternUndirected {
                        left_endpoint,
                        arc,
                        right_endpoint,
                        span: pattern_span,
                    }),
                    inheritance,
                ))
            }
            EdgeKind::Directed | EdgeKind::Inferred => {
                let arc = DirectedArcType::PointingRight(ArcTypePointingRight {
                    filler: Some(filler),
                    span: pattern_span.clone(),
                });
                Ok((
                    EdgeTypePattern::Directed(EdgeTypePatternDirected {
                        left_endpoint,
                        arc,
                        right_endpoint,
                        span: pattern_span,
                    }),
                    inheritance,
                ))
            }
        }
    }

    /// Parses visual edge type pattern: `(node)-[edge]->(node)` or `(node)~[edge]~(node)`
    fn parse_edge_type_visual_pattern(
        &mut self,
    ) -> ParseResult<(EdgeTypePattern, Option<TypeInheritanceClause>)> {
        let start_span = self.stream.current().span.clone();

        // Parse left endpoint: node_type_pattern
        let (left_endpoint, _) = self.parse_node_type_pattern()?;

        // Check for directed or undirected arc
        let is_directed = self.stream.check(&TokenKind::Minus)
            || self.stream.check(&TokenKind::Lt)
            || self.stream.check(&TokenKind::LeftArrow);
        let is_undirected = self.stream.check(&TokenKind::Tilde);

        if is_directed {
            let is_pointing_left =
                self.stream.check(&TokenKind::Lt) || self.stream.check(&TokenKind::LeftArrow);

            if is_pointing_left {
                // <-[edge]-
                if self.stream.consume(&TokenKind::LeftArrow) {
                    // already consumed "<-"
                } else {
                    self.stream.expect(TokenKind::Lt)?;
                    self.stream.expect(TokenKind::Minus)?;
                }
                let filler = self.parse_edge_arc_filler()?;
                self.stream.expect(TokenKind::Minus)?;

                let arc = DirectedArcType::PointingLeft(ArcTypePointingLeft {
                    filler,
                    span: start_span.clone(),
                });

                // Parse right endpoint
                let (right_endpoint, _) = self.parse_node_type_pattern()?;
                let end_span = right_endpoint.span.clone();

                Ok((
                    EdgeTypePattern::Directed(EdgeTypePatternDirected {
                        left_endpoint,
                        arc,
                        right_endpoint,
                        span: merge_spans(&start_span, &end_span),
                    }),
                    None,
                ))
            } else {
                // -[edge]->
                self.stream.expect(TokenKind::Minus)?;
                let filler = self.parse_edge_arc_filler()?;
                if !self.stream.consume(&TokenKind::Arrow) {
                    self.stream.expect(TokenKind::Minus)?;
                    self.stream.expect(TokenKind::Gt)?;
                }

                let arc = DirectedArcType::PointingRight(ArcTypePointingRight {
                    filler,
                    span: start_span.clone(),
                });

                // Parse right endpoint
                let (right_endpoint, _) = self.parse_node_type_pattern()?;
                let end_span = right_endpoint.span.clone();

                Ok((
                    EdgeTypePattern::Directed(EdgeTypePatternDirected {
                        left_endpoint,
                        arc,
                        right_endpoint,
                        span: merge_spans(&start_span, &end_span),
                    }),
                    None,
                ))
            }
        } else if is_undirected {
            // ~[edge]~
            self.stream.expect(TokenKind::Tilde)?;
            let filler = self.parse_edge_arc_filler()?;
            self.stream.expect(TokenKind::Tilde)?;

            let arc = ArcTypeUndirected {
                filler,
                span: start_span.clone(),
            };

            // Parse right endpoint
            let (right_endpoint, _) = self.parse_node_type_pattern()?;
            let end_span = right_endpoint.span.clone();

            Ok((
                EdgeTypePattern::Undirected(EdgeTypePatternUndirected {
                    left_endpoint,
                    arc,
                    right_endpoint,
                    span: merge_spans(&start_span, &end_span),
                }),
                None,
            ))
        } else {
            Err(self.error_here("expected edge pattern: -, <-, or ~".to_string()))
        }
    }

    /// Parses edge arc filler within brackets: `[edge_type_filler?]`
    fn parse_edge_arc_filler(&mut self) -> ParseResult<Option<EdgeTypeFiller>> {
        self.stream.expect(TokenKind::LBracket)?;

        // Empty brackets allowed
        if self.stream.check(&TokenKind::RBracket) {
            self.stream.advance();
            return Ok(None);
        }

        // Parse edge type filler if present
        let filler = if self.is_edge_type_filler_start() {
            Some(self.parse_edge_type_filler()?)
        } else {
            None
        };

        self.stream.expect(TokenKind::RBracket)?;
        Ok(filler)
    }

    /// Checks if current position starts edge type filler.
    fn is_edge_type_filler_start(&self) -> bool {
        self.is_label_set_phrase_start()
            || self.stream.check(&TokenKind::LBrace)
            || matches!(self.stream.current().kind, TokenKind::Identifier(_))
            || self.is_constraint_start()
    }

    /// Parses edge type filler.
    fn parse_edge_type_filler(&mut self) -> ParseResult<EdgeTypeFiller> {
        let start_span = self.stream.current().span.clone();
        let mut constraints = Vec::new();

        // For now, we'll create a simple phrase with minimal content
        // A full implementation would parse edge labels/properties more completely
        let filler_content =
            if self.is_label_set_phrase_start() || self.stream.check(&TokenKind::LBrace) {
                Some(self.parse_edge_type_phrase_content()?)
            } else {
                None
            };

        while self.is_constraint_start() {
            constraints.push(self.parse_graph_type_constraint()?);
        }

        let end_span = constraints
            .last()
            .map(|constraint| constraint.span())
            .or_else(|| filler_content.as_ref().map(|content| content.span.clone()))
            .unwrap_or_else(|| start_span.clone());

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
            span: merge_spans(&start_span, &end_span),
        };

        Ok(EdgeTypeFiller {
            constraints,
            phrase,
            span: merge_spans(&start_span, &end_span),
        })
    }

    /// Parses edge type phrase content (labels and properties).
    fn parse_edge_type_phrase_content(&mut self) -> ParseResult<EdgeTypePhraseContent> {
        let start_span = self.stream.current().span.clone();
        let mut label_set = None;
        let mut property_types = None;

        // Parse optional label set
        if self.is_label_set_phrase_start() {
            label_set = Some(self.parse_edge_type_label_set()?);
        }

        // Parse optional property types
        if self.stream.check(&TokenKind::LBrace) {
            property_types = Some(self.parse_edge_type_property_types()?);
        }

        let end_span = property_types
            .as_ref()
            .map(|p| p.span.clone())
            .or_else(|| label_set.as_ref().map(|l| l.span.clone()))
            .unwrap_or_else(|| start_span.clone());

        Ok(EdgeTypePhraseContent {
            label_set,
            property_types,
            span: merge_spans(&start_span, &end_span),
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
        self.stream.expect(TokenKind::LParen)?;
        let endpoint_pair = self.parse_endpoint_pair()?;
        let end_span = self.stream.expect(TokenKind::RParen)?;
        let span = merge_spans(&endpoint_pair.span, &end_span);

        Ok(EndpointPairPhrase {
            endpoint_pair,
            span,
        })
    }

    /// Parses an endpoint pair.
    ///
    /// Syntax: `source_node_type TO destination_node_type`
    fn parse_endpoint_pair(&mut self) -> ParseResult<EndpointPair> {
        let start_span = self.stream.current().span.clone();

        // Parse source node type
        let source = self.parse_node_type_reference()?;

        // Expect TO keyword
        self.stream.expect(TokenKind::To)?;

        // Parse destination node type
        let destination = self.parse_node_type_reference()?;
        let end_span = destination.span.clone();

        Ok(EndpointPair {
            source,
            destination,
            span: merge_spans(&start_span, &end_span),
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
            self.parse_node_type_pattern()?.0
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
    pub fn parse_property_types_specification(
        &mut self,
    ) -> ParseResult<PropertyTypesSpecification> {
        let start_span = self.stream.expect(TokenKind::LBrace)?;

        // Empty braces allowed
        if self.stream.check(&TokenKind::RBrace) {
            let end_span = self.stream.expect(TokenKind::RBrace)?;
            return Ok(PropertyTypesSpecification {
                property_types: None,
                span: merge_spans(&start_span, &end_span),
            });
        }

        // Parse property type list
        let property_types = Some(self.parse_property_type_list()?);
        let end_span = self.stream.expect(TokenKind::RBrace)?;

        Ok(PropertyTypesSpecification {
            property_types,
            span: merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a property type list (comma-separated).
    fn parse_property_type_list(&mut self) -> ParseResult<PropertyTypeList> {
        let start_span = self.stream.current().span.clone();
        let mut types = Vec::new();

        // Parse first property type
        types.push(self.parse_property_type()?);

        // Parse remaining comma-separated property types
        while self.stream.consume(&TokenKind::Comma) {
            // Allow trailing comma
            if self.stream.check(&TokenKind::RBrace) {
                break;
            }
            types.push(self.parse_property_type()?);
        }

        let end_span = types
            .last()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| start_span.clone());

        Ok(PropertyTypeList {
            types,
            span: merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a property type.
    ///
    /// Syntax: `property_name [:: | TYPED] value_type [NOT NULL]`
    fn parse_property_type(&mut self) -> ParseResult<PropertyType> {
        let name = self.parse_property_name()?;
        let name_span = name.span.clone();

        // Typed marker is optional per grammar.
        let _typed =
            self.stream.consume(&TokenKind::DoubleColon) || self.stream.consume(&TokenKind::Typed);

        // Parse value type using TypeParser
        let value_type = self.parse_property_value_type()?;

        // Check for NOT NULL constraint
        let not_null = if self.stream.consume(&TokenKind::Not) {
            self.stream.expect(TokenKind::Null)?;
            true
        } else {
            false
        };

        let end_span = if not_null {
            self.stream.tokens()[self.stream.position().saturating_sub(1)]
                .span
                .clone()
        } else {
            value_type.span.clone()
        };

        Ok(PropertyType {
            name,
            value_type,
            not_null,
            span: merge_spans(&name_span, &end_span),
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
        let mut type_parser = TypeParser::new(&self.stream.tokens()[self.stream.position()..]);
        let value_type = type_parser
            .parse_value_type()
            .map_err(|_e| self.error_here("failed to parse property value type".to_string()))?;

        // Advance our position by how much the type parser consumed
        self.stream
            .set_position(self.stream.position() + type_parser.current_position());

        let span = value_type.span();

        Ok(PropertyValueType { value_type, span })
    }

    // ========================================================================
    // Label Set Phrases and Specifications
    // ========================================================================

    /// Checks if current position starts a label set phrase.
    fn is_label_set_phrase_start(&self) -> bool {
        matches!(
            self.stream.current().kind,
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
        if self.stream.consume(&TokenKind::Label) {
            // LABEL <label_name>
            let label = self.parse_label_name()?;
            Ok(LabelSetPhrase::Label(label))
        } else if self.stream.consume(&TokenKind::Labels) {
            // LABELS <label_set_specification>
            let label_set = self.parse_label_set_specification()?;
            Ok(LabelSetPhrase::Labels(label_set))
        } else if self.stream.consume(&TokenKind::Is) || self.stream.consume(&TokenKind::Colon) {
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
        let start_span = self.stream.current().span.clone();
        let mut labels = Vec::new();

        // Parse first label
        labels.push(self.parse_label_name()?);

        // Parse remaining ampersand-separated labels
        while self.stream.consume(&TokenKind::Ampersand) {
            labels.push(self.parse_label_name()?);
        }

        let end_span = labels
            .last()
            .map(|l| l.span.clone())
            .unwrap_or_else(|| start_span.clone());

        Ok(LabelSetSpecification {
            labels,
            span: merge_spans(&start_span, &end_span),
        })
    }

    /// Parses a label name.
    fn parse_label_name(&mut self) -> ParseResult<LabelName> {
        let (name, span) = self.parse_regular_identifier("label name identifier")?;
        Ok(LabelName { name, span })
    }

    fn parse_inheritance_clause_opt(&mut self) -> ParseResult<Option<TypeInheritanceClause>> {
        if !self.is_inheritance_start() {
            return Ok(None);
        }

        let start_span = self.stream.current().span.clone();
        self.stream.advance();

        let mut parents = Vec::new();
        let first = self.parse_type_reference_identifier("parent type name")?;
        parents.push(InheritedTypeReference {
            name: first.0,
            span: first.1.clone(),
        });

        // Parse additional parent types separated by commas
        // IMPORTANT: Stop if we see keywords that start a new element type or filler
        while self.stream.consume(&TokenKind::Comma) {
            // Check if this comma actually starts a new element type (not another parent)
            if self.is_element_type_start() {
                // Put the comma back by repositioning
                self.stream.set_position(self.stream.position() - 1);
                break;
            }

            let parent = self.parse_type_reference_identifier("parent type name")?;
            parents.push(InheritedTypeReference {
                name: parent.0,
                span: parent.1.clone(),
            });
        }

        let end_span = parents
            .last()
            .map(|parent| parent.span.clone())
            .unwrap_or_else(|| start_span.clone());

        Ok(Some(TypeInheritanceClause {
            parents,
            span: merge_spans(&start_span, &end_span),
        }))
    }

    fn is_element_type_start(&self) -> bool {
        // Check for keywords that start an element type specification
        self.check_word("ABSTRACT")
            || self.stream.check(&TokenKind::Node)
            || self.stream.check(&TokenKind::Vertex)
            || self.stream.check(&TokenKind::Edge)
            || self.stream.check(&TokenKind::Relationship)
            || self.stream.check(&TokenKind::Directed)
            || self.stream.check(&TokenKind::Undirected)
    }

    fn is_inheritance_start(&self) -> bool {
        self.check_word("INHERITS") || self.check_word("EXTENDS") || self.check_word("UNDER")
    }

    fn is_constraint_start(&self) -> bool {
        self.check_word("CONSTRAINT")
            || self.check_word("UNIQUE")
            || self.check_word("CHECK")
            || self.check_word("MANDATORY")
            || self.check_word("KEY")
    }

    fn parse_graph_type_constraint(&mut self) -> ParseResult<GraphTypeConstraint> {
        let start_span = self.stream.current().span.clone();

        if self.consume_word("CONSTRAINT") && !self.is_constraint_start() {
            return Err(self.error_here("expected constraint name after CONSTRAINT".to_string()));
        }

        let (raw_name, name_span) = self.parse_type_reference_identifier("constraint name")?;
        let mut arguments = if self.stream.check(&TokenKind::LParen) {
            self.parse_constraint_arguments()?
        } else {
            Vec::new()
        };

        // Normalize a standalone KEY constraint with no argument list to a single
        // synthetic argument if KEY-style label-set syntax follows.
        if raw_name.eq_ignore_ascii_case("KEY")
            && arguments.is_empty()
            && self.is_regular_identifier_start()
        {
            let (name, span) = self.parse_regular_identifier("constraint argument")?;
            arguments.push(GraphTypeConstraintArgument { raw: name, span });
        }

        let end_span = arguments
            .last()
            .map(|argument| argument.span.clone())
            .unwrap_or(name_span.clone());
        let span = merge_spans(&start_span, &end_span);
        let upper = raw_name.to_ascii_uppercase();

        Ok(match upper.as_str() {
            "KEY" => GraphTypeConstraint::Key { arguments, span },
            "UNIQUE" => GraphTypeConstraint::Unique { arguments, span },
            "MANDATORY" => GraphTypeConstraint::Mandatory { arguments, span },
            "CHECK" => GraphTypeConstraint::Check { arguments, span },
            _ => GraphTypeConstraint::Custom {
                name: raw_name,
                arguments,
                span,
            },
        })
    }

    fn parse_constraint_arguments(&mut self) -> ParseResult<Vec<GraphTypeConstraintArgument>> {
        self.stream.expect(TokenKind::LParen)?;
        let mut arguments = Vec::new();
        let mut segment_start = self.stream.position();
        let mut depth = 0usize;

        while !self.stream.check(&TokenKind::Eof) {
            if depth == 0 && self.stream.check(&TokenKind::RParen) {
                if segment_start < self.stream.position() {
                    arguments.push(
                        self.build_constraint_argument(segment_start, self.stream.position())?,
                    );
                }
                self.stream.advance();
                return Ok(arguments);
            }

            let current_kind = self.stream.current().kind.clone();
            if depth == 0 && matches!(current_kind, TokenKind::Comma) {
                if segment_start < self.stream.position() {
                    arguments.push(
                        self.build_constraint_argument(segment_start, self.stream.position())?,
                    );
                }
                self.stream.advance();
                segment_start = self.stream.position();
                continue;
            }

            match current_kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }

            self.stream.advance();
        }

        Err(self.error_here("expected ')' to close constraint argument list".to_string()))
    }

    fn build_constraint_argument(
        &self,
        start: usize,
        end: usize,
    ) -> ParseResult<GraphTypeConstraintArgument> {
        let tokens = &self.stream.tokens()[start..end];
        if tokens.is_empty() {
            return Err(self.error_here("empty constraint argument".to_string()));
        }
        let span = merge_spans(&tokens[0].span, &tokens[tokens.len() - 1].span);
        let raw = tokens
            .iter()
            .map(|token| token.kind.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        Ok(GraphTypeConstraintArgument {
            raw: raw.into(),
            span,
        })
    }

    fn parse_type_reference_identifier(
        &mut self,
        expected: &str,
    ) -> ParseResult<(smol_str::SmolStr, Span)> {
        let token = self.stream.current().clone();
        match &token.kind {
            TokenKind::Identifier(name)
            | TokenKind::DelimitedIdentifier(name)
            | TokenKind::ReservedKeyword(name)
            | TokenKind::PreReservedKeyword(name)
            | TokenKind::NonReservedKeyword(name) => {
                self.stream.advance();
                Ok((name.clone(), token.span))
            }
            kind if kind.is_keyword() => {
                self.stream.advance();
                Ok((smol_str::SmolStr::new(kind.to_string()), token.span))
            }
            _ => Err(self.error_here(format!("expected {expected}"))),
        }
    }

    fn check_word(&self, word: &str) -> bool {
        token_matches_word(&self.stream.current().kind, word)
    }

    fn consume_word(&mut self, word: &str) -> bool {
        if self.check_word(word) {
            self.stream.advance();
            true
        } else {
            false
        }
    }

    fn parse_regular_identifier(
        &mut self,
        expected: &str,
    ) -> ParseResult<(smol_str::SmolStr, Span)> {
        let token = self.stream.current().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                self.stream.advance();
                Ok((name.clone(), token.span))
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                self.stream.advance();
                Ok((smol_str::SmolStr::new(kind.to_string()), token.span))
            }
            _ => Err(self.error_here(format!("expected {expected}"))),
        }
    }

    fn is_regular_identifier_start(&self) -> bool {
        matches!(self.stream.current().kind, TokenKind::Identifier(_))
            || self
                .stream
                .current()
                .kind
                .is_non_reserved_identifier_keyword()
    }

    fn is_edge_pattern_after_left_endpoint(&self) -> bool {
        if !self.stream.check(&TokenKind::LParen) {
            return false;
        }
        let mut depth = 0usize;
        let mut cursor = self.stream.position();
        while cursor < self.stream.tokens().len() {
            match self.stream.tokens()[cursor].kind {
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
            self.stream.tokens().get(cursor).map(|t| &t.kind),
            Some(TokenKind::Minus | TokenKind::Lt | TokenKind::LeftArrow | TokenKind::Tilde)
        )
    }

    /// Returns the current parser position (for integration with TypeParser).
    pub fn current_position(&self) -> usize {
        self.stream.position()
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

fn token_matches_word(kind: &TokenKind, word: &str) -> bool {
    match kind {
        TokenKind::Identifier(name)
        | TokenKind::DelimitedIdentifier(name)
        | TokenKind::ReservedKeyword(name)
        | TokenKind::PreReservedKeyword(name)
        | TokenKind::NonReservedKeyword(name) => name.eq_ignore_ascii_case(word),
        _ => kind.to_string().eq_ignore_ascii_case(word),
    }
}
