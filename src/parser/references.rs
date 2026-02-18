//! Catalog and object reference parsing for GQL.
//!
//! This module implements comprehensive parsing of catalog and object references,
//! including schema references, graph references, graph type references, binding
//! table references, procedure references, and catalog-qualified names.
//!
//! # Grammar Overview
//!
//! ```text
//! schema_reference ::=
//!     | absolute_schema_path
//!     | relative_schema_path
//!     | identifier
//!     | HOME_SCHEMA
//!     | CURRENT_SCHEMA
//!     | .
//!     | $$name
//!
//! graph_reference ::=
//!     | catalog_qualified_name
//!     | delimited_identifier
//!     | HOME_GRAPH
//!     | HOME_PROPERTY_GRAPH
//!     | $$name
//!
//! catalog_qualified_name ::=
//!     [ catalog_object_parent_reference :: ] name
//!
//! catalog_object_parent_reference ::=
//!     | schema_reference
//!     | catalog_qualified_name
//! ```

use crate::ast::Span;
use crate::ast::references::{
    BindingTableReference, CatalogObjectParentReference, CatalogQualifiedName, GraphReference,
    GraphTypeReference, ProcedureReference, SchemaReference,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use smol_str::SmolStr;

type ParseError = Box<Diag>;
type ParseResult<T> = Result<T, ParseError>;

/// Parser for catalog and object references.
pub struct ReferenceParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> ReferenceParser<'a> {
    /// Creates a new reference parser.
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
                .with_code("P_REF"),
        )
    }

    // ========================================================================
    // Schema Reference Parsing
    // ========================================================================

    /// Parses a schema reference.
    ///
    /// # Grammar
    ///
    /// ```text
    /// schema_reference ::=
    ///     | / [ identifier / ]* identifier    -- Absolute path
    ///     | ( .. / )+ [ identifier / ]* identifier  -- Relative path
    ///     | identifier                         -- Plain identifier
    ///     | HOME_SCHEMA                        -- Predefined
    ///     | CURRENT_SCHEMA                     -- Predefined
    ///     | .                                  -- Current schema (dot)
    ///     | $$name                             -- Reference parameter
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// /my_schema
    /// /dir/my_schema
    /// ../other_schema
    /// ../../another/schema
    /// my_schema
    /// HOME_SCHEMA
    /// CURRENT_SCHEMA
    /// .
    /// $$schema_param
    /// ```
    pub fn parse_schema_reference(&mut self) -> ParseResult<SchemaReference> {
        let start = self.current().span.start;

        match &self.current().kind {
            // Absolute path: /schema or /dir/schema
            TokenKind::Slash => {
                self.advance();
                let components = self.parse_schema_path_components()?;
                if components.is_empty() {
                    return Err(self.error_here("absolute schema path cannot be empty"));
                }
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                Ok(SchemaReference::AbsolutePath {
                    components,
                    span: start..end,
                })
            }

            // Relative path: ../schema or ../../other/schema
            TokenKind::DotDot => {
                let (up_levels, components) = self.parse_relative_schema_path()?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                Ok(SchemaReference::RelativePath {
                    up_levels,
                    components,
                    span: start..end,
                })
            }

            // Dot: . (current schema)
            TokenKind::Dot => {
                let span = self.current().span.clone();
                self.advance();
                Ok(SchemaReference::Dot { span })
            }

            // Reference parameter: $$name
            TokenKind::ReferenceParameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(SchemaReference::ReferenceParameter { name, span })
            }

            // Predefined: HOME_SCHEMA, CURRENT_SCHEMA
            TokenKind::Identifier(name) | TokenKind::ReservedKeyword(name) => {
                let name_upper = name.to_ascii_uppercase();
                match name_upper.as_str() {
                    "HOME_SCHEMA" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(SchemaReference::HomeSchema { span })
                    }
                    "CURRENT_SCHEMA" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(SchemaReference::CurrentSchema { span })
                    }
                    _ => {
                        let name = name.clone();
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(SchemaReference::Identifier { name, span })
                    }
                }
            }

            // HOME keyword followed by SCHEMA
            TokenKind::Home => {
                let start_span = self.current().span.clone();
                self.advance();
                if self.check(&TokenKind::Schema) {
                    self.advance();
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start_span.start);
                    Ok(SchemaReference::HomeSchema {
                        span: start_span.start..end,
                    })
                } else {
                    Err(self.error_here("expected SCHEMA after HOME"))
                }
            }

            // CURRENT keyword followed by SCHEMA
            TokenKind::Current => {
                let start_span = self.current().span.clone();
                self.advance();
                if self.check(&TokenKind::Schema) {
                    self.advance();
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start_span.start);
                    Ok(SchemaReference::CurrentSchema {
                        span: start_span.start..end,
                    })
                } else {
                    Err(self.error_here("expected SCHEMA after CURRENT"))
                }
            }

            _ => Err(self.error_here(format!(
                "expected schema reference, found {}",
                self.current().kind
            ))),
        }
    }

    /// Parses schema path components after the initial / or ../ prefix.
    ///
    /// Parses: identifier [ / identifier ]*
    fn parse_schema_path_components(&mut self) -> ParseResult<Vec<SmolStr>> {
        let mut components = Vec::new();

        while let TokenKind::Identifier(name) = &self.current().kind {
            components.push(name.clone());
            self.advance();

            // Check for continuation with /
            if self.check(&TokenKind::Slash) {
                self.advance();
                // Must be followed by another identifier
                if !matches!(self.current().kind, TokenKind::Identifier(_)) {
                    return Err(self.error_here("expected identifier after /"));
                }
            } else {
                break;
            }
        }

        Ok(components)
    }

    /// Parses a relative schema path starting with ..
    ///
    /// Returns (up_levels, components)
    fn parse_relative_schema_path(&mut self) -> ParseResult<(u32, Vec<SmolStr>)> {
        let mut up_levels = 0;

        // Count the number of .. segments
        while self.check(&TokenKind::DotDot) {
            up_levels += 1;
            self.advance();

            // Expect /
            if !self.check(&TokenKind::Slash) {
                return Err(self.error_here("expected / after .."));
            }
            self.advance();
        }

        if up_levels == 0 {
            return Err(self.error_here("relative path must start with .."));
        }

        // Parse the remaining path components
        let components = self.parse_schema_path_components()?;
        if components.is_empty() {
            return Err(self.error_here("relative schema path must include at least one component"));
        }

        Ok((up_levels, components))
    }

    // ========================================================================
    // Graph Reference Parsing
    // ========================================================================

    /// Parses a graph reference.
    ///
    /// # Grammar
    ///
    /// ```text
    /// graph_reference ::=
    ///     | catalog_qualified_name             -- Catalog-qualified
    ///     | delimited_identifier               -- Delimited identifier
    ///     | HOME_GRAPH                         -- Predefined
    ///     | HOME_PROPERTY_GRAPH                -- Predefined
    ///     | $$name                             -- Reference parameter
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// my_schema::my_graph
    /// "my graph"
    /// HOME_GRAPH
    /// HOME_PROPERTY_GRAPH
    /// $$graph_param
    /// ```
    pub fn parse_graph_reference(&mut self) -> ParseResult<GraphReference> {
        let start = self.current().span.start;

        match &self.current().kind {
            // Delimited identifier
            TokenKind::DelimitedIdentifier(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(GraphReference::Delimited { name, span })
            }

            // Reference parameter: $$name
            TokenKind::ReferenceParameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(GraphReference::ReferenceParameter { name, span })
            }

            // Predefined: HOME_GRAPH, HOME_PROPERTY_GRAPH, CURRENT_GRAPH, CURRENT_PROPERTY_GRAPH
            TokenKind::Identifier(name) | TokenKind::ReservedKeyword(name) => {
                let name_upper = name.to_ascii_uppercase();
                match name_upper.as_str() {
                    "HOME_GRAPH" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(GraphReference::HomeGraph { span })
                    }
                    "HOME_PROPERTY_GRAPH" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(GraphReference::HomePropertyGraph { span })
                    }
                    "CURRENT_GRAPH" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(GraphReference::CurrentGraph { span })
                    }
                    "CURRENT_PROPERTY_GRAPH" => {
                        let span = self.current().span.clone();
                        self.advance();
                        Ok(GraphReference::CurrentPropertyGraph { span })
                    }
                    _ => {
                        // Try catalog-qualified name
                        let name = self.parse_catalog_qualified_name()?;
                        let end = name.span.end;
                        Ok(GraphReference::CatalogQualified {
                            name,
                            span: start..end,
                        })
                    }
                }
            }

            // HOME keyword followed by GRAPH or PROPERTY
            TokenKind::Home => {
                let start_span = self.current().span.clone();
                self.advance();

                match &self.current().kind {
                    TokenKind::Graph => {
                        self.advance();
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|t| t.span.end)
                            .unwrap_or(start_span.start);
                        Ok(GraphReference::HomeGraph {
                            span: start_span.start..end,
                        })
                    }
                    TokenKind::Property => {
                        self.advance();
                        self.expect(TokenKind::Graph)?;
                        let end = self
                            .tokens
                            .get(self.pos.saturating_sub(1))
                            .map(|t| t.span.end)
                            .unwrap_or(start_span.start);
                        Ok(GraphReference::HomePropertyGraph {
                            span: start_span.start..end,
                        })
                    }
                    _ => Err(self.error_here("expected GRAPH or PROPERTY after HOME")),
                }
            }

            // Absolute graph path: /graph or /dir/graph
            TokenKind::Slash => {
                let (name, span) = self.parse_absolute_path_graph_name()?;
                Ok(GraphReference::CatalogQualified { name, span })
            }

            // Could be a catalog-qualified name starting with relative schema references
            TokenKind::DotDot | TokenKind::Dot | TokenKind::Current => {
                let name = self.parse_catalog_qualified_name()?;
                let end = name.span.end;
                Ok(GraphReference::CatalogQualified {
                    name,
                    span: start..end,
                })
            }

            _ => Err(self.error_here(format!(
                "expected graph reference, found {}",
                self.current().kind
            ))),
        }
    }

    // ========================================================================
    // Graph Type Reference Parsing
    // ========================================================================

    /// Parses a graph type reference.
    ///
    /// # Grammar
    ///
    /// ```text
    /// graph_type_reference ::=
    ///     | catalog_qualified_name             -- Catalog-qualified
    ///     | $$name                             -- Reference parameter
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// my_schema::my_graph_type
    /// $$type_param
    /// ```
    pub fn parse_graph_type_reference(&mut self) -> ParseResult<GraphTypeReference> {
        let start = self.current().span.start;

        match &self.current().kind {
            // Reference parameter: $$name
            TokenKind::ReferenceParameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(GraphTypeReference::ReferenceParameter { name, span })
            }

            // Catalog-qualified name
            _ => {
                let name = self.parse_catalog_qualified_name()?;
                let end = name.span.end;
                Ok(GraphTypeReference::CatalogQualified {
                    name,
                    span: start..end,
                })
            }
        }
    }

    // ========================================================================
    // Binding Table Reference Parsing
    // ========================================================================

    /// Parses a binding table reference.
    ///
    /// # Grammar
    ///
    /// ```text
    /// binding_table_reference ::=
    ///     | catalog_qualified_name             -- Catalog-qualified
    ///     | delimited_identifier               -- Delimited identifier
    ///     | $$name                             -- Reference parameter
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// my_schema::my_table
    /// "my table"
    /// $$table_param
    /// ```
    pub fn parse_binding_table_reference(&mut self) -> ParseResult<BindingTableReference> {
        let start = self.current().span.start;

        match &self.current().kind {
            // Delimited identifier
            TokenKind::DelimitedIdentifier(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(BindingTableReference::Delimited { name, span })
            }

            // Reference parameter: $$name
            TokenKind::ReferenceParameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(BindingTableReference::ReferenceParameter { name, span })
            }

            // Catalog-qualified name
            _ => {
                let name = self.parse_catalog_qualified_name()?;
                let end = name.span.end;
                Ok(BindingTableReference::CatalogQualified {
                    name,
                    span: start..end,
                })
            }
        }
    }

    // ========================================================================
    // Procedure Reference Parsing
    // ========================================================================

    /// Parses a procedure reference.
    ///
    /// # Grammar
    ///
    /// ```text
    /// procedure_reference ::=
    ///     | catalog_qualified_name             -- Catalog-qualified
    ///     | $$name                             -- Reference parameter
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// my_schema::my_procedure
    /// $$proc_param
    /// ```
    pub fn parse_procedure_reference(&mut self) -> ParseResult<ProcedureReference> {
        let start = self.current().span.start;

        match &self.current().kind {
            // Reference parameter: $$name
            TokenKind::ReferenceParameter(name) => {
                let name = name.clone();
                let span = self.current().span.clone();
                self.advance();
                Ok(ProcedureReference::ReferenceParameter { name, span })
            }

            // Catalog-qualified name
            _ => {
                let name = self.parse_catalog_qualified_name()?;
                let end = name.span.end;
                Ok(ProcedureReference::CatalogQualified {
                    name,
                    span: start..end,
                })
            }
        }
    }

    // ========================================================================
    // Catalog-Qualified Name Parsing
    // ========================================================================

    /// Parses a catalog-qualified name.
    ///
    /// # Grammar
    ///
    /// ```text
    /// catalog_qualified_name ::=
    ///     [ catalog_object_parent_reference :: ] name
    ///
    /// catalog_object_parent_reference ::=
    ///     | schema_reference
    ///     | catalog_qualified_name
    /// ```
    ///
    /// # Examples
    ///
    /// ```text
    /// schema::name                  -- Single-level qualification
    /// parent::child::name           -- Multi-level qualification
    /// /my_schema::name              -- Absolute schema path
    /// ../other::name                -- Relative schema path
    /// ```
    pub fn parse_catalog_qualified_name(&mut self) -> ParseResult<CatalogQualifiedName> {
        let start = self.current().span.start;

        // Try to parse parent reference
        let parent = self.try_parse_catalog_object_parent_reference()?;

        // Parse the object name
        let (name, _) = self.parse_regular_identifier("object name")?;

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);

        Ok(CatalogQualifiedName {
            parent,
            name,
            span: start..end,
        })
    }

    fn parse_absolute_path_graph_name(&mut self) -> ParseResult<(CatalogQualifiedName, Span)> {
        let start = self.current().span.start;
        self.expect(TokenKind::Slash)?;

        let mut components: Vec<(SmolStr, Span)> = Vec::new();
        while !matches!(self.current().kind, TokenKind::Eof) {
            let (name, span) = self.parse_regular_identifier("graph path component")?;
            components.push((name, span));
            if self.check(&TokenKind::Slash) {
                self.advance();
            } else {
                break;
            }
        }

        if components.is_empty() {
            return Err(self.error_here("absolute graph path must include a graph name"));
        }

        let (graph_name, graph_span) = components
            .last()
            .cloned()
            .expect("components is guaranteed non-empty");
        let parent_span_end = if components.len() == 1 {
            start + 1
        } else {
            components[components.len() - 2].1.end
        };
        let parent_span = start..parent_span_end;

        let schema = SchemaReference::AbsolutePath {
            components: components[..components.len() - 1]
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
            span: parent_span.clone(),
        };

        let name = CatalogQualifiedName {
            parent: Some(CatalogObjectParentReference::Schema {
                schema,
                span: parent_span,
            }),
            name: graph_name,
            span: start..graph_span.end,
        };

        let span = name.span.clone();
        Ok((name, span))
    }

    fn parse_regular_identifier(&mut self, expected: &str) -> ParseResult<(SmolStr, Span)> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok((name.clone(), token.span))
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                self.advance();
                Ok((SmolStr::new(kind.to_string()), token.span))
            }
            _ => Err(self.error_here(format!("expected {expected}"))),
        }
    }

    /// Tries to parse a catalog object parent reference.
    ///
    /// Returns None if no parent reference is present.
    fn try_parse_catalog_object_parent_reference(
        &mut self,
    ) -> ParseResult<Option<CatalogObjectParentReference>> {
        // Look ahead to determine if we have a parent reference
        // We need to check if there's a :: following a potential parent

        // Save current position for potential backtracking
        let saved_pos = self.pos;
        let saved_start = self.current().span.start;

        // Try to parse as schema reference first (/, .., ., HOME_SCHEMA, CURRENT_SCHEMA)
        let parent = match &self.current().kind {
            TokenKind::Slash | TokenKind::DotDot | TokenKind::Dot => {
                // Try parsing schema reference
                match self.parse_schema_reference() {
                    Ok(schema) => {
                        // Check for ::
                        if self.check(&TokenKind::DoubleColon) {
                            self.advance();
                            let end = self
                                .tokens
                                .get(self.pos.saturating_sub(1))
                                .map(|t| t.span.end)
                                .unwrap_or(schema.span().start);
                            Some(CatalogObjectParentReference::Schema {
                                schema,
                                span: saved_start..end,
                            })
                        } else {
                            // No ::, backtrack
                            self.pos = saved_pos;
                            None
                        }
                    }
                    Err(_) => {
                        // Failed to parse, backtrack
                        self.pos = saved_pos;
                        None
                    }
                }
            }

            TokenKind::Home | TokenKind::Current => {
                // Try parsing HOME_SCHEMA or CURRENT_SCHEMA
                match self.parse_schema_reference() {
                    Ok(schema) => {
                        // Check for ::
                        if self.check(&TokenKind::DoubleColon) {
                            self.advance();
                            let end = self
                                .tokens
                                .get(self.pos.saturating_sub(1))
                                .map(|t| t.span.end)
                                .unwrap_or(schema.span().start);
                            Some(CatalogObjectParentReference::Schema {
                                schema,
                                span: saved_start..end,
                            })
                        } else {
                            // No ::, backtrack
                            self.pos = saved_pos;
                            None
                        }
                    }
                    Err(_) => {
                        // Failed to parse, backtrack
                        self.pos = saved_pos;
                        None
                    }
                }
            }

            TokenKind::Identifier(name) | TokenKind::ReservedKeyword(name) => {
                let name_upper = name.to_ascii_uppercase();
                // Check for predefined schema references
                if matches!(name_upper.as_str(), "HOME_SCHEMA" | "CURRENT_SCHEMA") {
                    match self.parse_schema_reference() {
                        Ok(schema) => {
                            // Check for ::
                            if self.check(&TokenKind::DoubleColon) {
                                self.advance();
                                let end = self
                                    .tokens
                                    .get(self.pos.saturating_sub(1))
                                    .map(|t| t.span.end)
                                    .unwrap_or(schema.span().start);
                                Some(CatalogObjectParentReference::Schema {
                                    schema,
                                    span: saved_start..end,
                                })
                            } else {
                                // No ::, backtrack
                                self.pos = saved_pos;
                                None
                            }
                        }
                        Err(_) => {
                            // Failed to parse, backtrack
                            self.pos = saved_pos;
                            None
                        }
                    }
                } else {
                    // Try parsing as nested catalog-qualified name
                    // Look ahead to find :: pattern
                    self.try_parse_nested_parent_reference(saved_pos, saved_start)?
                }
            }

            _ => None,
        };

        Ok(parent)
    }

    /// Tries to parse a nested parent reference (multi-level qualification).
    ///
    /// This handles cases like: parent::child::name
    fn try_parse_nested_parent_reference(
        &mut self,
        saved_pos: usize,
        saved_start: usize,
    ) -> ParseResult<Option<CatalogObjectParentReference>> {
        // Count how many :: we have to determine nesting depth
        let mut depth = 0;
        let mut temp_pos = self.pos;

        while let Some(token) = self.tokens.get(temp_pos) {
            // Look for identifier
            if !matches!(token.kind, TokenKind::Identifier(_)) {
                break;
            }
            temp_pos += 1;

            // Look for ::
            if let Some(token) = self.tokens.get(temp_pos) {
                if matches!(token.kind, TokenKind::DoubleColon) {
                    depth += 1;
                    temp_pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if depth == 0 {
            // No :: found, no parent
            self.pos = saved_pos;
            return Ok(None);
        }

        if depth == 1 {
            // Single level: identifier::
            // This is a simple parent reference
            let name = match &self.current().kind {
                TokenKind::Identifier(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => {
                    self.pos = saved_pos;
                    return Ok(None);
                }
            };

            self.expect(TokenKind::DoubleColon)?;

            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(saved_start);

            // Create a simple catalog-qualified name as parent
            let parent_name = CatalogQualifiedName {
                parent: None,
                name,
                span: saved_start..end,
            };

            Ok(Some(CatalogObjectParentReference::Object {
                name: Box::new(parent_name),
                span: saved_start..end,
            }))
        } else {
            // Multi-level: parent::child::...
            // Recursively parse the nested structure
            let parent_name = self.parse_catalog_qualified_name_recursive(depth - 1)?;

            self.expect(TokenKind::DoubleColon)?;

            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(saved_start);

            Ok(Some(CatalogObjectParentReference::Object {
                name: Box::new(parent_name),
                span: saved_start..end,
            }))
        }
    }

    /// Recursively parses nested catalog-qualified names.
    ///
    /// The depth parameter indicates how many levels to parse.
    fn parse_catalog_qualified_name_recursive(
        &mut self,
        depth: usize,
    ) -> ParseResult<CatalogQualifiedName> {
        let start = self.current().span.start;

        if depth == 0 {
            // Base case: just parse identifier
            let name = match &self.current().kind {
                TokenKind::Identifier(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => {
                    return Err(self.error_here("expected identifier"));
                }
            };

            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(CatalogQualifiedName {
                parent: None,
                name,
                span: start..end,
            })
        } else {
            // Recursive case: parse parent, then ::, then identifier
            let parent_name = self.parse_catalog_qualified_name_recursive(depth - 1)?;

            self.expect(TokenKind::DoubleColon)?;

            let parent_end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            let parent = Some(CatalogObjectParentReference::Object {
                name: Box::new(parent_name),
                span: start..parent_end,
            });

            let name = match &self.current().kind {
                TokenKind::Identifier(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                _ => {
                    return Err(self.error_here("expected identifier"));
                }
            };

            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(start);

            Ok(CatalogQualifiedName {
                parent,
                name,
                span: start..end,
            })
        }
    }

    /// Parses a catalog object parent reference.
    ///
    /// This is used when we know a parent reference must be present.
    pub fn parse_catalog_object_parent_reference(
        &mut self,
    ) -> ParseResult<CatalogObjectParentReference> {
        let start = self.current().span.start;

        // Try schema reference first
        match &self.current().kind {
            TokenKind::Slash
            | TokenKind::DotDot
            | TokenKind::Dot
            | TokenKind::Home
            | TokenKind::Current => {
                let schema = self.parse_schema_reference()?;
                self.expect(TokenKind::DoubleColon)?;
                let end = self
                    .tokens
                    .get(self.pos.saturating_sub(1))
                    .map(|t| t.span.end)
                    .unwrap_or(start);
                Ok(CatalogObjectParentReference::Schema {
                    schema,
                    span: start..end,
                })
            }

            TokenKind::Identifier(name) => {
                let name_upper = name.to_ascii_uppercase();
                // Check for predefined schema references
                if matches!(name_upper.as_str(), "HOME_SCHEMA" | "CURRENT_SCHEMA") {
                    let schema = self.parse_schema_reference()?;
                    self.expect(TokenKind::DoubleColon)?;
                    let end = self
                        .tokens
                        .get(self.pos.saturating_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(start);
                    Ok(CatalogObjectParentReference::Schema {
                        schema,
                        span: start..end,
                    })
                } else {
                    // Parse as catalog-qualified name
                    let saved_pos = self.pos;
                    let saved_start = self.current().span.start;
                    match self.try_parse_nested_parent_reference(saved_pos, saved_start)? {
                        Some(parent) => Ok(parent),
                        None => Err(self.error_here("expected catalog object parent reference")),
                    }
                }
            }

            _ => Err(self.error_here(format!(
                "expected catalog object parent reference, found {}",
                self.current().kind
            ))),
        }
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Parses a schema reference from a token slice.
///
/// # Examples
///
/// ```rust
/// use gql_parser::ast::SchemaReference;
/// use gql_parser::parser::references::parse_schema_reference;
/// use gql_parser::{Token, TokenKind};
///
/// let tokens = vec![
///     Token::new(TokenKind::Slash, 0..1),
///     Token::new(TokenKind::Identifier("my_schema".into()), 1..10),
/// ];
/// let schema_ref = parse_schema_reference(&tokens).unwrap();
/// assert!(matches!(schema_ref, SchemaReference::AbsolutePath { .. }));
/// ```
pub fn parse_schema_reference(tokens: &[Token]) -> ParseResult<SchemaReference> {
    parse_with_full_consumption(tokens, |parser| parser.parse_schema_reference())
}

/// Parses a graph reference from a token slice.
pub fn parse_graph_reference(tokens: &[Token]) -> ParseResult<GraphReference> {
    parse_with_full_consumption(tokens, |parser| parser.parse_graph_reference())
}

/// Parses a graph type reference from a token slice.
pub fn parse_graph_type_reference(tokens: &[Token]) -> ParseResult<GraphTypeReference> {
    parse_with_full_consumption(tokens, |parser| parser.parse_graph_type_reference())
}

/// Parses a binding table reference from a token slice.
pub fn parse_binding_table_reference(tokens: &[Token]) -> ParseResult<BindingTableReference> {
    parse_with_full_consumption(tokens, |parser| parser.parse_binding_table_reference())
}

/// Parses a procedure reference from a token slice.
pub fn parse_procedure_reference(tokens: &[Token]) -> ParseResult<ProcedureReference> {
    parse_with_full_consumption(tokens, |parser| parser.parse_procedure_reference())
}

/// Parses a catalog-qualified name from a token slice.
pub fn parse_catalog_qualified_name(tokens: &[Token]) -> ParseResult<CatalogQualifiedName> {
    parse_with_full_consumption(tokens, |parser| parser.parse_catalog_qualified_name())
}

/// Parses a catalog object parent reference from a token slice.
pub fn parse_catalog_object_parent_reference(
    tokens: &[Token],
) -> ParseResult<CatalogObjectParentReference> {
    parse_with_full_consumption(tokens, |parser| {
        parser.parse_catalog_object_parent_reference()
    })
}

fn parse_with_full_consumption<T>(
    tokens: &[Token],
    parse: impl FnOnce(&mut ReferenceParser<'_>) -> ParseResult<T>,
) -> ParseResult<T> {
    let normalized = normalize_tokens(tokens);
    let mut parser = ReferenceParser::new(&normalized);
    let parsed = parse(&mut parser)?;

    if !matches!(parser.current().kind, TokenKind::Eof) {
        return Err(Box::new(
            Diag::error("unexpected trailing tokens after reference")
                .with_primary_label(parser.current().span.clone(), "unexpected token")
                .with_code("P_REF"),
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

    // ========================================================================
    // Schema Reference Tests
    // ========================================================================

    #[test]
    fn test_parse_absolute_schema_path() {
        // /my_schema
        let tokens = vec![
            make_token(TokenKind::Slash, 0, 1),
            make_token(TokenKind::Identifier("my_schema".into()), 1, 10),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        let schema_ref = result.unwrap();
        assert!(matches!(schema_ref, SchemaReference::AbsolutePath { .. }));
    }

    #[test]
    fn test_parse_absolute_schema_path_with_directory() {
        // /dir/my_schema
        let tokens = vec![
            make_token(TokenKind::Slash, 0, 1),
            make_token(TokenKind::Identifier("dir".into()), 1, 4),
            make_token(TokenKind::Slash, 4, 5),
            make_token(TokenKind::Identifier("my_schema".into()), 5, 14),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        if let SchemaReference::AbsolutePath { components, .. } = result.unwrap() {
            assert_eq!(components.len(), 2);
            assert_eq!(components[0].as_str(), "dir");
            assert_eq!(components[1].as_str(), "my_schema");
        } else {
            panic!("Expected AbsolutePath");
        }
    }

    #[test]
    fn test_parse_relative_schema_path() {
        // ../other_schema
        let tokens = vec![
            make_token(TokenKind::DotDot, 0, 2),
            make_token(TokenKind::Slash, 2, 3),
            make_token(TokenKind::Identifier("other_schema".into()), 3, 15),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        if let SchemaReference::RelativePath {
            up_levels,
            components,
            ..
        } = result.unwrap()
        {
            assert_eq!(up_levels, 1);
            assert_eq!(components.len(), 1);
            assert_eq!(components[0].as_str(), "other_schema");
        } else {
            panic!("Expected RelativePath");
        }
    }

    #[test]
    fn test_parse_relative_schema_path_multiple_levels() {
        // ../../another/schema
        let tokens = vec![
            make_token(TokenKind::DotDot, 0, 2),
            make_token(TokenKind::Slash, 2, 3),
            make_token(TokenKind::DotDot, 3, 5),
            make_token(TokenKind::Slash, 5, 6),
            make_token(TokenKind::Identifier("another".into()), 6, 13),
            make_token(TokenKind::Slash, 13, 14),
            make_token(TokenKind::Identifier("schema".into()), 14, 20),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        if let SchemaReference::RelativePath {
            up_levels,
            components,
            ..
        } = result.unwrap()
        {
            assert_eq!(up_levels, 2);
            assert_eq!(components.len(), 2);
            assert_eq!(components[0].as_str(), "another");
            assert_eq!(components[1].as_str(), "schema");
        } else {
            panic!("Expected RelativePath");
        }
    }

    #[test]
    fn test_parse_relative_schema_path_requires_component() {
        // ../
        let tokens = vec![
            make_token(TokenKind::DotDot, 0, 2),
            make_token(TokenKind::Slash, 2, 3),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dot_schema_reference() {
        // .
        let tokens = vec![make_token(TokenKind::Dot, 0, 1)];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SchemaReference::Dot { .. }));
    }

    #[test]
    fn test_parse_plain_identifier_schema_reference() {
        // my_schema
        let tokens = vec![make_token(TokenKind::Identifier("my_schema".into()), 0, 9)];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::Identifier { .. }
        ));
    }

    #[test]
    fn test_parse_home_schema() {
        // HOME_SCHEMA (as identifier)
        let tokens = vec![make_token(
            TokenKind::Identifier("HOME_SCHEMA".into()),
            0,
            11,
        )];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::HomeSchema { .. }
        ));
    }

    #[test]
    fn test_parse_current_schema() {
        // CURRENT_SCHEMA (as identifier)
        let tokens = vec![make_token(
            TokenKind::Identifier("CURRENT_SCHEMA".into()),
            0,
            14,
        )];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::CurrentSchema { .. }
        ));
    }

    #[test]
    fn test_parse_schema_reference_parameter() {
        // $$schema_param
        let tokens = vec![make_token(
            TokenKind::ReferenceParameter("schema_param".into()),
            0,
            14,
        )];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::ReferenceParameter { .. }
        ));
    }

    #[test]
    fn test_parse_home_schema_with_keywords() {
        // HOME SCHEMA (as separate tokens)
        let tokens = vec![
            make_token(TokenKind::Home, 0, 4),
            make_token(TokenKind::Schema, 5, 11),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::HomeSchema { .. }
        ));
    }

    #[test]
    fn test_parse_current_schema_with_keywords() {
        // CURRENT SCHEMA (as separate tokens)
        let tokens = vec![
            make_token(TokenKind::Current, 0, 7),
            make_token(TokenKind::Schema, 8, 14),
        ];
        let result = parse_schema_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            SchemaReference::CurrentSchema { .. }
        ));
    }

    // ========================================================================
    // Graph Reference Tests
    // ========================================================================

    #[test]
    fn test_parse_graph_reference_delimited() {
        // "my graph"
        let tokens = vec![make_token(
            TokenKind::DelimitedIdentifier("my graph".into()),
            0,
            10,
        )];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), GraphReference::Delimited { .. }));
    }

    #[test]
    fn test_parse_graph_reference_parameter() {
        // $$graph_param
        let tokens = vec![make_token(
            TokenKind::ReferenceParameter("graph_param".into()),
            0,
            13,
        )];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphReference::ReferenceParameter { .. }
        ));
    }

    #[test]
    fn test_parse_home_graph() {
        // HOME_GRAPH
        let tokens = vec![make_token(
            TokenKind::Identifier("HOME_GRAPH".into()),
            0,
            10,
        )];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), GraphReference::HomeGraph { .. }));
    }

    #[test]
    fn test_parse_home_property_graph() {
        // HOME_PROPERTY_GRAPH
        let tokens = vec![make_token(
            TokenKind::Identifier("HOME_PROPERTY_GRAPH".into()),
            0,
            19,
        )];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphReference::HomePropertyGraph { .. }
        ));
    }

    #[test]
    fn test_parse_home_graph_with_keywords() {
        // HOME GRAPH
        let tokens = vec![
            make_token(TokenKind::Home, 0, 4),
            make_token(TokenKind::Graph, 5, 10),
        ];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), GraphReference::HomeGraph { .. }));
    }

    #[test]
    fn test_parse_home_property_graph_with_keywords() {
        // HOME PROPERTY GRAPH
        let tokens = vec![
            make_token(TokenKind::Home, 0, 4),
            make_token(TokenKind::Property, 5, 13),
            make_token(TokenKind::Graph, 14, 19),
        ];
        let result = parse_graph_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphReference::HomePropertyGraph { .. }
        ));
    }

    // ========================================================================
    // Catalog-Qualified Name Tests
    // ========================================================================

    #[test]
    fn test_parse_simple_name() {
        // my_graph
        let tokens = vec![make_token(TokenKind::Identifier("my_graph".into()), 0, 8)];
        let result = parse_catalog_qualified_name(&tokens);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.parent.is_none());
        assert_eq!(name.name.as_str(), "my_graph");
    }

    #[test]
    fn test_parse_single_level_qualified_name() {
        // schema::graph
        let tokens = vec![
            make_token(TokenKind::Identifier("schema".into()), 0, 6),
            make_token(TokenKind::DoubleColon, 6, 8),
            make_token(TokenKind::Identifier("graph".into()), 8, 13),
        ];
        let result = parse_catalog_qualified_name(&tokens);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.parent.is_some());
        assert_eq!(name.name.as_str(), "graph");
    }

    #[test]
    fn test_parent_span_uses_byte_offsets_not_token_index() {
        let tokens = vec![
            make_token(TokenKind::Identifier("schema".into()), 3, 9),
            make_token(TokenKind::DoubleColon, 9, 11),
            make_token(TokenKind::Identifier("graph".into()), 11, 16),
        ];
        let result = parse_catalog_qualified_name(&tokens);
        assert!(result.is_ok());
        let name = result.unwrap();
        let Some(parent) = name.parent else {
            panic!("expected parent reference");
        };
        assert_eq!(parent.span().start, 3);
    }

    #[test]
    fn test_parse_multi_level_qualified_name() {
        // parent::child::name
        let tokens = vec![
            make_token(TokenKind::Identifier("parent".into()), 0, 6),
            make_token(TokenKind::DoubleColon, 6, 8),
            make_token(TokenKind::Identifier("child".into()), 8, 13),
            make_token(TokenKind::DoubleColon, 13, 15),
            make_token(TokenKind::Identifier("name".into()), 15, 19),
        ];
        let result = parse_catalog_qualified_name(&tokens);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.parent.is_some());
        assert_eq!(name.name.as_str(), "name");
    }

    #[test]
    fn test_parse_absolute_schema_qualified_name() {
        // /my_schema::graph
        let tokens = vec![
            make_token(TokenKind::Slash, 0, 1),
            make_token(TokenKind::Identifier("my_schema".into()), 1, 10),
            make_token(TokenKind::DoubleColon, 10, 12),
            make_token(TokenKind::Identifier("graph".into()), 12, 17),
        ];
        let result = parse_catalog_qualified_name(&tokens);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.parent.is_some());
        assert_eq!(name.name.as_str(), "graph");
    }

    // ========================================================================
    // Graph Type Reference Tests
    // ========================================================================

    #[test]
    fn test_parse_graph_type_reference() {
        // schema::graph_type
        let tokens = vec![
            make_token(TokenKind::Identifier("schema".into()), 0, 6),
            make_token(TokenKind::DoubleColon, 6, 8),
            make_token(TokenKind::Identifier("graph_type".into()), 8, 18),
        ];
        let result = parse_graph_type_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphTypeReference::CatalogQualified { .. }
        ));
    }

    #[test]
    fn test_parse_graph_type_reference_parameter() {
        // $$type_param
        let tokens = vec![make_token(
            TokenKind::ReferenceParameter("type_param".into()),
            0,
            12,
        )];
        let result = parse_graph_type_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            GraphTypeReference::ReferenceParameter { .. }
        ));
    }

    // ========================================================================
    // Binding Table Reference Tests
    // ========================================================================

    #[test]
    fn test_parse_binding_table_reference() {
        // schema::table
        let tokens = vec![
            make_token(TokenKind::Identifier("schema".into()), 0, 6),
            make_token(TokenKind::DoubleColon, 6, 8),
            make_token(TokenKind::Identifier("table".into()), 8, 13),
        ];
        let result = parse_binding_table_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BindingTableReference::CatalogQualified { .. }
        ));
    }

    #[test]
    fn test_parse_binding_table_reference_delimited() {
        // "my table"
        let tokens = vec![make_token(
            TokenKind::DelimitedIdentifier("my table".into()),
            0,
            10,
        )];
        let result = parse_binding_table_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BindingTableReference::Delimited { .. }
        ));
    }

    // ========================================================================
    // Procedure Reference Tests
    // ========================================================================

    #[test]
    fn test_parse_procedure_reference() {
        // schema::procedure
        let tokens = vec![
            make_token(TokenKind::Identifier("schema".into()), 0, 6),
            make_token(TokenKind::DoubleColon, 6, 8),
            make_token(TokenKind::Identifier("procedure".into()), 8, 17),
        ];
        let result = parse_procedure_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            ProcedureReference::CatalogQualified { .. }
        ));
    }

    #[test]
    fn test_parse_procedure_reference_parameter() {
        // $$proc_param
        let tokens = vec![make_token(
            TokenKind::ReferenceParameter("proc_param".into()),
            0,
            12,
        )];
        let result = parse_procedure_reference(&tokens);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            ProcedureReference::ReferenceParameter { .. }
        ));
    }
}
