//! Catalog trait for optional catalog-dependent validation.
//!
//! This module defines the Catalog trait that allows the semantic validator
//! to validate references against an optional catalog of graphs, schemas,
//! procedures, and types.

/// Result type for catalog lookups that may fail.
pub type CatalogResult<T> = Result<T, CatalogError>;

/// Error type for catalog validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// A graph was not found in the catalog.
    GraphNotFound { graph: String },

    /// A schema was not found in the catalog.
    SchemaNotFound { schema: String },

    /// A procedure was not found in the catalog.
    ProcedureNotFound { procedure: String },

    /// A type was not found in the catalog.
    TypeNotFound { type_name: String },

    /// The catalog is unavailable or not configured.
    CatalogUnavailable,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::GraphNotFound { graph } => {
                write!(f, "Graph '{}' not found in catalog", graph)
            }
            CatalogError::SchemaNotFound { schema } => {
                write!(f, "Schema '{}' not found in catalog", schema)
            }
            CatalogError::ProcedureNotFound { procedure } => {
                write!(f, "Procedure '{}' not found in catalog", procedure)
            }
            CatalogError::TypeNotFound { type_name } => {
                write!(f, "Type '{}' not found in catalog", type_name)
            }
            CatalogError::CatalogUnavailable => {
                write!(f, "Catalog is not available")
            }
        }
    }
}

impl std::error::Error for CatalogError {}

/// Represents a graph definition in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDefinition {
    /// The name of the graph.
    pub name: String,

    /// The schema associated with this graph (if any).
    pub schema: Option<String>,

    /// Additional metadata about the graph.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Represents a schema definition in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDefinition {
    /// The name of the schema.
    pub name: String,

    /// Labels defined in this schema.
    pub labels: Vec<String>,

    /// Additional metadata about the schema.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Represents a procedure definition in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureDefinition {
    /// The name of the procedure.
    pub name: String,

    /// Parameter types for the procedure.
    pub parameters: Vec<String>,

    /// Return type of the procedure.
    pub return_type: Option<String>,

    /// Additional metadata about the procedure.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Trait for catalog access, allowing validation of references.
///
/// Implement this trait to provide catalog information to the semantic validator.
/// The validator can then check that:
/// - Graph references (e.g., `USE GRAPH mygraph`) are valid
/// - Schema references are valid
/// - Procedure calls reference existing procedures
/// - Type references are valid
///
/// # Example
///
/// ```ignore
/// use gql_parser::semantic::catalog::{Catalog, GraphDefinition};
///
/// struct MyCatalog {
///     graphs: HashMap<String, GraphDefinition>,
/// }
///
/// impl Catalog for MyCatalog {
///     fn get_graph(&self, name: &str) -> Option<&GraphDefinition> {
///         self.graphs.get(name)
///     }
///
///     fn get_schema(&self, name: &str) -> Option<&SchemaDefinition> {
///         None // Not implemented in this example
///     }
///
///     fn get_procedure(&self, name: &str) -> Option<&ProcedureDefinition> {
///         None // Not implemented in this example
///     }
/// }
/// ```
pub trait Catalog {
    /// Looks up a graph by name.
    ///
    /// Returns `Some` if the graph exists in the catalog.
    fn get_graph(&self, name: &str) -> Option<&GraphDefinition>;

    /// Looks up a schema by name.
    ///
    /// Returns `Some` if the schema exists in the catalog.
    fn get_schema(&self, name: &str) -> Option<&SchemaDefinition>;

    /// Looks up a procedure by name.
    ///
    /// Returns `Some` if the procedure exists in the catalog.
    fn get_procedure(&self, name: &str) -> Option<&ProcedureDefinition>;

    /// Validates that a graph exists in the catalog.
    ///
    /// Returns `Ok(())` if the graph exists, or an error if not.
    fn validate_graph(&self, name: &str) -> CatalogResult<()> {
        if self.get_graph(name).is_some() {
            Ok(())
        } else {
            Err(CatalogError::GraphNotFound {
                graph: name.to_string(),
            })
        }
    }

    /// Validates that a schema exists in the catalog.
    ///
    /// Returns `Ok(())` if the schema exists, or an error if not.
    fn validate_schema(&self, name: &str) -> CatalogResult<()> {
        if self.get_schema(name).is_some() {
            Ok(())
        } else {
            Err(CatalogError::SchemaNotFound {
                schema: name.to_string(),
            })
        }
    }

    /// Validates that a procedure exists in the catalog.
    ///
    /// Returns `Ok(())` if the procedure exists, or an error if not.
    fn validate_procedure(&self, name: &str) -> CatalogResult<()> {
        if self.get_procedure(name).is_some() {
            Ok(())
        } else {
            Err(CatalogError::ProcedureNotFound {
                procedure: name.to_string(),
            })
        }
    }
}

/// Mock catalog implementation for testing.
///
/// This is a simple in-memory catalog that can be used for testing
/// catalog-dependent validation.
#[derive(Debug, Clone)]
pub struct MockCatalog {
    /// Graphs in the catalog.
    pub graphs: std::collections::HashMap<String, GraphDefinition>,

    /// Schemas in the catalog.
    pub schemas: std::collections::HashMap<String, SchemaDefinition>,

    /// Procedures in the catalog.
    pub procedures: std::collections::HashMap<String, ProcedureDefinition>,
}

impl MockCatalog {
    /// Creates a new empty mock catalog.
    pub fn new() -> Self {
        Self {
            graphs: std::collections::HashMap::new(),
            schemas: std::collections::HashMap::new(),
            procedures: std::collections::HashMap::new(),
        }
    }

    /// Adds a graph to the mock catalog.
    pub fn add_graph(&mut self, name: impl Into<String>, schema: Option<String>) {
        let name = name.into();
        self.graphs.insert(
            name.clone(),
            GraphDefinition {
                name,
                schema,
                metadata: std::collections::HashMap::new(),
            },
        );
    }

    /// Adds a schema to the mock catalog.
    pub fn add_schema(&mut self, name: impl Into<String>, labels: Vec<String>) {
        let name = name.into();
        self.schemas.insert(
            name.clone(),
            SchemaDefinition {
                name,
                labels,
                metadata: std::collections::HashMap::new(),
            },
        );
    }

    /// Adds a procedure to the mock catalog.
    pub fn add_procedure(
        &mut self,
        name: impl Into<String>,
        parameters: Vec<String>,
        return_type: Option<String>,
    ) {
        let name = name.into();
        self.procedures.insert(
            name.clone(),
            ProcedureDefinition {
                name,
                parameters,
                return_type,
                metadata: std::collections::HashMap::new(),
            },
        );
    }

    /// Creates a simple test catalog with common entries.
    pub fn example() -> Self {
        let mut catalog = Self::new();

        // Add some graphs
        catalog.add_graph("social", Some("social_schema".to_string()));
        catalog.add_graph("financial", Some("financial_schema".to_string()));
        catalog.add_graph("test", None);

        // Add some schemas
        catalog.add_schema(
            "social_schema",
            vec!["Person".to_string(), "KNOWS".to_string()],
        );
        catalog.add_schema(
            "financial_schema",
            vec!["Account".to_string(), "Transaction".to_string()],
        );

        // Add some procedures
        catalog.add_procedure(
            "shortest_path",
            vec!["start".to_string(), "end".to_string()],
            Some("PATH".to_string()),
        );
        catalog.add_procedure("page_rank", vec![], Some("TABLE".to_string()));

        catalog
    }
}

impl Default for MockCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog for MockCatalog {
    fn get_graph(&self, name: &str) -> Option<&GraphDefinition> {
        self.graphs.get(name)
    }

    fn get_schema(&self, name: &str) -> Option<&SchemaDefinition> {
        self.schemas.get(name)
    }

    fn get_procedure(&self, name: &str) -> Option<&ProcedureDefinition> {
        self.procedures.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_catalog_creation() {
        let catalog = MockCatalog::new();
        assert!(catalog.graphs.is_empty());
        assert!(catalog.schemas.is_empty());
        assert!(catalog.procedures.is_empty());
    }

    #[test]
    fn test_mock_catalog_add_entries() {
        let mut catalog = MockCatalog::new();
        catalog.add_graph("mygraph", None);
        catalog.add_schema("myschema", vec![]);
        catalog.add_procedure("myproc", vec![], None);

        assert_eq!(catalog.graphs.len(), 1);
        assert_eq!(catalog.schemas.len(), 1);
        assert_eq!(catalog.procedures.len(), 1);
    }

    #[test]
    fn test_mock_catalog_example() {
        let catalog = MockCatalog::example();

        // Check graphs
        assert!(catalog.get_graph("social").is_some());
        assert!(catalog.get_graph("financial").is_some());
        assert!(catalog.get_graph("nonexistent").is_none());

        // Check schemas
        assert!(catalog.get_schema("social_schema").is_some());
        assert!(catalog.get_schema("financial_schema").is_some());
        assert!(catalog.get_schema("nonexistent").is_none());

        // Check procedures
        assert!(catalog.get_procedure("shortest_path").is_some());
        assert!(catalog.get_procedure("page_rank").is_some());
        assert!(catalog.get_procedure("nonexistent").is_none());
    }

    #[test]
    fn test_catalog_validate_graph() {
        let catalog = MockCatalog::example();

        // Valid graphs
        assert!(catalog.validate_graph("social").is_ok());
        assert!(catalog.validate_graph("financial").is_ok());

        // Invalid graph
        assert!(catalog.validate_graph("nonexistent").is_err());
    }

    #[test]
    fn test_catalog_validate_schema() {
        let catalog = MockCatalog::example();

        // Valid schemas
        assert!(catalog.validate_schema("social_schema").is_ok());
        assert!(catalog.validate_schema("financial_schema").is_ok());

        // Invalid schema
        assert!(catalog.validate_schema("nonexistent").is_err());
    }

    #[test]
    fn test_catalog_validate_procedure() {
        let catalog = MockCatalog::example();

        // Valid procedures
        assert!(catalog.validate_procedure("shortest_path").is_ok());
        assert!(catalog.validate_procedure("page_rank").is_ok());

        // Invalid procedure
        assert!(catalog.validate_procedure("nonexistent").is_err());
    }
}
