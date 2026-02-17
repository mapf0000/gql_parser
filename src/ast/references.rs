//! Catalog and object reference AST nodes for GQL.
//!
//! This module defines all catalog and object reference forms including:
//! - Schema references (absolute paths, relative paths, predefined)
//! - Graph references (catalog-qualified, delimited, predefined)
//! - Graph type references
//! - Binding table references
//! - Procedure references
//! - Catalog-qualified names with parent references
//!
//! These references are used throughout the language in session commands,
//! catalog operations, and type specifications.

use crate::ast::Span;
use smol_str::SmolStr;

// ============================================================================
// Schema References
// ============================================================================

/// Schema reference variants.
///
/// Schema references can use absolute paths, relative paths, predefined names,
/// or reference parameters.
///
/// # Examples
///
/// ```text
/// /my_schema              -- Absolute path
/// /dir/my_schema          -- Absolute path with directory
/// ../other_schema         -- Relative path
/// ../../another/schema    -- Relative path with multiple levels
/// HOME_SCHEMA             -- Predefined reference
/// CURRENT_SCHEMA          -- Predefined reference
/// .                       -- Current schema (dot notation)
/// $$schema_param          -- Reference parameter
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaReference {
    /// Absolute path starting with /
    ///
    /// Example: `/my_schema`, `/dir/my_schema`
    AbsolutePath {
        /// Path components (schema name and optional directory components)
        components: Vec<SmolStr>,
        /// Source span
        span: Span,
    },

    /// Relative path using ../
    ///
    /// Example: `../other_schema`, `../../another/schema`
    RelativePath {
        /// Number of levels to go up (..)
        up_levels: u32,
        /// Path components after the relative navigation
        components: Vec<SmolStr>,
        /// Source span
        span: Span,
    },

    /// HOME_SCHEMA - reference to the home schema
    HomeSchema {
        /// Source span
        span: Span,
    },

    /// CURRENT_SCHEMA - reference to the current schema
    CurrentSchema {
        /// Source span
        span: Span,
    },

    /// . (dot) - reference to the current schema
    Dot {
        /// Source span
        span: Span,
    },

    /// $$name - reference parameter
    ReferenceParameter {
        /// Parameter name (without the $$ prefix)
        name: SmolStr,
        /// Source span
        span: Span,
    },
}

impl SchemaReference {
    /// Returns the span of this schema reference
    pub fn span(&self) -> Span {
        match self {
            SchemaReference::AbsolutePath { span, .. }
            | SchemaReference::RelativePath { span, .. }
            | SchemaReference::HomeSchema { span }
            | SchemaReference::CurrentSchema { span }
            | SchemaReference::Dot { span }
            | SchemaReference::ReferenceParameter { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Graph References
// ============================================================================

/// Graph reference variants.
///
/// Graph references can be catalog-qualified, delimited identifiers,
/// predefined names, or reference parameters.
///
/// # Examples
///
/// ```text
/// my_schema::my_graph     -- Catalog-qualified
/// "my graph"              -- Delimited identifier
/// HOME_GRAPH              -- Predefined reference
/// HOME_PROPERTY_GRAPH     -- Predefined reference
/// $$graph_param           -- Reference parameter
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum GraphReference {
    /// Catalog-qualified graph name
    ///
    /// Example: `schema::graph_name`
    CatalogQualified {
        /// Catalog-qualified name
        name: CatalogQualifiedName,
        /// Source span
        span: Span,
    },

    /// Delimited identifier
    ///
    /// Example: `"my graph with spaces"`
    Delimited {
        /// Graph name (delimited identifier content)
        name: SmolStr,
        /// Source span
        span: Span,
    },

    /// HOME_GRAPH - reference to the home graph
    HomeGraph {
        /// Source span
        span: Span,
    },

    /// HOME_PROPERTY_GRAPH - reference to the home property graph
    HomePropertyGraph {
        /// Source span
        span: Span,
    },

    /// $$name - reference parameter
    ReferenceParameter {
        /// Parameter name (without the $$ prefix)
        name: SmolStr,
        /// Source span
        span: Span,
    },
}

impl GraphReference {
    /// Returns the span of this graph reference
    pub fn span(&self) -> Span {
        match self {
            GraphReference::CatalogQualified { span, .. }
            | GraphReference::Delimited { span, .. }
            | GraphReference::HomeGraph { span }
            | GraphReference::HomePropertyGraph { span }
            | GraphReference::ReferenceParameter { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Graph Type References
// ============================================================================

/// Graph type reference variants.
///
/// Graph type references can be catalog-qualified or reference parameters.
///
/// # Examples
///
/// ```text
/// my_schema::my_graph_type    -- Catalog-qualified
/// $$type_param                -- Reference parameter
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum GraphTypeReference {
    /// Catalog-qualified graph type name
    ///
    /// Example: `schema::graph_type_name`
    CatalogQualified {
        /// Catalog-qualified name
        name: CatalogQualifiedName,
        /// Source span
        span: Span,
    },

    /// $$name - reference parameter
    ReferenceParameter {
        /// Parameter name (without the $$ prefix)
        name: SmolStr,
        /// Source span
        span: Span,
    },
}

impl GraphTypeReference {
    /// Returns the span of this graph type reference
    pub fn span(&self) -> Span {
        match self {
            GraphTypeReference::CatalogQualified { span, .. }
            | GraphTypeReference::ReferenceParameter { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Binding Table References
// ============================================================================

/// Binding table reference variants.
///
/// Binding table references can be catalog-qualified, delimited identifiers,
/// or reference parameters.
///
/// # Examples
///
/// ```text
/// my_schema::my_table     -- Catalog-qualified
/// "my table"              -- Delimited identifier
/// $$table_param           -- Reference parameter
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum BindingTableReference {
    /// Catalog-qualified binding table name
    ///
    /// Example: `schema::table_name`
    CatalogQualified {
        /// Catalog-qualified name
        name: CatalogQualifiedName,
        /// Source span
        span: Span,
    },

    /// Delimited identifier
    ///
    /// Example: `"my table with spaces"`
    Delimited {
        /// Table name (delimited identifier content)
        name: SmolStr,
        /// Source span
        span: Span,
    },

    /// $$name - reference parameter
    ReferenceParameter {
        /// Parameter name (without the $$ prefix)
        name: SmolStr,
        /// Source span
        span: Span,
    },
}

impl BindingTableReference {
    /// Returns the span of this binding table reference
    pub fn span(&self) -> Span {
        match self {
            BindingTableReference::CatalogQualified { span, .. }
            | BindingTableReference::Delimited { span, .. }
            | BindingTableReference::ReferenceParameter { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Procedure References
// ============================================================================

/// Procedure reference variants.
///
/// Procedure references can be catalog-qualified or reference parameters.
///
/// # Examples
///
/// ```text
/// my_schema::my_procedure     -- Catalog-qualified
/// $$proc_param                -- Reference parameter
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ProcedureReference {
    /// Catalog-qualified procedure name
    ///
    /// Example: `schema::procedure_name`
    CatalogQualified {
        /// Catalog-qualified name
        name: CatalogQualifiedName,
        /// Source span
        span: Span,
    },

    /// $$name - reference parameter
    ReferenceParameter {
        /// Parameter name (without the $$ prefix)
        name: SmolStr,
        /// Source span
        span: Span,
    },
}

impl ProcedureReference {
    /// Returns the span of this procedure reference
    pub fn span(&self) -> Span {
        match self {
            ProcedureReference::CatalogQualified { span, .. }
            | ProcedureReference::ReferenceParameter { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Catalog-Qualified Names
// ============================================================================

/// Catalog-qualified name with optional parent reference.
///
/// Catalog-qualified names use the :: separator to indicate catalog hierarchy.
///
/// # Examples
///
/// ```text
/// schema::name                -- Single-level qualification
/// parent::child::name         -- Multi-level qualification
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogQualifiedName {
    /// Optional parent reference (for multi-level qualification)
    pub parent: Option<CatalogObjectParentReference>,
    /// Object name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
}

/// Catalog object parent reference variants.
///
/// Parent references can be schema references or nested catalog-qualified names.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogObjectParentReference {
    /// Schema reference as parent
    ///
    /// Example: `/my_schema::` in `/my_schema::graph_name`
    Schema {
        /// Schema reference
        schema: SchemaReference,
        /// Source span
        span: Span,
    },

    /// Another catalog-qualified name as parent (for multi-level qualification)
    ///
    /// Example: `parent::child::` in `parent::child::name`
    Object {
        /// Nested catalog-qualified name
        name: Box<CatalogQualifiedName>,
        /// Source span
        span: Span,
    },
}

impl CatalogObjectParentReference {
    /// Returns the span of this parent reference
    pub fn span(&self) -> Span {
        match self {
            CatalogObjectParentReference::Schema { span, .. }
            | CatalogObjectParentReference::Object { span, .. } => span.clone(),
        }
    }
}

// ============================================================================
// Binding Variables
// ============================================================================

/// A binding variable for variable declarations in queries.
///
/// Used in LET, FOR, WITH ORDINALITY/OFFSET, and other binding contexts.
///
/// # Examples
///
/// ```text
/// LET x = 5           -- x is a binding variable
/// FOR item IN list    -- item is a binding variable
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingVariable {
    /// Variable name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
}

