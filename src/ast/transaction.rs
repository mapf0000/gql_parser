//! Transaction statement AST nodes for Sprint 4.
//!
//! This module defines AST nodes for transaction management commands including:
//! - START TRANSACTION
//! - COMMIT [WORK]
//! - ROLLBACK [WORK]

use crate::ast::Span;

/// A transaction management command.
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionCommand {
    /// START TRANSACTION command
    Start(StartTransactionCommand),
    /// COMMIT [WORK] command
    Commit(CommitCommand),
    /// ROLLBACK [WORK] command
    Rollback(RollbackCommand),
}

/// START TRANSACTION command.
#[derive(Debug, Clone, PartialEq)]
pub struct StartTransactionCommand {
    /// Optional transaction characteristics
    pub characteristics: Option<TransactionCharacteristics>,
    pub span: Span,
}

/// Transaction characteristics.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionCharacteristics {
    /// Transaction modes
    pub modes: Vec<TransactionMode>,
    pub span: Span,
}

/// Transaction mode variants.
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionMode {
    /// READ ONLY or READ WRITE
    AccessMode(TransactionAccessMode),
    // Other transaction modes can be added in future sprints
}

/// Transaction access mode.
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionAccessMode {
    /// READ ONLY
    ReadOnly,
    /// READ WRITE
    ReadWrite,
}

/// COMMIT [WORK] command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitCommand {
    /// Whether WORK keyword was present
    pub work: bool,
    pub span: Span,
}

/// ROLLBACK [WORK] command.
#[derive(Debug, Clone, PartialEq)]
pub struct RollbackCommand {
    /// Whether WORK keyword was present
    pub work: bool,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_command_variants() {
        let start = TransactionCommand::Start(StartTransactionCommand {
            characteristics: None,
            span: 0..17,
        });
        assert!(matches!(start, TransactionCommand::Start(_)));

        let commit = TransactionCommand::Commit(CommitCommand {
            work: false,
            span: 0..6,
        });
        assert!(matches!(commit, TransactionCommand::Commit(_)));

        let rollback = TransactionCommand::Rollback(RollbackCommand {
            work: true,
            span: 0..13,
        });
        assert!(matches!(rollback, TransactionCommand::Rollback(_)));
    }

    #[test]
    fn test_transaction_access_modes() {
        let readonly = TransactionAccessMode::ReadOnly;
        let readwrite = TransactionAccessMode::ReadWrite;

        assert!(matches!(readonly, TransactionAccessMode::ReadOnly));
        assert!(matches!(readwrite, TransactionAccessMode::ReadWrite));
    }

    #[test]
    fn test_transaction_with_characteristics() {
        let chars = TransactionCharacteristics {
            modes: vec![TransactionMode::AccessMode(TransactionAccessMode::ReadOnly)],
            span: 18..28,
        };

        let cmd = TransactionCommand::Start(StartTransactionCommand {
            characteristics: Some(chars),
            span: 0..28,
        });

        if let TransactionCommand::Start(start) = cmd {
            assert!(start.characteristics.is_some());
        } else {
            panic!("Expected Start command");
        }
    }
}
