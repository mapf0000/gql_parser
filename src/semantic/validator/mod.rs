//! Main semantic validator coordinating validation passes.

mod scope_analysis;
mod type_inference;
mod variable_validation;
mod pattern_validation;
mod context_validation;
mod type_checking;
mod expression_validation;
mod reference_validation;
mod schema_validation;

use std::collections::HashMap;

use crate::ast::program::Program;
use crate::diag::DiagSeverity;
use crate::ir::symbol_table::ScopeId;
use crate::ir::{IR, ValidationOutcome};

/// Tracks the scope context where an expression is evaluated.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) struct ExpressionContext {
    /// Scope ID where the expression is evaluated.
    pub(super) scope_id: ScopeId,

    /// Statement ID for statement isolation (variables don't leak across statements).
    pub(super) statement_id: usize,
}

/// Metadata collected during scope analysis for reference-site-aware lookups.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct ScopeMetadata {
    /// Maps expression spans to their evaluation context.
    pub(super) expr_contexts: HashMap<(usize, usize), ExpressionContext>,

    /// Maps statement indices to their root scope IDs.
    pub(super) statement_scopes: Vec<ScopeId>,
}

/// Configuration for semantic validation.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable strict mode (more stringent validation).
    pub strict_mode: bool,

    /// Enable schema-dependent validation.
    pub schema_validation: bool,

    /// Enable catalog-dependent validation.
    pub catalog_validation: bool,

    /// Enable variable shadowing warnings.
    pub warn_on_shadowing: bool,

    /// Enable disconnected pattern warnings.
    pub warn_on_disconnected_patterns: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            schema_validation: false,
            catalog_validation: false,
            warn_on_shadowing: true,
            warn_on_disconnected_patterns: true,
        }
    }
}

/// Main semantic validator coordinating all validation passes.
pub struct SemanticValidator<'s, 'c> {
    /// Validation configuration.
    pub(super) config: ValidationConfig,

    /// Optional schema for schema-dependent validation.
    pub(super) schema: Option<&'s dyn crate::semantic::schema::Schema>,

    /// Optional catalog for catalog-dependent validation.
    pub(super) catalog: Option<&'c dyn crate::semantic::catalog::Catalog>,
}

impl<'s, 'c> SemanticValidator<'s, 'c> {
    /// Creates a new semantic validator with default configuration.
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            schema: None,
            catalog: None,
        }
    }

    /// Creates a new semantic validator with custom configuration.
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            schema: None,
            catalog: None,
        }
    }

    /// Sets the schema for schema-dependent validation.
    pub fn with_schema(mut self, schema: &'s dyn crate::semantic::schema::Schema) -> Self {
        self.schema = Some(schema);
        self.config.schema_validation = true;
        self
    }

    /// Sets the catalog for catalog-dependent validation.
    pub fn with_catalog(mut self, catalog: &'c dyn crate::semantic::catalog::Catalog) -> Self {
        self.catalog = Some(catalog);
        self.config.catalog_validation = true;
        self
    }

    /// Sets strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.config.strict_mode = strict;
        self
    }

    /// Enables schema-dependent validation.
    pub fn with_schema_validation(mut self, enabled: bool) -> Self {
        self.config.schema_validation = enabled;
        self
    }

    /// Enables catalog-dependent validation.
    pub fn with_catalog_validation(mut self, enabled: bool) -> Self {
        self.config.catalog_validation = enabled;
        self
    }

    /// Validates an AST and produces an IR or diagnostics.
    ///
    /// # Multi-Pass Validation
    ///
    /// The validator runs multiple passes in sequence:
    /// 1. Scope Analysis - Build symbol table
    /// 2. Type Inference - Infer expression types
    /// 3. Variable Validation - Check undefined variables
    /// 4. Pattern Validation - Check pattern connectivity
    /// 5. Context Validation - Check clause usage
    /// 6. Type Checking - Check type compatibility
    /// 7. Expression Validation - Check expressions
    /// 8. Reference Validation (optional) - Check references
    /// 9. Label/Property Validation (optional) - Check schema references
    ///
    /// # Error Recovery
    ///
    /// Validation continues after errors to report multiple issues.
    /// Returns `ValidationOutcome` which always includes diagnostics and
    /// optionally includes IR if no errors occurred (warnings are allowed).
    pub fn validate(&self, program: &Program) -> ValidationOutcome {
        let mut diagnostics = Vec::new();

        // Pass 1: Scope Analysis - Builds symbol table and tracks expression contexts
        let (symbol_table, scope_metadata) =
            scope_analysis::run_scope_analysis(self, program, &mut diagnostics);

        // Pass 2: Type Inference
        let type_table =
            type_inference::run_type_inference(self, program, &symbol_table, &mut diagnostics);

        // Pass 3: Variable Validation - Now uses scope metadata for reference-site-aware lookups
        variable_validation::run_variable_validation(
            self,
            program,
            &symbol_table,
            &scope_metadata,
            &mut diagnostics,
        );

        // Pass 4: Pattern Validation
        pattern_validation::run_pattern_validation(self, program, &mut diagnostics);

        // Pass 5: Context Validation
        context_validation::run_context_validation(self, program, &mut diagnostics);

        // Pass 6: Type Checking
        type_checking::run_type_checking(self, program, &type_table, &mut diagnostics);

        // Pass 7: Expression Validation
        expression_validation::run_expression_validation(self, program, &type_table, &mut diagnostics);

        // Pass 8: Reference Validation (optional)
        if self.config.catalog_validation {
            reference_validation::run_reference_validation(self, program, &mut diagnostics);
        }

        // Pass 9: Label/Property Validation (optional)
        if self.config.schema_validation {
            schema_validation::run_schema_validation(self, program, &mut diagnostics);
        }

        // Return IR or diagnostics
        // Only fail validation if there are errors (not warnings or notes)
        let has_errors = diagnostics
            .iter()
            .any(|d| d.severity == DiagSeverity::Error);

        if has_errors {
            ValidationOutcome::failure(diagnostics)
        } else {
            // Warnings don't prevent IR creation - return both IR and warnings
            let ir = IR::new(program.clone(), symbol_table, type_table);
            ValidationOutcome::success(ir, diagnostics)
        }
    }
}

impl<'s, 'c> Default for SemanticValidator<'s, 'c> {
    fn default() -> Self {
        Self::new()
    }
}
