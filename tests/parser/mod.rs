//! Parser tests
//!
//! This module contains tests for the GQL parser,
//! including pattern matching, queries, mutations, procedures,
//! aggregates, graph types, and type references.

mod patterns;
mod queries;
mod mutations;
mod procedures;
mod aggregates;
mod graph_types;
mod graph_types_comprehensive;
mod type_references;
mod case_insensitivity;

// Advanced test modules
mod expressions;
mod path_patterns_advanced;
mod modifications_advanced;
mod composite_queries;
mod schema_advanced;
mod session_transaction;
mod procedures_advanced;
