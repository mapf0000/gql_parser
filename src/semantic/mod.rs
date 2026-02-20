//! Semantic validation for GQL AST.
//!
//! This module provides semantic validation that goes beyond syntax checking:
//! - Variable scoping and binding validation
//! - Type inference and compatibility checking
//! - Pattern connectivity validation
//! - Context validation (clause usage in appropriate contexts)
//! - Aggregation and grouping validation
//! - Reference validation (optional, catalog-dependent)
//! - Label and property validation (optional, schema-dependent)
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
//! 8. **Reference Validation** (optional) - Validate references with catalog
//! 9. **Label/Property Validation** (optional) - Validate with schema
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
pub mod catalog;
pub mod diag;
pub mod schema;
pub mod schema_catalog;
pub mod validator;

pub use validator::{SemanticValidator, ValidationConfig};
