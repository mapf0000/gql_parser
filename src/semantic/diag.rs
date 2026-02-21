//! Semantic diagnostics extending the base diagnostic system.
//!
//! This module provides specialized diagnostic types for semantic validation errors.

use crate::ast::Span;
use crate::diag::Diag;

/// Categories of semantic errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticDiagKind {
    /// Undefined variable reference.
    UndefinedVariable,

    /// Type mismatch in operation.
    TypeMismatch,

    /// Disconnected pattern detected.
    DisconnectedPattern,

    /// Clause used in inappropriate context.
    ContextViolation,

    /// Aggregation function usage error.
    AggregationError,

    /// Unknown reference (schema, graph, procedure, type).
    UnknownReference,

    /// Variable scope violation.
    ScopeViolation,

    /// Variable shadowing detected.
    VariableShadowing,

    /// Invalid property access.
    InvalidPropertyAccess,

    /// Invalid null handling.
    InvalidNullHandling,

    /// CASE expression type inconsistency.
    CaseTypeInconsistency,

    /// Subquery result type error.
    SubqueryTypeError,

    /// List operation error.
    ListOperationError,

    /// Pattern validation error.
    PatternValidationError,

    /// Grouping/aggregation mixing error.
    GroupingAggregationMixing,

    /// Invalid function call.
    InvalidFunctionCall,

    /// General semantic error (fallback).
    SemanticError,
}

impl SemanticDiagKind {
    /// Returns a human-readable name for this diagnostic kind.
    pub fn name(self) -> &'static str {
        match self {
            Self::UndefinedVariable => "UndefinedVariable",
            Self::TypeMismatch => "TypeMismatch",
            Self::DisconnectedPattern => "DisconnectedPattern",
            Self::ContextViolation => "ContextViolation",
            Self::AggregationError => "AggregationError",
            Self::UnknownReference => "UnknownReference",
            Self::ScopeViolation => "ScopeViolation",
            Self::VariableShadowing => "VariableShadowing",
            Self::InvalidPropertyAccess => "InvalidPropertyAccess",
            Self::InvalidNullHandling => "InvalidNullHandling",
            Self::CaseTypeInconsistency => "CaseTypeInconsistency",
            Self::SubqueryTypeError => "SubqueryTypeError",
            Self::ListOperationError => "ListOperationError",
            Self::PatternValidationError => "PatternValidationError",
            Self::GroupingAggregationMixing => "GroupingAggregationMixing",
            Self::InvalidFunctionCall => "InvalidFunctionCall",
            Self::SemanticError => "SemanticError",
        }
    }
}

/// Helper functions for creating common semantic diagnostics.
///
/// These functions return `Diag` directly, avoiding the need for a separate builder layer.

/// Creates an undefined variable diagnostic.
pub fn undefined_variable(var_name: &str, span: Span) -> Diag {
    Diag::error(format!("Undefined variable '{}'", var_name))
        .with_primary_label(span, "variable not defined")
}

/// Creates a type mismatch diagnostic.
pub fn type_mismatch(expected: &str, found: &str, span: Span) -> Diag {
    Diag::error(format!(
        "Type mismatch: expected {}, found {}",
        expected, found
    ))
    .with_primary_label(span, format!("expected {}, found {}", expected, found))
}

/// Creates a disconnected pattern diagnostic.
///
/// Note: Disconnected patterns (comma-separated path patterns) are ISO-conformant.
/// This diagnostic is optional and emits a warning (not an error) to inform users
/// about potential connectivity issues.
pub fn disconnected_pattern(span: Span) -> Diag {
    Diag::warning("Disconnected pattern detected (ISO-conformant but may be unintentional)")
        .with_primary_label(span, "pattern is not connected to the rest of the graph")
        .with_note("Disconnected comma-separated patterns are ISO-conformant. However, if this is unintentional, consider adding an edge connecting the patterns or using separate MATCH clauses.")
}

/// Creates a context violation diagnostic.
pub fn context_violation(clause: &str, context: &str, span: Span) -> Diag {
    Diag::error(format!(
        "{} clause cannot be used in {} context",
        clause, context
    ))
    .with_primary_label(span, format!("{} not allowed here", clause))
}

/// Creates an aggregation error diagnostic.
pub fn aggregation_error(message: impl Into<String>, span: Span) -> Diag {
    Diag::error(message).with_primary_label(span, "aggregation error")
}

/// Creates an unknown reference diagnostic.
pub fn unknown_reference(ref_kind: &str, ref_name: &str, span: Span) -> Diag {
    Diag::error(format!("Unknown {} '{}'", ref_kind, ref_name))
        .with_primary_label(span, format!("{} not found", ref_kind))
}

/// Creates a scope violation diagnostic.
pub fn scope_violation(var_name: &str, span: Span) -> Diag {
    Diag::error(format!(
        "Variable '{}' is not visible in this scope",
        var_name
    ))
    .with_primary_label(span, "not visible in this scope")
}

/// Creates a variable shadowing diagnostic.
pub fn variable_shadowing(var_name: &str, span: Span, original_span: Span) -> Diag {
    Diag::warning(format!(
        "Variable '{}' shadows a previous declaration",
        var_name
    ))
    .with_primary_label(span, "shadows previous declaration")
    .with_secondary_label(original_span, "originally declared here")
}
