//! Internal diagnostic model for syntax-phase errors, warnings, and notes.

use crate::ast::Span;
use miette::{Diagnostic, LabeledSpan, Report, Severity};
use std::fmt;

/// Severity level for a diagnostic.
///
/// This covers the full taxonomy required for syntax-phase diagnostics:
/// errors that prevent compilation, warnings about suspicious patterns,
/// and informational notes or advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagSeverity {
    /// A fatal error that prevents further processing.
    Error,
    /// A warning about potentially problematic code.
    Warning,
    /// An informational note or advice.
    Note,
}

impl fmt::Display for DiagSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagSeverity::Error => write!(f, "error"),
            DiagSeverity::Warning => write!(f, "warning"),
            DiagSeverity::Note => write!(f, "note"),
        }
    }
}

/// Role of a diagnostic label in the overall diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRole {
    /// The primary location related to this diagnostic.
    Primary,
    /// A secondary or supporting location.
    Secondary,
}

/// A labeled span within a diagnostic.
///
/// Each label associates a span with explanatory text and indicates
/// whether it's the primary focus or a supporting context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagLabel {
    /// The span this label refers to.
    pub span: Span,
    /// The label text explaining this span's relevance.
    pub message: String,
    /// Whether this is a primary or secondary label.
    pub role: LabelRole,
}

impl DiagLabel {
    /// Creates a new primary label.
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            role: LabelRole::Primary,
        }
    }

    /// Creates a new secondary label.
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            role: LabelRole::Secondary,
        }
    }
}

/// A structured diagnostic message.
///
/// This is the internal diagnostic representation used throughout the parser
/// and lexer. It captures all information needed to render rich error reports
/// with source context, multiple labeled spans, help text, and notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// The severity level of this diagnostic.
    pub severity: DiagSeverity,
    /// The main diagnostic message.
    pub message: String,
    /// Labeled spans showing relevant source locations.
    pub labels: Vec<DiagLabel>,
    /// Optional help text suggesting how to fix the issue.
    pub help: Option<String>,
    /// Additional notes providing context or related information.
    pub notes: Vec<String>,
    /// Optional diagnostic code (e.g., "E0001" or "syntax::unclosed_string").
    pub code: Option<String>,
}

impl Diag {
    /// Creates a new diagnostic with the given severity and message.
    pub fn new(severity: DiagSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            labels: Vec::new(),
            help: None,
            notes: Vec::new(),
            code: None,
        }
    }

    /// Creates a new error diagnostic.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(DiagSeverity::Error, message)
    }

    /// Creates a new warning diagnostic.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(DiagSeverity::Warning, message)
    }

    /// Creates a new note diagnostic.
    pub fn note(message: impl Into<String>) -> Self {
        Self::new(DiagSeverity::Note, message)
    }

    /// Adds a primary label to this diagnostic.
    pub fn with_primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagLabel::primary(span, message));
        self
    }

    /// Adds a secondary label to this diagnostic.
    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagLabel::secondary(span, message));
        self
    }

    /// Adds a label to this diagnostic.
    pub fn with_label(mut self, label: DiagLabel) -> Self {
        self.labels.push(label);
        self
    }

    /// Sets the help text for this diagnostic.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Adds a note to this diagnostic.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Sets the diagnostic code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// A wrapper around source text for diagnostic rendering.
///
/// This type manages source text ownership and provides safe access
/// for diagnostic conversion, ensuring spans are validated against
/// actual source bounds.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The source text content.
    content: String,
    /// Optional filename for display purposes.
    name: Option<String>,
}

impl SourceFile {
    /// Creates a new source file from the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            name: None,
        }
    }

    /// Creates a new source file with a name.
    pub fn with_name(content: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            name: Some(name.into()),
        }
    }

    /// Returns the source content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the source file name, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Validates that a span is within bounds of this source.
    pub fn is_valid_span(&self, span: &Span) -> bool {
        span.start <= self.content.len() && span.end <= self.content.len() && span.start <= span.end
    }

    /// Clamps a span to valid bounds within this source.
    pub fn clamp_span(&self, span: &Span) -> Span {
        let len = self.content.len();
        let start = span.start.min(len);
        let end = span.end.min(len).max(start);
        start..end
    }
}

/// Converts internal diagnostics to miette Reports with source context.
///
/// This function provides the bridge from our internal diagnostic model
/// to miette's rich error reporting. It safely handles invalid spans and
/// preserves all diagnostic information (labels, help, notes, severity).
pub fn convert_diagnostics_to_reports(diagnostics: &[Diag], source: &SourceFile) -> Vec<Report> {
    diagnostics
        .iter()
        .map(|diag| convert_diag_to_report(diag, source))
        .collect()
}

/// Converts a single diagnostic to a miette Report.
///
/// This handles the full conversion including:
/// - Mapping severity levels
/// - Converting labeled spans (primary and secondary)
/// - Attaching help text and notes
/// - Including diagnostic codes
/// - Safely handling out-of-bounds spans
pub fn convert_diag_to_report(diag: &Diag, source: &SourceFile) -> Report {
    let diagnostic = build_diagnostic(diag, source);

    // Create the report with source context
    let mut report = Report::new(diagnostic);

    // Attach source code if we have a filename
    if let Some(name) = source.name() {
        report =
            report.with_source_code(miette::NamedSource::new(name, source.content().to_string()));
    } else {
        report = report.with_source_code(source.content().to_string());
    }

    report
}

fn build_diagnostic(diag: &Diag, source: &SourceFile) -> BuiltDiagnostic {
    // Build the labels first
    let mut labels = Vec::new();
    for label in &diag.labels {
        let clamped_span = source.clamp_span(&label.span);
        let span = (clamped_span.start, clamped_span.end - clamped_span.start);
        let labeled_span = match label.role {
            LabelRole::Primary => {
                LabeledSpan::new_primary_with_span(Some(label.message.clone()), span)
            }
            LabelRole::Secondary => LabeledSpan::new_with_span(Some(label.message.clone()), span),
        };
        labels.push(labeled_span);
    }

    // Create the diagnostic struct
    BuiltDiagnostic {
        message: diag.message.clone(),
        severity: match diag.severity {
            DiagSeverity::Error => Severity::Error,
            DiagSeverity::Warning => Severity::Warning,
            DiagSeverity::Note => Severity::Advice,
        },
        code: diag.code.clone(),
        help: diag.help.clone(),
        labels,
        related: diag
            .notes
            .iter()
            .cloned()
            .map(NoteDiagnostic::new)
            .collect(),
    }
}

/// The final diagnostic type that implements miette's Diagnostic trait.
#[derive(Debug)]
struct BuiltDiagnostic {
    message: String,
    severity: Severity,
    code: Option<String>,
    help: Option<String>,
    labels: Vec<LabeledSpan>,
    related: Vec<NoteDiagnostic>,
}

#[derive(Debug)]
struct NoteDiagnostic {
    message: String,
}

impl NoteDiagnostic {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for NoteDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for BuiltDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BuiltDiagnostic {}
impl std::error::Error for NoteDiagnostic {}

impl Diagnostic for BuiltDiagnostic {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }

    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.code
            .as_ref()
            .map(|c| Box::new(c) as Box<dyn fmt::Display>)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            None
        } else {
            Some(Box::new(self.labels.clone().into_iter()))
        }
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        if self.related.is_empty() {
            None
        } else {
            Some(Box::new(
                self.related.iter().map(|diag| diag as &dyn Diagnostic),
            ))
        }
    }
}

impl Diagnostic for NoteDiagnostic {
    fn severity(&self) -> Option<Severity> {
        Some(Severity::Advice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_display() {
        assert_eq!(DiagSeverity::Error.to_string(), "error");
        assert_eq!(DiagSeverity::Warning.to_string(), "warning");
        assert_eq!(DiagSeverity::Note.to_string(), "note");
    }

    #[test]
    fn diag_label_primary() {
        let label = DiagLabel::primary(0..5, "unexpected token");
        assert_eq!(label.span, 0..5);
        assert_eq!(label.message, "unexpected token");
        assert_eq!(label.role, LabelRole::Primary);
    }

    #[test]
    fn diag_label_secondary() {
        let label = DiagLabel::secondary(10..15, "defined here");
        assert_eq!(label.span, 10..15);
        assert_eq!(label.message, "defined here");
        assert_eq!(label.role, LabelRole::Secondary);
    }

    #[test]
    fn diag_builder_error() {
        let diag = Diag::error("syntax error")
            .with_primary_label(0..5, "here")
            .with_help("try adding a semicolon");

        assert_eq!(diag.severity, DiagSeverity::Error);
        assert_eq!(diag.message, "syntax error");
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.help, Some("try adding a semicolon".to_string()));
    }

    #[test]
    fn diag_builder_multi_label() {
        let diag = Diag::error("conflicting definitions")
            .with_primary_label(20..25, "second definition here")
            .with_secondary_label(5..10, "first definition here")
            .with_note("names must be unique");

        assert_eq!(diag.labels.len(), 2);
        assert_eq!(diag.labels[0].role, LabelRole::Primary);
        assert_eq!(diag.labels[1].role, LabelRole::Secondary);
        assert_eq!(diag.notes.len(), 1);
    }

    #[test]
    fn diag_with_code() {
        let diag = Diag::error("parse error").with_code("E0001");
        assert_eq!(diag.code, Some("E0001".to_string()));
    }

    #[test]
    fn source_file_basic() {
        let src = SourceFile::new("hello world");
        assert_eq!(src.content(), "hello world");
        assert_eq!(src.name(), None);
    }

    #[test]
    fn source_file_with_name() {
        let src = SourceFile::with_name("test content", "test.gql");
        assert_eq!(src.content(), "test content");
        assert_eq!(src.name(), Some("test.gql"));
    }

    #[test]
    fn source_file_valid_span() {
        let src = SourceFile::new("hello");
        assert!(src.is_valid_span(&(0..5)));
        assert!(src.is_valid_span(&(0..0)));
        assert!(src.is_valid_span(&(2..4)));
        assert!(!src.is_valid_span(&(0..6))); // past end
        let inverted = std::ops::Range { start: 3, end: 2 };
        assert!(!src.is_valid_span(&inverted));
    }

    #[test]
    fn source_file_clamp_span() {
        let src = SourceFile::new("hello");
        assert_eq!(src.clamp_span(&(0..10)), 0..5);
        let inverted = std::ops::Range { start: 3, end: 2 };
        assert_eq!(src.clamp_span(&inverted), 3..3);
        assert_eq!(src.clamp_span(&(2..4)), 2..4);
        assert_eq!(src.clamp_span(&(10..20)), 5..5);
    }

    #[test]
    fn diag_warning_and_note() {
        let warn = Diag::warning("deprecated syntax");
        assert_eq!(warn.severity, DiagSeverity::Warning);

        let note = Diag::note("for your information");
        assert_eq!(note.severity, DiagSeverity::Note);
    }

    #[test]
    fn convert_simple_error() {
        let source = SourceFile::with_name("hello world", "test.gql");
        let diag = Diag::error("unexpected token").with_primary_label(6..11, "this token");

        // Should not panic and should produce a valid report
        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "unexpected token");
    }

    #[test]
    fn convert_multi_label_error() {
        let source = SourceFile::with_name("let x = 1;\nlet x = 2;", "test.gql");
        let diag = Diag::error("duplicate definition")
            .with_primary_label(11..16, "second definition")
            .with_secondary_label(0..9, "first definition");

        // Should not panic and should produce a valid report
        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "duplicate definition");
    }

    #[test]
    fn convert_with_help_and_code() {
        let source = SourceFile::new("test");
        let diag = Diag::error("parse error")
            .with_primary_label(0..4, "here")
            .with_help("try adding quotes")
            .with_code("E0001");

        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "parse error");
        let built = build_diagnostic(&diag, &source);
        assert_eq!(built.message, "parse error");
        assert_eq!(built.help.as_deref(), Some("try adding quotes"));
        assert_eq!(built.code.as_deref(), Some("E0001"));
        assert_eq!(built.severity, Severity::Error);
    }

    #[test]
    fn convert_warning() {
        let source = SourceFile::new("deprecated");
        let diag = Diag::warning("deprecated syntax").with_primary_label(0..10, "here");

        // Should not panic and should produce a valid report
        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "deprecated syntax");
    }

    #[test]
    fn convert_note() {
        let source = SourceFile::new("info");
        let diag = Diag::note("informational").with_primary_label(0..4, "here");

        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "informational");
        let built = build_diagnostic(&diag, &source);
        assert_eq!(built.message, "informational");
        assert_eq!(built.severity, Severity::Advice);
    }

    #[test]
    fn convert_preserves_label_roles() {
        let source = SourceFile::new("abcdefghij");
        let diag = Diag::error("role check")
            .with_primary_label(2..5, "primary label")
            .with_secondary_label(7..9, "secondary label");

        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "role check");
        let built = build_diagnostic(&diag, &source);
        assert_eq!(built.labels.len(), 2);
        assert!(built.labels[0].primary());
        assert!(!built.labels[1].primary());
        assert_eq!(built.labels[0].label(), Some("primary label"));
        assert_eq!(built.labels[1].label(), Some("secondary label"));
    }

    #[test]
    fn convert_exposes_notes_as_related_diagnostics() {
        let source = SourceFile::new("content");
        let diag = Diag::error("root issue")
            .with_note("first note")
            .with_note("second note");

        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "root issue");
        let built = build_diagnostic(&diag, &source);
        let related = built
            .related()
            .expect("expected related diagnostics")
            .collect::<Vec<_>>();
        assert_eq!(related.len(), 2);
        assert_eq!(related[0].to_string(), "first note");
        assert_eq!(related[1].to_string(), "second note");
        assert_eq!(related[0].severity(), Some(Severity::Advice));
    }

    #[test]
    fn convert_with_invalid_span() {
        let source = SourceFile::new("short");
        let diag = Diag::error("error").with_primary_label(0..100, "out of bounds");

        // Should not panic - span should be clamped
        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "error");
    }

    #[test]
    fn convert_multiple_diagnostics() {
        let source = SourceFile::new("test source");
        let diags = vec![
            Diag::error("error 1").with_primary_label(0..4, "here"),
            Diag::warning("warning 1").with_primary_label(5..11, "there"),
        ];

        let reports = convert_diagnostics_to_reports(&diags, &source);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].to_string(), "error 1");
        assert_eq!(reports[1].to_string(), "warning 1");
    }

    #[test]
    fn convert_empty_labels() {
        let source = SourceFile::new("test");
        let diag = Diag::error("no labels");

        // Should not panic even without labels
        let report = convert_diag_to_report(&diag, &source);
        assert_eq!(report.to_string(), "no labels");
    }

    #[test]
    fn structural_assertions_for_diag() {
        // Test the internal diagnostic structure before conversion
        let diag = Diag::error("test error")
            .with_primary_label(0..5, "primary")
            .with_secondary_label(10..15, "secondary")
            .with_help("some help")
            .with_note("note 1")
            .with_note("note 2")
            .with_code("E001");

        assert_eq!(diag.message, "test error");
        assert_eq!(diag.severity, DiagSeverity::Error);
        assert_eq!(diag.labels.len(), 2);
        assert_eq!(diag.labels[0].message, "primary");
        assert_eq!(diag.labels[0].role, LabelRole::Primary);
        assert_eq!(diag.labels[1].message, "secondary");
        assert_eq!(diag.labels[1].role, LabelRole::Secondary);
        assert_eq!(diag.help, Some("some help".to_string()));
        assert_eq!(diag.notes.len(), 2);
        assert_eq!(diag.code, Some("E001".to_string()));
    }
}
