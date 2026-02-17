//! GraphQL parser with rich diagnostics.
//!
//! This library provides a GraphQL parser with comprehensive error reporting
//! built on miette for beautiful diagnostic messages.

pub mod ast;
pub mod diag;

// Re-export key types for convenience
pub use ast::{Span, Spanned};
pub use diag::{
    convert_diag_to_report, convert_diagnostics_to_reports, Diag, DiagLabel, DiagSeverity,
    LabelRole, SourceFile,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_accessible() {
        // Verify that key types are accessible through the public API
        let _span: Span = 0..5;
        let _spanned = Spanned::new(42, 0..5);
        let _diag = Diag::error("test").with_primary_label(0..5, "here");
        let _source = SourceFile::new("test");
    }
}

