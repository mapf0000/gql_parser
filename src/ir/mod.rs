//! Intermediate Representation (IR) for GQL queries.
//!
//! The IR enriches the AST with semantic information:
//! - Symbol table with variable bindings
//! - Type table with expression types
//! - Scope information
//! - Resolution information (references to definitions)
//!
//! The IR maintains references to the original AST and preserves all source
//! location information for diagnostics.

pub mod symbol_table;
pub mod type_table;

use crate::ast::Program;
use crate::diag::Diag;
pub use symbol_table::SymbolTable;
pub use type_table::TypeTable;

/// Intermediate Representation enriching AST with semantic information.
#[derive(Debug, Clone)]
pub struct IR {
    /// The original AST from parsing.
    program: Program,

    /// Symbol table tracking variable bindings and scopes.
    symbol_table: SymbolTable,

    /// Type table tracking expression types.
    type_table: TypeTable,
}

impl IR {
    /// Creates a new IR from an AST and semantic analysis results.
    pub fn new(program: Program, symbol_table: SymbolTable, type_table: TypeTable) -> Self {
        Self {
            program,
            symbol_table,
            type_table,
        }
    }

    /// Returns a reference to the original AST program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns a reference to the symbol table.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a reference to the type table.
    pub fn type_table(&self) -> &TypeTable {
        &self.type_table
    }
}

/// Result type for semantic validation operations.
pub type ValidationResult = Result<IR, Vec<Diag>>;

/// Outcome of semantic validation, always carrying diagnostics.
///
/// This allows warnings and notes to be reported even when validation succeeds
/// and IR is produced.
#[derive(Debug, Clone)]
pub struct ValidationOutcome {
    /// The IR, if validation produced no errors (warnings are allowed).
    pub ir: Option<IR>,

    /// All diagnostics collected during validation (errors, warnings, notes).
    pub diagnostics: Vec<Diag>,
}

impl ValidationOutcome {
    /// Creates a successful outcome with IR and optional diagnostics (warnings/notes).
    pub fn success(ir: IR, diagnostics: Vec<Diag>) -> Self {
        Self {
            ir: Some(ir),
            diagnostics,
        }
    }

    /// Creates a failed outcome with errors (and optional warnings/notes).
    pub fn failure(diagnostics: Vec<Diag>) -> Self {
        Self {
            ir: None,
            diagnostics,
        }
    }

    /// Returns true if validation succeeded (IR is available).
    pub fn is_success(&self) -> bool {
        self.ir.is_some()
    }

    /// Returns true if validation failed (no IR available).
    pub fn is_failure(&self) -> bool {
        self.ir.is_none()
    }

    /// Returns true if there are any diagnostics (errors, warnings, or notes).
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
