//! AST node types for GQL program structure.

use crate::ast::Span;
use crate::ast::catalog::CatalogStatementKind;
use crate::ast::session::SessionCommand;
use crate::ast::transaction::TransactionCommand;

/// Root AST node representing a complete GQL program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Top-level statement in a GQL program.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Query statement (MATCH, SELECT, etc.)
    Query(Box<QueryStatement>),
    /// Mutation statement (INSERT, DELETE, SET, REMOVE)
    Mutation(Box<MutationStatement>),
    /// Session statement (SESSION SET, SESSION CLOSE, SESSION RESET)
    Session(Box<SessionStatement>),
    /// Transaction statement (START TRANSACTION, COMMIT, ROLLBACK)
    Transaction(Box<TransactionStatement>),
    /// Catalog statement (CREATE, DROP)
    Catalog(Box<CatalogStatement>),
    /// Empty statement or recovery placeholder
    Empty(Span),
}

/// Query statement AST node (placeholder for Sprint 7).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryStatement {
    pub span: Span,
    // Body to be implemented in Sprint 7
}

/// Mutation statement AST node (placeholder for Sprint 10).
#[derive(Debug, Clone, PartialEq)]
pub struct MutationStatement {
    pub span: Span,
    // Body to be implemented in Sprint 10
}

/// Session statement AST node (Sprint 4 - implemented).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionStatement {
    pub command: SessionCommand,
    pub span: Span,
}

/// Transaction statement AST node (Sprint 4 - implemented).
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionStatement {
    pub command: TransactionCommand,
    pub span: Span,
}

/// Catalog statement AST node (Sprint 4 - implemented).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogStatement {
    pub kind: CatalogStatementKind,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::catalog::{CallCatalogModifyingProcedureStatement, CatalogStatementKind};
    use crate::ast::session::{SessionCloseCommand, SessionCommand};
    use crate::ast::transaction::{CommitCommand, TransactionCommand};

    #[test]
    fn test_program_construction() {
        let program = Program {
            statements: vec![],
            span: 0..0,
        };
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn test_statement_types() {
        let query = Statement::Query(Box::new(QueryStatement { span: 0..5 }));
        assert!(matches!(query, Statement::Query(_)));

        let mutation = Statement::Mutation(Box::new(MutationStatement { span: 0..5 }));
        assert!(matches!(mutation, Statement::Mutation(_)));

        let session = Statement::Session(Box::new(SessionStatement {
            command: SessionCommand::Close(SessionCloseCommand { span: 0..5 }),
            span: 0..5,
        }));
        assert!(matches!(session, Statement::Session(_)));

        let transaction = Statement::Transaction(Box::new(TransactionStatement {
            command: TransactionCommand::Commit(CommitCommand {
                work: false,
                span: 0..5,
            }),
            span: 0..5,
        }));
        assert!(matches!(transaction, Statement::Transaction(_)));

        let catalog = Statement::Catalog(Box::new(CatalogStatement {
            kind: CatalogStatementKind::CallCatalogModifyingProcedure(
                CallCatalogModifyingProcedureStatement {
                    procedure_name: "test".into(),
                    span: 0..5,
                },
            ),
            span: 0..5,
        }));
        assert!(matches!(catalog, Statement::Catalog(_)));

        let empty = Statement::Empty(0..0);
        assert!(matches!(empty, Statement::Empty(_)));
    }
}
