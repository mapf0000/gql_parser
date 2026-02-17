//! AST foundation types and node structures.

mod catalog;
mod mutation;
pub mod program;
mod query;
mod session;
mod span;

// Re-export span types
pub use span::{Span, Spanned};

// Re-export program structure
pub use program::{
    CatalogStatement, MutationStatement, Program, QueryStatement, SessionStatement, Statement,
    TransactionStatement,
};
