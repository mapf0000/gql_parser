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
use crate::diag::{Diag, DiagSeverity};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Core Types
// ============================================================================

/// Result type for catalog operations.
pub type CatalogResult<T> = Result<T, CatalogError>;

/// Errors that can occur during catalog operations.
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
    ) -> CatalogResult<Vec<CallableSignature>>;

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
// BuiltinCallableCatalog
// ============================================================================

/// Catalog of built-in GQL functions and aggregates.
///
/// This catalog contains all standard GQL built-in functions organized by category:
/// - Numeric: abs, mod, floor, ceil, sqrt, power, exp, ln, log, sin, cos, tan
/// - String: length, substring, upper, lower, trim, replace
/// - Temporal: current_date, current_time, current_timestamp
/// - Aggregates: count, sum, avg, min, max
/// - Other: coalesce, nullif
pub struct BuiltinCallableCatalog {
    signatures: HashMap<(SmolStr, CallableKind), Vec<CallableSignature>>,
}

impl BuiltinCallableCatalog {
    /// Creates a new built-in callable catalog with all standard GQL functions.
    pub fn new() -> Self {
        let mut catalog = Self {
            signatures: HashMap::new(),
        };
        catalog.register_all_builtins();
        catalog
    }

    /// Registers all built-in functions and aggregates.
    fn register_all_builtins(&mut self) {
        self.register_numeric_functions();
        self.register_string_functions();
        self.register_temporal_functions();
        self.register_list_and_cardinality_functions();
        self.register_graph_functions();
        self.register_aggregate_functions();
        self.register_other_functions();
    }

    /// Registers a single signature.
    fn register(&mut self, sig: CallableSignature) {
        let key = (sig.name.clone(), sig.kind);
        self.signatures.entry(key).or_default().push(sig);
    }

    /// Registers numeric functions.
    fn register_numeric_functions(&mut self) {
        // ABS(x) -> numeric
        self.register(CallableSignature::new(
            "abs",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // MOD(x, y) -> numeric
        self.register(CallableSignature::new(
            "mod",
            CallableKind::Function,
            vec![
                ParameterSignature::required("x", "NUMERIC"),
                ParameterSignature::required("y", "NUMERIC"),
            ],
            Some("NUMERIC"),
        ));

        // FLOOR(x) -> numeric
        self.register(CallableSignature::new(
            "floor",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // CEIL(x) -> numeric
        self.register(CallableSignature::new(
            "ceil",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // SQRT(x) -> numeric
        self.register(CallableSignature::new(
            "sqrt",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // POWER(x, y) -> numeric
        self.register(CallableSignature::new(
            "power",
            CallableKind::Function,
            vec![
                ParameterSignature::required("base", "NUMERIC"),
                ParameterSignature::required("exponent", "NUMERIC"),
            ],
            Some("NUMERIC"),
        ));

        // EXP(x) -> numeric
        self.register(CallableSignature::new(
            "exp",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // LN(x) -> numeric (natural logarithm)
        self.register(CallableSignature::new(
            "ln",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // LOG(base, x) -> numeric
        self.register(CallableSignature::new(
            "log",
            CallableKind::Function,
            vec![
                ParameterSignature::required("base", "NUMERIC"),
                ParameterSignature::required("x", "NUMERIC"),
            ],
            Some("NUMERIC"),
        ));

        // LOG10(x) -> numeric
        self.register(CallableSignature::new(
            "log10",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // Trigonometric functions
        for func in ["sin", "cos", "tan", "asin", "acos", "atan", "cot", "sinh", "cosh", "tanh"] {
            self.register(CallableSignature::new(
                func,
                CallableKind::Function,
                vec![ParameterSignature::required("x", "NUMERIC")],
                Some("NUMERIC"),
            ));
        }

        // ATAN2(y, x) -> numeric
        self.register(CallableSignature::new(
            "atan2",
            CallableKind::Function,
            vec![
                ParameterSignature::required("y", "NUMERIC"),
                ParameterSignature::required("x", "NUMERIC"),
            ],
            Some("NUMERIC"),
        ));

        // DEGREES(x) -> numeric (radians to degrees)
        self.register(CallableSignature::new(
            "degrees",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // RADIANS(x) -> numeric (degrees to radians)
        self.register(CallableSignature::new(
            "radians",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // ROUND(x, [decimals]) -> numeric
        self.register(CallableSignature::new(
            "round",
            CallableKind::Function,
            vec![
                ParameterSignature::required("x", "NUMERIC"),
                ParameterSignature::optional("decimals", "INT"),
            ],
            Some("NUMERIC"),
        ));
    }

    /// Registers string functions.
    fn register_string_functions(&mut self) {
        // LENGTH(s) -> int
        self.register(CallableSignature::new(
            "length",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("INT"),
        ));

        // SUBSTRING(s, start, [length]) -> string
        self.register(CallableSignature::new(
            "substring",
            CallableKind::Function,
            vec![
                ParameterSignature::required("s", "STRING"),
                ParameterSignature::required("start", "INT"),
                ParameterSignature::optional("length", "INT"),
            ],
            Some("STRING"),
        ));

        // UPPER(s) -> string
        self.register(CallableSignature::new(
            "upper",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("STRING"),
        ));

        // LOWER(s) -> string
        self.register(CallableSignature::new(
            "lower",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("STRING"),
        ));

        // TRIM(s) -> string
        self.register(CallableSignature::new(
            "trim",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("STRING"),
        ));

        // LTRIM(s) -> string
        self.register(CallableSignature::new(
            "ltrim",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("STRING"),
        ));

        // RTRIM(s) -> string
        self.register(CallableSignature::new(
            "rtrim",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("STRING"),
        ));

        // REPLACE(s, search, replace) -> string
        self.register(CallableSignature::new(
            "replace",
            CallableKind::Function,
            vec![
                ParameterSignature::required("s", "STRING"),
                ParameterSignature::required("search", "STRING"),
                ParameterSignature::required("replace", "STRING"),
            ],
            Some("STRING"),
        ));

        // CONCAT(s1, s2, ...) -> string (variadic)
        self.register(CallableSignature::new(
            "concat",
            CallableKind::Function,
            vec![ParameterSignature::variadic("strings", "STRING")],
            Some("STRING"),
        ));

        // LEFT(s, n) -> string
        self.register(CallableSignature::new(
            "left",
            CallableKind::Function,
            vec![
                ParameterSignature::required("s", "STRING"),
                ParameterSignature::required("n", "INT"),
            ],
            Some("STRING"),
        ));

        // RIGHT(s, n) -> string
        self.register(CallableSignature::new(
            "right",
            CallableKind::Function,
            vec![
                ParameterSignature::required("s", "STRING"),
                ParameterSignature::required("n", "INT"),
            ],
            Some("STRING"),
        ));

        // NORMALIZE(s, [form]) -> string
        self.register(CallableSignature::new(
            "normalize",
            CallableKind::Function,
            vec![
                ParameterSignature::required("s", "STRING"),
                ParameterSignature::optional("form", "STRING"),
            ],
            Some("STRING"),
        ));

        // CHAR_LENGTH(s) -> int
        self.register(CallableSignature::new(
            "char_length",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("INT"),
        ));

        // BYTE_LENGTH(s) -> int
        self.register(CallableSignature::new(
            "byte_length",
            CallableKind::Function,
            vec![ParameterSignature::required("s", "STRING")],
            Some("INT"),
        ));
    }

    /// Registers temporal functions.
    fn register_temporal_functions(&mut self) {
        // CURRENT_DATE() -> date
        self.register(
            CallableSignature::new("current_date", CallableKind::Function, vec![], Some("DATE"))
                .with_volatility(Volatility::Stable),
        );

        // CURRENT_TIME() -> time
        self.register(
            CallableSignature::new("current_time", CallableKind::Function, vec![], Some("TIME"))
                .with_volatility(Volatility::Stable),
        );

        // CURRENT_TIMESTAMP() -> timestamp
        self.register(
            CallableSignature::new(
                "current_timestamp",
                CallableKind::Function,
                vec![],
                Some("TIMESTAMP"),
            )
            .with_volatility(Volatility::Stable),
        );

        // NOW() -> timestamp (alias for CURRENT_TIMESTAMP)
        self.register(
            CallableSignature::new("now", CallableKind::Function, vec![], Some("TIMESTAMP"))
                .with_volatility(Volatility::Stable),
        );

        // Temporal constructor functions (simplified - real signatures may vary)
        // DATE(year, month, day) -> date
        self.register(CallableSignature::new(
            "date",
            CallableKind::Function,
            vec![
                ParameterSignature::required("year", "INT"),
                ParameterSignature::required("month", "INT"),
                ParameterSignature::required("day", "INT"),
            ],
            Some("DATE"),
        ));

        // TIME(hour, minute, second, [nanosecond]) -> time
        self.register(CallableSignature::new(
            "time",
            CallableKind::Function,
            vec![
                ParameterSignature::required("hour", "INT"),
                ParameterSignature::required("minute", "INT"),
                ParameterSignature::required("second", "INT"),
                ParameterSignature::optional("nanosecond", "INT"),
            ],
            Some("TIME"),
        ));

        // DATETIME - similar to date + time
        self.register(CallableSignature::new(
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
        ));

        // DURATION(value) -> duration
        self.register(CallableSignature::new(
            "duration",
            CallableKind::Function,
            vec![ParameterSignature::required("value", "STRING")],
            Some("DURATION"),
        ));

        // DURATION_BETWEEN(start, end) -> duration
        self.register(CallableSignature::new(
            "duration_between",
            CallableKind::Function,
            vec![
                ParameterSignature::required("start", "ANY"),
                ParameterSignature::required("end", "ANY"),
            ],
            Some("DURATION"),
        ));
    }

    /// Registers list and cardinality functions.
    fn register_list_and_cardinality_functions(&mut self) {
        // ELEMENTS(list) -> list elements
        self.register(CallableSignature::new(
            "elements",
            CallableKind::Function,
            vec![ParameterSignature::required("list", "LIST")],
            Some("LIST"),
        ));

        // CARDINALITY(collection) -> int
        self.register(CallableSignature::new(
            "cardinality",
            CallableKind::Function,
            vec![ParameterSignature::required("collection", "ANY")],
            Some("INT"),
        ));

        // SIZE(collection) -> int
        self.register(CallableSignature::new(
            "size",
            CallableKind::Function,
            vec![ParameterSignature::required("collection", "ANY")],
            Some("INT"),
        ));

        // PATH_LENGTH(path) -> int
        self.register(CallableSignature::new(
            "path_length",
            CallableKind::Function,
            vec![ParameterSignature::required("path", "PATH")],
            Some("INT"),
        ));
    }

    /// Registers graph functions.
    fn register_graph_functions(&mut self) {
        // ELEMENT_ID(element) -> string
        self.register(CallableSignature::new(
            "element_id",
            CallableKind::Function,
            vec![ParameterSignature::required("element", "ANY")],
            Some("STRING"),
        ));
    }

    /// Registers aggregate functions.
    fn register_aggregate_functions(&mut self) {
        // COUNT(*) or COUNT(expr)
        self.register(
            CallableSignature::new(
                "count",
                CallableKind::AggregateFunction,
                vec![ParameterSignature::required("expr", "ANY")],
                Some("INT"),
            )
            .with_nullability(Nullability::CalledOnNullInput),
        );

        // SUM(expr) -> numeric
        self.register(CallableSignature::new(
            "sum",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // AVG(expr) -> numeric
        self.register(CallableSignature::new(
            "avg",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // MIN(expr) -> same as input type
        self.register(CallableSignature::new(
            "min",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "ANY")],
            Some("ANY"),
        ));

        // MAX(expr) -> same as input type
        self.register(CallableSignature::new(
            "max",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "ANY")],
            Some("ANY"),
        ));

        // COLLECT(expr) -> list
        self.register(CallableSignature::new(
            "collect",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "ANY")],
            Some("LIST<ANY>"),
        ));

        // STDDEV_SAMP(expr) -> numeric (sample standard deviation)
        self.register(CallableSignature::new(
            "stddev_samp",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "NUMERIC")],
            Some("NUMERIC"),
        ));

        // STDDEV_POP(expr) -> numeric (population standard deviation)
        self.register(CallableSignature::new(
            "stddev_pop",
            CallableKind::AggregateFunction,
            vec![ParameterSignature::required("expr", "NUMERIC")],
            Some("NUMERIC"),
        ));
    }

    /// Registers other utility functions.
    fn register_other_functions(&mut self) {
        // COALESCE(expr1, expr2, ...) -> first non-null value (variadic)
        self.register(
            CallableSignature::new(
                "coalesce",
                CallableKind::Function,
                vec![ParameterSignature::variadic("exprs", "ANY")],
                Some("ANY"),
            )
            .with_nullability(Nullability::CalledOnNullInput),
        );

        // NULLIF(expr1, expr2) -> expr1 if expr1 != expr2, else NULL
        self.register(CallableSignature::new(
            "nullif",
            CallableKind::Function,
            vec![
                ParameterSignature::required("expr1", "ANY"),
                ParameterSignature::required("expr2", "ANY"),
            ],
            Some("ANY"),
        ));

        // CAST(expr AS type) - handled separately in parser/validator
        // Not registered here as it has special syntax

        // TYPE_OF(expr) -> string
        self.register(CallableSignature::new(
            "type_of",
            CallableKind::Function,
            vec![ParameterSignature::required("expr", "ANY")],
            Some("STRING"),
        ));

        // COLLECT(expr) -> list (can also be used as regular function in some contexts)
        self.register(CallableSignature::new(
            "collect",
            CallableKind::Function,
            vec![ParameterSignature::required("expr", "ANY")],
            Some("LIST<ANY>"),
        ));
    }
}

impl Default for BuiltinCallableCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl CallableCatalog for BuiltinCallableCatalog {
    fn resolve(
        &self,
        name: &str,
        kind: CallableKind,
        _ctx: &CallableLookupContext,
    ) -> CatalogResult<Vec<CallableSignature>> {
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
    ) -> CatalogResult<Vec<CallableSignature>> {
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
    ) -> CatalogResult<Vec<CallableSignature>> {
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
    ) -> CatalogResult<Vec<CallableSignature>> {
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
    fn test_builtin_catalog_numeric_functions() {
        let catalog = BuiltinCallableCatalog::new();
        let ctx = CallableLookupContext::new();

        // Test ABS
        let sigs = catalog
            .resolve("abs", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].min_arity(), 1);
        assert_eq!(sigs[0].max_arity(), Some(1));

        // Test MOD
        let sigs = catalog
            .resolve("mod", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].min_arity(), 2);
        assert_eq!(sigs[0].max_arity(), Some(2));

        // Test POWER
        let sigs = catalog
            .resolve("power", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].min_arity(), 2);
    }

    #[test]
    fn test_builtin_catalog_string_functions() {
        let catalog = BuiltinCallableCatalog::new();
        let ctx = CallableLookupContext::new();

        // Test LENGTH
        let sigs = catalog
            .resolve("length", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].return_type, Some("INT".into()));

        // Test SUBSTRING (with optional length parameter)
        let sigs = catalog
            .resolve("substring", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].min_arity(), 2);
        assert_eq!(sigs[0].max_arity(), Some(3));

        // Test CONCAT (variadic)
        let sigs = catalog
            .resolve("concat", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert!(sigs[0].max_arity().is_none()); // variadic
    }

    #[test]
    fn test_builtin_catalog_aggregates() {
        let catalog = BuiltinCallableCatalog::new();
        let ctx = CallableLookupContext::new();

        // Test COUNT
        let sigs = catalog
            .resolve("count", CallableKind::AggregateFunction, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].return_type, Some("INT".into()));

        // Test SUM
        let sigs = catalog
            .resolve("sum", CallableKind::AggregateFunction, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].return_type, Some("NUMERIC".into()));
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
        let builtins = BuiltinCallableCatalog::new();
        let mut custom = InMemoryCallableCatalog::new();

        custom.register(CallableSignature::new(
            "custom_func",
            CallableKind::Function,
            vec![ParameterSignature::required("x", "STRING")],
            Some("INT"),
        ));

        let catalog = CompositeCallableCatalog::new(builtins, custom);
        let ctx = CallableLookupContext::new();

        // Can resolve built-in
        let sigs = catalog.resolve("abs", CallableKind::Function, &ctx).unwrap();
        assert_eq!(sigs.len(), 1);

        // Can resolve custom
        let sigs = catalog
            .resolve("custom_func", CallableKind::Function, &ctx)
            .unwrap();
        assert_eq!(sigs.len(), 1);

        // Can list both
        let names = catalog.list(CallableKind::Function, &ctx);
        assert!(names.contains(&"abs".into()));
        assert!(names.contains(&"custom_func".into()));
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
