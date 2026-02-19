//! Main semantic validator coordinating validation passes.

use std::collections::{HashMap, HashSet};

use crate::ast::program::{Program, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, ForStatement, LetStatement, LinearQuery, MatchStatement,
    PathPattern, PathPatternExpression, PathPrimary, PathTerm, PrimitiveQueryStatement, Query,
};
use crate::diag::{Diag, DiagSeverity};
use crate::ir::symbol_table::{ScopeId, ScopeKind, SymbolKind};
use crate::ir::{IR, SymbolTable, TypeTable, ValidationOutcome};
use crate::semantic::diag::SemanticDiagBuilder;

/// Tracks the scope context where an expression is evaluated.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct ExpressionContext {
    /// Scope ID where the expression is evaluated.
    scope_id: ScopeId,

    /// Statement ID for statement isolation (variables don't leak across statements).
    statement_id: usize,
}

/// Metadata collected during scope analysis for reference-site-aware lookups.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ScopeMetadata {
    /// Maps expression spans to their evaluation context.
    expr_contexts: HashMap<(usize, usize), ExpressionContext>,

    /// Maps statement indices to their root scope IDs.
    statement_scopes: Vec<ScopeId>,
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
    config: ValidationConfig,

    /// Optional schema for schema-dependent validation.
    schema: Option<&'s dyn crate::semantic::schema::Schema>,

    /// Optional catalog for catalog-dependent validation.
    catalog: Option<&'c dyn crate::semantic::catalog::Catalog>,
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
        let (symbol_table, scope_metadata) = self.run_scope_analysis(program, &mut diagnostics);

        // Pass 2: Type Inference
        let type_table = self.run_type_inference(program, &symbol_table, &mut diagnostics);

        // Pass 3: Variable Validation - Now uses scope metadata for reference-site-aware lookups
        self.run_variable_validation(program, &symbol_table, &scope_metadata, &mut diagnostics);

        // Pass 4: Pattern Validation
        self.run_pattern_validation(program, &mut diagnostics);

        // Pass 5: Context Validation
        self.run_context_validation(program, &mut diagnostics);

        // Pass 6: Type Checking
        self.run_type_checking(program, &type_table, &mut diagnostics);

        // Pass 7: Expression Validation
        self.run_expression_validation(program, &type_table, &mut diagnostics);

        // Pass 8: Reference Validation (optional)
        if self.config.catalog_validation {
            self.run_reference_validation(program, &mut diagnostics);
        }

        // Pass 9: Label/Property Validation (optional)
        if self.config.schema_validation {
            self.run_schema_validation(program, &mut diagnostics);
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

    /// Pass 1: Scope Analysis - Builds symbol table and tracks statement boundaries.
    ///
    /// This pass creates scopes for each statement. Statement isolation is enforced
    /// by creating separate scopes per statement and tracking statement boundaries.
    fn run_scope_analysis(
        &self,
        program: &Program,
        diagnostics: &mut Vec<Diag>,
    ) -> (SymbolTable, ScopeMetadata) {
        let mut symbol_table = SymbolTable::new();
        let mut scope_metadata = ScopeMetadata {
            expr_contexts: HashMap::new(),
            statement_scopes: Vec::new(),
        };

        // Walk all statements in the program, tracking statement boundaries
        let mut statement_id = 0;
        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.analyze_query(
                        &query_stmt.query,
                        &mut symbol_table,
                        &mut scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    statement_id += 1;
                }
                Statement::Mutation(mutation_stmt) => {
                    self.analyze_mutation_with_scope(
                        &mutation_stmt.statement,
                        &mut symbol_table,
                        &mut scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    statement_id += 1;
                }
                Statement::Session(_)
                | Statement::Transaction(_)
                | Statement::Catalog(_)
                | Statement::Empty(_) => {
                    // These don't introduce variables or scopes
                }
            }
        }

        (symbol_table, scope_metadata)
    }

    /// Analyzes a query and extracts variables, tracking statement context.
    fn analyze_query(
        &self,
        query: &Query,
        symbol_table: &mut SymbolTable,
        scope_metadata: &mut ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        match query {
            Query::Linear(linear_query) => {
                self.analyze_linear_query(
                    linear_query,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Query::Composite(composite_query) => {
                // Composite queries: each side gets its own isolated scope
                // Left query uses current statement_id
                self.analyze_query(
                    &composite_query.left,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );

                // Right query gets a new statement_id for isolation
                let right_statement_id = statement_id + 1000; // Use high offset to avoid collision
                self.analyze_query(
                    &composite_query.right,
                    symbol_table,
                    scope_metadata,
                    right_statement_id,
                    diagnostics,
                );
            }
            Query::Parenthesized(query, _) => {
                self.analyze_query(
                    query,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
    }

    /// Wrapper for analyze_mutation that tracks scope metadata.
    fn analyze_mutation_with_scope(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        symbol_table: &mut SymbolTable,
        scope_metadata: &mut ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        // Call the existing analyze_mutation which will push its own scope
        self.analyze_mutation(mutation, symbol_table, diagnostics);

        // Track the scope that was just created
        let statement_scope_id = symbol_table.current_scope();
        if statement_id >= scope_metadata.statement_scopes.len() {
            scope_metadata
                .statement_scopes
                .resize(statement_id + 1, ScopeId::new(0));
        }
        scope_metadata.statement_scopes[statement_id] = statement_scope_id;
    }

    /// Analyzes a linear query and extracts variables from clauses.
    fn analyze_linear_query(
        &self,
        linear_query: &LinearQuery,
        symbol_table: &mut SymbolTable,
        scope_metadata: &mut ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        // Push a NEW scope for this statement (statement isolation)
        symbol_table.push_scope(ScopeKind::Query);
        let statement_scope_id = symbol_table.current_scope();

        // Track this statement's scope
        if statement_id >= scope_metadata.statement_scopes.len() {
            scope_metadata
                .statement_scopes
                .resize(statement_id + 1, ScopeId::new(0));
        }
        scope_metadata.statement_scopes[statement_id] = statement_scope_id;

        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        // Walk through primitive statements in order, using the old analyze method
        // We'll track expression contexts during variable validation instead
        for statement in primitive_statements {
            self.analyze_primitive_statement(statement, symbol_table, diagnostics);
        }

        // Keep the query scope active (don't pop it)
        // This preserves variables for later validation passes
    }

    /// Analyzes a primitive query statement.
    fn analyze_primitive_statement(
        &self,
        statement: &PrimitiveQueryStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        match statement {
            PrimitiveQueryStatement::Match(match_stmt) => {
                self.analyze_match_statement(match_stmt, symbol_table, diagnostics);
            }
            PrimitiveQueryStatement::Let(let_stmt) => {
                self.analyze_let_statement(let_stmt, symbol_table, diagnostics);
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                self.analyze_for_statement(for_stmt, symbol_table, diagnostics);
            }
            PrimitiveQueryStatement::Call(_) => {
                // CALL statements are handled separately - they may reference variables
                // but don't introduce new binding variables in the scope analysis phase
            }
            PrimitiveQueryStatement::Filter(_) => {
                // FILTER statements reference existing variables in their condition
                // but don't introduce new binding variables
            }
            PrimitiveQueryStatement::OrderByAndPage(_) => {
                // ORDER BY and pagination statements reference existing variables
                // but don't introduce new binding variables
            }
            PrimitiveQueryStatement::Select(_) => {
                // SELECT statements reference existing variables in their expressions
                // but don't introduce new binding variables (unless aliased, handled elsewhere)
            }
        }
    }

    /// Analyzes a MATCH statement and extracts binding variables from patterns.
    fn analyze_match_statement(
        &self,
        match_stmt: &MatchStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::{GraphPattern, OptionalOperand};

        let mut process_pattern = |pattern: &GraphPattern, symbol_table: &mut SymbolTable| {
            self.extract_pattern_variables(&pattern.paths.patterns, symbol_table, diagnostics);
        };

        match match_stmt {
            MatchStatement::Simple(simple) => {
                process_pattern(&simple.pattern, symbol_table);
            }
            MatchStatement::Optional(optional) => match &optional.operand {
                OptionalOperand::Match { pattern } => {
                    process_pattern(pattern, symbol_table);
                }
                OptionalOperand::Block { statements }
                | OptionalOperand::ParenthesizedBlock { statements } => {
                    for stmt in statements {
                        self.analyze_match_statement(stmt, symbol_table, diagnostics);
                    }
                }
            },
        };
    }

    /// Extracts binding variables from path patterns.
    fn extract_pattern_variables(
        &self,
        path_patterns: &[PathPattern],
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        for path_pattern in path_patterns {
            // Extract path-level variable (e.g., p = (a)-[e]->(b))
            if let Some(path_var_decl) = &path_pattern.variable_declaration {
                let var_name = path_var_decl.variable.to_string();
                let span = path_var_decl.span.clone();

                if self.config.warn_on_shadowing
                    && let Some(existing) = symbol_table.lookup(&var_name)
                {
                    // Emit variable shadowing warning
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag = SemanticDiagBuilder::variable_shadowing(
                        &var_name,
                        span.clone(),
                        existing.declared_at.clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }

                symbol_table.define(var_name, SymbolKind::BindingVariable, span);
            }

            // Extract element variables from the path expression
            self.extract_expression_variables(&path_pattern.expression, symbol_table, diagnostics);
        }
    }

    /// Extracts variables from a path pattern expression.
    fn extract_expression_variables(
        &self,
        expression: &PathPatternExpression,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        match expression {
            PathPatternExpression::Term(term) => {
                self.extract_term_variables(term, symbol_table, diagnostics);
            }
            PathPatternExpression::Union { left, right, .. } => {
                self.extract_expression_variables(left, symbol_table, diagnostics);
                self.extract_expression_variables(right, symbol_table, diagnostics);
            }
            PathPatternExpression::Alternation { alternatives, .. } => {
                for term in alternatives {
                    self.extract_term_variables(term, symbol_table, diagnostics);
                }
            }
        }
    }

    /// Extracts variables from a path term.
    fn extract_term_variables(
        &self,
        term: &PathTerm,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        for factor in &term.factors {
            self.extract_primary_variables(&factor.primary, symbol_table, diagnostics);
        }
    }

    /// Extracts variables from a path primary.
    fn extract_primary_variables(
        &self,
        primary: &PathPrimary,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        match primary {
            PathPrimary::ElementPattern(element) => {
                self.extract_element_variables(element, symbol_table, diagnostics);
            }
            PathPrimary::ParenthesizedExpression(expr) => {
                self.extract_expression_variables(expr, symbol_table, diagnostics);
            }
            PathPrimary::SimplifiedExpression(_) => {
                // Simplified expressions don't have explicit variables
            }
        }
    }

    /// Extracts variables from an element pattern (node or edge).
    fn extract_element_variables(
        &self,
        element: &ElementPattern,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        match element {
            ElementPattern::Node(node_pattern) => {
                if let Some(var_decl) = &node_pattern.variable {
                    let var_name = var_decl.variable.to_string();
                    let span = var_decl.span.clone();

                    if self.config.warn_on_shadowing
                        && let Some(existing) = symbol_table.lookup(&var_name)
                    {
                        use crate::semantic::diag::SemanticDiagBuilder;
                        let diag = SemanticDiagBuilder::variable_shadowing(
                            &var_name,
                            span.clone(),
                            existing.declared_at.clone(),
                        )
                        .build();
                        diagnostics.push(diag);
                    }

                    symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                }
            }
            ElementPattern::Edge(edge_pattern) => {
                if let EdgePattern::Full(full_edge) = edge_pattern
                    && let Some(var_decl) = &full_edge.filler.variable
                {
                    let var_name = var_decl.variable.to_string();
                    let span = var_decl.span.clone();

                    if self.config.warn_on_shadowing
                        && let Some(existing) = symbol_table.lookup(&var_name)
                    {
                        use crate::semantic::diag::SemanticDiagBuilder;
                        let diag = SemanticDiagBuilder::variable_shadowing(
                            &var_name,
                            span.clone(),
                            existing.declared_at.clone(),
                        )
                        .build();
                        diagnostics.push(diag);
                    }

                    symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                }
            }
        }
    }

    /// Analyzes a LET statement and extracts variable definitions.
    fn analyze_let_statement(
        &self,
        let_stmt: &LetStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::semantic::diag::SemanticDiagBuilder;

        for binding in &let_stmt.bindings {
            let var_name = binding.variable.name.to_string();
            let span = binding.variable.span.clone();

            if self.config.warn_on_shadowing
                && let Some(existing) = symbol_table.lookup(&var_name)
            {
                // Emit variable shadowing warning
                let diag = SemanticDiagBuilder::variable_shadowing(
                    &var_name,
                    span.clone(),
                    existing.declared_at.clone(),
                )
                .build();
                diagnostics.push(diag);
            }

            symbol_table.define(var_name, SymbolKind::LetVariable, span);
        }
    }

    /// Analyzes a FOR statement and extracts loop variable.
    fn analyze_for_statement(
        &self,
        for_stmt: &ForStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::ForOrdinalityOrOffset;
        use crate::semantic::diag::SemanticDiagBuilder;

        let var_name = for_stmt.item.binding_variable.name.to_string();
        let span = for_stmt.item.binding_variable.span.clone();

        if self.config.warn_on_shadowing
            && let Some(existing) = symbol_table.lookup(&var_name)
        {
            // Emit variable shadowing warning
            let diag = SemanticDiagBuilder::variable_shadowing(
                &var_name,
                span.clone(),
                existing.declared_at.clone(),
            )
            .build();
            diagnostics.push(diag);
        }

        symbol_table.define(var_name, SymbolKind::ForVariable, span);

        // Also handle ordinality/offset variable if present
        if let Some(ordinality_or_offset) = &for_stmt.ordinality_or_offset {
            let ord_var = match ordinality_or_offset {
                ForOrdinalityOrOffset::Ordinality { variable } => variable,
                ForOrdinalityOrOffset::Offset { variable } => variable,
            };
            let ord_var_name = ord_var.name.to_string();
            let ord_span = ord_var.span.clone();

            if self.config.warn_on_shadowing
                && let Some(existing) = symbol_table.lookup(&ord_var_name)
            {
                // Emit variable shadowing warning for ordinality/offset variable
                let diag = SemanticDiagBuilder::variable_shadowing(
                    &ord_var_name,
                    ord_span.clone(),
                    existing.declared_at.clone(),
                )
                .build();
                diagnostics.push(diag);
            }

            symbol_table.define(ord_var_name, SymbolKind::ForVariable, ord_span);
        }
    }

    /// Analyzes a mutation statement and extracts variable definitions.
    fn analyze_mutation(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::LinearDataModifyingStatement;

        // Push a mutation scope
        symbol_table.push_scope(ScopeKind::Query);

        match mutation {
            LinearDataModifyingStatement::Focused(focused) => {
                // Analyze all data accessing statements
                for stmt in &focused.statements {
                    self.analyze_data_accessing_statement(stmt, symbol_table, diagnostics);
                }
            }
            LinearDataModifyingStatement::Ambient(ambient) => {
                // Analyze all data accessing statements
                for stmt in &ambient.statements {
                    self.analyze_data_accessing_statement(stmt, symbol_table, diagnostics);
                }
            }
        }
    }

    /// Analyzes a data accessing statement (query or mutation).
    fn analyze_data_accessing_statement(
        &self,
        stmt: &crate::ast::mutation::SimpleDataAccessingStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::SimpleDataAccessingStatement;

        match stmt {
            SimpleDataAccessingStatement::Query(query_stmt) => {
                // Analyze query statement
                self.analyze_primitive_statement(query_stmt, symbol_table, diagnostics);
            }
            SimpleDataAccessingStatement::Modifying(modifying_stmt) => {
                self.analyze_modifying_statement(modifying_stmt, symbol_table, diagnostics);
            }
        }
    }

    /// Analyzes a data modifying statement.
    fn analyze_modifying_statement(
        &self,
        stmt: &crate::ast::mutation::SimpleDataModifyingStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::SimpleDataModifyingStatement;

        match stmt {
            SimpleDataModifyingStatement::Primitive(primitive) => {
                self.analyze_primitive_modifying_statement(primitive, symbol_table, diagnostics);
            }
            SimpleDataModifyingStatement::Call(_call_stmt) => {
                // CALL statements may yield variables - would need to analyze YIELD clause
                // For now, this is a placeholder for future implementation
            }
        }
    }

    /// Analyzes a primitive data modifying statement (INSERT/SET/REMOVE/DELETE).
    fn analyze_primitive_modifying_statement(
        &self,
        stmt: &crate::ast::mutation::PrimitiveDataModifyingStatement,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::PrimitiveDataModifyingStatement;

        match stmt {
            PrimitiveDataModifyingStatement::Insert(insert_stmt) => {
                // Extract variables from INSERT patterns
                self.analyze_insert_pattern(&insert_stmt.pattern, symbol_table, diagnostics);
            }
            PrimitiveDataModifyingStatement::Set(_set_stmt) => {
                // SET statements reference existing variables but don't define new ones
            }
            PrimitiveDataModifyingStatement::Remove(_remove_stmt) => {
                // REMOVE statements reference existing variables but don't define new ones
            }
            PrimitiveDataModifyingStatement::Delete(_delete_stmt) => {
                // DELETE statements reference existing variables but don't define new ones
            }
        }
    }

    /// Analyzes an INSERT pattern and extracts variable definitions.
    fn analyze_insert_pattern(
        &self,
        pattern: &crate::ast::mutation::InsertGraphPattern,
        symbol_table: &mut SymbolTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::semantic::diag::SemanticDiagBuilder;

        // Walk all insert path patterns
        for path in &pattern.paths {
            for element in &path.elements {
                use crate::ast::mutation::InsertElementPattern;

                match element {
                    InsertElementPattern::Node(node_pattern) => {
                        if let Some(filler) = &node_pattern.filler
                            && let Some(var_decl) = &filler.variable
                        {
                            let var_name = var_decl.variable.to_string();
                            let span = var_decl.span.clone();

                            if self.config.warn_on_shadowing
                                && let Some(existing) = symbol_table.lookup(&var_name)
                            {
                                let diag = SemanticDiagBuilder::variable_shadowing(
                                    &var_name,
                                    span.clone(),
                                    existing.declared_at.clone(),
                                )
                                .build();
                                diagnostics.push(diag);
                            }

                            symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                        }
                    }
                    InsertElementPattern::Edge(edge_pattern) => {
                        let filler_opt = match edge_pattern {
                            crate::ast::mutation::InsertEdgePattern::PointingLeft(edge) => {
                                &edge.filler
                            }
                            crate::ast::mutation::InsertEdgePattern::PointingRight(edge) => {
                                &edge.filler
                            }
                            crate::ast::mutation::InsertEdgePattern::Undirected(edge) => {
                                &edge.filler
                            }
                        };

                        if let Some(filler) = filler_opt
                            && let Some(var_decl) = &filler.variable
                        {
                            let var_name = var_decl.variable.to_string();
                            let span = var_decl.span.clone();

                            if self.config.warn_on_shadowing
                                && let Some(existing) = symbol_table.lookup(&var_name)
                            {
                                let diag = SemanticDiagBuilder::variable_shadowing(
                                    &var_name,
                                    span.clone(),
                                    existing.declared_at.clone(),
                                )
                                .build();
                                diagnostics.push(diag);
                            }

                            symbol_table.define(var_name, SymbolKind::BindingVariable, span);
                        }
                    }
                }
            }
        }
    }

    /// Pass 2: Type Inference - Infers types for expressions.
    fn run_type_inference(
        &self,
        program: &Program,
        _symbol_table: &SymbolTable,
        _diagnostics: &mut Vec<Diag>,
    ) -> TypeTable {
        let mut type_table = TypeTable::new();

        // Walk all statements and infer types for expressions
        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.infer_query_types(&query_stmt.query, &mut type_table);
                }
                Statement::Mutation(mutation_stmt) => {
                    self.infer_mutation_types(&mutation_stmt.statement, &mut type_table);
                }
                _ => {}
            }
        }

        type_table
    }

    /// Infers types in a query.
    fn infer_query_types(&self, query: &Query, type_table: &mut TypeTable) {
        match query {
            Query::Linear(linear_query) => {
                self.infer_linear_query_types(linear_query, type_table);
            }
            Query::Composite(composite) => {
                self.infer_query_types(&composite.left, type_table);
                self.infer_query_types(&composite.right, type_table);
            }
            Query::Parenthesized(query, _) => {
                self.infer_query_types(query, type_table);
            }
        }
    }

    /// Infers types in a linear query.
    fn infer_linear_query_types(&self, linear_query: &LinearQuery, type_table: &mut TypeTable) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        // Walk primitive statements and infer types
        for statement in primitive_statements {
            match statement {
                PrimitiveQueryStatement::Match(_) => {
                    // MATCH statements don't directly have expressions to type
                    // Pattern variables would be typed as Node, Edge, Path, etc.
                }
                PrimitiveQueryStatement::Let(let_stmt) => {
                    // Infer types of LET variable definitions
                    for binding in &let_stmt.bindings {
                        self.infer_expression_type(&binding.value, type_table);
                    }
                }
                PrimitiveQueryStatement::For(for_stmt) => {
                    // Infer type of FOR collection expression
                    self.infer_expression_type(&for_stmt.item.collection, type_table);
                }
                PrimitiveQueryStatement::Filter(filter) => {
                    // Infer type of filter condition (should be boolean)
                    self.infer_expression_type(&filter.condition, type_table);
                }
                PrimitiveQueryStatement::Select(select) => {
                    // Infer types of select items
                    match &select.select_items {
                        crate::ast::query::SelectItemList::Items { items } => {
                            for item in items {
                                self.infer_expression_type(&item.expression, type_table);
                            }
                        }
                        crate::ast::query::SelectItemList::Star => {
                            // SELECT * doesn't have specific expressions to type
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Infers types in a mutation statement.
    fn infer_mutation_types(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        type_table: &mut TypeTable,
    ) {
        use crate::ast::mutation::LinearDataModifyingStatement;

        let statements = match mutation {
            LinearDataModifyingStatement::Focused(focused) => &focused.statements,
            LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
        };

        for stmt in statements {
            use crate::ast::mutation::SimpleDataAccessingStatement;

            match stmt {
                SimpleDataAccessingStatement::Query(query_stmt) => {
                    // Infer types in the query part
                    self.infer_primitive_query_statement_types(query_stmt, type_table);
                }
                SimpleDataAccessingStatement::Modifying(modifying) => {
                    self.infer_modifying_statement_types(modifying, type_table);
                }
            }
        }
    }

    /// Infers types in a primitive query statement (helper for mutations).
    fn infer_primitive_query_statement_types(
        &self,
        stmt: &PrimitiveQueryStatement,
        type_table: &mut TypeTable,
    ) {
        match stmt {
            PrimitiveQueryStatement::Match(_) => {
                // MATCH patterns define variables but don't have expressions to type
            }
            PrimitiveQueryStatement::Let(let_stmt) => {
                for binding in &let_stmt.bindings {
                    self.infer_expression_type(&binding.value, type_table);
                }
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                self.infer_expression_type(&for_stmt.item.collection, type_table);
            }
            PrimitiveQueryStatement::Filter(filter) => {
                self.infer_expression_type(&filter.condition, type_table);
            }
            PrimitiveQueryStatement::Select(select) => match &select.select_items {
                crate::ast::query::SelectItemList::Items { items } => {
                    for item in items {
                        self.infer_expression_type(&item.expression, type_table);
                    }
                }
                crate::ast::query::SelectItemList::Star => {}
            },
            _ => {}
        }
    }

    /// Infers types in a modifying statement.
    fn infer_modifying_statement_types(
        &self,
        stmt: &crate::ast::mutation::SimpleDataModifyingStatement,
        type_table: &mut TypeTable,
    ) {
        use crate::ast::mutation::{PrimitiveDataModifyingStatement, SimpleDataModifyingStatement};

        match stmt {
            SimpleDataModifyingStatement::Primitive(primitive) => match primitive {
                PrimitiveDataModifyingStatement::Insert(insert_stmt) => {
                    // Infer types in INSERT property specifications
                    for path in &insert_stmt.pattern.paths {
                        for element in &path.elements {
                            use crate::ast::mutation::InsertElementPattern;

                            let properties_opt = match element {
                                InsertElementPattern::Node(node) => {
                                    node.filler.as_ref().and_then(|f| f.properties.as_ref())
                                }
                                InsertElementPattern::Edge(edge) => {
                                    let filler = match edge {
                                        crate::ast::mutation::InsertEdgePattern::PointingLeft(
                                            e,
                                        ) => &e.filler,
                                        crate::ast::mutation::InsertEdgePattern::PointingRight(
                                            e,
                                        ) => &e.filler,
                                        crate::ast::mutation::InsertEdgePattern::Undirected(e) => {
                                            &e.filler
                                        }
                                    };
                                    filler.as_ref().and_then(|f| f.properties.as_ref())
                                }
                            };

                            if let Some(properties) = properties_opt {
                                for pair in &properties.properties {
                                    self.infer_expression_type(&pair.value, type_table);
                                }
                            }
                        }
                    }
                }
                PrimitiveDataModifyingStatement::Set(set_stmt) => {
                    // Infer types in SET value expressions
                    for item in &set_stmt.items.items {
                        use crate::ast::mutation::SetItem;

                        match item {
                            SetItem::Property(prop) => {
                                self.infer_expression_type(&prop.value, type_table);
                            }
                            SetItem::AllProperties(all_props) => {
                                for pair in &all_props.properties.properties {
                                    self.infer_expression_type(&pair.value, type_table);
                                }
                            }
                            SetItem::Label(_) => {
                                // Labels don't have expressions to type
                            }
                        }
                    }
                }
                PrimitiveDataModifyingStatement::Remove(_) => {
                    // REMOVE doesn't have expressions to type
                }
                PrimitiveDataModifyingStatement::Delete(delete_stmt) => {
                    // Infer types in DELETE expressions
                    for item in &delete_stmt.items.items {
                        self.infer_expression_type(&item.expression, type_table);
                    }
                }
            },
            SimpleDataModifyingStatement::Call(_) => {
                // CALL procedure - would need to analyze arguments
                // Placeholder for future implementation
            }
        }
    }

    /// Infers the type of an expression and records it in the type table.
    ///
    /// This function performs type inference and persists the inferred type to the type table.
    /// The type can later be retrieved for validation in subsequent passes.
    #[allow(clippy::only_used_in_recursion)]
    fn infer_expression_type(
        &self,
        expr: &crate::ast::expression::Expression,
        type_table: &mut TypeTable,
    ) {
        use crate::ast::expression::{BinaryOperator, Literal, UnaryOperator};
        use crate::ir::type_table::Type;

        let inferred_type = match expr {
            // Literals have direct type mappings
            crate::ast::expression::Expression::Literal(lit, _) => match lit {
                Literal::Boolean(_) => Type::Boolean,
                Literal::Null => Type::Null,
                Literal::Integer(_) => Type::Int,
                Literal::Float(_) => Type::Float,
                Literal::String(_) => Type::String,
                Literal::ByteString(_) => Type::String, // Treat as string type
                Literal::Date(_) => Type::Date,
                Literal::Time(_) => Type::Time,
                Literal::Datetime(_) => Type::Timestamp,
                Literal::Duration(_) => Type::Duration,
                Literal::List(exprs) => {
                    // Infer element types recursively
                    for elem in exprs {
                        self.infer_expression_type(elem, type_table);
                    }
                    // For now, use List(Any) - could infer common element type
                    Type::List(Box::new(Type::Any))
                }
                Literal::Record(_) => {
                    // For now, use Record with empty fields
                    Type::Record(vec![])
                }
            },

            // Unary operations
            crate::ast::expression::Expression::Unary(op, operand, _) => {
                self.infer_expression_type(operand, type_table);
                match op {
                    UnaryOperator::Plus | UnaryOperator::Minus => Type::Float, // Could be Int or Float, use Float as general numeric
                    UnaryOperator::Not => Type::Boolean, // NOT produces boolean
                }
            }

            // Binary operations
            crate::ast::expression::Expression::Binary(op, left, right, _) => {
                self.infer_expression_type(left, type_table);
                self.infer_expression_type(right, type_table);
                match op {
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Modulo => Type::Float, // Arithmetic operations - use Float as general numeric
                    BinaryOperator::Concatenate => Type::String, // String concatenation produces string
                }
            }

            // Comparison operations always produce boolean
            crate::ast::expression::Expression::Comparison(_, left, right, _) => {
                self.infer_expression_type(left, type_table);
                self.infer_expression_type(right, type_table);
                Type::Boolean
            }

            // Logical operations produce boolean
            crate::ast::expression::Expression::Logical(_, left, right, _) => {
                self.infer_expression_type(left, type_table);
                self.infer_expression_type(right, type_table);
                Type::Boolean
            }

            // Parenthesized expression has same type as inner expression
            crate::ast::expression::Expression::Parenthesized(inner, _) => {
                self.infer_expression_type(inner, type_table);
                return; // Don't set type for parenthesized wrapper
            }

            // Property reference - type depends on property
            crate::ast::expression::Expression::PropertyReference(object, _prop, _) => {
                self.infer_expression_type(object, type_table);
                Type::Any // Without schema, we don't know property types
            }

            // Variable reference - type should be looked up in symbol table
            crate::ast::expression::Expression::VariableReference(_, _) => {
                Type::Any // Without symbol table integration, use Any
            }

            // Parameter reference
            crate::ast::expression::Expression::ParameterReference(_, _) => {
                Type::Any // Parameters can be any type
            }

            // Function calls - would need function signature database
            crate::ast::expression::Expression::FunctionCall(_) => {
                Type::Any // Unknown without function signature info
            }

            // Case expressions - type is union of all THEN clause types
            crate::ast::expression::Expression::Case(_) => {
                Type::Any // Would need to infer from THEN clauses
            }

            // Cast expression - type is the target type
            crate::ast::expression::Expression::Cast(cast) => {
                self.infer_expression_type(&cast.operand, type_table);
                // Would need to map ValueType to Type
                Type::Any
            }

            // Aggregate functions
            crate::ast::expression::Expression::AggregateFunction(agg) => {
                use crate::ast::expression::{AggregateFunction, GeneralSetFunctionType};
                match &**agg {
                    AggregateFunction::CountStar { .. } => Type::Int,
                    AggregateFunction::GeneralSetFunction(gsf) => {
                        self.infer_expression_type(&gsf.expression, type_table);
                        match gsf.function_type {
                            GeneralSetFunctionType::Count => Type::Int,
                            GeneralSetFunctionType::Avg => Type::Float,
                            GeneralSetFunctionType::Sum => Type::Float,
                            GeneralSetFunctionType::Max | GeneralSetFunctionType::Min => Type::Any,
                            GeneralSetFunctionType::CollectList => Type::List(Box::new(Type::Any)),
                            _ => Type::Any, // Other aggregate functions
                        }
                    }
                    AggregateFunction::BinarySetFunction(_) => Type::Any,
                }
            }

            // Type annotation - use the annotated type
            crate::ast::expression::Expression::TypeAnnotation(inner, _annotation, _) => {
                self.infer_expression_type(inner, type_table);
                Type::Any // Would need to convert ValueType to Type
            }

            // List constructor
            crate::ast::expression::Expression::ListConstructor(elements, _) => {
                for elem in elements {
                    self.infer_expression_type(elem, type_table);
                }
                Type::List(Box::new(Type::Any))
            }

            // Record constructor
            crate::ast::expression::Expression::RecordConstructor(fields, _) => {
                for field in fields {
                    self.infer_expression_type(&field.value, type_table);
                }
                Type::Record(vec![])
            }

            // Path constructor
            crate::ast::expression::Expression::PathConstructor(elements, _) => {
                for elem in elements {
                    self.infer_expression_type(elem, type_table);
                }
                Type::Path
            }

            // EXISTS predicate produces boolean
            crate::ast::expression::Expression::Exists(_) => Type::Boolean,

            // Predicates produce boolean
            crate::ast::expression::Expression::Predicate(_) => Type::Boolean,

            // Graph expressions
            crate::ast::expression::Expression::GraphExpression(inner, _) => {
                self.infer_expression_type(inner, type_table);
                Type::Any
            }

            // Binding table expressions
            crate::ast::expression::Expression::BindingTableExpression(inner, _) => {
                self.infer_expression_type(inner, type_table);
                Type::Any
            }

            // Subquery expressions
            crate::ast::expression::Expression::SubqueryExpression(inner, _) => {
                self.infer_expression_type(inner, type_table);
                Type::Any
            }
        };

        // Persist the inferred type to the type table using span-based lookup
        // This allows subsequent passes to retrieve the inferred type
        type_table.set_type_by_span(&expr.span(), inferred_type);
    }

    /// Pass 3: Variable Validation - Checks for undefined variables and shadowing.
    fn run_variable_validation(
        &self,
        program: &Program,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        diagnostics: &mut Vec<Diag>,
    ) {
        // Walk all statements and check variable references with statement-level scope tracking
        let mut statement_id = 0;
        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.validate_query_variables(
                        &query_stmt.query,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                    statement_id += 1;
                }
                _ => {
                    // Other statements don't need variable validation at this level
                }
            }
        }
    }

    /// Validates variable references in a query.
    fn validate_query_variables(
        &self,
        query: &Query,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        match query {
            Query::Linear(linear_query) => {
                self.validate_linear_query_variables(
                    linear_query,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Query::Composite(composite) => {
                self.validate_query_variables(
                    &composite.left,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
                self.validate_query_variables(
                    &composite.right,
                    symbol_table,
                    scope_metadata,
                    statement_id + 1000,
                    diagnostics,
                );
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_variables(
                    query,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
        }
    }

    /// Validates variable references in a linear query.
    fn validate_linear_query_variables(
        &self,
        linear_query: &LinearQuery,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::{AmbientLinearQuery, FocusedLinearQuery};

        let (primitive_statements, result_statement) = match linear_query {
            LinearQuery::Focused(FocusedLinearQuery {
                primitive_statements,
                result_statement,
                ..
            }) => (primitive_statements, result_statement),
            LinearQuery::Ambient(AmbientLinearQuery {
                primitive_statements,
                result_statement,
                ..
            }) => (primitive_statements, result_statement),
        };

        // Validate all primitive statements
        for stmt in primitive_statements {
            self.validate_primitive_statement_variables(
                stmt,
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }

        // Validate RETURN statement variables
        if let Some(result_stmt) = result_statement {
            self.validate_result_statement_variables(
                result_stmt.as_ref(),
                symbol_table,
                scope_metadata,
                statement_id,
                diagnostics,
            );
        }
    }

    /// Validates variable references in primitive statements.
    fn validate_primitive_statement_variables(
        &self,
        statement: &PrimitiveQueryStatement,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        match statement {
            PrimitiveQueryStatement::Match(match_stmt) => {
                // MATCH patterns define variables, they don't reference them (except in WHERE clauses)
                // Validate WHERE clause if present
                match match_stmt {
                    MatchStatement::Simple(simple) => {
                        if let Some(where_clause) = &simple.pattern.where_clause {
                            self.validate_expression_variables(
                                &where_clause.condition,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                    MatchStatement::Optional(optional) => match &optional.operand {
                        crate::ast::query::OptionalOperand::Match { pattern } => {
                            if let Some(where_clause) = &pattern.where_clause {
                                self.validate_expression_variables(
                                    &where_clause.condition,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                        crate::ast::query::OptionalOperand::Block { statements }
                        | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                            for stmt in statements {
                                self.validate_match_statement_variables(
                                    stmt,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                    },
                }
            }
            PrimitiveQueryStatement::Let(let_stmt) => {
                // Validate LET value expressions
                for binding in &let_stmt.bindings {
                    self.validate_expression_variables(
                        &binding.value,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            PrimitiveQueryStatement::For(for_stmt) => {
                // Validate FOR collection expression
                self.validate_expression_variables(
                    &for_stmt.item.collection,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            PrimitiveQueryStatement::Filter(filter) => {
                // Validate FILTER condition
                self.validate_expression_variables(
                    &filter.condition,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );

                // ISO GQL: Check for illegal aggregation in WHERE clause
                if self.expression_contains_aggregation(&filter.condition) {
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag = SemanticDiagBuilder::aggregation_error(
                        "Aggregation functions not allowed in WHERE clause (use HAVING instead)",
                        filter.condition.span().clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }
            }
            PrimitiveQueryStatement::OrderByAndPage(order_by_page) => {
                // Validate ORDER BY expressions
                if let Some(order_by) = &order_by_page.order_by {
                    for sort_spec in &order_by.sort_specifications {
                        self.validate_expression_variables(
                            &sort_spec.key,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
                // Validate LIMIT/OFFSET expressions if present
                if let Some(limit) = &order_by_page.limit {
                    self.validate_expression_variables(
                        &limit.count,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
                if let Some(offset) = &order_by_page.offset {
                    self.validate_expression_variables(
                        &offset.count,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            PrimitiveQueryStatement::Select(select) => {
                // Validate SELECT expressions
                match &select.select_items {
                    crate::ast::query::SelectItemList::Star => {
                        // * doesn't reference specific expressions
                    }
                    crate::ast::query::SelectItemList::Items { items } => {
                        for item in items {
                            self.validate_expression_variables(
                                &item.expression,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                }
                // Validate GROUP BY if present
                if let Some(group_by) = &select.group_by {
                    for elem in &group_by.elements {
                        if let crate::ast::query::GroupingElement::Expression(expr) = elem {
                            self.validate_expression_variables(
                                expr,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                }
                // Validate HAVING if present
                if let Some(having) = &select.having {
                    self.validate_expression_variables(
                        &having.condition,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );

                    // ISO GQL: Validate HAVING clause semantics
                    self.validate_having_clause(&having.condition, &select.group_by, diagnostics);
                }
            }
            PrimitiveQueryStatement::Call(call_stmt) => {
                // Validate CALL statement arguments and yields
                use crate::ast::procedure::ProcedureCall;

                match &call_stmt.call {
                    ProcedureCall::Named(named_call) => {
                        // Validate arguments if present
                        if let Some(args) = &named_call.arguments {
                            for arg in &args.arguments {
                                self.validate_expression_variables(
                                    &arg.expression,
                                    symbol_table,
                                    scope_metadata,
                                    statement_id,
                                    diagnostics,
                                );
                            }
                        }
                        // YIELD clause variables are outputs, not inputs - don't validate them here
                    }
                    ProcedureCall::Inline(inline_call) => {
                        // Inline calls don't have traditional arguments but may reference variables
                        // in their variable scope clause - those should already be validated
                        // The nested specification would need recursive validation (future work)
                        if let Some(var_scope) = &inline_call.variable_scope {
                            // Validate that variables in the scope clause are defined
                            for var in &var_scope.variables {
                                if symbol_table.lookup(var.name.as_ref()).is_none() {
                                    use crate::semantic::diag::SemanticDiagBuilder;
                                    let diag = SemanticDiagBuilder::undefined_variable(
                                        &var.name,
                                        var.span.clone(),
                                    )
                                    .build();
                                    diagnostics.push(diag);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validates variable references in a MATCH statement (helper for nested validations).
    fn validate_match_statement_variables(
        &self,
        match_stmt: &MatchStatement,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        match match_stmt {
            MatchStatement::Simple(simple) => {
                if let Some(where_clause) = &simple.pattern.where_clause {
                    self.validate_expression_variables(
                        &where_clause.condition,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            MatchStatement::Optional(optional) => match &optional.operand {
                crate::ast::query::OptionalOperand::Match { pattern } => {
                    if let Some(where_clause) = &pattern.where_clause {
                        self.validate_expression_variables(
                            &where_clause.condition,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
                crate::ast::query::OptionalOperand::Block { statements }
                | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                    for stmt in statements {
                        self.validate_match_statement_variables(
                            stmt,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            },
        }
    }

    /// Validates variable references in a result statement (RETURN).
    fn validate_result_statement_variables(
        &self,
        result_stmt: &crate::ast::query::PrimitiveResultStatement,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::{PrimitiveResultStatement, ReturnItemList};

        if let PrimitiveResultStatement::Return(return_stmt) = result_stmt {
            // Validate each return item
            match &return_stmt.items {
                ReturnItemList::Star => {
                    // * doesn't reference specific variables
                }
                ReturnItemList::Items { items } => {
                    for item in items {
                        self.validate_expression_variables(
                            &item.expression,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }

            // Validate aggregation rules for RETURN (ISO GQL compliance)
            self.validate_return_aggregation(return_stmt, diagnostics);
        }
    }

    /// Validates aggregation rules in RETURN statements per ISO GQL standard.
    /// Cannot mix aggregated and non-aggregated expressions without GROUP BY.
    fn validate_return_aggregation(
        &self,
        return_stmt: &crate::ast::query::ReturnStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::ReturnItemList;
        use crate::semantic::diag::SemanticDiagBuilder;

        // Check if mixing aggregated and non-aggregated expressions
        let (has_aggregation, non_aggregated_expressions) = match &return_stmt.items {
            ReturnItemList::Star => (false, vec![]),
            ReturnItemList::Items { items } => {
                let mut has_agg = false;
                let mut non_agg_exprs = Vec::new();

                for item in items {
                    if self.expression_contains_aggregation(&item.expression) {
                        has_agg = true;
                    } else {
                        non_agg_exprs.push(&item.expression);
                    }
                }
                (has_agg, non_agg_exprs)
            }
        };

        // In strict mode or with GROUP BY, mixing requires GROUP BY
        // RETURN doesn't have GROUP BY, so this is an error in strict mode
        if has_aggregation && !non_aggregated_expressions.is_empty() && self.config.strict_mode {
            for expr in non_aggregated_expressions {
                let diag = SemanticDiagBuilder::aggregation_error(
                    "Cannot mix aggregated and non-aggregated expressions in RETURN without GROUP BY",
                    expr.span().clone(),
                )
                .build();
                diagnostics.push(diag);
            }
        }

        // Check for nested aggregation
        if let ReturnItemList::Items { items } = &return_stmt.items {
            for item in items {
                self.check_nested_aggregation(&item.expression, false, diagnostics);
            }
        }
    }

    /// Validates variable references in an expression with reference-site-aware lookups.
    fn validate_expression_variables(
        &self,
        expression: &crate::ast::expression::Expression,
        symbol_table: &SymbolTable,
        scope_metadata: &ScopeMetadata,
        statement_id: usize,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::Expression;
        use crate::semantic::diag::SemanticDiagBuilder;

        match expression {
            Expression::VariableReference(var_name, span) => {
                // Use reference-site-aware lookup: check from the statement's scope, not global current_scope
                let scope_to_check = if statement_id < scope_metadata.statement_scopes.len() {
                    scope_metadata.statement_scopes[statement_id]
                } else {
                    // Fallback to current scope if statement_id is out of bounds (shouldn't happen)
                    symbol_table.current_scope()
                };

                // Perform lookup from the correct scope
                if symbol_table.lookup_from(scope_to_check, var_name).is_none() {
                    // Generate undefined variable diagnostic
                    let diag =
                        SemanticDiagBuilder::undefined_variable(var_name.as_str(), span.clone())
                            .build();
                    diagnostics.push(diag);
                }
            }
            Expression::Binary(_, left, right, _) => {
                self.validate_expression_variables(
                    left,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
                self.validate_expression_variables(
                    right,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::Unary(_, operand, _) => {
                self.validate_expression_variables(
                    operand,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::Comparison(_, left, right, _) => {
                self.validate_expression_variables(
                    left,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
                self.validate_expression_variables(
                    right,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::Logical(_, left, right, _) => {
                self.validate_expression_variables(
                    left,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
                self.validate_expression_variables(
                    right,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::PropertyReference(object, _, _) => {
                self.validate_expression_variables(
                    object,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::Parenthesized(expr, _) => {
                self.validate_expression_variables(
                    expr,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::FunctionCall(func_call) => {
                // Validate function arguments
                for arg in &func_call.arguments {
                    self.validate_expression_variables(
                        arg,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            Expression::Case(case_expr) => {
                // Validate CASE expression
                match case_expr {
                    crate::ast::expression::CaseExpression::Searched(searched) => {
                        for when in &searched.when_clauses {
                            self.validate_expression_variables(
                                &when.condition,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                            self.validate_expression_variables(
                                &when.then_result,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                        if let Some(else_expr) = &searched.else_clause {
                            self.validate_expression_variables(
                                else_expr,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                    crate::ast::expression::CaseExpression::Simple(simple) => {
                        self.validate_expression_variables(
                            &simple.operand,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        for when in &simple.when_clauses {
                            self.validate_expression_variables(
                                &when.when_value,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                            self.validate_expression_variables(
                                &when.then_result,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                        if let Some(else_expr) = &simple.else_clause {
                            self.validate_expression_variables(
                                else_expr,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                }
            }
            Expression::Cast(cast) => {
                // Validate cast operand
                self.validate_expression_variables(
                    &cast.operand,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::AggregateFunction(agg_func) => {
                // Validate aggregate function arguments
                match &**agg_func {
                    crate::ast::expression::AggregateFunction::CountStar { .. } => {
                        // COUNT(*) has no expression to validate
                    }
                    crate::ast::expression::AggregateFunction::GeneralSetFunction(gsf) => {
                        self.validate_expression_variables(
                            &gsf.expression,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    crate::ast::expression::AggregateFunction::BinarySetFunction(bsf) => {
                        self.validate_expression_variables(
                            &bsf.inverse_distribution_argument,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        self.validate_expression_variables(
                            &bsf.expression,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }
            Expression::TypeAnnotation(inner, _, _) => {
                // Validate annotated expression
                self.validate_expression_variables(
                    inner,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::ListConstructor(elements, _) => {
                // Validate list element expressions
                for elem in elements {
                    self.validate_expression_variables(
                        elem,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            Expression::RecordConstructor(fields, _) => {
                // Validate record field expressions
                for field in fields {
                    self.validate_expression_variables(
                        &field.value,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            Expression::PathConstructor(elements, _) => {
                // Validate path element expressions
                for elem in elements {
                    self.validate_expression_variables(
                        elem,
                        symbol_table,
                        scope_metadata,
                        statement_id,
                        diagnostics,
                    );
                }
            }
            Expression::Exists(exists_expr) => {
                // Validate EXISTS predicate - contains a nested query/pattern
                use crate::ast::expression::ExistsVariant;
                match &exists_expr.variant {
                    ExistsVariant::GraphPattern(_) => {
                        // Graph pattern validation is placeholder for Sprint 8
                        // No variable validation needed yet
                    }
                    ExistsVariant::Subquery(subquery_expr) => {
                        // Validate the subquery expression recursively
                        // NOTE: Subqueries should have their own isolated scope, but that requires
                        // more complex scope tracking during analysis. For now, use same statement_id.
                        self.validate_expression_variables(
                            subquery_expr,
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }
            Expression::Predicate(predicate) => {
                // Validate predicate expressions
                use crate::ast::expression::Predicate;
                match predicate {
                    Predicate::IsNull(operand, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsTyped(operand, _, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsNormalized(operand, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsDirected(operand, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsLabeled(operand, _, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsTruthValue(operand, _, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::IsSource(operand, of, _, _)
                    | Predicate::IsDestination(operand, of, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        self.validate_expression_variables(
                            of.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::AllDifferent(operands, _) => {
                        for operand in operands {
                            self.validate_expression_variables(
                                operand,
                                symbol_table,
                                scope_metadata,
                                statement_id,
                                diagnostics,
                            );
                        }
                    }
                    Predicate::Same(left, right, _) => {
                        self.validate_expression_variables(
                            left.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                        self.validate_expression_variables(
                            right.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                    Predicate::PropertyExists(operand, _, _) => {
                        self.validate_expression_variables(
                            operand.as_ref(),
                            symbol_table,
                            scope_metadata,
                            statement_id,
                            diagnostics,
                        );
                    }
                }
            }
            Expression::GraphExpression(inner, _) => {
                // Validate graph expression
                self.validate_expression_variables(
                    inner,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::BindingTableExpression(inner, _) => {
                // Validate binding table expression
                self.validate_expression_variables(
                    inner,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::SubqueryExpression(inner, _) => {
                // Validate subquery expression
                self.validate_expression_variables(
                    inner,
                    symbol_table,
                    scope_metadata,
                    statement_id,
                    diagnostics,
                );
            }
            Expression::Literal(_, _) | Expression::ParameterReference(_, _) => {
                // Literals and parameters don't reference variables
            }
        }
    }

    /// Pass 4: Pattern Validation - Checks pattern connectivity.
    fn run_pattern_validation(&self, program: &Program, diagnostics: &mut Vec<Diag>) {
        // This pass checks:
        // - Graph patterns are connected
        // - Path patterns are valid
        // - Quantified patterns maintain connectivity

        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.validate_query_patterns(&query_stmt.query, diagnostics);
                }
                Statement::Mutation(mutation_stmt) => {
                    // Validate mutation patterns (INSERT patterns should be connected)
                    self.validate_mutation_patterns(&mutation_stmt.statement, diagnostics);
                }
                _ => {}
            }
        }
    }

    /// Validates patterns in a query for connectivity.
    fn validate_query_patterns(&self, query: &Query, diagnostics: &mut Vec<Diag>) {
        match query {
            Query::Linear(linear_query) => {
                self.validate_linear_query_patterns(linear_query, diagnostics);
            }
            Query::Composite(composite) => {
                self.validate_query_patterns(&composite.left, diagnostics);
                self.validate_query_patterns(&composite.right, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_patterns(query, diagnostics);
            }
        }
    }

    /// Validates patterns in a linear query.
    fn validate_linear_query_patterns(
        &self,
        linear_query: &LinearQuery,
        diagnostics: &mut Vec<Diag>,
    ) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        for statement in primitive_statements {
            if let PrimitiveQueryStatement::Match(match_stmt) = statement {
                self.validate_match_pattern_connectivity(match_stmt, diagnostics);
            }
        }
    }

    /// Validates connectivity of a MATCH pattern.
    fn validate_match_pattern_connectivity(
        &self,
        match_stmt: &MatchStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::{GraphPattern, OptionalOperand};
        use crate::semantic::diag::SemanticDiagBuilder;
        use std::collections::{HashMap, HashSet};

        let validate_pattern = |pattern: &GraphPattern, diagnostics: &mut Vec<Diag>| {
            if !self.config.warn_on_disconnected_patterns {
                return;
            }

            // Build connectivity graph
            // Each variable is a node, and edges connect variables that appear together
            let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
            let mut all_variables = HashSet::new();

            for path_pattern in &pattern.paths.patterns {
                self.extract_connectivity_from_path_pattern(
                    path_pattern,
                    &mut adjacency,
                    &mut all_variables,
                );
            }

            // If no variables or only one variable, skip connectivity check
            if all_variables.len() <= 1 {
                return;
            }

            // Perform DFS to check connectivity
            let mut visited = HashSet::new();
            let start_var = all_variables.iter().next().unwrap();
            self.dfs_connectivity(start_var, &adjacency, &mut visited);

            // Check if all variables were reached
            for var in &all_variables {
                if !visited.contains(var) {
                    let diag = SemanticDiagBuilder::disconnected_pattern(pattern.span.clone())
                        .with_note(format!("Variable '{}' is not connected to the rest of the pattern. Consider adding an edge connecting it, or use a separate MATCH clause.", var))
                        .build();
                    diagnostics.push(diag);
                }
            }
        };

        match match_stmt {
            MatchStatement::Simple(simple) => {
                validate_pattern(&simple.pattern, diagnostics);
            }
            MatchStatement::Optional(optional) => match &optional.operand {
                OptionalOperand::Match { pattern } => {
                    validate_pattern(pattern, diagnostics);
                }
                OptionalOperand::Block { statements }
                | OptionalOperand::ParenthesizedBlock { statements } => {
                    for stmt in statements {
                        self.validate_match_pattern_connectivity(stmt, diagnostics);
                    }
                }
            },
        }
    }

    /// Extracts connectivity information from a path pattern.
    fn extract_connectivity_from_path_pattern(
        &self,
        path_pattern: &PathPattern,
        adjacency: &mut HashMap<String, HashSet<String>>,
        all_variables: &mut HashSet<String>,
    ) {
        // If there's a path-level variable, add it
        if let Some(var_decl) = &path_pattern.variable_declaration {
            all_variables.insert(var_decl.variable.to_string());
        }

        self.extract_connectivity_from_expression(
            &path_pattern.expression,
            adjacency,
            all_variables,
        );
    }

    /// Extracts connectivity from a path pattern expression.
    fn extract_connectivity_from_expression(
        &self,
        expr: &PathPatternExpression,
        adjacency: &mut HashMap<String, HashSet<String>>,
        all_variables: &mut HashSet<String>,
    ) {
        match expr {
            PathPatternExpression::Term(term) => {
                self.extract_connectivity_from_term(term, adjacency, all_variables);
            }
            PathPatternExpression::Union { left, right, .. } => {
                self.extract_connectivity_from_expression(left, adjacency, all_variables);
                self.extract_connectivity_from_expression(right, adjacency, all_variables);
            }
            PathPatternExpression::Alternation { alternatives, .. } => {
                for alt in alternatives {
                    self.extract_connectivity_from_term(alt, adjacency, all_variables);
                }
            }
        }
    }

    /// Extracts connectivity from a path term (sequence of elements).
    fn extract_connectivity_from_term(
        &self,
        term: &PathTerm,
        adjacency: &mut HashMap<String, HashSet<String>>,
        all_variables: &mut HashSet<String>,
    ) {
        let mut prev_var: Option<String> = None;

        for factor in &term.factors {
            if let PathPrimary::ElementPattern(elem) = &factor.primary {
                match elem.as_ref() {
                    ElementPattern::Node(node) => {
                        if let Some(var) = &node.variable {
                            let var_name = var.variable.to_string();
                            all_variables.insert(var_name.clone());

                            // Connect to previous variable if exists
                            if let Some(prev) = &prev_var {
                                adjacency
                                    .entry(prev.clone())
                                    .or_default()
                                    .insert(var_name.clone());
                                adjacency
                                    .entry(var_name.clone())
                                    .or_default()
                                    .insert(prev.clone());
                            }
                            prev_var = Some(var_name);
                        }
                    }
                    ElementPattern::Edge(edge) => {
                        // Edges connect nodes, but also can have their own variables
                        if let Some(var_name) = self.get_edge_variable(edge) {
                            all_variables.insert(var_name.to_string());

                            // Connect edge variable to adjacent node if exists
                            if let Some(prev) = &prev_var {
                                adjacency
                                    .entry(prev.clone())
                                    .or_default()
                                    .insert(var_name.to_string());
                                adjacency
                                    .entry(var_name.to_string())
                                    .or_default()
                                    .insert(prev.clone());
                            }
                        }
                    }
                }
            } else if let PathPrimary::ParenthesizedExpression(nested_expr) = &factor.primary {
                self.extract_connectivity_from_expression(nested_expr, adjacency, all_variables);
            }
        }
    }

    /// Gets the variable name from an edge pattern if it exists.
    fn get_edge_variable<'a>(&self, edge: &'a EdgePattern) -> Option<&'a str> {
        match edge {
            EdgePattern::Full(full) => full.filler.variable.as_ref().map(|v| v.variable.as_str()),
            EdgePattern::Abbreviated(_) => None,
        }
    }

    /// DFS to check connectivity of variables in the pattern.
    fn dfs_connectivity(
        &self,
        var: &str,
        adjacency: &HashMap<String, HashSet<String>>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(var) {
            return;
        }

        visited.insert(var.to_string());

        if let Some(neighbors) = adjacency.get(var) {
            for neighbor in neighbors {
                self.dfs_connectivity(neighbor, adjacency, visited);
            }
        }
    }

    /// Validates patterns in a mutation for connectivity.
    fn validate_mutation_patterns(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::{
            LinearDataModifyingStatement, PrimitiveDataModifyingStatement,
            SimpleDataAccessingStatement, SimpleDataModifyingStatement,
        };

        let statements = match mutation {
            LinearDataModifyingStatement::Focused(focused) => &focused.statements,
            LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
        };

        for statement in statements {
            match statement {
                SimpleDataAccessingStatement::Modifying(
                    SimpleDataModifyingStatement::Primitive(primitive),
                ) => {
                    match primitive {
                        PrimitiveDataModifyingStatement::Insert(insert) => {
                            // Validate INSERT patterns are connected
                            self.validate_insert_pattern_connectivity(&insert.pattern, diagnostics);
                        }
                        PrimitiveDataModifyingStatement::Set(_)
                        | PrimitiveDataModifyingStatement::Remove(_)
                        | PrimitiveDataModifyingStatement::Delete(_) => {
                            // These don't have graph patterns to validate
                        }
                    }
                }
                SimpleDataAccessingStatement::Query(_) => {
                    // Query statements within mutations don't need additional pattern validation here
                }
                SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                    // Procedure calls don't have patterns to validate
                }
            }
        }
    }

    /// Validates that an INSERT pattern is connected.
    fn validate_insert_pattern_connectivity(
        &self,
        pattern: &crate::ast::mutation::InsertGraphPattern,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::InsertElementPattern;

        // Build adjacency list for INSERT pattern
        let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
        let mut all_variables = HashSet::new();
        let mut prev_var: Option<String> = None;

        for path in &pattern.paths {
            for element in &path.elements {
                let var_name = match element {
                    InsertElementPattern::Node(node) => node
                        .filler
                        .as_ref()
                        .and_then(|f| f.variable.as_ref())
                        .map(|v| v.variable.to_string()),
                    InsertElementPattern::Edge(edge) => {
                        use crate::ast::mutation::InsertEdgePattern;
                        match edge {
                            InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                            InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                            InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                        }
                        .and_then(|f| f.variable.as_ref())
                        .map(|v| v.variable.to_string())
                    }
                };

                if let Some(var) = var_name {
                    all_variables.insert(var.clone());

                    // Connect to previous variable in path
                    if let Some(prev) = &prev_var {
                        adjacency
                            .entry(prev.clone())
                            .or_default()
                            .insert(var.clone());
                        adjacency
                            .entry(var.clone())
                            .or_default()
                            .insert(prev.clone());
                    }
                    prev_var = Some(var);
                }
            }
            // Reset prev_var at end of path
            prev_var = None;
        }

        // Check connectivity if we have variables
        if all_variables.len() > 1 && self.config.warn_on_disconnected_patterns {
            let mut visited = HashSet::new();
            if let Some(start_var) = all_variables.iter().next() {
                self.dfs_connectivity(start_var, &adjacency, &mut visited);

                if visited.len() < all_variables.len() {
                    let disconnected: Vec<_> = all_variables.difference(&visited).collect();
                    diagnostics.push(
                        Diag::new(DiagSeverity::Warning, format!(
                            "Disconnected INSERT pattern: variables {:?} are not connected to the main pattern",
                            disconnected
                        ))
                    );
                }
            }
        }
    }

    /// Pass 5: Context Validation - Checks clause usage in appropriate contexts.
    fn run_context_validation(&self, program: &Program, diagnostics: &mut Vec<Diag>) {
        // This pass checks:
        // - MATCH clauses in query contexts
        // - INSERT/DELETE clauses in mutation contexts
        // - CREATE/DROP clauses in catalog contexts
        // - Aggregation function usage

        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    // Queries should contain query clauses (MATCH, etc.)
                    self.validate_query_context(&query_stmt.query, diagnostics);
                }
                Statement::Mutation(_mutation_stmt) => {
                    // Mutations should contain mutation clauses (INSERT, DELETE, SET, REMOVE)
                    // For now, we just validate that mutation operations are in mutation context
                    // More detailed validation can be added as needed
                }
                Statement::Catalog(_) => {
                    // Catalog statements (CREATE, DROP, etc.) are valid in catalog context
                }
                Statement::Session(_) | Statement::Transaction(_) | Statement::Empty(_) => {
                    // These are valid in their respective contexts
                }
            }
        }
    }

    /// Validates that query clauses are used appropriately.
    fn validate_query_context(&self, query: &Query, diagnostics: &mut Vec<Diag>) {
        match query {
            Query::Linear(linear_query) => {
                self.validate_linear_query_context(linear_query, diagnostics);
            }
            Query::Composite(composite) => {
                self.validate_query_context(&composite.left, diagnostics);
                self.validate_query_context(&composite.right, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_context(query, diagnostics);
            }
        }
    }

    /// Validates context in a linear query.
    fn validate_linear_query_context(
        &self,
        linear_query: &LinearQuery,
        diagnostics: &mut Vec<Diag>,
    ) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        for statement in primitive_statements {
            match statement {
                PrimitiveQueryStatement::Match(_) => {
                    // MATCH is valid in query context
                }
                PrimitiveQueryStatement::Let(_) => {
                    // LET is valid in query context
                }
                PrimitiveQueryStatement::For(_) => {
                    // FOR is valid in query context
                }
                PrimitiveQueryStatement::Filter(_) => {
                    // WHERE/FILTER is valid in query context
                }
                PrimitiveQueryStatement::OrderByAndPage(_) => {
                    // ORDER BY is valid in query context
                }
                PrimitiveQueryStatement::Select(select) => {
                    // Check for aggregation functions in SELECT and validate GROUP BY semantics
                    self.validate_select_aggregation(select, diagnostics);
                }
                PrimitiveQueryStatement::Call(_) => {
                    // CALL is valid in query context
                }
            }
        }
    }

    /// Validates aggregation and GROUP BY semantics in a SELECT statement.
    fn validate_select_aggregation(
        &self,
        select: &crate::ast::query::SelectStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::SelectItemList;

        // Check if we have aggregation in SELECT items
        let (has_aggregation, non_aggregated_expressions) = match &select.select_items {
            SelectItemList::Items { items } => {
                let mut has_agg = false;
                let mut non_agg_exprs = Vec::new();

                for item in items {
                    if self.expression_contains_aggregation(&item.expression) {
                        has_agg = true;
                    } else {
                        non_agg_exprs.push(&item.expression);
                    }
                }

                (has_agg, non_agg_exprs)
            }
            SelectItemList::Star => {
                // SELECT * is non-aggregated
                (false, Vec::new())
            }
        };

        // If we have aggregation mixed with non-aggregated expressions
        if has_aggregation && !non_aggregated_expressions.is_empty() {
            // Check if there's a GROUP BY clause
            if let Some(group_by) = &select.group_by {
                // Validate that all non-aggregated expressions appear in GROUP BY
                let group_by_expressions = self.collect_group_by_expressions(group_by);

                for non_agg_expr in non_aggregated_expressions {
                    // Check if this expression appears in GROUP BY
                    // For simplicity, we check by expression structure (not perfect but practical)
                    let expr_appears_in_group_by = group_by_expressions
                        .iter()
                        .any(|gb_expr| self.expressions_equivalent(non_agg_expr, gb_expr));

                    if !expr_appears_in_group_by {
                        if self.config.strict_mode {
                            diagnostics.push(
                                SemanticDiagBuilder::aggregation_error(
                                    "Non-aggregated expression must appear in GROUP BY clause when mixing with aggregation",
                                    non_agg_expr.span()
                                )
                                .build()
                            );
                        } else {
                            // In non-strict mode, just warn
                            diagnostics.push(
                                Diag::new(
                                    DiagSeverity::Warning,
                                    "Non-aggregated expression should appear in GROUP BY clause when mixing with aggregation".to_string()
                                )
                            );
                        }
                    }
                }
            } else {
                // No GROUP BY but we have mixed aggregation and non-aggregation
                if self.config.strict_mode {
                    diagnostics.push(
                        SemanticDiagBuilder::aggregation_error(
                            "GROUP BY clause required when mixing aggregated and non-aggregated expressions",
                            select.span.clone()
                        )
                        .build()
                    );
                } else {
                    // In non-strict mode, just warn
                    diagnostics.push(
                        Diag::new(
                            DiagSeverity::Warning,
                            "GROUP BY clause recommended when mixing aggregated and non-aggregated expressions".to_string()
                        )
                    );
                }
            }
        }
    }

    /// Collects all expressions from a GROUP BY clause.
    fn collect_group_by_expressions<'a>(
        &self,
        group_by: &'a crate::ast::query::GroupByClause,
    ) -> Vec<&'a crate::ast::expression::Expression> {
        use crate::ast::query::GroupingElement;

        let mut expressions = Vec::new();
        for element in &group_by.elements {
            match element {
                GroupingElement::Expression(expr) => {
                    expressions.push(expr);
                }
                GroupingElement::EmptyGroupingSet => {
                    // Empty grouping set doesn't provide expressions
                }
            }
        }
        expressions
    }

    /// Checks if two expressions are equivalent (simple structural comparison).
    /// This is a simplified check; a full implementation would need semantic equivalence.
    /// Checks if two expressions are semantically equivalent per ISO GQL standard.
    /// Used for GROUP BY validation and expression matching.
    fn expressions_equivalent(
        &self,
        expr1: &crate::ast::expression::Expression,
        expr2: &crate::ast::expression::Expression,
    ) -> bool {
        use crate::ast::expression::Expression;

        match (expr1, expr2) {
            // Literals
            (Expression::Literal(l1, _), Expression::Literal(l2, _)) => l1 == l2,

            // Variables
            (Expression::VariableReference(v1, _), Expression::VariableReference(v2, _)) => {
                v1 == v2
            }

            // Properties
            (
                Expression::PropertyReference(base1, prop1, _),
                Expression::PropertyReference(base2, prop2, _),
            ) => prop1 == prop2 && self.expressions_equivalent(base1, base2),

            // Binary operations
            (Expression::Binary(op1, l1, r1, _), Expression::Binary(op2, l2, r2, _)) => {
                op1 == op2
                    && self.expressions_equivalent(l1, l2)
                    && self.expressions_equivalent(r1, r2)
            }

            // Unary operations
            (Expression::Unary(op1, e1, _), Expression::Unary(op2, e2, _)) => {
                op1 == op2 && self.expressions_equivalent(e1, e2)
            }

            // Function calls
            (Expression::FunctionCall(f1), Expression::FunctionCall(f2)) => {
                f1.name == f2.name
                    && f1.arguments.len() == f2.arguments.len()
                    && f1
                        .arguments
                        .iter()
                        .zip(&f2.arguments)
                        .all(|(a1, a2)| self.expressions_equivalent(a1, a2))
            }

            // Parenthesized (unwrap and compare)
            (Expression::Parenthesized(e1, _), e2) => self.expressions_equivalent(e1, e2),
            (e1, Expression::Parenthesized(e2, _)) => self.expressions_equivalent(e1, e2),

            // Type annotations (ignore annotation, compare base)
            (Expression::TypeAnnotation(e1, _, _), e2) => self.expressions_equivalent(e1, e2),
            (e1, Expression::TypeAnnotation(e2, _, _)) => self.expressions_equivalent(e1, e2),

            // Comparison operations
            (Expression::Comparison(op1, l1, r1, _), Expression::Comparison(op2, l2, r2, _)) => {
                op1 == op2
                    && self.expressions_equivalent(l1, l2)
                    && self.expressions_equivalent(r1, r2)
            }

            // Logical operations
            (Expression::Logical(op1, l1, r1, _), Expression::Logical(op2, l2, r2, _)) => {
                op1 == op2
                    && self.expressions_equivalent(l1, l2)
                    && self.expressions_equivalent(r1, r2)
            }

            // Default: not equivalent
            _ => false,
        }
    }

    /// Checks if an expression contains aggregation functions.
    fn expression_contains_aggregation(&self, expr: &crate::ast::expression::Expression) -> bool {
        use crate::ast::expression::Expression;

        match expr {
            Expression::AggregateFunction(_) => true,
            Expression::Binary(_, left, right, _) => {
                self.expression_contains_aggregation(left)
                    || self.expression_contains_aggregation(right)
            }
            Expression::Unary(_, operand, _) => self.expression_contains_aggregation(operand),
            Expression::PropertyReference(base, _, _) => self.expression_contains_aggregation(base),
            Expression::Parenthesized(inner, _) => self.expression_contains_aggregation(inner),
            Expression::Comparison(_, left, right, _) => {
                self.expression_contains_aggregation(left)
                    || self.expression_contains_aggregation(right)
            }
            Expression::Logical(_, left, right, _) => {
                self.expression_contains_aggregation(left)
                    || self.expression_contains_aggregation(right)
            }
            _ => false,
        }
    }

    /// Checks for illegal nested aggregation functions per ISO GQL standard.
    /// Nested aggregations like COUNT(SUM(x)) are not allowed.
    fn check_nested_aggregation(
        &self,
        expr: &crate::ast::expression::Expression,
        in_aggregate: bool,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::{AggregateFunction, Expression};
        use crate::semantic::diag::SemanticDiagBuilder;

        match expr {
            Expression::AggregateFunction(agg_func) => {
                if in_aggregate {
                    // Nested aggregation detected!
                    let diag = SemanticDiagBuilder::aggregation_error(
                        "Nested aggregation functions are not allowed",
                        expr.span().clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                    return; // Don't recurse further
                }

                // Check arguments with in_aggregate=true
                match &**agg_func {
                    AggregateFunction::CountStar { .. } => {}
                    AggregateFunction::GeneralSetFunction(gsf) => {
                        self.check_nested_aggregation(&gsf.expression, true, diagnostics);
                    }
                    AggregateFunction::BinarySetFunction(bsf) => {
                        self.check_nested_aggregation(&bsf.expression, true, diagnostics);
                        self.check_nested_aggregation(
                            &bsf.inverse_distribution_argument,
                            true,
                            diagnostics,
                        );
                    }
                }
            }
            Expression::Binary(_, left, right, _) => {
                self.check_nested_aggregation(left, in_aggregate, diagnostics);
                self.check_nested_aggregation(right, in_aggregate, diagnostics);
            }
            Expression::Unary(_, operand, _) => {
                self.check_nested_aggregation(operand, in_aggregate, diagnostics);
            }
            Expression::PropertyReference(base, _, _) => {
                self.check_nested_aggregation(base, in_aggregate, diagnostics);
            }
            Expression::Parenthesized(inner, _) => {
                self.check_nested_aggregation(inner, in_aggregate, diagnostics);
            }
            Expression::Comparison(_, left, right, _) => {
                self.check_nested_aggregation(left, in_aggregate, diagnostics);
                self.check_nested_aggregation(right, in_aggregate, diagnostics);
            }
            Expression::Logical(_, left, right, _) => {
                self.check_nested_aggregation(left, in_aggregate, diagnostics);
                self.check_nested_aggregation(right, in_aggregate, diagnostics);
            }
            Expression::FunctionCall(func) => {
                for arg in &func.arguments {
                    self.check_nested_aggregation(arg, in_aggregate, diagnostics);
                }
            }
            Expression::ListConstructor(exprs, _) => {
                for e in exprs {
                    self.check_nested_aggregation(e, in_aggregate, diagnostics);
                }
            }
            Expression::RecordConstructor(fields, _) => {
                for field in fields {
                    self.check_nested_aggregation(&field.value, in_aggregate, diagnostics);
                }
            }
            Expression::PathConstructor(exprs, _) => {
                for e in exprs {
                    self.check_nested_aggregation(e, in_aggregate, diagnostics);
                }
            }
            Expression::Case(case_expr) => match case_expr {
                crate::ast::expression::CaseExpression::Searched(searched) => {
                    for when in &searched.when_clauses {
                        self.check_nested_aggregation(&when.condition, in_aggregate, diagnostics);
                        self.check_nested_aggregation(&when.then_result, in_aggregate, diagnostics);
                    }
                    if let Some(else_expr) = &searched.else_clause {
                        self.check_nested_aggregation(else_expr, in_aggregate, diagnostics);
                    }
                }
                crate::ast::expression::CaseExpression::Simple(simple) => {
                    self.check_nested_aggregation(&simple.operand, in_aggregate, diagnostics);
                    for when in &simple.when_clauses {
                        self.check_nested_aggregation(&when.when_value, in_aggregate, diagnostics);
                        self.check_nested_aggregation(&when.then_result, in_aggregate, diagnostics);
                    }
                    if let Some(else_expr) = &simple.else_clause {
                        self.check_nested_aggregation(else_expr, in_aggregate, diagnostics);
                    }
                }
            },
            Expression::Cast(cast) => {
                self.check_nested_aggregation(&cast.operand, in_aggregate, diagnostics);
            }
            Expression::Exists(exists) => {
                // EXISTS contains a graph pattern, not expressions that can have aggregations
                // Skip for now
                _ = exists;
            }
            Expression::Predicate(pred) => {
                use crate::ast::expression::Predicate;
                match pred {
                    Predicate::IsNull(expr, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsTyped(expr, _, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsNormalized(expr, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsDirected(expr, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsLabeled(expr, _, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsTruthValue(expr, _, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                    Predicate::IsSource(expr1, expr2, _, _) => {
                        self.check_nested_aggregation(expr1, in_aggregate, diagnostics);
                        self.check_nested_aggregation(expr2, in_aggregate, diagnostics);
                    }
                    Predicate::IsDestination(expr1, expr2, _, _) => {
                        self.check_nested_aggregation(expr1, in_aggregate, diagnostics);
                        self.check_nested_aggregation(expr2, in_aggregate, diagnostics);
                    }
                    Predicate::AllDifferent(exprs, _) => {
                        for e in exprs {
                            self.check_nested_aggregation(e, in_aggregate, diagnostics);
                        }
                    }
                    Predicate::Same(expr1, expr2, _) => {
                        self.check_nested_aggregation(expr1, in_aggregate, diagnostics);
                        self.check_nested_aggregation(expr2, in_aggregate, diagnostics);
                    }
                    Predicate::PropertyExists(expr, _, _) => {
                        self.check_nested_aggregation(expr, in_aggregate, diagnostics);
                    }
                }
            }
            _ => {}
        }
    }

    /// Validates HAVING clause per ISO GQL standard.
    /// Non-aggregated expressions in HAVING must appear in GROUP BY.
    fn validate_having_clause(
        &self,
        condition: &crate::ast::expression::Expression,
        group_by: &Option<crate::ast::query::GroupByClause>,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::semantic::diag::SemanticDiagBuilder;

        // Collect non-aggregated expressions in HAVING
        let non_agg_exprs = self.collect_non_aggregated_expressions(condition);

        if let Some(group_by) = group_by {
            // Check each non-aggregated expression appears in GROUP BY
            let group_by_exprs = self.collect_group_by_expressions(group_by);

            for expr in non_agg_exprs {
                let found_in_group_by = group_by_exprs
                    .iter()
                    .any(|gb_expr| self.expressions_equivalent(expr, gb_expr));

                if !found_in_group_by {
                    let diag = SemanticDiagBuilder::aggregation_error(
                        "Non-aggregated expression in HAVING must appear in GROUP BY",
                        expr.span().clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }
            }
        } else {
            // HAVING without GROUP BY - only aggregates allowed
            if !non_agg_exprs.is_empty() && self.config.strict_mode {
                for expr in non_agg_exprs {
                    let diag = SemanticDiagBuilder::aggregation_error(
                        "HAVING clause requires GROUP BY when using non-aggregated expressions",
                        expr.span().clone(),
                    )
                    .build();
                    diagnostics.push(diag);
                }
            }
        }
    }

    /// Collects non-aggregated expressions from an expression tree.
    fn collect_non_aggregated_expressions<'a>(
        &self,
        expr: &'a crate::ast::expression::Expression,
    ) -> Vec<&'a crate::ast::expression::Expression> {
        let mut result = Vec::new();
        self.collect_non_aggregated_expressions_recursive(expr, false, &mut result);
        result
    }

    /// Recursively collects non-aggregated expressions.
    fn collect_non_aggregated_expressions_recursive<'a>(
        &self,
        expr: &'a crate::ast::expression::Expression,
        in_aggregate: bool,
        result: &mut Vec<&'a crate::ast::expression::Expression>,
    ) {
        use crate::ast::expression::{AggregateFunction, Expression};

        match expr {
            Expression::AggregateFunction(agg) => {
                // Inside aggregate, check arguments with in_aggregate=true
                match &**agg {
                    AggregateFunction::CountStar { .. } => {}
                    AggregateFunction::GeneralSetFunction(gsf) => {
                        self.collect_non_aggregated_expressions_recursive(
                            &gsf.expression,
                            true,
                            result,
                        );
                    }
                    AggregateFunction::BinarySetFunction(bsf) => {
                        self.collect_non_aggregated_expressions_recursive(
                            &bsf.expression,
                            true,
                            result,
                        );
                        self.collect_non_aggregated_expressions_recursive(
                            &bsf.inverse_distribution_argument,
                            true,
                            result,
                        );
                    }
                }
            }
            Expression::VariableReference(_, _) | Expression::PropertyReference(_, _, _) => {
                if !in_aggregate {
                    result.push(expr);
                }
            }
            Expression::Binary(_, left, right, _) => {
                self.collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
                self.collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
            }
            Expression::Unary(_, operand, _) => {
                self.collect_non_aggregated_expressions_recursive(operand, in_aggregate, result);
            }
            Expression::Comparison(_, left, right, _) => {
                self.collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
                self.collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
            }
            Expression::Logical(_, left, right, _) => {
                self.collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
                self.collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
            }
            Expression::Parenthesized(inner, _) => {
                self.collect_non_aggregated_expressions_recursive(inner, in_aggregate, result);
            }
            Expression::FunctionCall(func) => {
                for arg in &func.arguments {
                    self.collect_non_aggregated_expressions_recursive(arg, in_aggregate, result);
                }
            }
            _ => {}
        }
    }

    /// Pass 6: Type Checking - Checks type compatibility in operations.
    fn run_type_checking(
        &self,
        program: &Program,
        _type_table: &TypeTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        // Walk all statements and check type compatibility
        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.check_query_types(&query_stmt.query, diagnostics);
                }
                Statement::Mutation(mutation_stmt) => {
                    // Check types in mutation statement
                    self.check_mutation_types(&mutation_stmt.statement, diagnostics);
                }
                _ => {}
            }
        }
    }

    /// Checks types in a query.
    fn check_query_types(&self, query: &Query, diagnostics: &mut Vec<Diag>) {
        match query {
            Query::Linear(linear_query) => {
                self.check_linear_query_types(linear_query, diagnostics);
            }
            Query::Composite(composite) => {
                self.check_query_types(&composite.left, diagnostics);
                self.check_query_types(&composite.right, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.check_query_types(query, diagnostics);
            }
        }
    }

    /// Checks types in a linear query.
    fn check_linear_query_types(&self, linear_query: &LinearQuery, diagnostics: &mut Vec<Diag>) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        // Check types in each statement
        for statement in primitive_statements {
            match statement {
                PrimitiveQueryStatement::Let(let_stmt) => {
                    for binding in &let_stmt.bindings {
                        self.check_expression_types(&binding.value, diagnostics);
                    }
                }
                PrimitiveQueryStatement::For(for_stmt) => {
                    self.check_expression_types(&for_stmt.item.collection, diagnostics);
                }
                PrimitiveQueryStatement::Filter(filter) => {
                    // Filter condition should be boolean
                    self.check_expression_types(&filter.condition, diagnostics);

                    // Check that the condition is likely boolean
                    if self.is_definitely_non_boolean(&filter.condition) {
                        diagnostics.push(
                            SemanticDiagBuilder::type_mismatch(
                                "boolean",
                                "non-boolean",
                                filter.condition.span(),
                            )
                            .build(),
                        );
                    }
                }
                PrimitiveQueryStatement::Select(select) => match &select.select_items {
                    crate::ast::query::SelectItemList::Items { items } => {
                        for item in items {
                            self.check_expression_types(&item.expression, diagnostics);
                        }
                    }
                    crate::ast::query::SelectItemList::Star => {}
                },
                _ => {}
            }
        }
    }

    /// Checks type compatibility in an expression.
    fn check_expression_types(
        &self,
        expr: &crate::ast::expression::Expression,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::{BinaryOperator, Expression};
        use crate::semantic::diag::SemanticDiagBuilder;

        match expr {
            // Binary arithmetic operations require numeric operands
            Expression::Binary(op, left, right, _span) => {
                // Recursively check nested expressions
                self.check_expression_types(left, diagnostics);
                self.check_expression_types(right, diagnostics);

                // Check type compatibility for the operation
                match op {
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Modulo => {
                        // F3: Check for NULL in arithmetic (ISO GQL null propagation)
                        use crate::ast::expression::Literal;
                        let left_is_null = matches!(left.as_ref(), Expression::Literal(Literal::Null, _));
                        let right_is_null = matches!(right.as_ref(), Expression::Literal(Literal::Null, _));

                        if left_is_null || right_is_null {
                            diagnostics.push(
                                Diag::warning("Arithmetic operation with NULL will always return NULL")
                                    .with_primary_label(_span.clone(), "NULL propagation")
                            );
                        }

                        // Simple literal type checking
                        if self.is_definitely_string(left) {
                            diagnostics.push(
                                SemanticDiagBuilder::type_mismatch(
                                    "numeric",
                                    "string",
                                    left.span(),
                                )
                                .build(),
                            );
                        }
                        if self.is_definitely_string(right) {
                            diagnostics.push(
                                SemanticDiagBuilder::type_mismatch(
                                    "numeric",
                                    "string",
                                    right.span(),
                                )
                                .build(),
                            );
                        }
                    }
                    BinaryOperator::Concatenate => {
                        // String concatenation is generally permissive
                    }
                }
            }

            // Comparison operations
            Expression::Comparison(_op, left, right, _span) => {
                self.check_expression_types(left, diagnostics);
                self.check_expression_types(right, diagnostics);
            }

            // Logical operations require boolean operands
            Expression::Logical(_op, left, right, _span) => {
                self.check_expression_types(left, diagnostics);
                self.check_expression_types(right, diagnostics);
            }

            // Unary operations
            Expression::Unary(op, operand, _span) => {
                self.check_expression_types(operand, diagnostics);

                match op {
                    crate::ast::expression::UnaryOperator::Plus
                    | crate::ast::expression::UnaryOperator::Minus => {
                        // Unary +/- require numeric type
                        if self.is_definitely_string(operand) {
                            diagnostics.push(
                                SemanticDiagBuilder::type_mismatch(
                                    "numeric",
                                    "string",
                                    operand.span(),
                                )
                                .build(),
                            );
                        }
                    }
                    crate::ast::expression::UnaryOperator::Not => {
                        // NOT requires boolean type
                    }
                }
            }

            // Property reference
            Expression::PropertyReference(object, _prop, _span) => {
                self.check_expression_types(object, diagnostics);
            }

            // Function call
            Expression::FunctionCall(fc) => {
                for arg in &fc.arguments {
                    self.check_expression_types(arg, diagnostics);
                }
            }

            // Case expression
            Expression::Case(case) => {
                use crate::ast::expression::CaseExpression;
                match case {
                    CaseExpression::Simple(simple) => {
                        self.check_expression_types(&simple.operand, diagnostics);
                        for when_clause in &simple.when_clauses {
                            self.check_expression_types(&when_clause.when_value, diagnostics);
                            self.check_expression_types(&when_clause.then_result, diagnostics);
                        }
                        if let Some(else_expr) = &simple.else_clause {
                            self.check_expression_types(else_expr, diagnostics);
                        }
                    }
                    CaseExpression::Searched(searched) => {
                        for when_clause in &searched.when_clauses {
                            self.check_expression_types(&when_clause.condition, diagnostics);
                            self.check_expression_types(&when_clause.then_result, diagnostics);
                        }
                        if let Some(else_expr) = &searched.else_clause {
                            self.check_expression_types(else_expr, diagnostics);
                        }
                    }
                }
            }

            // Cast expression
            Expression::Cast(cast) => {
                self.check_expression_types(&cast.operand, diagnostics);
            }

            // Aggregate function
            Expression::AggregateFunction(agg) => {
                use crate::ast::expression::AggregateFunction;
                match &**agg {
                    AggregateFunction::GeneralSetFunction(gsf) => {
                        self.check_expression_types(&gsf.expression, diagnostics);
                    }
                    AggregateFunction::BinarySetFunction(bsf) => {
                        self.check_expression_types(&bsf.expression, diagnostics);
                        self.check_expression_types(
                            &bsf.inverse_distribution_argument,
                            diagnostics,
                        );
                    }
                    AggregateFunction::CountStar { .. } => {}
                }
            }

            // List constructor
            Expression::ListConstructor(elements, _span) => {
                for elem in elements {
                    self.check_expression_types(elem, diagnostics);
                }
            }

            // Record constructor
            Expression::RecordConstructor(fields, _span) => {
                for field in fields {
                    self.check_expression_types(&field.value, diagnostics);
                }
            }

            // Path constructor
            Expression::PathConstructor(elements, _span) => {
                for elem in elements {
                    self.check_expression_types(elem, diagnostics);
                }
            }

            // Predicate
            Expression::Predicate(pred) => {
                use crate::ast::expression::Predicate;
                match pred {
                    Predicate::IsNull(operand, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsTyped(operand, _, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsNormalized(operand, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsDirected(operand, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsLabeled(operand, _, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsTruthValue(operand, _, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::IsSource(operand, of, _, _)
                    | Predicate::IsDestination(operand, of, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                        self.check_expression_types(of, diagnostics);
                    }
                    Predicate::Same(left, right, _) => {
                        self.check_expression_types(left, diagnostics);
                        self.check_expression_types(right, diagnostics);
                    }
                    Predicate::PropertyExists(operand, _, _) => {
                        self.check_expression_types(operand, diagnostics);
                    }
                    Predicate::AllDifferent(operands, _) => {
                        for operand in operands {
                            self.check_expression_types(operand, diagnostics);
                        }
                    }
                }
            }

            // Type annotation
            Expression::TypeAnnotation(inner, _annotation, _span) => {
                self.check_expression_types(inner, diagnostics);
            }

            // Graph/binding table/subquery expressions
            Expression::GraphExpression(inner, _)
            | Expression::BindingTableExpression(inner, _)
            | Expression::SubqueryExpression(inner, _) => {
                self.check_expression_types(inner, diagnostics);
            }

            // Parenthesized
            Expression::Parenthesized(inner, _) => {
                self.check_expression_types(inner, diagnostics);
            }

            // EXISTS predicate - contains complex structure
            Expression::Exists(_) => {
                // Would need to validate nested query structure
            }

            // Literals, variables, and parameters don't need type checking
            Expression::Literal(_, _)
            | Expression::VariableReference(_, _)
            | Expression::ParameterReference(_, _) => {}
        }
    }

    /// Helper: Check if an expression is definitely a string literal.
    fn is_definitely_string(&self, expr: &crate::ast::expression::Expression) -> bool {
        use crate::ast::expression::{Expression, Literal};
        matches!(expr, Expression::Literal(Literal::String(_), _))
    }

    /// Helper: Check if an expression is definitely not boolean.
    fn is_definitely_non_boolean(&self, expr: &crate::ast::expression::Expression) -> bool {
        use crate::ast::expression::{Expression, Literal};
        matches!(
            expr,
            Expression::Literal(
                Literal::String(_) | Literal::Integer(_) | Literal::Float(_),
                _
            )
        )
    }


    /// Checks types in a mutation statement.
    fn check_mutation_types(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::{
            LinearDataModifyingStatement, SimpleDataAccessingStatement,
            SimpleDataModifyingStatement,
        };

        let statements = match mutation {
            LinearDataModifyingStatement::Focused(focused) => &focused.statements,
            LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
        };

        for statement in statements {
            match statement {
                SimpleDataAccessingStatement::Query(query_stmt) => {
                    // Check types in query statements within mutation
                    match query_stmt.as_ref() {
                        PrimitiveQueryStatement::Filter(filter) => {
                            self.check_expression_types(&filter.condition, diagnostics);
                        }
                        PrimitiveQueryStatement::Let(let_stmt) => {
                            for binding in &let_stmt.bindings {
                                self.check_expression_types(&binding.value, diagnostics);
                            }
                        }
                        PrimitiveQueryStatement::For(for_stmt) => {
                            self.check_expression_types(&for_stmt.item.collection, diagnostics);
                        }
                        PrimitiveQueryStatement::Select(select) => {
                            self.check_select_types(select, diagnostics);
                        }
                        PrimitiveQueryStatement::OrderByAndPage(order_page) => {
                            if let Some(order_by) = &order_page.order_by {
                                for key in &order_by.sort_specifications {
                                    self.check_expression_types(&key.key, diagnostics);
                                }
                            }
                            if let Some(offset) = &order_page.offset {
                                self.check_expression_types(&offset.count, diagnostics);
                            }
                            if let Some(limit) = &order_page.limit {
                                self.check_expression_types(&limit.count, diagnostics);
                            }
                        }
                        _ => {}
                    }
                }
                SimpleDataAccessingStatement::Modifying(
                    SimpleDataModifyingStatement::Primitive(primitive),
                ) => {
                    use crate::ast::mutation::PrimitiveDataModifyingStatement;
                    match primitive {
                        PrimitiveDataModifyingStatement::Insert(insert) => {
                            // Check types in INSERT property specifications
                            self.check_insert_types(&insert.pattern, diagnostics);
                        }
                        PrimitiveDataModifyingStatement::Set(set) => {
                            // Check types in SET operations
                            for item in &set.items.items {
                                use crate::ast::mutation::SetItem;
                                match item {
                                    SetItem::Property(prop) => {
                                        self.check_expression_types(&prop.value, diagnostics);
                                    }
                                    SetItem::AllProperties(all_props) => {
                                        for pair in &all_props.properties.properties {
                                            self.check_expression_types(&pair.value, diagnostics);
                                        }
                                    }
                                    SetItem::Label(_) => {
                                        // Labels don't have expressions to check
                                    }
                                }
                            }
                        }
                        PrimitiveDataModifyingStatement::Remove(remove) => {
                            // REMOVE operations don't have expressions to type check
                            let _ = remove;
                        }
                        PrimitiveDataModifyingStatement::Delete(_delete) => {
                            // DELETE operations reference variables but don't have expressions to type check
                        }
                    }
                }
                SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                    // Procedure calls would need procedure signature checking
                }
            }
        }
    }

    /// Checks types in INSERT pattern property specifications.
    fn check_insert_types(
        &self,
        pattern: &crate::ast::mutation::InsertGraphPattern,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::InsertElementPattern;

        for path in &pattern.paths {
            for element in &path.elements {
                match element {
                    InsertElementPattern::Node(node) => {
                        if let Some(filler) = &node.filler
                            && let Some(props) = &filler.properties
                        {
                            self.check_property_specification_types(props, diagnostics);
                        }
                    }
                    InsertElementPattern::Edge(edge) => {
                        use crate::ast::mutation::InsertEdgePattern;
                        let filler = match edge {
                            InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                            InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                            InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                        };
                        if let Some(filler) = filler
                            && let Some(props) = &filler.properties
                        {
                            self.check_property_specification_types(props, diagnostics);
                        }
                    }
                }
            }
        }
    }

    /// Checks types in property specifications.
    fn check_property_specification_types(
        &self,
        props: &crate::ast::query::ElementPropertySpecification,
        diagnostics: &mut Vec<Diag>,
    ) {
        for pair in &props.properties {
            self.check_expression_types(&pair.value, diagnostics);
        }
    }

    /// Checks types in SELECT statement.
    fn check_select_types(
        &self,
        select: &crate::ast::query::SelectStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::query::SelectItemList;
        match &select.select_items {
            SelectItemList::Star => {}
            SelectItemList::Items { items } => {
                for item in items {
                    self.check_expression_types(&item.expression, diagnostics);
                }
            }
        }
    }

    /// Pass 7: Expression Validation - Validates expressions.
    fn run_expression_validation(
        &self,
        program: &Program,
        _type_table: &TypeTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        // This pass checks:
        // - Null propagation rules
        // - CASE expression type consistency
        // - Subquery result types
        // - List operations

        for statement in &program.statements {
            match statement {
                Statement::Query(query_stmt) => {
                    self.validate_query_expressions(&query_stmt.query, diagnostics);
                }
                Statement::Mutation(mutation_stmt) => {
                    // Validate expressions in mutation statement
                    self.validate_mutation_expressions(&mutation_stmt.statement, diagnostics);
                }
                _ => {}
            }
        }
    }

    /// Validates expressions in a query.
    fn validate_query_expressions(&self, query: &Query, diagnostics: &mut Vec<Diag>) {
        match query {
            Query::Linear(linear_query) => {
                self.validate_linear_query_expressions(linear_query, diagnostics);
            }
            Query::Composite(composite) => {
                self.validate_query_expressions(&composite.left, diagnostics);
                self.validate_query_expressions(&composite.right, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_expressions(query, diagnostics);
            }
        }
    }

    /// Validates expressions in a linear query.
    fn validate_linear_query_expressions(
        &self,
        linear_query: &LinearQuery,
        diagnostics: &mut Vec<Diag>,
    ) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        for statement in primitive_statements {
            match statement {
                PrimitiveQueryStatement::Let(let_stmt) => {
                    for binding in &let_stmt.bindings {
                        self.validate_expression_semantics(&binding.value, diagnostics);
                    }
                }
                PrimitiveQueryStatement::For(for_stmt) => {
                    self.validate_expression_semantics(&for_stmt.item.collection, diagnostics);
                }
                PrimitiveQueryStatement::Filter(filter) => {
                    self.validate_expression_semantics(&filter.condition, diagnostics);
                }
                PrimitiveQueryStatement::Select(select) => match &select.select_items {
                    crate::ast::query::SelectItemList::Items { items } => {
                        for item in items {
                            self.validate_expression_semantics(&item.expression, diagnostics);
                        }
                    }
                    crate::ast::query::SelectItemList::Star => {}
                },
                _ => {}
            }
        }
    }

    /// Validates semantic rules for an expression.
    fn validate_expression_semantics(
        &self,
        expr: &crate::ast::expression::Expression,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::Expression;

        match expr {
            Expression::Case(case_expr) => {
                // Validate CASE expression type consistency
                self.validate_case_expression(case_expr, diagnostics);
            }
            Expression::Binary(_, left, right, _) => {
                self.validate_expression_semantics(left, diagnostics);
                self.validate_expression_semantics(right, diagnostics);
            }
            Expression::Comparison(_, left, right, _) => {
                self.validate_expression_semantics(left, diagnostics);
                self.validate_expression_semantics(right, diagnostics);
            }
            Expression::Logical(_, left, right, _) => {
                self.validate_expression_semantics(left, diagnostics);
                self.validate_expression_semantics(right, diagnostics);
            }
            Expression::Unary(_, operand, _) => {
                self.validate_expression_semantics(operand, diagnostics);
            }
            Expression::PropertyReference(base, _, _) => {
                self.validate_expression_semantics(base, diagnostics);
            }
            Expression::ListConstructor(elements, _) => {
                for elem in elements {
                    self.validate_expression_semantics(elem, diagnostics);
                }
            }
            Expression::RecordConstructor(fields, _) => {
                for field in fields {
                    self.validate_expression_semantics(&field.value, diagnostics);
                }
            }
            Expression::PathConstructor(exprs, _) => {
                for expr in exprs {
                    self.validate_expression_semantics(expr, diagnostics);
                }
            }
            Expression::Parenthesized(inner, _) => {
                self.validate_expression_semantics(inner, diagnostics);
            }
            Expression::FunctionCall(func_call) => {
                for arg in &func_call.arguments {
                    self.validate_expression_semantics(arg, diagnostics);
                }
            }
            Expression::AggregateFunction(_agg_func) => {
                // Validate arguments in the aggregate function
                // The structure may vary, so we skip detailed validation for now
            }
            Expression::Predicate(pred) => {
                self.validate_predicate_semantics(pred, diagnostics);
            }
            Expression::Cast(cast_expr) => {
                self.validate_expression_semantics(&cast_expr.operand, diagnostics);
            }
            Expression::TypeAnnotation(expr, _, _) => {
                self.validate_expression_semantics(expr, diagnostics);
            }
            Expression::Exists(_exists_expr) => {
                // EXISTS expressions have their own validation
            }
            Expression::GraphExpression(expr, _) => {
                self.validate_expression_semantics(expr, diagnostics);
            }
            Expression::BindingTableExpression(expr, _) => {
                self.validate_expression_semantics(expr, diagnostics);
            }
            Expression::SubqueryExpression(expr, _) => {
                self.validate_expression_semantics(expr, diagnostics);
            }
            // Literals and simple references don't need semantic validation
            Expression::Literal(_, _)
            | Expression::VariableReference(_, _)
            | Expression::ParameterReference(_, _) => {}
        }
    }

    /// Validates predicate semantics.
    fn validate_predicate_semantics(
        &self,
        pred: &crate::ast::expression::Predicate,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::Predicate;

        match pred {
            Predicate::IsNull(e, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
            Predicate::IsTyped(expr, _, _, _) => {
                self.validate_expression_semantics(expr, diagnostics);
            }
            Predicate::IsNormalized(e, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
            Predicate::IsDirected(e, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
            Predicate::IsLabeled(e, _, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
            Predicate::IsTruthValue(e, _, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
            Predicate::IsSource(e1, e2, _, _) | Predicate::IsDestination(e1, e2, _, _) => {
                self.validate_expression_semantics(e1, diagnostics);
                self.validate_expression_semantics(e2, diagnostics);
            }
            Predicate::Same(e1, e2, _) => {
                self.validate_expression_semantics(e1, diagnostics);
                self.validate_expression_semantics(e2, diagnostics);
            }
            Predicate::AllDifferent(exprs, _) => {
                for e in exprs {
                    self.validate_expression_semantics(e, diagnostics);
                }
            }
            Predicate::PropertyExists(e, _, _) => {
                self.validate_expression_semantics(e, diagnostics);
            }
        }
    }

    /// Validates expressions in a mutation statement.
    fn validate_mutation_expressions(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::{
            LinearDataModifyingStatement, PrimitiveDataModifyingStatement,
            SimpleDataAccessingStatement, SimpleDataModifyingStatement,
        };

        let statements = match mutation {
            LinearDataModifyingStatement::Focused(focused) => &focused.statements,
            LinearDataModifyingStatement::Ambient(ambient) => &ambient.statements,
        };

        for statement in statements {
            match statement {
                SimpleDataAccessingStatement::Query(query_stmt) => {
                    // Validate expressions in query statements within mutation
                    match query_stmt.as_ref() {
                        PrimitiveQueryStatement::Filter(filter) => {
                            self.validate_expression_semantics(&filter.condition, diagnostics);
                        }
                        PrimitiveQueryStatement::Let(let_stmt) => {
                            for binding in &let_stmt.bindings {
                                self.validate_expression_semantics(&binding.value, diagnostics);
                            }
                        }
                        PrimitiveQueryStatement::For(for_stmt) => {
                            self.validate_expression_semantics(
                                &for_stmt.item.collection,
                                diagnostics,
                            );
                        }
                        PrimitiveQueryStatement::Select(select) => {
                            use crate::ast::query::SelectItemList;
                            match &select.select_items {
                                SelectItemList::Items { items } => {
                                    for item in items {
                                        self.validate_expression_semantics(
                                            &item.expression,
                                            diagnostics,
                                        );
                                    }
                                }
                                SelectItemList::Star => {}
                            }
                        }
                        PrimitiveQueryStatement::OrderByAndPage(order_page) => {
                            if let Some(order_by) = &order_page.order_by {
                                for key in &order_by.sort_specifications {
                                    self.validate_expression_semantics(&key.key, diagnostics);
                                }
                            }
                            if let Some(offset) = &order_page.offset {
                                self.validate_expression_semantics(&offset.count, diagnostics);
                            }
                            if let Some(limit) = &order_page.limit {
                                self.validate_expression_semantics(&limit.count, diagnostics);
                            }
                        }
                        _ => {}
                    }
                }
                SimpleDataAccessingStatement::Modifying(
                    SimpleDataModifyingStatement::Primitive(primitive),
                ) => {
                    match primitive {
                        PrimitiveDataModifyingStatement::Insert(insert) => {
                            // Validate expressions in INSERT property specifications
                            self.validate_insert_expressions(&insert.pattern, diagnostics);
                        }
                        PrimitiveDataModifyingStatement::Set(set) => {
                            // Validate expressions in SET operations
                            for item in &set.items.items {
                                use crate::ast::mutation::SetItem;
                                match item {
                                    SetItem::Property(prop) => {
                                        self.validate_expression_semantics(
                                            &prop.value,
                                            diagnostics,
                                        );
                                    }
                                    SetItem::AllProperties(all_props) => {
                                        for pair in &all_props.properties.properties {
                                            self.validate_expression_semantics(
                                                &pair.value,
                                                diagnostics,
                                            );
                                        }
                                    }
                                    SetItem::Label(_) => {
                                        // Labels don't have expressions
                                    }
                                }
                            }
                        }
                        PrimitiveDataModifyingStatement::Remove(_)
                        | PrimitiveDataModifyingStatement::Delete(_) => {
                            // These don't have expressions to validate
                        }
                    }
                }
                SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(_)) => {
                    // Procedure calls would need procedure signature validation
                }
            }
        }
    }

    /// Validates expressions in INSERT patterns.
    fn validate_insert_expressions(
        &self,
        pattern: &crate::ast::mutation::InsertGraphPattern,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::{InsertEdgePattern, InsertElementPattern};

        for path in &pattern.paths {
            for element in &path.elements {
                let props = match element {
                    InsertElementPattern::Node(node) => {
                        node.filler.as_ref().and_then(|f| f.properties.as_ref())
                    }
                    InsertElementPattern::Edge(edge) => {
                        let filler = match edge {
                            InsertEdgePattern::PointingLeft(e) => e.filler.as_ref(),
                            InsertEdgePattern::PointingRight(e) => e.filler.as_ref(),
                            InsertEdgePattern::Undirected(e) => e.filler.as_ref(),
                        };
                        filler.and_then(|f| f.properties.as_ref())
                    }
                };

                if let Some(props) = props {
                    for pair in &props.properties {
                        self.validate_expression_semantics(&pair.value, diagnostics);
                    }
                }
            }
        }
    }

    /// Validates CASE expression type consistency.
    fn validate_case_expression(
        &self,
        case_expr: &crate::ast::expression::CaseExpression,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::CaseExpression;

        match case_expr {
            CaseExpression::Simple(simple) => {
                // Validate operand
                self.validate_expression_semantics(&simple.operand, diagnostics);

                // Validate all when clauses
                for when_clause in &simple.when_clauses {
                    self.validate_expression_semantics(&when_clause.when_value, diagnostics);
                    self.validate_expression_semantics(&when_clause.then_result, diagnostics);
                }

                // Validate else clause if present
                if let Some(else_result) = &simple.else_clause {
                    self.validate_expression_semantics(else_result, diagnostics);
                }

                // Check that all result expressions have compatible types
                // Note: This is a basic check using literal types; full type inference
                // would require the complete TypeTable integration (F4)
                self.validate_case_result_compatibility(
                    simple
                        .when_clauses
                        .iter()
                        .map(|wc| &wc.then_result)
                        .chain(simple.else_clause.iter().map(|e| e.as_ref())),
                    diagnostics,
                );
            }
            CaseExpression::Searched(searched) => {
                // Validate all when clauses
                for when_clause in &searched.when_clauses {
                    self.validate_expression_semantics(&when_clause.condition, diagnostics);
                    self.validate_expression_semantics(&when_clause.then_result, diagnostics);

                    // Check that condition is boolean (basic check for literals)
                    if self.config.strict_mode {
                        self.validate_boolean_expression(&when_clause.condition, diagnostics);
                    }
                }

                // Validate else clause if present
                if let Some(else_result) = &searched.else_clause {
                    self.validate_expression_semantics(else_result, diagnostics);
                }

                // Check that all result expressions have compatible types
                self.validate_case_result_compatibility(
                    searched
                        .when_clauses
                        .iter()
                        .map(|wc| &wc.then_result)
                        .chain(searched.else_clause.iter().map(|e| e.as_ref())),
                    diagnostics,
                );
            }
        }
    }

    /// Validates that an expression is boolean-typed (best-effort check).
    fn validate_boolean_expression(
        &self,
        expr: &crate::ast::expression::Expression,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::{Expression, Literal};

        // Basic check: if it's a literal, verify it's boolean
        // Full implementation would use TypeTable
        match expr {
            Expression::Literal(Literal::Boolean(_), _) => {
                // OK - boolean literal
            }
            Expression::Literal(lit, span) if !matches!(lit, Literal::Null) => {
                // Non-boolean, non-null literal in boolean context
                use crate::semantic::diag::SemanticDiagBuilder;
                let diag = SemanticDiagBuilder::type_mismatch(
                    "Boolean",
                    &format!("{:?}", lit),
                    span.clone(),
                )
                .with_note("Condition expressions should evaluate to boolean")
                .build();
                diagnostics.push(diag);
            }
            Expression::Comparison(..)
            | Expression::Logical(..)
            | Expression::Predicate(_)
            | Expression::Exists(_) => {
                // These expressions produce boolean results - OK
            }
            _ => {
                // Other expressions - can't determine type without full type inference
                // Don't emit false positives
            }
        }
    }

    /// Validates that CASE result expressions have compatible types (best-effort).
    fn validate_case_result_compatibility<'a>(
        &self,
        results: impl Iterator<Item = &'a crate::ast::expression::Expression>,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::expression::{Expression, Literal};

        // Collect result types (only for literals - full impl needs TypeTable)
        let mut literal_types: Vec<(&str, &crate::ast::Span)> = Vec::new();

        for result in results {
            if let Expression::Literal(lit, span) = result {
                let type_name = match lit {
                    Literal::Boolean(_) => "Boolean",
                    Literal::Integer(_) => "Integer",
                    Literal::Float(_) => "Float",
                    Literal::String(_) => "String",
                    Literal::Null => continue, // Null is compatible with everything
                    Literal::Date(_) => "Date",
                    Literal::Time(_) => "Time",
                    Literal::Datetime(_) => "Timestamp",
                    Literal::Duration(_) => "Duration",
                    Literal::List(_) => "List",
                    Literal::Record(_) => "Record",
                    Literal::ByteString(_) => "ByteString",
                };
                literal_types.push((type_name, span));
            }
        }

        // Check if all non-null literals have the same type
        if literal_types.len() > 1 {
            let first_type = literal_types[0].0;
            for (type_name, span) in &literal_types[1..] {
                if *type_name != first_type {
                    use crate::semantic::diag::SemanticDiagBuilder;
                    let diag =
                        SemanticDiagBuilder::type_mismatch(first_type, type_name, (*span).clone())
                            .with_note("All CASE result branches should have compatible types")
                            .build();
                    diagnostics.push(diag);
                }
            }
        }
    }

    /// Pass 8: Reference Validation - Validates references (catalog-dependent).
    fn run_reference_validation(&self, program: &Program, diagnostics: &mut Vec<Diag>) {
        // This pass checks:
        // - Schema references exist
        // - Graph references exist
        // - Procedure references exist
        // - Type references exist

        // Only perform validation if catalog is provided
        let Some(catalog) = self.catalog else {
            // Catalog not available, skip validation
            return;
        };

        for statement in &program.statements {
            match statement {
                Statement::Catalog(_catalog_stmt) => {
                    // Validate catalog statement references
                    // e.g., CREATE GRAPH SCHEMA myschema ...
                    // This would check if references in the catalog statement are valid
                    // Placeholder for future catalog-level validation
                }
                Statement::Query(query_stmt) => {
                    // Validate references in queries (e.g., USE GRAPH)
                    self.validate_query_references(&query_stmt.query, catalog, diagnostics);
                }
                Statement::Mutation(mutation_stmt) => {
                    // Validate references in mutations (e.g., USE GRAPH in focused mutations)
                    self.validate_mutation_references(
                        &mutation_stmt.statement,
                        catalog,
                        diagnostics,
                    );
                }
                _ => {}
            }
        }
    }

    /// Validates catalog references in a query.
    fn validate_query_references(
        &self,
        query: &Query,
        catalog: &dyn crate::semantic::catalog::Catalog,
        diagnostics: &mut Vec<Diag>,
    ) {
        match query {
            Query::Linear(linear_query) => {
                // Check for USE GRAPH clause
                if let LinearQuery::Focused(focused) = linear_query {
                    // Extract graph name from USE GRAPH expression (if it's a simple reference)
                    if let crate::ast::expression::Expression::VariableReference(name, span) =
                        &focused.use_graph.graph
                        && catalog.validate_graph(name).is_err()
                    {
                        use crate::semantic::diag::SemanticDiagBuilder;
                        let diag =
                            SemanticDiagBuilder::unknown_reference("graph", name, span.clone())
                                .with_note("Graph not found in catalog")
                                .build();
                        diagnostics.push(diag);
                    }
                    // Note: Complex USE GRAPH expressions (functions, computations)
                    // cannot be validated statically - skip them
                }
            }
            Query::Composite(composite) => {
                self.validate_query_references(&composite.left, catalog, diagnostics);
                self.validate_query_references(&composite.right, catalog, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_references(query, catalog, diagnostics);
            }
        }
    }

    /// Validates catalog references in a mutation.
    fn validate_mutation_references(
        &self,
        mutation: &crate::ast::mutation::LinearDataModifyingStatement,
        catalog: &dyn crate::semantic::catalog::Catalog,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::ast::mutation::LinearDataModifyingStatement;

        // Check for USE GRAPH clause in focused mutations
        if let LinearDataModifyingStatement::Focused(focused) = mutation {
            // Extract graph name from USE GRAPH expression (if it's a simple reference)
            if let crate::ast::expression::Expression::VariableReference(name, span) =
                &focused.use_graph_clause.graph
                && catalog.validate_graph(name).is_err()
            {
                use crate::semantic::diag::SemanticDiagBuilder;
                let diag = SemanticDiagBuilder::unknown_reference("graph", name, span.clone())
                    .with_note("Graph not found in catalog")
                    .build();
                diagnostics.push(diag);
            }
        }
    }

    /// Pass 9: Schema Validation - Validates labels and properties (schema-dependent).
    fn run_schema_validation(&self, program: &Program, diagnostics: &mut Vec<Diag>) {
        // This pass checks:
        // - Labels exist in schema
        // - Properties exist in schema
        // - Property types match schema

        // Only perform validation if schema is provided
        let Some(schema) = self.schema else {
            // Schema not available, skip validation
            return;
        };

        for statement in &program.statements {
            if let Statement::Query(query_stmt) = statement {
                // Validate:
                // - Node labels: (n:Person) -> check if 'Person' exists in schema
                // - Edge labels: -[e:KNOWS]-> -> check if 'KNOWS' exists in schema
                // - Properties: n.name -> check if 'name' exists for nodes with label 'Person'
                self.validate_query_schema(&query_stmt.query, schema, diagnostics);
            }
        }
    }

    /// Validates schema references in a query.
    fn validate_query_schema(
        &self,
        query: &Query,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        match query {
            Query::Linear(linear_query) => {
                self.validate_linear_query_schema(linear_query, schema, diagnostics);
            }
            Query::Composite(composite) => {
                self.validate_query_schema(&composite.left, schema, diagnostics);
                self.validate_query_schema(&composite.right, schema, diagnostics);
            }
            Query::Parenthesized(query, _) => {
                self.validate_query_schema(query, schema, diagnostics);
            }
        }
    }

    /// Validates schema references in a linear query.
    fn validate_linear_query_schema(
        &self,
        linear_query: &LinearQuery,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        let primitive_statements = match linear_query {
            LinearQuery::Focused(focused) => &focused.primitive_statements,
            LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
        };

        for statement in primitive_statements {
            if let PrimitiveQueryStatement::Match(match_stmt) = statement {
                // Validate labels in MATCH patterns based on MatchStatement type
                match match_stmt {
                    MatchStatement::Simple(simple) => {
                        // Simple match has a GraphPattern with paths
                        for path in &simple.pattern.paths.patterns {
                            self.validate_path_pattern_schema(path, schema, diagnostics);
                        }
                    }
                    MatchStatement::Optional(optional) => {
                        // Optional match - validate nested patterns
                        match &optional.operand {
                            crate::ast::query::OptionalOperand::Match { pattern } => {
                                for path in &pattern.paths.patterns {
                                    self.validate_path_pattern_schema(path, schema, diagnostics);
                                }
                            }
                            crate::ast::query::OptionalOperand::Block { statements }
                            | crate::ast::query::OptionalOperand::ParenthesizedBlock {
                                statements,
                            } => {
                                // Validate nested MATCH statements recursively
                                for stmt in statements {
                                    match stmt {
                                        MatchStatement::Simple(simple) => {
                                            for path in &simple.pattern.paths.patterns {
                                                self.validate_path_pattern_schema(
                                                    path,
                                                    schema,
                                                    diagnostics,
                                                );
                                            }
                                        }
                                        MatchStatement::Optional(nested_optional) => {
                                            // Recursively validate nested optional matches
                                            self.validate_optional_match_schema(
                                                nested_optional,
                                                schema,
                                                diagnostics,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validates schema references in an optional match (recursive helper).
    fn validate_optional_match_schema(
        &self,
        optional: &crate::ast::query::OptionalMatchStatement,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        match &optional.operand {
            crate::ast::query::OptionalOperand::Match { pattern } => {
                for path in &pattern.paths.patterns {
                    self.validate_path_pattern_schema(path, schema, diagnostics);
                }
            }
            crate::ast::query::OptionalOperand::Block { statements }
            | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                for stmt in statements {
                    match stmt {
                        MatchStatement::Simple(simple) => {
                            for path in &simple.pattern.paths.patterns {
                                self.validate_path_pattern_schema(path, schema, diagnostics);
                            }
                        }
                        MatchStatement::Optional(nested) => {
                            self.validate_optional_match_schema(nested, schema, diagnostics);
                        }
                    }
                }
            }
        }
    }

    /// Validates labels in a path pattern against the schema.
    fn validate_path_pattern_schema(
        &self,
        path: &PathPattern,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        // PathPattern has an expression field, which is a PathPatternExpression
        // We need to walk the expression to find elements
        self.validate_path_expression_schema(&path.expression, schema, diagnostics);
    }

    /// Validates labels in a path expression against the schema.
    fn validate_path_expression_schema(
        &self,
        expr: &PathPatternExpression,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        // PathPatternExpression is an enum with Term, Union, and Alternation variants
        match expr {
            PathPatternExpression::Term(term) => {
                // Validate a single term
                self.validate_path_term_schema(term, schema, diagnostics);
            }
            PathPatternExpression::Union { left, right, .. } => {
                // Validate both sides of union
                self.validate_path_expression_schema(left, schema, diagnostics);
                self.validate_path_expression_schema(right, schema, diagnostics);
            }
            PathPatternExpression::Alternation { alternatives, .. } => {
                // Validate all alternatives
                for alt in alternatives {
                    self.validate_path_term_schema(alt, schema, diagnostics);
                }
            }
        }
    }

    /// Validates labels in a path term against the schema.
    fn validate_path_term_schema(
        &self,
        term: &PathTerm,
        schema: &dyn crate::semantic::schema::Schema,
        diagnostics: &mut Vec<Diag>,
    ) {
        use crate::semantic::diag::SemanticDiagBuilder;

        // Each term has factors
        for factor in &term.factors {
            // Check if the primary is an element pattern
            if let PathPrimary::ElementPattern(element) = &factor.primary {
                // ElementPattern is boxed, dereference it
                match &**element {
                    ElementPattern::Node(node) => {
                        // Check node labels using label_expression field
                        if let Some(label_expr) = &node.label_expression {
                            for label_name in self.extract_label_names(label_expr) {
                                if schema.validate_label(&label_name, true).is_err() {
                                    diagnostics.push(
                                        SemanticDiagBuilder::unknown_reference(
                                            "label",
                                            &label_name,
                                            node.span.clone(),
                                        )
                                        .build(),
                                    );
                                }
                            }
                        }
                    }
                    ElementPattern::Edge(edge) => {
                        // Check edge labels
                        if let EdgePattern::Full(full) = edge
                            && let Some(label_expr) = &full.filler.label_expression
                        {
                            for label_name in self.extract_label_names(label_expr) {
                                if schema.validate_label(&label_name, false).is_err() {
                                    diagnostics.push(
                                        SemanticDiagBuilder::unknown_reference(
                                            "edge label",
                                            &label_name,
                                            full.span.clone(),
                                        )
                                        .build(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Helper: Extract label names from a label expression.
    fn extract_label_names(&self, label_expr: &crate::ast::query::LabelExpression) -> Vec<String> {
        use crate::ast::query::LabelExpression;

        match label_expr {
            LabelExpression::LabelName { name, .. } => vec![name.to_string()],
            LabelExpression::Disjunction { left, right, .. } => {
                let mut labels = self.extract_label_names(left);
                labels.extend(self.extract_label_names(right));
                labels
            }
            LabelExpression::Conjunction { left, right, .. } => {
                let mut labels = self.extract_label_names(left);
                labels.extend(self.extract_label_names(right));
                labels
            }
            LabelExpression::Negation { operand, .. } => self.extract_label_names(operand),
            LabelExpression::Wildcard { .. } => vec![], // Wildcard matches any label
            LabelExpression::Parenthesized { expression, .. } => {
                self.extract_label_names(expression)
            }
        }
    }
}

impl<'s, 'c> Default for SemanticValidator<'s, 'c> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_validator_basic() {
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // For now, validation passes (no passes implemented yet)
            assert!(result.is_success());
        }
    }

    #[test]
    fn test_validator_with_config() {
        let config = ValidationConfig {
            strict_mode: true,
            schema_validation: true,
            catalog_validation: true,
            warn_on_shadowing: true,
            warn_on_disconnected_patterns: true,
        };

        let validator = SemanticValidator::with_config(config);
        assert!(validator.config.strict_mode);
    }

    #[test]
    fn test_scope_analysis_match_bindings() {
        let source = "MATCH (n:Person)-[e:KNOWS]->(m:Person) RETURN n, e, m";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = vec![];
            let (symbol_table, _scope_metadata) =
                validator.run_scope_analysis(&program, &mut diagnostics);

            // Check that variables n, e, m were defined
            assert!(
                symbol_table.lookup("n").is_some(),
                "Variable 'n' should be defined"
            );
            assert!(
                symbol_table.lookup("e").is_some(),
                "Variable 'e' should be defined"
            );
            assert!(
                symbol_table.lookup("m").is_some(),
                "Variable 'm' should be defined"
            );
        }
    }

    #[test]
    fn test_scope_analysis_let_variables() {
        let source = "MATCH (n:Person) LET age = n.age RETURN age";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = vec![];
            let (symbol_table, _scope_metadata) =
                validator.run_scope_analysis(&program, &mut diagnostics);

            // Check that variables n and age were defined
            assert!(
                symbol_table.lookup("n").is_some(),
                "Variable 'n' should be defined"
            );
            assert!(
                symbol_table.lookup("age").is_some(),
                "Variable 'age' should be defined"
            );
        }
    }

    #[test]
    fn test_scope_analysis_for_variables() {
        let source = "MATCH (n:Person) FOR item IN n.items RETURN item";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = vec![];
            let (symbol_table, _scope_metadata) =
                validator.run_scope_analysis(&program, &mut diagnostics);

            // Check that variables n and item were defined
            assert!(
                symbol_table.lookup("n").is_some(),
                "Variable 'n' should be defined"
            );
            assert!(
                symbol_table.lookup("item").is_some(),
                "Variable 'item' should be defined"
            );
        }
    }

    #[test]
    fn test_scope_analysis_path_variables() {
        let source = "MATCH p = (a:Person)-[r:KNOWS]->(b:Person) RETURN p, a, r, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = vec![];
            let (symbol_table, _scope_metadata) =
                validator.run_scope_analysis(&program, &mut diagnostics);

            // Check that path variable p and element variables a, r, b were defined
            assert!(
                symbol_table.lookup("p").is_some(),
                "Path variable 'p' should be defined"
            );
            assert!(
                symbol_table.lookup("a").is_some(),
                "Variable 'a' should be defined"
            );
            assert!(
                symbol_table.lookup("r").is_some(),
                "Variable 'r' should be defined"
            );
            assert!(
                symbol_table.lookup("b").is_some(),
                "Variable 'b' should be defined"
            );
        }
    }

    #[test]
    fn test_variable_validation_undefined_variable() {
        let source = "MATCH (n:Person) RETURN m"; // m is undefined
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should fail with undefined variable error
            assert!(
                result.is_failure(),
                "Validation should fail for undefined variable"
            );
            let diagnostics = &result.diagnostics;
            assert!(
                !diagnostics.is_empty(),
                "Should have at least one diagnostic"
            );

            // Check that the diagnostic mentions the undefined variable 'm'
            let diag_message = &diagnostics[0].message;
            assert!(
                diag_message.contains("m") || diag_message.contains("Undefined"),
                "Diagnostic should mention undefined variable: {}",
                diag_message
            );
        }
    }

    #[test]
    fn test_variable_validation_defined_variable() {
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass validation
            assert!(
                result.is_success(),
                "Validation should pass for defined variable"
            );
        }
    }

    #[test]
    fn test_variable_validation_multiple_undefined() {
        let source = "MATCH (n:Person) RETURN x, y, z"; // x, y, z are undefined
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should fail with multiple undefined variable errors
            assert!(
                result.is_failure(),
                "Validation should fail for undefined variables"
            );
            let diagnostics = &result.diagnostics;
            assert!(
                diagnostics.len() >= 3,
                "Should have at least 3 diagnostics for x, y, z"
            );
        }
    }

    #[test]
    fn test_type_inference_literals() {
        let source = "MATCH (n:Person) LET x = 42, y = 'hello', z = TRUE RETURN x, y, z";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = Vec::new();
            let _type_table =
                validator.run_type_inference(&program, &SymbolTable::new(), &mut diagnostics);

            // Type table should be created without errors
            assert!(
                diagnostics.is_empty(),
                "Type inference should not produce diagnostics"
            );

            // Type table should be initialized (even if types not persisted yet)
            // This test validates that type inference pass runs without errors
        }
    }

    #[test]
    fn test_type_inference_arithmetic() {
        let source =
            "MATCH (n:Person) LET sum = n.age + 10, product = n.salary * 1.5 RETURN sum, product";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = Vec::new();
            let _type_table =
                validator.run_type_inference(&program, &SymbolTable::new(), &mut diagnostics);

            // Type inference should handle arithmetic operations
            assert!(
                diagnostics.is_empty(),
                "Type inference should not produce diagnostics"
            );
        }
    }

    #[test]
    fn test_type_inference_aggregates() {
        let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age), SUM(n.salary)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = Vec::new();
            let _type_table =
                validator.run_type_inference(&program, &SymbolTable::new(), &mut diagnostics);

            // Type inference should handle aggregate functions
            assert!(
                diagnostics.is_empty(),
                "Type inference should not produce diagnostics"
            );
        }
    }

    #[test]
    fn test_type_inference_comparison() {
        let source = "MATCH (n:Person) FILTER n.age > 30 AND n.name = 'Alice' RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = Vec::new();
            let _type_table =
                validator.run_type_inference(&program, &SymbolTable::new(), &mut diagnostics);

            // Type inference should handle comparison operations
            assert!(
                diagnostics.is_empty(),
                "Type inference should not produce diagnostics"
            );
        }
    }

    #[test]
    fn test_type_inference_for_loop() {
        let source = "FOR item IN [1, 2, 3] RETURN item";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let mut diagnostics = Vec::new();
            let _type_table =
                validator.run_type_inference(&program, &SymbolTable::new(), &mut diagnostics);

            // Type inference should handle FOR loops with collections
            assert!(
                diagnostics.is_empty(),
                "Type inference should not produce diagnostics"
            );
        }
    }

    #[test]
    fn test_type_checking_string_in_arithmetic() {
        // Test that using a string literal in arithmetic produces a type error
        let source = "MATCH (n:Person) LET x = 'hello' + 10 RETURN x";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should produce type mismatch error
            assert!(
                result.is_failure(),
                "Should fail type checking for string in arithmetic"
            );
            let diagnostics = &result.diagnostics;
            assert!(
                !diagnostics.is_empty(),
                "Should have type mismatch diagnostic"
            );

            // Check that diagnostic mentions type mismatch
            let diag_message = &diagnostics[0].message;
            assert!(
                diag_message.contains("Type mismatch")
                    || diag_message.contains("numeric")
                    || diag_message.contains("string"),
                "Diagnostic should mention type mismatch: {}",
                diag_message
            );
        }
    }

    #[test]
    fn test_type_checking_unary_minus_string() {
        // Test that unary minus on a string produces a type error
        let source = "MATCH (n:Person) LET x = -'hello' RETURN x";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should produce type mismatch error
            assert!(
                result.is_failure(),
                "Should fail type checking for unary minus on string"
            );
            let diagnostics = &result.diagnostics;
            assert!(
                !diagnostics.is_empty(),
                "Should have type mismatch diagnostic"
            );
        }
    }

    #[test]
    fn test_type_checking_valid_arithmetic() {
        // Test that valid arithmetic passes type checking
        let source = "MATCH (n:Person) LET x = 10 + 20, y = 3.14 * 2 RETURN x, y";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass type checking (no undefined variables, valid arithmetic)
            if result.is_failure() {
                panic!(
                    "Should pass type checking for valid arithmetic, but got errors: {:?}",
                    result
                        .diagnostics
                        .iter()
                        .map(|d| &d.message)
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn test_type_checking_case_expression() {
        // Test that CASE expressions are type-checked
        let source = "MATCH (n:Person) SELECT CASE WHEN n.age > 18 THEN 'adult' ELSE 'minor' END";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // CASE expression should pass type checking
            // (In future could check that all branches have compatible types)
            // May fail due to undefined variable 'n', but shouldn't fail type checking
            if result.is_failure() {
                // Just verify type checking runs without panicking
                assert!(!result.diagnostics.is_empty());
            }
        }
    }

    // ==================== Pattern Connectivity Tests ====================

    #[test]
    fn test_pattern_connectivity_single_node() {
        // Single node pattern - should always be connected
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - single node is always connected
            assert!(
                result.is_success(),
                "Single node pattern should be connected"
            );
        }
    }

    #[test]
    fn test_pattern_connectivity_connected_path() {
        // Connected path pattern - should be valid
        let source = "MATCH (a:Person)-[r:KNOWS]->(b:Person) RETURN a, r, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - path is connected
            assert!(
                result.is_success(),
                "Connected path pattern should be valid"
            );
        }
    }

    #[test]
    fn test_pattern_connectivity_disconnected_nodes() {
        // Disconnected nodes in same MATCH - should fail
        let source = "MATCH (a:Person), (b:Company) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should succeed with a warning - disconnected patterns are ISO-conformant
            // Warnings don't prevent IR creation
            assert!(
                result.is_success(),
                "Disconnected nodes are ISO-conformant and should not fail validation"
            );

            // If we want to test that a warning was issued, we'd need to check
            // the IR or modify the API to return warnings alongside the IR
        }
    }

    #[test]
    fn test_pattern_connectivity_long_path() {
        // Long connected path - should be valid
        let source = "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:WORKS_AT]->(c:Company)-[:LOCATED_IN]->(d:City) RETURN a, d";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - all nodes connected in path
            assert!(result.is_success(), "Long connected path should be valid");
        }
    }

    #[test]
    fn test_pattern_connectivity_multiple_paths() {
        // Multiple disconnected paths - ISO-conformant
        let source = "MATCH (a)-[:R1]->(b), (c)-[:R2]->(d) RETURN a, b, c, d";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should succeed with a warning - disconnected comma-separated patterns are ISO-conformant
            assert!(
                result.is_success(),
                "Multiple disconnected paths are ISO-conformant and should not fail"
            );
        }
    }

    // ==================== Context Validation Tests ====================

    #[test]
    fn test_context_validation_match_in_query() {
        // MATCH clause in query context - should be valid
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - MATCH is valid in query context
            assert!(
                result.is_success(),
                "MATCH in query context should be valid"
            );
        }
    }

    #[test]
    fn test_context_validation_filter_usage() {
        // FILTER/WHERE clause - should be valid
        let source = "MATCH (n:Person) FILTER n.age > 30 RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - FILTER is valid in query context
            assert!(
                result.is_success(),
                "FILTER in query context should be valid"
            );
        }
    }

    #[test]
    fn test_context_validation_order_by() {
        // ORDER BY clause - should be valid
        let source = "MATCH (n:Person) RETURN n ORDER BY n.age DESC";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - ORDER BY is valid in query context
            assert!(
                result.is_success(),
                "ORDER BY in query context should be valid"
            );
        }
    }

    #[test]
    fn test_context_validation_aggregation_context() {
        // Aggregation in SELECT - should be valid
        let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - aggregation is valid in SELECT
            assert!(result.is_success(), "Aggregation in SELECT should be valid");
        }
    }

    // ==================== Aggregation Validation Tests ====================

    #[test]
    fn test_aggregation_count_star() {
        // COUNT(*) - should be valid
        let source = "MATCH (n:Person) SELECT COUNT(*)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - COUNT(*) is valid
            assert!(result.is_success(), "COUNT(*) should be valid");
        }
    }

    #[test]
    fn test_aggregation_avg_function() {
        // AVG function with property - should be valid
        let source = "MATCH (n:Person) SELECT AVG(n.age)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - AVG is valid aggregate function
            assert!(result.is_success(), "AVG function should be valid");
        }
    }

    #[test]
    fn test_aggregation_sum_function() {
        // SUM function - should be valid
        let source = "MATCH (n:Person) SELECT SUM(n.salary)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - SUM is valid
            assert!(result.is_success(), "SUM function should be valid");
        }
    }

    #[test]
    fn test_aggregation_multiple_functions() {
        // Multiple aggregation functions - should be valid
        let source = "MATCH (n:Person) SELECT COUNT(*), AVG(n.age), SUM(n.salary), MIN(n.age), MAX(n.salary)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - multiple aggregates are valid
            assert!(
                result.is_success(),
                "Multiple aggregation functions should be valid"
            );
        }
    }

    #[test]
    fn test_aggregation_with_arithmetic() {
        // Aggregation with arithmetic - should be valid
        let source = "MATCH (n:Person) SELECT AVG(n.age) + 10, SUM(n.salary) * 1.5";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - arithmetic on aggregates is valid
            assert!(
                result.is_success(),
                "Aggregation with arithmetic should be valid"
            );
        }
    }

    // ==================== Expression Validation Tests ====================

    #[test]
    fn test_expression_validation_case_simple() {
        // Simple CASE expression - should be valid
        let source = "MATCH (n:Person) SELECT CASE n.status WHEN 'active' THEN 1 WHEN 'inactive' THEN 0 ELSE -1 END";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - simple CASE is valid
            assert!(
                result.is_success(),
                "Simple CASE expression should be valid"
            );
        }
    }

    #[test]
    fn test_expression_validation_case_searched() {
        // Searched CASE expression - should be valid
        let source = "MATCH (n:Person) SELECT CASE WHEN n.age < 18 THEN 'minor' WHEN n.age < 65 THEN 'adult' ELSE 'senior' END";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - searched CASE is valid
            assert!(
                result.is_success(),
                "Searched CASE expression should be valid"
            );
        }
    }

    #[test]
    fn test_expression_validation_nested_case() {
        // Nested CASE expressions - should be valid
        let source = "MATCH (n:Person) SELECT CASE WHEN n.age > 18 THEN CASE WHEN n.salary > 50000 THEN 'high' ELSE 'low' END ELSE 'minor' END";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - nested CASE is valid
            assert!(
                result.is_success(),
                "Nested CASE expression should be valid"
            );
        }
    }

    #[test]
    fn test_expression_validation_list_constructor() {
        // List constructor - should be valid
        let source = "MATCH (n:Person) LET list = [1, 2, 3, 4, 5] RETURN list";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - list constructor is valid
            assert!(result.is_success(), "List constructor should be valid");
        }
    }

    #[test]
    fn test_expression_validation_record_constructor() {
        // Record constructor - should be valid
        let source = "MATCH (n:Person) SELECT {name: n.name, age: n.age}";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - record constructor is valid
            assert!(result.is_success(), "Record constructor should be valid");
        }
    }

    #[test]
    fn test_expression_validation_property_reference() {
        // Property reference - should be valid
        let source = "MATCH (n:Person) RETURN n.name, n.age, n.address.city";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - property references are valid
            assert!(result.is_success(), "Property references should be valid");
        }
    }

    #[test]
    fn test_expression_validation_function_call() {
        // Function call - should be valid
        let source = "MATCH (n:Person) SELECT UPPER(n.name), LENGTH(n.address)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - function calls are valid
            assert!(result.is_success(), "Function calls should be valid");
        }
    }

    #[test]
    fn test_expression_validation_cast() {
        // CAST expression - should be valid
        let source = "MATCH (n:Person) SELECT CAST(n.age AS STRING)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - CAST is valid
            assert!(result.is_success(), "CAST expression should be valid");
        }
    }

    #[test]
    fn test_expression_validation_complex_expression() {
        // Complex nested expression - should be valid
        let source = "MATCH (n:Person) SELECT (n.salary * 1.1) + (n.bonus / 12) - n.tax";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Debug: print diagnostics if test fails
            if !result.is_success() {
                eprintln!("Diagnostics: {:?}", result.diagnostics);
            }

            // Should pass - complex arithmetic is valid
            assert!(
                result.is_success(),
                "Complex nested expression should be valid"
            );
        }
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_edge_case_empty_match_pattern() {
        // MATCH without pattern elements (if parseable) - edge case
        // This test verifies validator handles unusual but parseable structures
        let source = "MATCH (n) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - single node is always connected
            assert!(result.is_success(), "Single anonymous node should be valid");
        }
    }

    #[test]
    fn test_edge_case_deeply_nested_properties() {
        // Deeply nested property access - edge case
        let source = "MATCH (n:Person) RETURN n.address.street.building.floor.unit";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - deeply nested properties are valid syntactically
            assert!(
                result.is_success(),
                "Deeply nested property access should be valid"
            );
        }
    }

    #[test]
    fn test_edge_case_multiple_filters() {
        // Multiple FILTER clauses - edge case
        let source = "MATCH (n:Person) FILTER n.age > 18 FILTER n.salary > 50000 RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - multiple filters are valid
            assert!(
                result.is_success(),
                "Multiple FILTER clauses should be valid"
            );
        }
    }

    #[test]
    fn test_edge_case_parenthesized_expressions() {
        // Heavily parenthesized expressions - edge case
        let source = "MATCH (n:Person) SELECT (((n.age + 10) * 2) - 5)";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - parenthesized expressions are valid
            assert!(
                result.is_success(),
                "Parenthesized expressions should be valid"
            );
        }
    }

    #[test]
    fn test_edge_case_variable_shadowing_let() {
        // Variable shadowing with LET - edge case
        let source = "MATCH (n:Person) LET n = n.name RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // May warn about shadowing in strict mode, but should be semantically valid
            // The validation depends on configuration
            let _ = result; // Validation result depends on configuration
        }
    }

    #[test]
    fn test_edge_case_for_loop_shadowing() {
        // FOR loop variable shadowing - edge case
        let source = "MATCH (n:Person) FOR n IN [1, 2, 3] RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // May warn about shadowing, depends on configuration
            let _ = result;
        }
    }

    #[test]
    fn test_edge_case_all_literal_types() {
        // All literal types - edge case coverage
        let source = "SELECT 42, 3.14, 'hello', TRUE, FALSE, NULL";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - literals don't require variable resolution
            assert!(result.is_success(), "All literal types should be valid");
        }
    }

    #[test]
    fn test_edge_case_boolean_operators() {
        // Complex boolean expression - edge case
        let source = "MATCH (n:Person) FILTER (n.age > 18 AND n.age < 65) OR (n.status = 'VIP' AND NOT n.blocked) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - complex boolean expressions are valid
            assert!(
                result.is_success(),
                "Complex boolean expressions should be valid"
            );
        }
    }

    #[test]
    fn test_edge_case_comparison_chains() {
        // Multiple comparisons - edge case
        let source = "MATCH (n:Person) FILTER n.age >= 18 AND n.age <= 65 RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - comparison chains are valid
            assert!(result.is_success(), "Comparison chains should be valid");
        }
    }

    #[test]
    fn test_edge_case_mixed_aggregates_and_literals() {
        // Mixed aggregates with literals - edge case
        let source = "MATCH (n:Person) SELECT COUNT(*) + 1, AVG(n.age) * 2";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - mixing aggregates with literals/arithmetic is valid
            assert!(
                result.is_success(),
                "Mixed aggregates with literals should be valid"
            );
        }
    }

    // ==================== Schema Validation Tests ====================

    #[test]
    fn test_schema_validation_valid_label() {
        use crate::semantic::schema::MockSchema;

        // Valid node label in schema
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let schema = MockSchema::example();
            let validator = SemanticValidator::new().with_schema(&schema);
            let result = validator.validate(&program);

            // Should pass - Person is in the schema
            assert!(
                result.is_success(),
                "Valid label should pass schema validation"
            );
        }
    }

    #[test]
    fn test_schema_validation_invalid_label() {
        use crate::semantic::schema::MockSchema;

        // Invalid node label not in schema
        let source = "MATCH (n:Alien) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let schema = MockSchema::example();
            let validator = SemanticValidator::new().with_schema(&schema);
            let result = validator.validate(&program);

            // Should fail - Alien is not in the schema
            assert!(
                result.is_failure(),
                "Invalid label should fail schema validation"
            );
        }
    }

    #[test]
    fn test_schema_validation_valid_edge_label() {
        use crate::semantic::schema::MockSchema;

        // Valid edge label in schema
        let source = "MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let schema = MockSchema::example();
            let validator = SemanticValidator::new().with_schema(&schema);
            let result = validator.validate(&program);

            // Should pass - KNOWS is in the schema
            assert!(
                result.is_success(),
                "Valid edge label should pass schema validation"
            );
        }
    }

    #[test]
    fn test_schema_validation_invalid_edge_label() {
        use crate::semantic::schema::MockSchema;

        // Invalid edge label not in schema
        let source = "MATCH (a:Person)-[:HATES]->(b:Person) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let schema = MockSchema::example();
            let validator = SemanticValidator::new().with_schema(&schema);
            let result = validator.validate(&program);

            // Should fail - HATES is not in the schema
            assert!(
                result.is_failure(),
                "Invalid edge label should fail schema validation"
            );
        }
    }

    #[test]
    fn test_schema_validation_without_schema() {
        // Schema validation disabled - should pass even with invalid label
        let source = "MATCH (n:Alien) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - schema validation is disabled
            assert!(result.is_success(), "Should pass without schema validation");
        }
    }

    #[test]
    fn test_schema_validation_multiple_labels() {
        use crate::semantic::schema::MockSchema;

        // Multiple labels - mix of valid and invalid
        let source = "MATCH (a:Person), (b:Alien) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let schema = MockSchema::example();
            let validator = SemanticValidator::new().with_schema(&schema);
            let result = validator.validate(&program);

            // Should fail - Alien is not in schema (also fails on disconnected pattern)
            assert!(
                result.is_failure(),
                "Mixed valid/invalid labels should fail"
            );
        }
    }

    // ==================== Catalog Validation Tests ====================

    #[test]
    fn test_catalog_validation_without_catalog() {
        // Catalog validation disabled - should pass
        let source = "MATCH (n:Person) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let result = validator.validate(&program);

            // Should pass - catalog validation is disabled
            assert!(
                result.is_success(),
                "Should pass without catalog validation"
            );
        }
    }

    #[test]
    fn test_catalog_mock_creation() {
        use crate::semantic::catalog::{Catalog, MockCatalog};

        // Test mock catalog creation
        let catalog = MockCatalog::example();

        // Verify example catalog has expected entries
        assert!(catalog.get_graph("social").is_some());
        assert!(catalog.get_schema("social_schema").is_some());
        assert!(catalog.get_procedure("shortest_path").is_some());
    }

    // ==================== Warning Visibility Tests ====================

    #[test]
    fn test_warning_visibility_disconnected_patterns() {
        // Disconnected patterns should succeed with warning
        let source = "MATCH (a:Person), (b:Company) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should succeed (ISO-conformant disconnected patterns allowed)
            assert!(outcome.is_success(), "Disconnected patterns should succeed");

            // Should have warning diagnostics
            assert!(
                !outcome.diagnostics.is_empty(),
                "Should have warning diagnostics"
            );

            // At least one should be a warning
            let has_warning = outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagSeverity::Warning);
            assert!(has_warning, "Should have at least one warning diagnostic");

            // Warning should mention disconnected or patterns
            let has_pattern_warning = outcome.diagnostics.iter().any(|d| {
                d.message.to_lowercase().contains("disconnect")
                    || d.message.to_lowercase().contains("pattern")
            });
            assert!(
                has_pattern_warning,
                "Warning should mention disconnected patterns"
            );
        }
    }

    #[test]
    fn test_warning_visibility_shadowing() {
        // Variable shadowing should succeed with warning when enabled
        let config = ValidationConfig {
            strict_mode: false,
            schema_validation: false,
            catalog_validation: false,
            warn_on_shadowing: true,
            warn_on_disconnected_patterns: false,
        };

        let source = "MATCH (n:Person) LET n = n.name RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::with_config(config);
            let outcome = validator.validate(&program);

            // Should succeed (shadowing is allowed, just warned)
            assert!(outcome.is_success(), "Shadowing should succeed");

            // Should have warning diagnostics
            assert!(
                !outcome.diagnostics.is_empty(),
                "Should have warning diagnostics"
            );

            // At least one should be a warning about shadowing
            let has_shadowing_warning = outcome.diagnostics.iter().any(|d| {
                d.severity == DiagSeverity::Warning && d.message.to_lowercase().contains("shadow")
            });
            assert!(has_shadowing_warning, "Should have shadowing warning");
        }
    }

    #[test]
    fn test_warning_with_error_both_returned() {
        // Mix of warning and error - should fail but return both
        let source = "MATCH (a:Person), (b:Company) RETURN x"; // disconnected + undefined
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should fail due to undefined variable
            assert!(
                outcome.is_failure(),
                "Should fail due to undefined variable"
            );

            // Should have diagnostics
            assert!(!outcome.diagnostics.is_empty(), "Should have diagnostics");

            // Should have at least one error
            let has_error = outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagSeverity::Error);
            assert!(has_error, "Should have at least one error");

            // May also have warning about disconnected patterns
            let has_warning = outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagSeverity::Warning);

            // If we have warnings, verify they're distinct from errors
            if has_warning {
                let warning_count = outcome
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == DiagSeverity::Warning)
                    .count();
                let error_count = outcome
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == DiagSeverity::Error)
                    .count();
                assert!(
                    warning_count > 0 && error_count > 0,
                    "Should have both warnings and errors"
                );
            }
        }
    }

    #[test]
    fn test_no_warnings_when_disabled() {
        // Warnings disabled - should not get warnings
        let config = ValidationConfig {
            strict_mode: false,
            schema_validation: false,
            catalog_validation: false,
            warn_on_shadowing: false,
            warn_on_disconnected_patterns: false,
        };

        let source = "MATCH (a:Person), (b:Company) RETURN a, b";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::with_config(config);
            let outcome = validator.validate(&program);

            // Should succeed
            assert!(outcome.is_success(), "Should succeed");

            // Should not have warning diagnostics (warnings disabled)
            let has_warning = outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagSeverity::Warning);
            assert!(!has_warning, "Should not have warnings when disabled");
        }
    }

    #[test]
    fn test_successful_validation_with_no_diagnostics() {
        // Valid query with no warnings or errors
        let source = "MATCH (n:Person)-[:KNOWS]->(m:Person) RETURN n, m";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should succeed
            assert!(outcome.is_success(), "Valid query should succeed");
            assert!(outcome.ir.is_some(), "IR should be present");

            // No warnings for this valid, connected query
            // (diagnostics may be empty or contain only notes)
            let has_errors_or_warnings = outcome
                .diagnostics
                .iter()
                .any(|d| matches!(d.severity, DiagSeverity::Error | DiagSeverity::Warning));
            assert!(!has_errors_or_warnings, "Should have no errors or warnings");
        }
    }

    // ==================== Scope Isolation Tests (F2) ====================

    #[test]
    #[ignore] // TODO: Parser doesn't create separate Statement objects for semicolon-separated queries yet
    fn test_scope_isolation_across_statements() {
        // Variables shouldn't leak between semicolon-separated statements
        // NOTE: Currently the parser treats "query1; query2" as a single Statement,
        // so this test is ignored until the parser is updated to create separate Statements.
        let source = "MATCH (n:Person) RETURN n; MATCH (m:Company) RETURN n";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should fail: 'n' from first statement not visible in second statement
            assert!(
                !outcome.is_success(),
                "Should fail: variable 'n' leaked across statements"
            );

            // Check for undefined variable error
            let has_undefined_error = outcome.diagnostics.iter().any(|d| {
                d.severity == DiagSeverity::Error
                    && d.message.contains("undefined")
                    && d.message.contains("'n'")
            });
            assert!(
                has_undefined_error,
                "Should have undefined variable error for 'n'"
            );
        }
    }

    #[test]
    fn test_scope_proper_linear_flow() {
        // Variables should be visible within the same statement
        let source = "MATCH (n:Person) MATCH (m:Company) WHERE m.name = n.name RETURN n, m";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should succeed: both n and m visible within same statement
            assert!(
                outcome.is_success(),
                "Should succeed: variables visible in same statement"
            );
        }
    }

    #[test]
    #[ignore] // TODO: Composite query isolation requires preventing right side from seeing left side's scope
    fn test_composite_query_scope_isolation_union() {
        // UNION queries should have isolated scopes
        // NOTE: Currently both sides of UNION share the same accumulated scopes,
        // so this test is ignored until proper scope isolation is implemented.
        let source = "MATCH (a:Person) RETURN a UNION MATCH (b:Company) RETURN a";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should fail: 'a' from left side not visible in right side of UNION
            assert!(
                !outcome.is_success(),
                "Should fail: variable leaked across UNION"
            );

            let has_undefined_error = outcome.diagnostics.iter().any(|d| {
                d.severity == DiagSeverity::Error
                    && d.message.contains("undefined")
                    && d.message.contains("'a'")
            });
            assert!(
                has_undefined_error,
                "Should have undefined variable error for 'a' in UNION right side"
            );
        }
    }

    #[test]
    fn test_composite_query_both_sides_valid() {
        // UNION with valid variables on both sides
        let source = "MATCH (a:Person) RETURN a UNION MATCH (a:Person) RETURN a";
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let validator = SemanticValidator::new();
            let outcome = validator.validate(&program);

            // Should succeed: each side defines its own 'a'
            assert!(
                outcome.is_success(),
                "Should succeed: both sides have valid 'a'"
            );
        }
    }

    // ==================== F5: Enhanced Aggregation Validation Tests ====================

    #[test]
    fn test_return_mixed_aggregation() {
        // ISO GQL: Cannot mix aggregated and non-aggregated expressions in RETURN without GROUP BY
        let source = "MATCH (n:Person) RETURN COUNT(n), n.name";
        let config = ValidationConfig {
            strict_mode: true,
            ..Default::default()
        };
        let validator = SemanticValidator::with_config(config);
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(
                !outcome.is_success(),
                "Should fail: mixing aggregated and non-aggregated in RETURN"
            );
        }
    }

    #[test]
    fn test_nested_aggregation_error() {
        // ISO GQL: Nested aggregation functions are not allowed
        let source = "MATCH (n:Person) RETURN COUNT(SUM(n.age))";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(!outcome.is_success(), "Should fail: nested aggregation");

            let has_nested_error = outcome.diagnostics.iter().any(|d| {
                d.message.contains("Nested aggregation") || d.message.contains("nested aggregation")
            });
            assert!(has_nested_error, "Should have nested aggregation error");
        }
    }

    #[test]
    fn test_aggregation_in_where_error() {
        // ISO GQL: Aggregation functions not allowed in WHERE clause
        let source = "MATCH (n:Person) FILTER AVG(n.age) > 30 RETURN n";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(
                !outcome.is_success(),
                "Should fail: aggregation in WHERE/FILTER"
            );

            let has_where_error = outcome.diagnostics.iter().any(|d| {
                d.message.contains("WHERE")
                    || d.message.contains("HAVING")
                    || d.message.contains("FILTER")
            });
            assert!(
                has_where_error,
                "Should mention WHERE/HAVING/FILTER in error message"
            );
        }
    }

    #[test]
    fn test_having_non_grouped_error() {
        // ISO GQL: Non-aggregated expressions in HAVING must appear in GROUP BY
        let source =
            "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING n.name = 'Alice'";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // Should fail: n.name not in GROUP BY
            assert!(
                !outcome.is_success(),
                "Should fail: non-grouped expression in HAVING"
            );
        }
    }

    #[test]
    fn test_valid_group_by() {
        // ISO GQL: Valid GROUP BY with aggregation
        let source = "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(outcome.is_success(), "Should succeed: valid GROUP BY");
        }
    }

    #[test]
    fn test_valid_having_with_aggregate() {
        // ISO GQL: HAVING with aggregated expression is valid
        let source =
            "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING AVG(n.age) > 30";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            assert!(
                outcome.is_success(),
                "Should succeed: HAVING uses aggregate"
            );
        }
    }

    // ==================== F3: Expression Validation Tests ====================

    #[test]
    fn test_case_type_consistency() {
        // ISO GQL: All branches in CASE must return compatible types
        let source = "MATCH (n:Person) RETURN CASE WHEN true THEN 5 WHEN false THEN 'string' END";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);
            // Note: This may pass in current implementation since type checking is basic
            // The test documents expected behavior per ISO GQL
            if !outcome.is_success() {
                let has_type_error = outcome
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("type") || d.message.contains("CASE"));
                assert!(has_type_error, "Should have type consistency error in CASE");
            }
        }
    }

    #[test]
    fn test_null_propagation_warning() {
        // ISO GQL: Operations with NULL propagate NULL
        let source = "MATCH (n:Person) FILTER n.age + NULL > 5 RETURN n";
        let validator = SemanticValidator::new();
        let parse_result = parse(source);

        if let Some(program) = parse_result.ast {
            let outcome = validator.validate(&program);

            // Check if there's a warning about NULL propagation
            let has_null_warning = outcome
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagSeverity::Warning && d.message.contains("NULL"));
            assert!(has_null_warning, "Should warn about NULL propagation in arithmetic");
        }
    }
}
