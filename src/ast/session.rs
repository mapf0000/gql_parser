//! Session statement AST nodes.
//!
//! This module defines AST nodes for session management commands including:
//! - SESSION SET (schema, graph, time zone, parameters)
//! - SESSION RESET
//! - SESSION CLOSE

use crate::ast::references::{GraphReference, SchemaReference};
use crate::ast::{Expression, Span};
use smol_str::SmolStr;

/// A session management command.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionCommand {
    /// SESSION SET command
    Set(SessionSetCommand),
    /// SESSION RESET command
    Reset(SessionResetCommand),
    /// SESSION CLOSE command
    Close(SessionCloseCommand),
}

/// SESSION SET command variants.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSetCommand {
    /// SESSION SET SCHEMA
    Schema(SessionSetSchemaClause),
    /// SESSION SET [PROPERTY] GRAPH
    Graph(SessionSetGraphClause),
    /// SESSION SET TIME ZONE
    TimeZone(SessionSetTimeZoneClause),
    /// SESSION SET parameter
    Parameter(SessionSetParameterClause),
}

/// SESSION SET SCHEMA clause.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSetSchemaClause {
    /// Parsed schema reference
    pub schema_reference: SchemaReference,
    pub span: Span,
}

/// SESSION SET [PROPERTY] GRAPH clause.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSetGraphClause {
    /// Whether PROPERTY keyword was present
    pub property: bool,
    /// Parsed graph reference
    pub graph_reference: GraphReference,
    pub span: Span,
}

/// SESSION SET TIME ZONE clause.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSetTimeZoneClause {
    /// The time zone value (expression from Sprint 5)
    pub value: Expression,
    pub span: Span,
}

/// SESSION SET parameter variants.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSetParameterClause {
    /// Graph parameter
    GraphParameter {
        name: SmolStr,
        value: Expression,
        span: Span,
    },
    /// Binding table parameter
    BindingTableParameter {
        name: SmolStr,
        value: Expression,
        span: Span,
    },
    /// Value parameter
    ValueParameter {
        name: SmolStr,
        value: Expression,
        span: Span,
    },
}

/// SESSION RESET command.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionResetCommand {
    /// What to reset
    pub target: SessionResetTarget,
    pub span: Span,
}

/// Target for SESSION RESET command.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionResetTarget {
    /// RESET ALL
    All,
    /// RESET PARAMETERS
    Parameters,
    /// RESET CHARACTERISTICS
    Characteristics,
    /// RESET SCHEMA
    Schema,
    /// RESET GRAPH
    Graph,
    /// RESET TIME ZONE
    TimeZone,
}

/// SESSION CLOSE command.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionCloseCommand {
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_command_variants() {
        let schema_cmd = SessionCommand::Set(SessionSetCommand::Schema(SessionSetSchemaClause {
            schema_reference: SchemaReference::HomeSchema { span: 0..11 },
            span: 0..20,
        }));
        assert!(matches!(schema_cmd, SessionCommand::Set(_)));

        let reset_cmd = SessionCommand::Reset(SessionResetCommand {
            target: SessionResetTarget::All,
            span: 0..10,
        });
        assert!(matches!(reset_cmd, SessionCommand::Reset(_)));

        let close_cmd = SessionCommand::Close(SessionCloseCommand { span: 0..10 });
        assert!(matches!(close_cmd, SessionCommand::Close(_)));
    }

    #[test]
    fn test_session_reset_targets() {
        let targets = [
            SessionResetTarget::All,
            SessionResetTarget::Parameters,
            SessionResetTarget::Characteristics,
            SessionResetTarget::Schema,
            SessionResetTarget::Graph,
            SessionResetTarget::TimeZone,
        ];
        assert_eq!(targets.len(), 6);
    }
}
