//! AST foundation types and node structures.

mod catalog;
mod mutation;
pub mod program;
mod query;
mod session;
mod span;
mod transaction;

// Re-export span types
pub use span::{Span, Spanned};

// Re-export program structure
pub use program::{
    CatalogStatement, MutationStatement, Program, QueryStatement, SessionStatement, Statement,
    TransactionStatement,
};

// Re-export session types
pub use session::{
    ExpressionPlaceholder, GraphReferencePlaceholder, SchemaReferencePlaceholder,
    SessionCloseCommand, SessionCommand, SessionResetCommand, SessionResetTarget,
    SessionSetCommand, SessionSetGraphClause, SessionSetParameterClause, SessionSetSchemaClause,
    SessionSetTimeZoneClause,
};

// Re-export transaction types
pub use transaction::{
    CommitCommand, RollbackCommand, StartTransactionCommand, TransactionAccessMode,
    TransactionCharacteristics, TransactionCommand, TransactionMode,
};

// Re-export catalog types
pub use catalog::{
    CallCatalogModifyingProcedureStatement, CatalogStatementKind, CreateGraphStatement,
    CreateGraphTypeStatement, CreateSchemaStatement, DropGraphStatement, DropGraphTypeStatement,
    DropSchemaStatement, GraphReference, GraphTypeReference, GraphTypeSource, GraphTypeSpec,
    SchemaReference,
};
