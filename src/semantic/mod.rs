//! Semantic validation for GQL AST.
//!
//! This module provides semantic validation that goes beyond syntax checking:
//! - Variable scoping and binding validation
//! - Type inference and compatibility checking
//! - Pattern connectivity validation
//! - Context validation (clause usage in appropriate contexts)
//! - Aggregation and grouping validation
//! - Metadata-dependent validation (optional):
//!   - Reference validation (USE GRAPH clauses)
//!   - Label and property validation (schema)
//!   - Callable validation (functions/procedures)
//!
//! The semantic validator takes an AST and produces either an IR (Intermediate
//! Representation) enriched with semantic information, or a list of semantic
//! diagnostics if validation fails.
//!
//! # Architecture
//!
//! The semantic validator follows a multi-pass architecture:
//!
//! 1. **Scope Analysis** - Build symbol table, track variable declarations
//! 2. **Type Inference** - Infer types for expressions
//! 3. **Variable Validation** - Detect undefined variables, shadowing
//! 4. **Pattern Validation** - Validate pattern connectivity
//! 5. **Context Validation** - Validate clause usage in appropriate contexts
//! 6. **Type Checking** - Validate type compatibility
//! 7. **Expression Validation** - Validate null handling, CASE, subqueries
//! 8. **Metadata Validation** (optional) - Validate with metadata provider
//!
//! # Example
//!
//! ```ignore
//! use gql_parser::{parse, semantic::SemanticValidator};
//!
//! let source = "MATCH (n:Person) RETURN n.name";
//! let parse_result = parse(source);
//!
//! if let Some(ast) = parse_result.ast {
//!     let validator = SemanticValidator::new();
//!     match validator.validate(&ast) {
//!         Ok(ir) => {
//!             // IR with semantic information
//!             println!("Validation successful");
//!         }
//!         Err(diagnostics) => {
//!             // Semantic errors
//!             for diag in diagnostics {
//!                 eprintln!("{:?}", diag);
//!             }
//!         }
//!     }
//! }
//! ```

pub mod callable;
pub mod diag;
pub mod metadata_provider;
pub mod schema_catalog;
pub mod type_metadata;
pub mod validator;

pub use metadata_provider::{MetadataProvider, MockMetadataProvider};
pub use validator::{SemanticValidator, ValidationConfig};
