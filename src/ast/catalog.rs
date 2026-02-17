//! Catalog statement AST nodes for Sprint 4.
//!
//! This module defines AST nodes for catalog management including:
//! - Schema references, graph references, graph type references
//! - CREATE/DROP SCHEMA statements
//! - CREATE/DROP GRAPH statements
//! - CREATE/DROP GRAPH TYPE statements
//! - CALL catalog-modifying procedure statements

use crate::ast::Span;
use smol_str::SmolStr;

// ============================================================================
// Catalog References (Task 8)
// ============================================================================

/// Schema reference in a catalog context.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaReference {
    /// Absolute path (/schema/path)
    AbsolutePath { path: Vec<SmolStr>, span: Span },
    /// Relative path (../schema or ./schema)
    RelativePath { path: Vec<SmolStr>, span: Span },
    /// HOME_SCHEMA predefined reference
    HomeSchema(Span),
    /// CURRENT_SCHEMA predefined reference
    CurrentSchema(Span),
    /// Single dot (.) reference
    Dot(Span),
    /// Reference parameter ($$name)
    Parameter { name: SmolStr, span: Span },
}

/// Graph reference in a catalog context.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphReference {
    /// Catalog-qualified name (catalog.schema.graph)
    CatalogQualified {
        catalog: Option<SmolStr>,
        schema: Option<SmolStr>,
        name: SmolStr,
        span: Span,
    },
    /// Delimited identifier
    Delimited { name: SmolStr, span: Span },
    /// HOME_GRAPH predefined reference
    HomeGraph(Span),
    /// HOME_PROPERTY_GRAPH predefined reference
    HomePropertyGraph(Span),
    /// Reference parameter ($$name)
    Parameter { name: SmolStr, span: Span },
}

/// Graph type reference in a catalog context.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphTypeReference {
    /// Catalog-qualified name (catalog.schema.type)
    CatalogQualified {
        catalog: Option<SmolStr>,
        schema: Option<SmolStr>,
        name: SmolStr,
        span: Span,
    },
    /// Reference parameter ($$name)
    Parameter { name: SmolStr, span: Span },
}

// ============================================================================
// Schema Statements (Task 4)
// ============================================================================

/// CREATE SCHEMA statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSchemaStatement {
    /// Whether OR REPLACE was specified
    pub or_replace: bool,
    /// Whether IF NOT EXISTS was specified
    pub if_not_exists: bool,
    /// Schema reference
    pub schema: SchemaReference,
    pub span: Span,
}

/// DROP SCHEMA statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DropSchemaStatement {
    /// Whether IF EXISTS was specified
    pub if_exists: bool,
    /// Schema reference
    pub schema: SchemaReference,
    pub span: Span,
}

// ============================================================================
// Graph Statements (Task 5)
// ============================================================================

/// CREATE GRAPH statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateGraphStatement {
    /// Whether PROPERTY keyword was present
    pub property: bool,
    /// Whether OR REPLACE was specified
    pub or_replace: bool,
    /// Whether IF NOT EXISTS was specified
    pub if_not_exists: bool,
    /// Graph name
    pub graph: GraphReference,
    /// Graph type specification (if any)
    pub graph_type_spec: Option<GraphTypeSpec>,
    pub span: Span,
}

/// Graph type specification for CREATE GRAPH.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphTypeSpec {
    /// Open graph type
    Open { span: Span },
    /// OF <graph_type>
    Of {
        graph_type: GraphTypeReference,
        span: Span,
    },
    /// LIKE <graph_reference>
    Like { graph: GraphReference, span: Span },
    /// AS COPY OF <graph_reference>
    AsCopyOf { graph: GraphReference, span: Span },
}

/// DROP GRAPH statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DropGraphStatement {
    /// Whether PROPERTY keyword was present
    pub property: bool,
    /// Whether IF EXISTS was specified
    pub if_exists: bool,
    /// Graph reference
    pub graph: GraphReference,
    pub span: Span,
}

// ============================================================================
// Graph Type Statements (Task 6)
// ============================================================================

/// CREATE GRAPH TYPE statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateGraphTypeStatement {
    /// Whether PROPERTY keyword was present
    pub property: bool,
    /// Whether OR REPLACE was specified
    pub or_replace: bool,
    /// Whether IF NOT EXISTS was specified
    pub if_not_exists: bool,
    /// Graph type name
    pub graph_type: GraphTypeReference,
    /// Graph type source (if any)
    pub source: Option<GraphTypeSource>,
    pub span: Span,
}

/// Graph type source specification.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphTypeSource {
    /// AS COPY OF <graph_type_reference>
    AsCopyOf {
        graph_type: GraphTypeReference,
        span: Span,
    },
    // Detailed type specifications deferred to Sprint 12
    Detailed {
        span: Span,
    },
}

/// DROP GRAPH TYPE statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DropGraphTypeStatement {
    /// Whether PROPERTY keyword was present
    pub property: bool,
    /// Whether IF EXISTS was specified
    pub if_exists: bool,
    /// Graph type reference
    pub graph_type: GraphTypeReference,
    pub span: Span,
}

// ============================================================================
// Catalog Procedure Call (Task 7)
// ============================================================================

/// CALL catalog-modifying procedure statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CallCatalogModifyingProcedureStatement {
    /// Procedure name (simplified for now)
    pub procedure_name: SmolStr,
    /// Procedure arguments placeholder (deferred to Sprint 11)
    pub span: Span,
}

// ============================================================================
// Unified Catalog Statement Enum
// ============================================================================

/// All catalog statement variants.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogStatementKind {
    CreateSchema(CreateSchemaStatement),
    DropSchema(DropSchemaStatement),
    CreateGraph(CreateGraphStatement),
    DropGraph(DropGraphStatement),
    CreateGraphType(CreateGraphTypeStatement),
    DropGraphType(DropGraphTypeStatement),
    CallCatalogModifyingProcedure(CallCatalogModifyingProcedureStatement),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_reference_variants() {
        let abs = SchemaReference::AbsolutePath {
            path: vec!["catalog".into(), "schema".into()],
            span: 0..10,
        };
        assert!(matches!(abs, SchemaReference::AbsolutePath { .. }));

        let home = SchemaReference::HomeSchema(0..11);
        assert!(matches!(home, SchemaReference::HomeSchema(_)));

        let param = SchemaReference::Parameter {
            name: "myschema".into(),
            span: 0..10,
        };
        assert!(matches!(param, SchemaReference::Parameter { .. }));
    }

    #[test]
    fn test_graph_reference_variants() {
        let qualified = GraphReference::CatalogQualified {
            catalog: Some("cat".into()),
            schema: Some("sch".into()),
            name: "graph".into(),
            span: 0..15,
        };
        assert!(matches!(qualified, GraphReference::CatalogQualified { .. }));

        let home = GraphReference::HomeGraph(0..10);
        assert!(matches!(home, GraphReference::HomeGraph(_)));
    }

    #[test]
    fn test_create_schema_statement() {
        let stmt = CreateSchemaStatement {
            or_replace: true,
            if_not_exists: false,
            schema: SchemaReference::Dot(15..16),
            span: 0..30,
        };
        assert!(stmt.or_replace);
        assert!(!stmt.if_not_exists);
    }

    #[test]
    fn test_graph_type_spec_variants() {
        let open = GraphTypeSpec::Open { span: 0..10 };
        assert!(matches!(open, GraphTypeSpec::Open { .. }));

        let of_type = GraphTypeSpec::Of {
            graph_type: GraphTypeReference::Parameter {
                name: "mytype".into(),
                span: 3..11,
            },
            span: 0..11,
        };
        assert!(matches!(of_type, GraphTypeSpec::Of { .. }));

        let like = GraphTypeSpec::Like {
            graph: GraphReference::HomeGraph(5..15),
            span: 0..15,
        };
        assert!(matches!(like, GraphTypeSpec::Like { .. }));
    }

    #[test]
    fn test_drop_statements() {
        let drop_schema = DropSchemaStatement {
            if_exists: true,
            schema: SchemaReference::CurrentSchema(11..25),
            span: 0..25,
        };
        assert!(drop_schema.if_exists);

        let drop_graph = DropGraphStatement {
            property: true,
            if_exists: false,
            graph: GraphReference::HomePropertyGraph(11..31),
            span: 0..31,
        };
        assert!(drop_graph.property);
        assert!(!drop_graph.if_exists);
    }
}
