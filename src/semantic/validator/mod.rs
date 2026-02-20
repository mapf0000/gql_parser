//! Main semantic validator coordinating validation passes.

mod callable_validation;
mod context_validation;
mod expression_validation;
mod pattern_validation;
mod reference_validation;
mod schema_validation;
mod scope_analysis;
mod type_checking;
mod type_inference;
mod variable_validation;

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

    /// Enable advanced schema catalog validation (Milestone 3).
    ///
    /// When enabled, the validator will accept schema catalog, graph context
    /// resolver, and variable type context provider dependencies. However,
    /// the actual validation logic that uses these dependencies is not yet
    /// implemented. This flag currently only controls whether the dependencies
    /// are stored and made available for future validation passes.
    ///
    /// See `SemanticValidator::with_schema_catalog()` for implementation status.
    pub advanced_schema_validation: bool,

    /// Enable callable validation (Milestone 4).
    ///
    /// When enabled, the validator will validate function and procedure calls
    /// against their signatures, including arity checking and parameter validation.
    /// Requires a callable catalog and validator to be configured.
    pub callable_validation: bool,

    /// Enable enhanced type inference (Milestone 5).
    ///
    /// When enabled, the validator will use type metadata catalogs for improved
    /// type inference quality, reducing Type::Any fallbacks in complex expressions.
    pub enhanced_type_inference: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            schema_validation: false,
            catalog_validation: false,
            warn_on_shadowing: true,
            warn_on_disconnected_patterns: true,
            advanced_schema_validation: false,
            callable_validation: false,
            enhanced_type_inference: false,
        }
    }
}

/// Main semantic validator coordinating all validation passes.
pub struct SemanticValidator<'s, 'c> {
    /// Validation configuration.
    pub(super) config: ValidationConfig,

    /// Optional schema for schema-dependent validation (legacy).
    pub(super) schema: Option<&'s dyn crate::semantic::schema::Schema>,

    /// Optional catalog for catalog-dependent validation (legacy).
    pub(super) catalog: Option<&'c dyn crate::semantic::catalog::Catalog>,

    /// Optional schema catalog for advanced schema validation (Milestone 3).
    pub(super) schema_catalog: Option<&'s dyn crate::semantic::schema_catalog::SchemaCatalog>,

    /// Optional graph context resolver (Milestone 3).
    pub(super) graph_context_resolver: Option<&'s dyn crate::semantic::schema_catalog::GraphContextResolver>,

    /// Optional variable type context provider (Milestone 3).
    pub(super) variable_context_provider: Option<&'s dyn crate::semantic::schema_catalog::VariableTypeContextProvider>,

    /// Optional callable catalog for function/procedure validation (Milestone 4).
    pub(super) callable_catalog: Option<&'c dyn crate::semantic::callable::CallableCatalog>,

    /// Optional callable validator for call site validation (Milestone 4).
    pub(super) callable_validator: Option<&'c dyn crate::semantic::callable::CallableValidator>,

    /// Optional type metadata catalog for enhanced type inference (Milestone 5).
    pub(super) type_metadata: Option<&'c dyn crate::semantic::type_metadata::TypeMetadataCatalog>,

    /// Optional type check context provider (Milestone 5).
    pub(super) context_provider: Option<&'c dyn crate::semantic::type_metadata::TypeCheckContextProvider>,

    /// Inference policy for type inference (Milestone 5).
    pub(super) inference_policy: crate::semantic::type_metadata::InferencePolicy,
}

impl<'s, 'c> SemanticValidator<'s, 'c> {
    /// Creates a new semantic validator with default configuration.
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            schema: None,
            catalog: None,
            schema_catalog: None,
            graph_context_resolver: None,
            variable_context_provider: None,
            callable_catalog: None,
            callable_validator: None,
            type_metadata: None,
            context_provider: None,
            inference_policy: crate::semantic::type_metadata::InferencePolicy::default(),
        }
    }

    /// Creates a new semantic validator with custom configuration.
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            schema: None,
            catalog: None,
            schema_catalog: None,
            graph_context_resolver: None,
            variable_context_provider: None,
            callable_catalog: None,
            callable_validator: None,
            type_metadata: None,
            context_provider: None,
            inference_policy: crate::semantic::type_metadata::InferencePolicy::default(),
        }
    }

    /// Sets the schema for schema-dependent validation (legacy).
    pub fn with_schema(mut self, schema: &'s dyn crate::semantic::schema::Schema) -> Self {
        self.schema = Some(schema);
        self.config.schema_validation = true;
        self
    }

    /// Sets the catalog for catalog-dependent validation (legacy).
    pub fn with_catalog(mut self, catalog: &'c dyn crate::semantic::catalog::Catalog) -> Self {
        self.catalog = Some(catalog);
        self.config.catalog_validation = true;
        self
    }

    /// Sets the schema catalog for advanced schema validation (Milestone 3).
    ///
    /// # Implementation Status
    ///
    /// The schema catalog infrastructure is fully implemented and ready for use.
    /// However, the actual validation passes that utilize the schema catalog
    /// are not yet implemented. When `advanced_schema_validation` is enabled,
    /// the validator will store the catalog reference but will not perform
    /// any additional validation beyond the standard checks.
    ///
    /// Future validation passes will include:
    /// - Property existence validation against schema
    /// - Type compatibility checking with schema metadata
    /// - Constraint enforcement (PRIMARY KEY, UNIQUE, FOREIGN KEY, etc.)
    /// - Schema-aware type inference improvements
    pub fn with_schema_catalog(mut self, catalog: &'s dyn crate::semantic::schema_catalog::SchemaCatalog) -> Self {
        self.schema_catalog = Some(catalog);
        self.config.advanced_schema_validation = true;
        self
    }

    /// Sets the graph context resolver (Milestone 3).
    ///
    /// # Implementation Status
    ///
    /// Infrastructure complete. The resolver is stored but not yet used
    /// in validation passes. Future implementation will use this to determine
    /// the active graph/schema context for validation.
    pub fn with_graph_context_resolver(mut self, resolver: &'s dyn crate::semantic::schema_catalog::GraphContextResolver) -> Self {
        self.graph_context_resolver = Some(resolver);
        self
    }

    /// Sets the variable type context provider (Milestone 3).
    ///
    /// # Implementation Status
    ///
    /// Infrastructure complete. The provider is stored but not yet used
    /// in validation passes. Future implementation will use this for
    /// enhanced type inference and scope analysis.
    pub fn with_variable_context_provider(mut self, provider: &'s dyn crate::semantic::schema_catalog::VariableTypeContextProvider) -> Self {
        self.variable_context_provider = Some(provider);
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

    /// Enables advanced schema validation (Milestone 3).
    pub fn with_advanced_schema_validation(mut self, enabled: bool) -> Self {
        self.config.advanced_schema_validation = enabled;
        self
    }

    /// Sets the callable catalog for function/procedure validation (Milestone 4).
    ///
    /// # Implementation Status
    ///
    /// The callable catalog infrastructure is fully implemented and ready for use.
    /// When `callable_validation` is enabled, the validator will:
    /// - Validate function arity (argument count)
    /// - Check aggregate function signatures
    /// - Report errors for undefined callables
    /// - Support built-in and custom callable resolution
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gql_parser::semantic::{SemanticValidator, callable::BuiltinCallableCatalog};
    ///
    /// let catalog = BuiltinCallableCatalog::new();
    /// let validator = SemanticValidator::new()
    ///     .with_callable_catalog(&catalog)
    ///     .with_callable_validation(true);
    /// ```
    pub fn with_callable_catalog(mut self, catalog: &'c dyn crate::semantic::callable::CallableCatalog) -> Self {
        self.callable_catalog = Some(catalog);
        self.config.callable_validation = true;
        self
    }

    /// Sets the callable validator for call site validation (Milestone 4).
    ///
    /// This is typically used in conjunction with `with_callable_catalog`.
    /// If not set, no call site validation will be performed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gql_parser::semantic::{
    ///     SemanticValidator,
    ///     callable::{BuiltinCallableCatalog, DefaultCallableValidator},
    /// };
    ///
    /// let catalog = BuiltinCallableCatalog::new();
    /// let validator_impl = DefaultCallableValidator::new();
    ///
    /// let validator = SemanticValidator::new()
    ///     .with_callable_catalog(&catalog)
    ///     .with_callable_validator(&validator_impl);
    /// ```
    pub fn with_callable_validator(mut self, validator: &'c dyn crate::semantic::callable::CallableValidator) -> Self {
        self.callable_validator = Some(validator);
        self
    }

    /// Enables callable validation (Milestone 4).
    ///
    /// Note: This requires both a callable catalog and validator to be configured.
    /// If either is missing, callable validation will be skipped.
    pub fn with_callable_validation(mut self, enabled: bool) -> Self {
        self.config.callable_validation = enabled;
        self
    }

    /// Sets the type metadata catalog for enhanced type inference (Milestone 5).
    ///
    /// # Implementation Status
    ///
    /// The type metadata catalog infrastructure is fully implemented. When enabled,
    /// the validator can use property type information and callable return types
    /// to improve type inference quality and reduce Type::Any fallbacks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gql_parser::semantic::{SemanticValidator, type_metadata::MockTypeMetadataCatalog};
    ///
    /// let mut catalog = MockTypeMetadataCatalog::new();
    /// // Register property types...
    ///
    /// let validator = SemanticValidator::new()
    ///     .with_type_metadata(&catalog);
    /// ```
    pub fn with_type_metadata(mut self, catalog: &'c dyn crate::semantic::type_metadata::TypeMetadataCatalog) -> Self {
        self.type_metadata = Some(catalog);
        self.config.enhanced_type_inference = true;
        self
    }

    /// Sets the type check context provider (Milestone 5).
    ///
    /// This provider supplies type contexts for statements, enabling downstream
    /// type checkers to consume inferred types uniformly.
    pub fn with_context_provider(mut self, provider: &'c dyn crate::semantic::type_metadata::TypeCheckContextProvider) -> Self {
        self.context_provider = Some(provider);
        self
    }

    /// Sets the inference policy (Milestone 5).
    ///
    /// The inference policy controls fallback behavior for type inference,
    /// such as whether to allow Type::Any and how to handle unknown callables.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gql_parser::semantic::{SemanticValidator, type_metadata::InferencePolicy};
    ///
    /// let policy = InferencePolicy::strict();
    /// let validator = SemanticValidator::new()
    ///     .with_inference_policy(policy);
    /// ```
    pub fn with_inference_policy(mut self, policy: crate::semantic::type_metadata::InferencePolicy) -> Self {
        self.inference_policy = policy;
        self
    }

    /// Enables enhanced type inference (Milestone 5).
    pub fn with_enhanced_type_inference(mut self, enabled: bool) -> Self {
        self.config.enhanced_type_inference = enabled;
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
    /// 10. Callable Validation (optional) - Check function/procedure calls
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

        // Pass 10: Callable Validation (optional, Milestone 4)
        if self.config.callable_validation {
            callable_validation::run_callable_validation(self, program, &mut diagnostics);
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
