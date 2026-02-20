//! Callable catalog for function and procedure signature validation (Milestone 4).
//!
//! This module provides the infrastructure for validating function calls against
//! their signatures, including arity checking, parameter type validation, and
//! return type inference.
//!
//! # Architecture
//!
//! The callable catalog system is designed to be:
//! - **Generic**: Works with built-in and external (UDF/stored procedure) callables
//! - **Composable**: Built-ins and external callables can be composed
//! - **Mockable**: Test doubles provided for isolated testing
//! - **Send + Sync**: Thread-safe for multi-threaded validation
//!
//! # Public API
//!
//! - [`CallableCatalog`]: Main trait for resolving callable signatures
//! - [`CallableValidator`]: Trait for validating function calls against signatures
//! - [`BuiltinCallableCatalog`]: Implementation for built-in GQL functions
//! - [`CompositeCallableCatalog`]: Combines built-in and external catalogs
//! - [`InMemoryCallableCatalog`]: Test double for custom callables
//!
//! # Example
//!
//! ```ignore
//! use gql_parser::semantic::callable::{
//!     CallableCatalog, BuiltinCallableCatalog, InMemoryCallableCatalog,
//!     CompositeCallableCatalog, CallableKind, CallableSignature, ParameterSignature,
//! };
//!
//! // Create a catalog with built-ins and custom functions
//! let builtins = BuiltinCallableCatalog::new();
//! let mut custom = InMemoryCallableCatalog::new();
//!
//! // Register a custom function
//! custom.register(CallableSignature {
//!     name: "my_func".into(),
//!     kind: CallableKind::Function,
//!     parameters: vec![
//!         ParameterSignature::required("x", "INT"),
//!         ParameterSignature::required("y", "INT"),
//!     ],
//!     return_type: Some("INT".into()),
//!     volatility: Volatility::Immutable,
//!     nullability: Nullability::NullOnNullInput,
//! });
//!
//! let catalog = CompositeCallableCatalog::new(builtins, custom);
//!
//! // Resolve function signature
//! let context = CallableLookupContext::default();
//! let signatures = catalog.resolve("abs", CallableKind::Function, &context)?;
//! ```

use crate::ast::Span;
use crate::diag::{Diag, DiagLabel, DiagSeverity};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Core Types
// ============================================================================

/// Errors that can occur during catalog operations.
///
/// These errors are returned by catalog trait methods. For diagnostic reporting,
/// convert to `Diag` using `.to_diag(span)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// Callable not found in catalog.
    CallableNotFound {
        name: String,
        kind: CallableKind,
    },

    /// Ambiguous callable reference (multiple matches).
    AmbiguousCallable {
        name: String,
        candidates: Vec<String>,
    },

    /// Invalid callable signature.
    InvalidSignature {
        name: String,
        reason: String,
    },

    /// Catalog is unavailable or not configured.
    CatalogUnavailable,

    /// Generic catalog error.
    Other(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::CallableNotFound { name, kind } => {
                write!(f, "{:?} '{}' not found in catalog", kind, name)
            }
            CatalogError::AmbiguousCallable { name, candidates } => {
                write!(
                    f,
                    "Ambiguous reference to '{}'. Candidates: {}",
                    name,
                    candidates.join(", ")
                )
            }
            CatalogError::InvalidSignature { name, reason } => {
                write!(f, "Invalid signature for '{}': {}", name, reason)
            }
            CatalogError::CatalogUnavailable => {
                write!(f, "Callable catalog is not available")
            }
            CatalogError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for CatalogError {}

impl CatalogError {
    /// Converts this catalog error to a diagnostic at the given span.
    ///
    /// Since catalog errors don't carry their own location information,
    /// callers must provide the relevant source span (e.g., the function call site).
    pub fn to_diag(&self, span: Span) -> Diag {
        match self {
            CatalogError::CallableNotFound { name, kind } => {
                Diag::error(format!("{:?} '{}' not found in catalog", kind, name))
                    .with_label(DiagLabel::primary(span, "undefined callable"))
                    .with_help(format!(
                        "Check if '{}' is defined and available in your catalog",
                        name
                    ))
            }
            CatalogError::AmbiguousCallable { name, candidates } => {
                let mut diag = Diag::error(format!("Ambiguous reference to '{}'", name))
                    .with_label(DiagLabel::primary(span, "ambiguous callable"));

                for candidate in candidates {
                    diag = diag.with_note(format!("Candidate: {}", candidate));
                }

                diag.with_help("Qualify the callable name to resolve the ambiguity")
            }
            CatalogError::InvalidSignature { name, reason } => {
                Diag::error(format!("Invalid signature for '{}'", name))
                    .with_label(DiagLabel::primary(span, reason.clone()))
            }
            CatalogError::CatalogUnavailable => {
                Diag::error("Callable catalog is not available")
                    .with_label(DiagLabel::primary(span, "cannot validate callable"))
                    .with_help("Configure a callable catalog to enable validation")
            }
            CatalogError::Other(msg) => {
                Diag::error(format!("Catalog error: {}", msg))
                    .with_label(DiagLabel::primary(span, "catalog error"))
            }
        }
    }
}

/// Kind of callable (function, procedure, aggregate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallableKind {
    /// Regular function (deterministic or not).
    Function,

    /// Stored procedure.
    Procedure,

    /// Aggregate function (COUNT, SUM, AVG, etc.).
    AggregateFunction,
}

/// Volatility classification for callables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Volatility {
    /// Always returns same result for same inputs (e.g., ABS, FLOOR).
    Immutable,

    /// May return different results in same transaction (e.g., RANDOM).
    Stable,

    /// May return different results per invocation (e.g., NOW).
    Volatile,
}

/// Nullability behavior for callables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nullability {
    /// Returns NULL if any input is NULL (default for most functions).
    NullOnNullInput,

    /// Callable handles NULL inputs explicitly.
    CalledOnNullInput,
}

/// Context for callable lookup with additional resolution hints.
#[derive(Debug, Clone, Default)]
pub struct CallableLookupContext {
    /// Optional schema context for qualified lookups.
    pub schema: Option<SmolStr>,

    /// Optional graph context.
    pub graph: Option<SmolStr>,

    /// Whether to include built-in callables.
    pub include_builtins: bool,
}

impl CallableLookupContext {
    /// Creates a new lookup context with default settings.
    pub fn new() -> Self {
        Self {
            schema: None,
            graph: None,
            include_builtins: true,
        }
    }

    /// Sets the schema context.
    pub fn with_schema(mut self, schema: impl Into<SmolStr>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Sets the graph context.
    pub fn with_graph(mut self, graph: impl Into<SmolStr>) -> Self {
        self.graph = Some(graph.into());
        self
    }

    /// Sets whether to include built-in callables.
    pub fn with_builtins(mut self, include: bool) -> Self {
        self.include_builtins = include;
        self
    }
}

// ============================================================================
// Signature Types
// ============================================================================

/// Complete signature for a callable (function or procedure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableSignature {
    /// Callable name.
    pub name: SmolStr,

    /// Callable kind.
    pub kind: CallableKind,

    /// Parameter signatures.
    pub parameters: Vec<ParameterSignature>,

    /// Return type (None for procedures that don't return values).
    pub return_type: Option<SmolStr>,

    /// Volatility classification.
    pub volatility: Volatility,

    /// Nullability behavior.
    pub nullability: Nullability,
}

impl CallableSignature {
    /// Creates a new callable signature.
    pub fn new(
        name: impl Into<SmolStr>,
        kind: CallableKind,
        parameters: Vec<ParameterSignature>,
        return_type: Option<impl Into<SmolStr>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            parameters,
            return_type: return_type.map(|s| s.into()),
            volatility: Volatility::Immutable,
            nullability: Nullability::NullOnNullInput,
        }
    }

    /// Sets the volatility.
    pub fn with_volatility(mut self, volatility: Volatility) -> Self {
        self.volatility = volatility;
        self
    }

    /// Sets the nullability behavior.
    pub fn with_nullability(mut self, nullability: Nullability) -> Self {
        self.nullability = nullability;
        self
    }

    /// Returns the minimum number of required arguments.
    pub fn min_arity(&self) -> usize {
        self.parameters
            .iter()
            .filter(|p| !p.optional && !p.variadic)
            .count()
    }

    /// Returns the maximum number of arguments (None if variadic).
    pub fn max_arity(&self) -> Option<usize> {
        if self.parameters.iter().any(|p| p.variadic) {
            None
        } else {
            Some(self.parameters.len())
        }
    }

    /// Checks if this signature matches the given arity.
    pub fn matches_arity(&self, arg_count: usize) -> bool {
        let min = self.min_arity();
        match self.max_arity() {
            Some(max) => arg_count >= min && arg_count <= max,
            None => arg_count >= min,
        }
    }
}

/// Parameter signature for a callable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSignature {
    /// Parameter name.
    pub name: SmolStr,

    /// Parameter type.
    pub param_type: SmolStr,

    /// Whether this parameter is optional.
    pub optional: bool,

    /// Whether this parameter is variadic (accepts multiple values).
    pub variadic: bool,
}

impl ParameterSignature {
    /// Creates a required parameter.
    pub fn required(name: impl Into<SmolStr>, param_type: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            optional: false,
            variadic: false,
        }
    }

    /// Creates an optional parameter.
    pub fn optional(name: impl Into<SmolStr>, param_type: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            optional: true,
            variadic: false,
        }
    }

    /// Creates a variadic parameter.
    pub fn variadic(name: impl Into<SmolStr>, param_type: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            param_type: param_type.into(),
            optional: false,
            variadic: true,
        }
    }
}

// ============================================================================
// CallableCatalog Trait
// ============================================================================

/// Trait for resolving callable signatures.
///
/// Implementations must be thread-safe (`Send + Sync`) to support
/// multi-threaded validation.
pub trait CallableCatalog: Send + Sync {
    /// Resolves a callable by name and kind.
    ///
    /// Returns all matching signatures (may be multiple for overloaded functions).
    /// Returns empty vector if not found.
    fn resolve(
        &self,
        name: &str,
        kind: CallableKind,
        ctx: &CallableLookupContext,
    ) -> Result<Vec<CallableSignature>, CatalogError>;

    /// Checks if a callable exists.
    fn exists(&self, name: &str, kind: CallableKind, ctx: &CallableLookupContext) -> bool {
        self.resolve(name, kind, ctx)
            .map(|sigs| !sigs.is_empty())
            .unwrap_or(false)
    }

    /// Lists all available callables of a given kind.
    fn list(&self, kind: CallableKind, ctx: &CallableLookupContext) -> Vec<SmolStr>;
}

// ============================================================================
// CallableValidator Trait
// ============================================================================

/// Information about a function call to validate.
#[derive(Debug, Clone)]
pub struct CallSite<'a> {
    /// Function name.
    pub name: &'a str,

    /// Kind of callable.
    pub kind: CallableKind,

    /// Number of arguments.
    pub arg_count: usize,

    /// Span of the call site for diagnostics.
    pub span: Span,
}

/// Trait for validating function calls against signatures.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait CallableValidator: Send + Sync {
    /// Validates a call site against resolved signatures.
    ///
    /// Returns diagnostics for any validation errors or warnings.
    fn validate_call(&self, call: &CallSite, sigs: &[CallableSignature]) -> Vec<Diag>;
}

// ============================================================================
// Built-in Function Resolution
// ============================================================================

/// Resolves a built-in callable signature by name and kind.
///
/// This function provides zero-cost access to all standard GQL built-in functions:
/// - Numeric: abs, mod, floor, ceil, sqrt, power, exp, ln, log, sin, cos, tan
/// - String: length, substring, upper, lower, trim, replace
/// - Temporal: current_date, current_time, current_timestamp
/// - Aggregates: count, sum, avg, min, max
/// - Other: coalesce, nullif
///
/// Uses match statements for O(1) lookup (compiler-optimized).
/// Returns `None` if the callable is not a built-in function.
pub fn lookup_builtin_callable(name: &str, kind: CallableKind) -> Option<CallableSignature> {
    resolve_builtin_signatures(name, kind)
        .and_then(|sigs| sigs.into_iter().next())
}

/// Resolves all overloaded signatures for a built-in callable.
///
/// Most built-ins have a single signature, but this supports overloading.
/// Returns `None` if the callable is not a built-in function.
pub fn resolve_builtin_signatures(name: &str, kind: CallableKind) -> Option<Vec<CallableSignature>> {
    // Normalize name to lowercase for case-insensitive lookup
    let name_lower = name.to_lowercase();
    let name = name_lower.as_str();

    match kind {
        CallableKind::Function => resolve_builtin_function(name),
        CallableKind::AggregateFunction => resolve_builtin_aggregate(name),
        CallableKind::Procedure => None, // No built-in procedures
    }
}

/// Resolves built-in regular functions.
fn resolve_builtin_function(name: &str) -> Option<Vec<CallableSignature>> {
        let sig = match name {
            // Numeric functions
            "abs" => CallableSignature::new(
                "abs",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "mod" => CallableSignature::new(
                "mod",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("x", "NUMERIC"),
                    ParameterSignature::required("y", "NUMERIC"),
                ],
                Some("NUMERIC"),
            ),
            "floor" => CallableSignature::new(
                "floor",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "ceil" => CallableSignature::new(
                "ceil",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "sqrt" => CallableSignature::new(
                "sqrt",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "power" => CallableSignature::new(
                "power",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("base", "NUMERIC"),
                    ParameterSignature::required("exponent", "NUMERIC"),
                ],
                Some("NUMERIC"),
            ),
            "exp" => CallableSignature::new(
                "exp",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "ln" => CallableSignature::new(
                "ln",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "log" => CallableSignature::new(
                "log",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("base", "NUMERIC"),
                    ParameterSignature::required("x", "NUMERIC"),
                ],
                Some("NUMERIC"),
            ),
            "log10" => CallableSignature::new(
                "log10",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "cot" | "sinh" | "cosh" | "tanh" => {
                CallableSignature::new(
                    name,
                    CallableKind::Function,
                    vec![ParameterSignature::required("x", "NUMERIC")],
                    Some("NUMERIC"),
                )
            }
            "atan2" => CallableSignature::new(
                "atan2",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("y", "NUMERIC"),
                    ParameterSignature::required("x", "NUMERIC"),
                ],
                Some("NUMERIC"),
            ),
            "degrees" => CallableSignature::new(
                "degrees",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "radians" => CallableSignature::new(
                "radians",
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "round" => CallableSignature::new(
                "round",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("x", "NUMERIC"),
                    ParameterSignature::optional("decimals", "INT"),
                ],
                Some("NUMERIC"),
            ),

            // String functions
            "length" => CallableSignature::new(
                "length",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("INT"),
            ),
            "substring" => CallableSignature::new(
                "substring",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("s", "STRING"),
                    ParameterSignature::required("start", "INT"),
                    ParameterSignature::optional("length", "INT"),
                ],
                Some("STRING"),
            ),
            "upper" => CallableSignature::new(
                "upper",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("STRING"),
            ),
            "lower" => CallableSignature::new(
                "lower",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("STRING"),
            ),
            "trim" => CallableSignature::new(
                "trim",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("STRING"),
            ),
            "ltrim" => CallableSignature::new(
                "ltrim",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("STRING"),
            ),
            "rtrim" => CallableSignature::new(
                "rtrim",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("STRING"),
            ),
            "replace" => CallableSignature::new(
                "replace",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("s", "STRING"),
                    ParameterSignature::required("search", "STRING"),
                    ParameterSignature::required("replace", "STRING"),
                ],
                Some("STRING"),
            ),
            "concat" => CallableSignature::new(
                "concat",
                CallableKind::Function,
                vec![ParameterSignature::variadic("strings", "STRING")],
                Some("STRING"),
            ),
            "left" => CallableSignature::new(
                "left",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("s", "STRING"),
                    ParameterSignature::required("n", "INT"),
                ],
                Some("STRING"),
            ),
            "right" => CallableSignature::new(
                "right",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("s", "STRING"),
                    ParameterSignature::required("n", "INT"),
                ],
                Some("STRING"),
            ),
            "normalize" => CallableSignature::new(
                "normalize",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("s", "STRING"),
                    ParameterSignature::optional("form", "STRING"),
                ],
                Some("STRING"),
            ),
            "char_length" => CallableSignature::new(
                "char_length",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("INT"),
            ),
            "byte_length" => CallableSignature::new(
                "byte_length",
                CallableKind::Function,
                vec![ParameterSignature::required("s", "STRING")],
                Some("INT"),
            ),

            // Temporal functions
            "current_date" => CallableSignature::new(
                "current_date",
                CallableKind::Function,
                vec![],
                Some("DATE"),
            ).with_volatility(Volatility::Stable),
            "current_time" => CallableSignature::new(
                "current_time",
                CallableKind::Function,
                vec![],
                Some("TIME"),
            ).with_volatility(Volatility::Stable),
            "current_timestamp" => CallableSignature::new(
                "current_timestamp",
                CallableKind::Function,
                vec![],
                Some("TIMESTAMP"),
            ).with_volatility(Volatility::Stable),
            "now" => CallableSignature::new(
                "now",
                CallableKind::Function,
                vec![],
                Some("TIMESTAMP"),
            ).with_volatility(Volatility::Stable),
            "date" => CallableSignature::new(
                "date",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("year", "INT"),
                    ParameterSignature::required("month", "INT"),
                    ParameterSignature::required("day", "INT"),
                ],
                Some("DATE"),
            ),
            "time" => CallableSignature::new(
                "time",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("hour", "INT"),
                    ParameterSignature::required("minute", "INT"),
                    ParameterSignature::required("second", "INT"),
                    ParameterSignature::optional("nanosecond", "INT"),
                ],
                Some("TIME"),
            ),
            "datetime" => CallableSignature::new(
                "datetime",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("year", "INT"),
                    ParameterSignature::required("month", "INT"),
                    ParameterSignature::required("day", "INT"),
                    ParameterSignature::optional("hour", "INT"),
                    ParameterSignature::optional("minute", "INT"),
                    ParameterSignature::optional("second", "INT"),
                ],
                Some("DATETIME"),
            ),
            "duration" => CallableSignature::new(
                "duration",
                CallableKind::Function,
                vec![ParameterSignature::required("value", "STRING")],
                Some("DURATION"),
            ),
            "duration_between" => CallableSignature::new(
                "duration_between",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("start", "ANY"),
                    ParameterSignature::required("end", "ANY"),
                ],
                Some("DURATION"),
            ),

            // List and cardinality functions
            "elements" => CallableSignature::new(
                "elements",
                CallableKind::Function,
                vec![ParameterSignature::required("list", "LIST")],
                Some("LIST"),
            ),
            "cardinality" => CallableSignature::new(
                "cardinality",
                CallableKind::Function,
                vec![ParameterSignature::required("collection", "ANY")],
                Some("INT"),
            ),
            "size" => CallableSignature::new(
                "size",
                CallableKind::Function,
                vec![ParameterSignature::required("collection", "ANY")],
                Some("INT"),
            ),
            "path_length" => CallableSignature::new(
                "path_length",
                CallableKind::Function,
                vec![ParameterSignature::required("path", "PATH")],
                Some("INT"),
            ),

            // Graph functions
            "element_id" => CallableSignature::new(
                "element_id",
                CallableKind::Function,
                vec![ParameterSignature::required("element", "ANY")],
                Some("STRING"),
            ),

            // Other utility functions
            "coalesce" => CallableSignature::new(
                "coalesce",
                CallableKind::Function,
                vec![ParameterSignature::variadic("exprs", "ANY")],
                Some("ANY"),
            ).with_nullability(Nullability::CalledOnNullInput),
            "nullif" => CallableSignature::new(
                "nullif",
                CallableKind::Function,
                vec![
                    ParameterSignature::required("expr1", "ANY"),
                    ParameterSignature::required("expr2", "ANY"),
                ],
                Some("ANY"),
            ),
            "type_of" => CallableSignature::new(
                "type_of",
                CallableKind::Function,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("STRING"),
            ),
            "collect" => CallableSignature::new(
                "collect",
                CallableKind::Function,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("LIST<ANY>"),
            ),

            _ => return None,
        };

    Some(vec![sig])
}

/// Resolves built-in aggregate functions.
fn resolve_builtin_aggregate(name: &str) -> Option<Vec<CallableSignature>> {
        let sig = match name {
            "count" => CallableSignature::new(
                "count",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::optional("expr", "ANY")],
                Some("INT"),
            ).with_nullability(Nullability::CalledOnNullInput),
            "sum" => CallableSignature::new(
                "sum",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "avg" => CallableSignature::new(
                "avg",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "min" => CallableSignature::new(
                "min",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("ANY"),
            ),
            "max" => CallableSignature::new(
                "max",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("ANY"),
            ),
            "collect" => CallableSignature::new(
                "collect",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("LIST<ANY>"),
            ),
            "stddev_samp" => CallableSignature::new(
                "stddev_samp",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "NUMERIC")],
                Some("NUMERIC"),
            ),
            "stddev_pop" => CallableSignature::new(
                "stddev_pop",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "NUMERIC")],
                Some("NUMERIC"),
            ),
            _ => return None,
        };

    Some(vec![sig])
}

/// Lists all built-in callable names of a given kind.
pub fn list_builtin_callables(kind: CallableKind) -> Vec<SmolStr> {
        match kind {
            CallableKind::Function => vec![
                // Numeric functions
                "abs", "mod", "floor", "ceil", "sqrt", "power", "exp", "ln", "log", "log10",
                "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "cot",
                "sinh", "cosh", "tanh", "degrees", "radians", "round",
                // String functions
                "length", "substring", "upper", "lower", "trim", "ltrim", "rtrim",
                "replace", "concat", "left", "right", "normalize", "char_length", "byte_length",
                // Temporal functions
                "current_date", "current_time", "current_timestamp", "now",
                "date", "time", "datetime", "duration", "duration_between",
                // List and cardinality functions
                "elements", "cardinality", "size", "path_length",
                // Graph functions
                "element_id",
                // Other utility functions
                "coalesce", "nullif", "type_of", "collect",
            ]
            .into_iter()
            .map(SmolStr::new)
            .collect(),
            CallableKind::AggregateFunction => vec![
                "count", "sum", "avg", "min", "max", "collect", "stddev_samp", "stddev_pop",
            ]
            .into_iter()
            .map(SmolStr::new)
            .collect(),
        CallableKind::Procedure => vec![], // No built-in procedures
    }
}

// ============================================================================
// CompositeCallableCatalog
// ============================================================================

/// Composite catalog that combines built-in and external callables.
///
/// This allows users to compose their own catalog with built-ins:
/// ```ignore
/// let catalog = CompositeCallableCatalog::new(
///     BuiltinCallableCatalog::new(),
///     my_external_catalog,
/// );
/// ```
pub struct CompositeCallableCatalog<B, E>
where
    B: CallableCatalog,
    E: CallableCatalog,
{
    builtins: B,
    external: E,
}

impl<B, E> CompositeCallableCatalog<B, E>
where
    B: CallableCatalog,
    E: CallableCatalog,
{
    /// Creates a new composite catalog.
    pub fn new(builtins: B, external: E) -> Self {
        Self { builtins, external }
    }

    /// Returns a reference to the built-in catalog.
    pub fn builtins(&self) -> &B {
        &self.builtins
    }

    /// Returns a reference to the external catalog.
    pub fn external(&self) -> &E {
        &self.external
    }
}

impl<B, E> CallableCatalog for CompositeCallableCatalog<B, E>
where
    B: CallableCatalog,
    E: CallableCatalog,
{
    fn resolve(
        &self,
        name: &str,
        kind: CallableKind,
        ctx: &CallableLookupContext,
    ) -> Result<Vec<CallableSignature>, CatalogError> {
        let mut sigs = Vec::new();

        // Try external catalog first (allows overriding built-ins)
        if let Ok(external_sigs) = self.external.resolve(name, kind, ctx) {
            sigs.extend(external_sigs);
        }

        // Then try built-ins if requested
        if ctx.include_builtins {
            if let Ok(builtin_sigs) = self.builtins.resolve(name, kind, ctx) {
                sigs.extend(builtin_sigs);
            }
        }

        Ok(sigs)
    }

    fn list(&self, kind: CallableKind, ctx: &CallableLookupContext) -> Vec<SmolStr> {
        let mut names = self.external.list(kind, ctx);
        if ctx.include_builtins {
            names.extend(self.builtins.list(kind, ctx));
        }
        names.sort();
        names.dedup();
        names
    }
}

// ============================================================================
// InMemoryCallableCatalog
// ============================================================================

/// In-memory callable catalog for testing and custom callables.
///
/// This is a test double that allows registering custom callable signatures
/// with deterministic overload ordering.
#[derive(Debug, Clone, Default)]
pub struct InMemoryCallableCatalog {
    signatures: HashMap<(SmolStr, CallableKind), Vec<CallableSignature>>,
}

impl InMemoryCallableCatalog {
    /// Creates a new empty in-memory catalog.
    pub fn new() -> Self {
        Self {
            signatures: HashMap::new(),
        }
    }

    /// Registers a callable signature.
    pub fn register(&mut self, sig: CallableSignature) {
        // Normalize name to lowercase for case-insensitive lookup
        let name_lower = SmolStr::new(sig.name.to_lowercase());
        let key = (name_lower, sig.kind);
        self.signatures.entry(key).or_default().push(sig);
    }

    /// Removes all signatures for a callable.
    pub fn unregister(&mut self, name: &str, kind: CallableKind) {
        let name = SmolStr::new(name.to_lowercase());
        self.signatures.remove(&(name, kind));
    }

    /// Clears all registered signatures.
    pub fn clear(&mut self) {
        self.signatures.clear();
    }

    /// Returns the number of registered signatures.
    pub fn len(&self) -> usize {
        self.signatures.values().map(|v| v.len()).sum()
    }

    /// Returns whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }
}

impl CallableCatalog for InMemoryCallableCatalog {
    fn resolve(
        &self,
        name: &str,
        kind: CallableKind,
        _ctx: &CallableLookupContext,
    ) -> Result<Vec<CallableSignature>, CatalogError> {
        let name_lower = SmolStr::new(name.to_lowercase());
        let key = (name_lower, kind);
        Ok(self.signatures.get(&key).cloned().unwrap_or_default())
    }

    fn list(&self, kind: CallableKind, _ctx: &CallableLookupContext) -> Vec<SmolStr> {
        let mut names: Vec<_> = self
            .signatures
            .keys()
            .filter(|(_, k)| *k == kind)
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

// ============================================================================
// MetadataProvider implementation for InMemoryCallableCatalog
// ============================================================================

impl crate::semantic::metadata_provider::MetadataProvider for InMemoryCallableCatalog {
    fn get_schema_snapshot(
        &self,
        _graph: &crate::semantic::schema_catalog::GraphRef,
        _schema: Option<&crate::semantic::schema_catalog::SchemaRef>,
    ) -> Result<Arc<dyn crate::semantic::schema_catalog::SchemaSnapshot>, crate::semantic::schema_catalog::CatalogError> {
        // In-memory catalog doesn't provide schema information
        Err(crate::semantic::schema_catalog::CatalogError::GraphNotFound {
            graph: "no_graph".into(),
        })
    }

    fn resolve_active_graph(
        &self,
        _session: &crate::semantic::schema_catalog::SessionContext,
    ) -> Result<crate::semantic::schema_catalog::GraphRef, crate::semantic::schema_catalog::CatalogError> {
        // In-memory catalog doesn't manage session state
        Err(crate::semantic::schema_catalog::CatalogError::GraphNotFound {
            graph: "no_graph".into(),
        })
    }

    fn resolve_active_schema(
        &self,
        _graph: &crate::semantic::schema_catalog::GraphRef,
    ) -> Result<crate::semantic::schema_catalog::SchemaRef, crate::semantic::schema_catalog::CatalogError> {
        // In-memory catalog doesn't manage schemas
        Err(crate::semantic::schema_catalog::CatalogError::SchemaNotFound {
            schema: "no_schema".into(),
        })
    }

    fn validate_graph_exists(&self, _name: &str) -> Result<(), crate::semantic::schema_catalog::CatalogError> {
        // In-memory catalog doesn't validate graphs
        Ok(())
    }

    fn lookup_callable(&self, name: &str) -> Option<CallableSignature> {
        let name_lower = SmolStr::new(name.to_lowercase());
        // Try as function first
        if let Ok(sigs) = self.resolve(&name_lower, CallableKind::Function, &CallableLookupContext::new()) {
            if let Some(sig) = sigs.first() {
                return Some(sig.clone());
            }
        }
        // Try as aggregate
        if let Ok(sigs) = self.resolve(&name_lower, CallableKind::AggregateFunction, &CallableLookupContext::new()) {
            if let Some(sig) = sigs.first() {
                return Some(sig.clone());
            }
        }
        // Try as procedure
        if let Ok(sigs) = self.resolve(&name_lower, CallableKind::Procedure, &CallableLookupContext::new()) {
            if let Some(sig) = sigs.first() {
                return Some(sig.clone());
            }
        }
        None
    }
}

// ============================================================================
// DefaultCallableValidator
// ============================================================================

/// Default implementation of callable validator.
///
/// Validates:
/// - Function arity (argument count)
/// - Parameter count matches signature
pub struct DefaultCallableValidator;

impl DefaultCallableValidator {
    /// Creates a new default validator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultCallableValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CallableValidator for DefaultCallableValidator {
    fn validate_call(&self, call: &CallSite, sigs: &[CallableSignature]) -> Vec<Diag> {
        let mut diagnostics = Vec::new();

        if sigs.is_empty() {
            // Callable not found
            diagnostics.push(
                Diag::new(
                    DiagSeverity::Error,
                    format!("{:?} '{}' is not defined", call.kind, call.name),
                )
                .with_label(crate::diag::DiagLabel::primary(
                    call.span.clone(),
                    format!("undefined {:?}", call.kind),
                )),
            );
            return diagnostics;
        }

        // Check if any signature matches the arity
        let matching_sigs: Vec<_> = sigs
            .iter()
            .filter(|sig| sig.matches_arity(call.arg_count))
            .collect();

        if matching_sigs.is_empty() {
            // No signature matches the arity
            if sigs.len() == 1 {
                let sig = &sigs[0];
                let expected = if sig.max_arity().is_none() {
                    format!("at least {} arguments", sig.min_arity())
                } else if sig.min_arity() == sig.max_arity().unwrap() {
                    format!("{} arguments", sig.min_arity())
                } else {
                    format!(
                        "between {} and {} arguments",
                        sig.min_arity(),
                        sig.max_arity().unwrap()
                    )
                };

                diagnostics.push(
                    Diag::new(
                        DiagSeverity::Error,
                        format!(
                            "{:?} '{}' expects {}, but got {}",
                            call.kind, call.name, expected, call.arg_count
                        ),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "incorrect number of arguments",
                    )),
                );
            } else {
                // Multiple signatures, none match
                diagnostics.push(
                    Diag::new(
                        DiagSeverity::Error,
                        format!(
                            "{:?} '{}' has no overload that accepts {} arguments",
                            call.kind, call.name, call.arg_count
                        ),
                    )
                    .with_label(crate::diag::DiagLabel::primary(
                        call.span.clone(),
                        "no matching overload",
                    )),
                );
            }
        }

        diagnostics
    }
}

// ============================================================================
// Arc Implementations
// ============================================================================

impl CallableCatalog for Arc<dyn CallableCatalog> {
    fn resolve(
        &self,
        name: &str,
        kind: CallableKind,
        ctx: &CallableLookupContext,
    ) -> Result<Vec<CallableSignature>, CatalogError> {
        (**self).resolve(name, kind, ctx)
    }

    fn list(&self, kind: CallableKind, ctx: &CallableLookupContext) -> Vec<SmolStr> {
        (**self).list(kind, ctx)
    }
}

impl CallableValidator for Arc<dyn CallableValidator> {
    fn validate_call(&self, call: &CallSite, sigs: &[CallableSignature]) -> Vec<Diag> {
        (**self).validate_call(call, sigs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_numeric_functions() {
        // Test ABS
        let sig = lookup_builtin_callable("abs", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "abs");
        assert_eq!(sig.min_arity(), 1);
        assert_eq!(sig.max_arity(), Some(1));

        // Test MOD
        let sig = lookup_builtin_callable("mod", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "mod");
        assert_eq!(sig.min_arity(), 2);
        assert_eq!(sig.max_arity(), Some(2));

        // Test POWER
        let sig = lookup_builtin_callable("power", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "power");
        assert_eq!(sig.min_arity(), 2);
    }

    #[test]
    fn test_builtin_string_functions() {
        // Test LENGTH
        let sig = lookup_builtin_callable("length", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "length");
        assert_eq!(sig.return_type, Some("INT".into()));

        // Test SUBSTRING (with optional length parameter)
        let sig = lookup_builtin_callable("substring", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "substring");
        assert_eq!(sig.min_arity(), 2);
        assert_eq!(sig.max_arity(), Some(3));

        // Test CONCAT (variadic)
        let sig = lookup_builtin_callable("concat", CallableKind::Function).unwrap();
        assert_eq!(sig.name, "concat");
        assert!(sig.max_arity().is_none()); // variadic
    }

    #[test]
    fn test_builtin_aggregates() {
        // Test COUNT
        let sig = lookup_builtin_callable("count", CallableKind::AggregateFunction).unwrap();
        assert_eq!(sig.name, "count");
        assert_eq!(sig.return_type, Some("INT".into()));

        // Test SUM
        let sig = lookup_builtin_callable("sum", CallableKind::AggregateFunction).unwrap();
        assert_eq!(sig.name, "sum");
        assert_eq!(sig.return_type, Some("NUMERIC".into()));
    }

    #[test]
    fn test_builtin_not_found() {
        // Non-existent function
        assert!(lookup_builtin_callable("nonexistent", CallableKind::Function).is_none());
    }

    #[test]
    fn test_builtin_list() {
        let functions = list_builtin_callables(CallableKind::Function);
        assert!(functions.contains(&"abs".into()));
        assert!(functions.contains(&"upper".into()));
        assert!(functions.contains(&"current_date".into()));

        let aggregates = list_builtin_callables(CallableKind::AggregateFunction);
        assert!(aggregates.contains(&"count".into()));
        assert!(aggregates.contains(&"sum".into()));
    }

    #[test]
    fn test_inmemory_catalog() {
        let mut catalog = InMemoryCallableCatalog::new();
        let ctx = CallableLookupContext::new();

        // Register a custom function
        catalog.register(CallableSignature::new(
            "my_func",
            CallableKind::Function,
            vec![
                ParameterSignature::required("x", "INT"),
                ParameterSignature::required("y", "INT"),
            ],
            Some("INT"),
        ));

        // Resolve it
        let sigs = catalog
            .resolve("my_func", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].name, "my_func");
        assert_eq!(sigs[0].min_arity(), 2);

        // Unregister
        catalog.unregister("my_func", CallableKind::Function);
        let sigs = catalog
            .resolve("my_func", CallableKind::Function, &ctx)
            .unwrap();
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_composite_catalog() {
        // Create custom catalog with user-defined functions
        let mut custom = InMemoryCallableCatalog::new();

        custom.register(CallableSignature::new(
            "custom_func",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "STRING")],
            Some("INT"),
        ));

        let ctx = CallableLookupContext::new();

        // Can resolve built-in (would come from metadata provider default)
        let sig = lookup_builtin_callable("abs", CallableKind::Function);
        assert!(sig.is_some());

        // Can resolve custom
        let sigs = custom.resolve("custom_func", CallableKind::Function, &ctx).unwrap();
        assert_eq!(sigs.len(), 1);
    }

    #[test]
    fn test_default_validator() {
        let validator = DefaultCallableValidator::new();

        // Test matching arity
        let sig = CallableSignature::new(
            "test",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "INT")],
            Some("INT"),
        );

        let call = CallSite {
            name: "test",
            kind: CallableKind::Function,
            arg_count: 1,
            span: 0..4,
        };

        let diags = validator.validate_call(&call, &[sig]);
        assert!(diags.is_empty());

        // Test wrong arity
        let sig = CallableSignature::new(
            "test",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "INT")],
            Some("INT"),
        );

        let call = CallSite {
            name: "test",
            kind: CallableKind::Function,
            arg_count: 2,
            span: 0..4,
        };

        let diags = validator.validate_call(&call, &[sig]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Error);
    }

    #[test]
    fn test_signature_arity_matching() {
        // Required parameter only
        let sig = CallableSignature::new(
            "test",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "INT")],
            Some("INT"),
        );
        assert!(sig.matches_arity(1));
        assert!(!sig.matches_arity(0));
        assert!(!sig.matches_arity(2));

        // Optional parameter
        let sig = CallableSignature::new(
            "test",
            CallableKind::Function,
            vec![
                ParameterSignature::required("x", "INT"),
                ParameterSignature::optional("y", "INT"),
            ],
            Some("INT"),
        );
        assert!(sig.matches_arity(1));
        assert!(sig.matches_arity(2));
        assert!(!sig.matches_arity(0));
        assert!(!sig.matches_arity(3));

        // Variadic parameter
        let sig = CallableSignature::new(
            "test",
            CallableKind::Function,
            vec![ParameterSignature::variadic("args", "ANY")],
            Some("ANY"),
        );
        assert!(sig.matches_arity(0)); // variadic can accept 0
        assert!(sig.matches_arity(1));
        assert!(sig.matches_arity(100));
    }
}
