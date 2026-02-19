//! Schema trait for optional schema-dependent validation.
//!
//! This module defines the Schema trait that allows the semantic validator
//! to validate labels and properties against an optional schema definition.

/// Result type for schema lookups that may fail.
pub type SchemaResult<T> = Result<T, SchemaError>;

/// Error type for schema validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A label was not found in the schema.
    LabelNotFound { label: String },

    /// A property was not found for a given label.
    PropertyNotFound {
        label: Option<String>,
        property: String,
    },

    /// A property has an incompatible type.
    PropertyTypeMismatch {
        property: String,
        expected_type: String,
        actual_type: String,
    },

    /// The schema is unavailable or not configured.
    SchemaUnavailable,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::LabelNotFound { label } => {
                write!(f, "Label '{}' not found in schema", label)
            }
            SchemaError::PropertyNotFound { label, property } => {
                if let Some(lbl) = label {
                    write!(f, "Property '{}' not found for label '{}'", property, lbl)
                } else {
                    write!(f, "Property '{}' not found", property)
                }
            }
            SchemaError::PropertyTypeMismatch {
                property,
                expected_type,
                actual_type,
            } => {
                write!(
                    f,
                    "Property '{}' has type '{}' but expected '{}'",
                    property, actual_type, expected_type
                )
            }
            SchemaError::SchemaUnavailable => {
                write!(f, "Schema is not available")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

/// Represents a property definition in the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDefinition {
    /// The name of the property.
    pub name: String,

    /// The expected type of the property (e.g., "STRING", "INTEGER", "BOOLEAN").
    pub property_type: String,

    /// Whether the property is required (non-nullable).
    pub required: bool,
}

/// Represents a label definition in the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelDefinition {
    /// The name of the label.
    pub name: String,

    /// Whether this label applies to nodes.
    pub is_node_label: bool,

    /// Whether this label applies to edges.
    pub is_edge_label: bool,

    /// Properties defined for this label.
    pub properties: Vec<PropertyDefinition>,
}

/// Trait for schema access, allowing validation of labels and properties.
///
/// Implement this trait to provide schema information to the semantic validator.
/// The validator can then check that:
/// - Labels used in patterns exist in the schema
/// - Properties accessed on labeled elements exist in the schema
/// - Property types match the schema definitions
///
/// # Example
///
/// ```ignore
/// use gql_parser::semantic::schema::{Schema, LabelDefinition, PropertyDefinition};
///
/// struct MySchema {
///     labels: HashMap<String, LabelDefinition>,
/// }
///
/// impl Schema for MySchema {
///     fn get_node_label(&self, name: &str) -> Option<&LabelDefinition> {
///         self.labels.get(name).filter(|l| l.is_node_label)
///     }
///
///     fn get_edge_label(&self, name: &str) -> Option<&LabelDefinition> {
///         self.labels.get(name).filter(|l| l.is_edge_label)
///     }
///
///     fn get_property(&self, label: Option<&str>, property: &str) -> Option<&PropertyDefinition> {
///         if let Some(lbl) = label {
///             self.labels.get(lbl)?.properties.iter().find(|p| p.name == property)
///         } else {
///             // Search across all labels for the property
///             None
///         }
///     }
/// }
/// ```
pub trait Schema {
    /// Looks up a node label by name.
    ///
    /// Returns `Some` if the label exists and is a valid node label.
    fn get_node_label(&self, name: &str) -> Option<&LabelDefinition>;

    /// Looks up an edge label by name.
    ///
    /// Returns `Some` if the label exists and is a valid edge label.
    fn get_edge_label(&self, name: &str) -> Option<&LabelDefinition>;

    /// Looks up a property definition.
    ///
    /// - `label`: Optional label context (e.g., "Person" for `n:Person`)
    /// - `property`: The property name to look up
    ///
    /// Returns `Some` if the property is defined for the given label,
    /// or for any label if no label context is provided.
    fn get_property(&self, label: Option<&str>, property: &str) -> Option<&PropertyDefinition>;

    /// Validates that a label exists in the schema.
    ///
    /// Returns `Ok(())` if the label exists, or an error if not.
    fn validate_label(&self, name: &str, is_node: bool) -> SchemaResult<()> {
        let exists = if is_node {
            self.get_node_label(name).is_some()
        } else {
            self.get_edge_label(name).is_some()
        };

        if exists {
            Ok(())
        } else {
            Err(SchemaError::LabelNotFound {
                label: name.to_string(),
            })
        }
    }

    /// Validates that a property exists in the schema.
    ///
    /// Returns `Ok(())` if the property exists for the given label context,
    /// or an error if not.
    fn validate_property(&self, label: Option<&str>, property: &str) -> SchemaResult<()> {
        if self.get_property(label, property).is_some() {
            Ok(())
        } else {
            Err(SchemaError::PropertyNotFound {
                label: label.map(|s| s.to_string()),
                property: property.to_string(),
            })
        }
    }
}

/// Mock schema implementation for testing.
///
/// This is a simple in-memory schema that can be used for testing
/// schema-dependent validation.
#[derive(Debug, Clone)]
pub struct MockSchema {
    /// Node labels in the schema.
    pub node_labels: Vec<LabelDefinition>,

    /// Edge labels in the schema.
    pub edge_labels: Vec<LabelDefinition>,
}

impl MockSchema {
    /// Creates a new empty mock schema.
    pub fn new() -> Self {
        Self {
            node_labels: Vec::new(),
            edge_labels: Vec::new(),
        }
    }

    /// Adds a node label to the mock schema.
    pub fn add_node_label(&mut self, name: impl Into<String>, properties: Vec<PropertyDefinition>) {
        self.node_labels.push(LabelDefinition {
            name: name.into(),
            is_node_label: true,
            is_edge_label: false,
            properties,
        });
    }

    /// Adds an edge label to the mock schema.
    pub fn add_edge_label(&mut self, name: impl Into<String>, properties: Vec<PropertyDefinition>) {
        self.edge_labels.push(LabelDefinition {
            name: name.into(),
            is_node_label: false,
            is_edge_label: true,
            properties,
        });
    }

    /// Creates a simple test schema with common labels.
    pub fn example() -> Self {
        let mut schema = Self::new();

        // Add Person node label with properties
        schema.add_node_label(
            "Person",
            vec![
                PropertyDefinition {
                    name: "name".to_string(),
                    property_type: "STRING".to_string(),
                    required: true,
                },
                PropertyDefinition {
                    name: "age".to_string(),
                    property_type: "INTEGER".to_string(),
                    required: false,
                },
                PropertyDefinition {
                    name: "email".to_string(),
                    property_type: "STRING".to_string(),
                    required: false,
                },
            ],
        );

        // Add Company node label with properties
        schema.add_node_label(
            "Company",
            vec![
                PropertyDefinition {
                    name: "name".to_string(),
                    property_type: "STRING".to_string(),
                    required: true,
                },
                PropertyDefinition {
                    name: "founded".to_string(),
                    property_type: "INTEGER".to_string(),
                    required: false,
                },
            ],
        );

        // Add KNOWS edge label
        schema.add_edge_label(
            "KNOWS",
            vec![PropertyDefinition {
                name: "since".to_string(),
                property_type: "INTEGER".to_string(),
                required: false,
            }],
        );

        // Add WORKS_AT edge label
        schema.add_edge_label(
            "WORKS_AT",
            vec![
                PropertyDefinition {
                    name: "since".to_string(),
                    property_type: "INTEGER".to_string(),
                    required: false,
                },
                PropertyDefinition {
                    name: "position".to_string(),
                    property_type: "STRING".to_string(),
                    required: false,
                },
            ],
        );

        schema
    }
}

impl Default for MockSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema for MockSchema {
    fn get_node_label(&self, name: &str) -> Option<&LabelDefinition> {
        self.node_labels.iter().find(|l| l.name == name)
    }

    fn get_edge_label(&self, name: &str) -> Option<&LabelDefinition> {
        self.edge_labels.iter().find(|l| l.name == name)
    }

    fn get_property(&self, label: Option<&str>, property: &str) -> Option<&PropertyDefinition> {
        if let Some(lbl) = label {
            // Search in node labels first
            if let Some(node_label) = self.get_node_label(lbl)
                && let Some(prop) = node_label.properties.iter().find(|p| p.name == property)
            {
                return Some(prop);
            }

            // Then search in edge labels
            if let Some(edge_label) = self.get_edge_label(lbl)
                && let Some(prop) = edge_label.properties.iter().find(|p| p.name == property)
            {
                return Some(prop);
            }

            None
        } else {
            // Without label context, search across all labels
            for node_label in &self.node_labels {
                if let Some(prop) = node_label.properties.iter().find(|p| p.name == property) {
                    return Some(prop);
                }
            }

            for edge_label in &self.edge_labels {
                if let Some(prop) = edge_label.properties.iter().find(|p| p.name == property) {
                    return Some(prop);
                }
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_schema_creation() {
        let schema = MockSchema::new();
        assert!(schema.node_labels.is_empty());
        assert!(schema.edge_labels.is_empty());
    }

    #[test]
    fn test_mock_schema_add_labels() {
        let mut schema = MockSchema::new();
        schema.add_node_label("Person", vec![]);
        schema.add_edge_label("KNOWS", vec![]);

        assert_eq!(schema.node_labels.len(), 1);
        assert_eq!(schema.edge_labels.len(), 1);
    }

    #[test]
    fn test_mock_schema_example() {
        let schema = MockSchema::example();

        // Check node labels
        assert!(schema.get_node_label("Person").is_some());
        assert!(schema.get_node_label("Company").is_some());
        assert!(schema.get_node_label("NonExistent").is_none());

        // Check edge labels
        assert!(schema.get_edge_label("KNOWS").is_some());
        assert!(schema.get_edge_label("WORKS_AT").is_some());
        assert!(schema.get_edge_label("NonExistent").is_none());
    }

    #[test]
    fn test_schema_property_lookup() {
        let schema = MockSchema::example();

        // Property lookup with label context
        assert!(schema.get_property(Some("Person"), "name").is_some());
        assert!(schema.get_property(Some("Person"), "age").is_some());
        assert!(schema.get_property(Some("Person"), "nonexistent").is_none());

        // Property lookup without label context
        assert!(schema.get_property(None, "name").is_some());
        assert!(schema.get_property(None, "since").is_some());
    }

    #[test]
    fn test_schema_validate_label() {
        let schema = MockSchema::example();

        // Valid labels
        assert!(schema.validate_label("Person", true).is_ok());
        assert!(schema.validate_label("KNOWS", false).is_ok());

        // Invalid labels
        assert!(schema.validate_label("NonExistent", true).is_err());
        assert!(schema.validate_label("NonExistent", false).is_err());
    }

    #[test]
    fn test_schema_validate_property() {
        let schema = MockSchema::example();

        // Valid properties
        assert!(schema.validate_property(Some("Person"), "name").is_ok());
        assert!(schema.validate_property(Some("Person"), "age").is_ok());
        assert!(schema.validate_property(None, "name").is_ok());

        // Invalid properties
        assert!(
            schema
                .validate_property(Some("Person"), "nonexistent")
                .is_err()
        );
    }
}
