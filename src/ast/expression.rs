//! Expression AST nodes for GQL.
//!
//! This module defines the complete expression system including:
//! - Literals (boolean, numeric, string, temporal, collections)
//! - Value expressions (operators, function calls, property access)
//! - Predicates (comparison, IS NULL, EXISTS, etc.)
//! - Case and Cast expressions
//!
//! Expressions form the computational backbone of GQL queries.

use crate::ast::Span;
use smol_str::SmolStr;

// ============================================================================
// Expression - Top-level expression type
// ============================================================================

/// Represents any expression in GQL.
///
/// This is the main entry point for all expression forms, from simple literals
/// to complex nested predicates and function calls.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Literal value (boolean, numeric, string, temporal, collection)
    Literal(Literal, Span),

    /// Unary expression (+, -, NOT)
    Unary(UnaryOperator, Box<Expression>, Span),

    /// Binary arithmetic or string concatenation expression
    Binary(BinaryOperator, Box<Expression>, Box<Expression>, Span),

    /// Comparison expression
    Comparison(ComparisonOperator, Box<Expression>, Box<Expression>, Span),

    /// Logical expression (AND, OR, XOR)
    Logical(LogicalOperator, Box<Expression>, Box<Expression>, Span),

    /// Parenthesized expression
    Parenthesized(Box<Expression>, Span),

    /// Property reference (expr.property_name)
    PropertyReference(Box<Expression>, SmolStr, Span),

    /// Variable reference (binding variable)
    VariableReference(SmolStr, Span),

    /// Parameter reference ($name)
    ParameterReference(SmolStr, Span),

    /// Function call
    FunctionCall(FunctionCall),

    /// CASE expression (simple or searched)
    Case(CaseExpression),

    /// CAST expression
    Cast(CastExpression),

    /// List constructor [expr1, expr2, ...]
    ListConstructor(Vec<Expression>, Span),

    /// Record constructor RECORD {field: value, ...} or {field: value, ...}
    RecordConstructor(Vec<RecordField>, Span),

    /// Path constructor PATH[node, edge, node, ...]
    PathConstructor(Vec<Expression>, Span),

    /// EXISTS predicate
    Exists(ExistsExpression),

    /// Predicate (IS NULL, IS TYPED, etc.)
    Predicate(Predicate),

    /// PROPERTY GRAPH expression
    GraphExpression(Box<Expression>, Span),

    /// BINDING TABLE expression
    BindingTableExpression(Box<Expression>, Span),

    /// VALUE <nested_query> subquery expression
    SubqueryExpression(Box<Expression>, Span),
}

impl Expression {
    /// Returns the span of this expression
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(_, span) => span.clone(),
            Expression::Unary(_, _, span) => span.clone(),
            Expression::Binary(_, _, _, span) => span.clone(),
            Expression::Comparison(_, _, _, span) => span.clone(),
            Expression::Logical(_, _, _, span) => span.clone(),
            Expression::Parenthesized(_, span) => span.clone(),
            Expression::PropertyReference(_, _, span) => span.clone(),
            Expression::VariableReference(_, span) => span.clone(),
            Expression::ParameterReference(_, span) => span.clone(),
            Expression::FunctionCall(fc) => fc.span.clone(),
            Expression::Case(ce) => ce.span(),
            Expression::Cast(ce) => ce.span.clone(),
            Expression::ListConstructor(_, span) => span.clone(),
            Expression::RecordConstructor(_, span) => span.clone(),
            Expression::PathConstructor(_, span) => span.clone(),
            Expression::Exists(ee) => ee.span.clone(),
            Expression::Predicate(p) => p.span(),
            Expression::GraphExpression(_, span) => span.clone(),
            Expression::BindingTableExpression(_, span) => span.clone(),
            Expression::SubqueryExpression(_, span) => span.clone(),
        }
    }
}

// ============================================================================
// Literals
// ============================================================================

/// Literal value types.
///
/// All literal values preserve their original source text for precision
/// and diagnostic purposes.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Boolean literal: TRUE, FALSE, UNKNOWN
    Boolean(BooleanValue),

    /// NULL literal
    Null,

    /// Integer literal (decimal, hex, octal, binary)
    /// Preserves original text for precision
    Integer(SmolStr),

    /// Float literal (decimal and scientific notation)
    /// Preserves original text for precision
    Float(SmolStr),

    /// String literal (single or double-quoted)
    String(SmolStr),

    /// Byte string literal (X'...')
    ByteString(SmolStr),

    /// DATE literal (DATE '...')
    Date(SmolStr),

    /// TIME literal (TIME '...')
    Time(SmolStr),

    /// DATETIME/TIMESTAMP literal
    Datetime(SmolStr),

    /// DURATION literal (DURATION '...')
    Duration(SmolStr),

    /// List literal [expr1, expr2, ...]
    List(Vec<Expression>),

    /// Record literal {field: value, ...}
    Record(Vec<RecordField>),
}

/// Boolean literal values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanValue {
    True,
    False,
    Unknown,
}

/// Record field in a record literal or constructor
#[derive(Debug, Clone, PartialEq)]
pub struct RecordField {
    /// Field name
    pub name: SmolStr,
    /// Field value expression
    pub value: Expression,
    /// Span covering the entire field (name: value)
    pub span: Span,
}

// ============================================================================
// Operators
// ============================================================================

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// Unary plus (+)
    Plus,
    /// Unary minus (-)
    Minus,
    /// Logical NOT
    Not,
}

/// Binary arithmetic and string operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Subtract,
    /// Multiplication (*)
    Multiply,
    /// Division (/)
    Divide,
    /// Modulo (%)
    Modulo,
    /// String concatenation (||)
    Concatenate,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// Equal (=)
    Eq,
    /// Not equal (<>)
    NotEq,
    /// Less than (<)
    Lt,
    /// Greater than (>)
    Gt,
    /// Less than or equal (<=)
    LtEq,
    /// Greater than or equal (>=)
    GtEq,
}

/// Logical operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOperator {
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical XOR
    Xor,
}

// ============================================================================
// Predicates
// ============================================================================

/// Predicate expressions used in filtering and conditions
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// IS [NOT] NULL predicate
    IsNull(Box<Expression>, bool, Span),

    /// IS [NOT] TYPED type predicate
    IsTyped(Box<Expression>, TypeReference, bool, Span),

    /// IS [NOT] NORMALIZED predicate
    IsNormalized(Box<Expression>, bool, Span),

    /// IS [NOT] DIRECTED predicate
    IsDirected(Box<Expression>, bool, Span),

    /// IS [NOT] LABELED [:label] predicate
    IsLabeled(Box<Expression>, Option<LabelExpression>, bool, Span),

    /// IS [NOT] TRUE/FALSE/UNKNOWN predicate
    IsTruthValue(Box<Expression>, TruthValue, bool, Span),

    /// IS [NOT] SOURCE OF edge predicate
    IsSource(Box<Expression>, Box<Expression>, bool, Span),

    /// IS [NOT] DESTINATION OF edge predicate
    IsDestination(Box<Expression>, Box<Expression>, bool, Span),

    /// ALL_DIFFERENT(expr1, expr2, ...) predicate
    AllDifferent(Vec<Expression>, Span),

    /// SAME(expr1, expr2) predicate
    Same(Box<Expression>, Box<Expression>, Span),

    /// PROPERTY_EXISTS(element, property) predicate
    PropertyExists(Box<Expression>, SmolStr, Span),
}

impl Predicate {
    /// Returns the span of this predicate
    pub fn span(&self) -> Span {
        match self {
            Predicate::IsNull(_, _, span) => span.clone(),
            Predicate::IsTyped(_, _, _, span) => span.clone(),
            Predicate::IsNormalized(_, _, span) => span.clone(),
            Predicate::IsDirected(_, _, span) => span.clone(),
            Predicate::IsLabeled(_, _, _, span) => span.clone(),
            Predicate::IsTruthValue(_, _, _, span) => span.clone(),
            Predicate::IsSource(_, _, _, span) => span.clone(),
            Predicate::IsDestination(_, _, _, span) => span.clone(),
            Predicate::AllDifferent(_, span) => span.clone(),
            Predicate::Same(_, _, span) => span.clone(),
            Predicate::PropertyExists(_, _, span) => span.clone(),
        }
    }
}

/// Truth values for IS TRUE/FALSE/UNKNOWN predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthValue {
    True,
    False,
    Unknown,
}

/// Label expression (placeholder for Sprint 8)
#[derive(Debug, Clone, PartialEq)]
pub struct LabelExpression {
    pub label: SmolStr,
    pub span: Span,
}

/// Type reference (placeholder for Sprint 6 - Type System)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeReference {
    pub type_name: SmolStr,
    pub span: Span,
}

// ============================================================================
// EXISTS Expression
// ============================================================================

/// EXISTS predicate expression
#[derive(Debug, Clone, PartialEq)]
pub struct ExistsExpression {
    /// EXISTS variant (graph pattern or subquery)
    pub variant: ExistsVariant,
    pub span: Span,
}

/// Variants of EXISTS predicate
#[derive(Debug, Clone, PartialEq)]
pub enum ExistsVariant {
    /// EXISTS { graph_pattern } - to be implemented in Sprint 8
    GraphPattern(GraphPatternPlaceholder),
    /// EXISTS (query) - subquery form
    Subquery(Box<Expression>),
}

/// Placeholder for graph patterns (Sprint 8)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatternPlaceholder {
    pub span: Span,
}

// ============================================================================
// Function Calls
// ============================================================================

/// Function call expression
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Function name
    pub name: FunctionName,
    /// Function arguments
    pub arguments: Vec<Expression>,
    /// Span covering the entire function call
    pub span: Span,
}

/// Built-in and user-defined function names
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionName {
    // Numeric functions
    Abs,
    Mod,
    Floor,
    Ceil,
    Sqrt,
    Power,
    Exp,
    Ln,
    Log,
    Log10,

    // Trigonometric functions
    Sin,
    Cos,
    Tan,
    Cot,
    Sinh,
    Cosh,
    Tanh,
    Asin,
    Acos,
    Atan,
    Atan2,
    Degrees,
    Radians,

    // String functions
    Upper,
    Lower,
    Trim(TrimSpecification),
    BTrim,
    LTrim,
    RTrim,
    Left,
    Right,
    Normalize,
    CharLength,
    ByteLength,
    Substring,

    // Datetime functions
    CurrentDate,
    CurrentTime,
    CurrentTimestamp,
    Date,
    Time,
    Datetime,
    ZonedTime,
    ZonedDatetime,
    LocalTime,
    LocalDatetime,
    Duration,
    DurationBetween,

    // List functions
    TrimList,
    Elements,

    // Cardinality functions
    Cardinality,
    Size,
    PathLength,

    // Graph functions
    ElementId,

    // Conditional functions
    Coalesce,
    NullIf,

    // User-defined function (extensibility)
    Custom(SmolStr),
}

/// TRIM specification for TRIM functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimSpecification {
    Leading,
    Trailing,
    Both,
}

// ============================================================================
// CASE Expression
// ============================================================================

/// CASE expression (simple or searched form)
#[derive(Debug, Clone, PartialEq)]
pub enum CaseExpression {
    /// Simple CASE: CASE operand WHEN value THEN result ...
    Simple(SimpleCaseExpression),
    /// Searched CASE: CASE WHEN condition THEN result ...
    Searched(SearchedCaseExpression),
}

impl CaseExpression {
    /// Returns the span of this CASE expression
    pub fn span(&self) -> Span {
        match self {
            CaseExpression::Simple(s) => s.span.clone(),
            CaseExpression::Searched(s) => s.span.clone(),
        }
    }
}

/// Simple CASE expression: CASE operand WHEN value THEN result ... END
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCaseExpression {
    /// The operand to compare against
    pub operand: Box<Expression>,
    /// WHEN clauses
    pub when_clauses: Vec<SimpleWhenClause>,
    /// Optional ELSE clause
    pub else_clause: Option<Box<Expression>>,
    /// Span covering the entire CASE expression
    pub span: Span,
}

/// WHEN clause in simple CASE
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleWhenClause {
    /// Value to compare operand against
    pub when_value: Expression,
    /// Result if matched
    pub then_result: Expression,
    /// Span covering this WHEN clause
    pub span: Span,
}

/// Searched CASE expression: CASE WHEN condition THEN result ... END
#[derive(Debug, Clone, PartialEq)]
pub struct SearchedCaseExpression {
    /// WHEN clauses with predicates
    pub when_clauses: Vec<SearchedWhenClause>,
    /// Optional ELSE clause
    pub else_clause: Option<Box<Expression>>,
    /// Span covering the entire CASE expression
    pub span: Span,
}

/// WHEN clause in searched CASE
#[derive(Debug, Clone, PartialEq)]
pub struct SearchedWhenClause {
    /// Condition to evaluate (predicate)
    pub condition: Expression,
    /// Result if condition is true
    pub then_result: Expression,
    /// Span covering this WHEN clause
    pub span: Span,
}

// ============================================================================
// CAST Expression
// ============================================================================

/// CAST expression: CAST(expr AS type)
#[derive(Debug, Clone, PartialEq)]
pub struct CastExpression {
    /// Expression to cast
    pub operand: Box<Expression>,
    /// Target type (will be fully implemented in Sprint 6)
    pub target_type: TypeReference,
    /// Span covering the entire CAST expression
    pub span: Span,
}
