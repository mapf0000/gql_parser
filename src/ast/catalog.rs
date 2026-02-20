//! Catalog statement AST nodes.
//!
//! This module defines AST nodes for catalog management including:
//! - CREATE/DROP SCHEMA statements
//! - CREATE/DROP GRAPH statements
//! - CREATE/DROP GRAPH TYPE statements
//! - CREATE/DROP PROCEDURE statements
//! - CALL catalog-modifying procedure statements

use crate::ast::Span;
use crate::ast::graph_type::NestedGraphTypeSpecification;
use crate::ast::procedure::{CallProcedureStatement, NestedProcedureSpecification};
use crate::ast::references::ProcedureReference;
use crate::ast::references::{GraphReference, GraphTypeReference, SchemaReference};

// ============================================================================
// Schema Statements
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
// Graph Statements
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
// Graph Type Statements
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
    /// LIKE <graph_reference>
    LikeGraph { graph: GraphReference, span: Span },
    /// Embedded nested graph type specification.
    Detailed {
        specification: NestedGraphTypeSpecification,
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
// Catalog Procedure Call
// ============================================================================

/// CREATE PROCEDURE statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateProcedureStatement {
    /// Whether OR REPLACE was specified.
    pub or_replace: bool,
    /// Whether IF NOT EXISTS was specified.
    pub if_not_exists: bool,
    /// Procedure reference.
    pub procedure: ProcedureReference,
    /// Procedure body specification.
    pub specification: NestedProcedureSpecification,
    pub span: Span,
}

/// DROP PROCEDURE statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DropProcedureStatement {
    /// Whether IF EXISTS was specified.
    pub if_exists: bool,
    /// Procedure reference.
    pub procedure: ProcedureReference,
    pub span: Span,
}

/// CALL catalog-modifying procedure statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CallCatalogModifyingProcedureStatement {
    /// Full CALL statement payload.
    pub call: CallProcedureStatement,
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
    CreateProcedure(CreateProcedureStatement),
    DropProcedure(DropProcedureStatement),
    CallCatalogModifyingProcedure(CallCatalogModifyingProcedureStatement),
}

impl CatalogStatementKind {
    /// Returns the source span of this catalog statement.
    pub fn span(&self) -> &Span {
        match self {
            CatalogStatementKind::CreateSchema(stmt) => &stmt.span,
            CatalogStatementKind::DropSchema(stmt) => &stmt.span,
            CatalogStatementKind::CreateGraph(stmt) => &stmt.span,
            CatalogStatementKind::DropGraph(stmt) => &stmt.span,
            CatalogStatementKind::CreateGraphType(stmt) => &stmt.span,
            CatalogStatementKind::DropGraphType(stmt) => &stmt.span,
            CatalogStatementKind::CreateProcedure(stmt) => &stmt.span,
            CatalogStatementKind::DropProcedure(stmt) => &stmt.span,
            CatalogStatementKind::CallCatalogModifyingProcedure(stmt) => &stmt.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::references::CatalogQualifiedName;

    #[test]
    fn test_schema_reference_variants() {
        let abs = SchemaReference::AbsolutePath {
            components: vec!["catalog".into(), "schema".into()],
            span: 0..10,
        };
        assert!(matches!(abs, SchemaReference::AbsolutePath { .. }));

        let home = SchemaReference::HomeSchema { span: 0..11 };
        assert!(matches!(home, SchemaReference::HomeSchema { .. }));

        let param = SchemaReference::ReferenceParameter {
            name: "myschema".into(),
            span: 0..10,
        };
        assert!(matches!(param, SchemaReference::ReferenceParameter { .. }));
    }

    #[test]
    fn test_graph_reference_variants() {
        let qualified = GraphReference::CatalogQualified {
            name: CatalogQualifiedName {
                parent: None,
                name: "graph".into(),
                span: 0..5,
            },
            span: 0..5,
        };
        assert!(matches!(qualified, GraphReference::CatalogQualified { .. }));

        let home = GraphReference::HomeGraph { span: 0..10 };
        assert!(matches!(home, GraphReference::HomeGraph { .. }));
    }

    #[test]
    fn test_create_schema_statement() {
        let stmt = CreateSchemaStatement {
            or_replace: true,
            if_not_exists: false,
            schema: SchemaReference::Dot { span: 15..16 },
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
            graph_type: GraphTypeReference::ReferenceParameter {
                name: "mytype".into(),
                span: 3..11,
            },
            span: 0..11,
        };
        assert!(matches!(of_type, GraphTypeSpec::Of { .. }));

        let like = GraphTypeSpec::Like {
            graph: GraphReference::HomeGraph { span: 5..15 },
            span: 0..15,
        };
        assert!(matches!(like, GraphTypeSpec::Like { .. }));
    }

    #[test]
    fn test_drop_statements() {
        let drop_schema = DropSchemaStatement {
            if_exists: true,
            schema: SchemaReference::CurrentSchema { span: 11..25 },
            span: 0..25,
        };
        assert!(drop_schema.if_exists);

        let drop_graph = DropGraphStatement {
            property: true,
            if_exists: false,
            graph: GraphReference::HomePropertyGraph { span: 11..31 },
            span: 0..31,
        };
        assert!(drop_graph.property);
        assert!(!drop_graph.if_exists);
    }
}
