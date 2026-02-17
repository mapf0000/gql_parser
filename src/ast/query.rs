//! Query statement AST nodes for GQL query pipeline.
//!
//! This module implements the query pipeline infrastructure that forms the backbone
//! of GQL's compositional query model. Queries can be composed using set operators
//! (UNION, EXCEPT, INTERSECT, OTHERWISE) and sequential clause chaining.
//!
//! # Query Hierarchy
//!
//! - **Composite Queries**: Combine queries with set operators
//! - **Linear Queries**: Chain primitive operations sequentially
//! - **Primitive Query Statements**: Individual operations (MATCH, FILTER, LET, FOR, SELECT)
//! - **Result Statements**: Optional RETURN/FINISH clauses
//!
//! # Examples
//!
//! ```text
//! // Composite query with UNION
//! MATCH (n:Person) RETURN n
//! UNION
//! MATCH (m:Company) RETURN m
//!
//! // Linear query with chaining
//! MATCH (n:Person)
//! FILTER n.age > 18
//! LET adult_name = n.name
//! RETURN adult_name
//! ```

use crate::ast::{Expression, Span, ValueType};
use crate::ast::references::BindingVariable;
use smol_str::SmolStr;

// ============================================================================
// Top-level Query Types (Tasks 1-2)
// ============================================================================

/// A GQL query statement.
///
/// Queries can be either linear (sequential operations), composite (combined
/// with set operators), or parenthesized (for precedence control).
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Linear query statement (sequential primitive operations).
    Linear(LinearQuery),
    /// Composite query with set operators (UNION, EXCEPT, INTERSECT, OTHERWISE).
    Composite(CompositeQuery),
    /// Parenthesized query (for precedence control).
    Parenthesized(Box<Query>, Span),
}

impl Query {
    /// Returns the span of this query.
    pub fn span(&self) -> &Span {
        match self {
            Query::Linear(q) => q.span(),
            Query::Composite(q) => &q.span,
            Query::Parenthesized(_, span) => span,
        }
    }
}

// ============================================================================
// Composite Query Types (Task 1)
// ============================================================================

/// A composite query combining two queries with a set operator.
///
/// All set operators (UNION, EXCEPT, INTERSECT, OTHERWISE) have the same
/// precedence and are left-associative.
///
/// # Examples
///
/// ```text
/// query1 UNION query2
/// query1 EXCEPT ALL query2
/// query1 INTERSECT DISTINCT query2
/// query1 OTHERWISE query2
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeQuery {
    /// Left operand query.
    pub left: Box<Query>,
    /// Set operator.
    pub operator: SetOperator,
    /// Right operand query.
    pub right: Box<Query>,
    /// Source span.
    pub span: Span,
}

/// Set operators for combining queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetOperator {
    /// UNION [ALL | DISTINCT] - combines results from both queries.
    Union { quantifier: SetQuantifier },
    /// EXCEPT [ALL | DISTINCT] - removes right query results from left.
    Except { quantifier: SetQuantifier },
    /// INTERSECT [ALL | DISTINCT] - returns only common results.
    Intersect { quantifier: SetQuantifier },
    /// OTHERWISE - returns left query results, or right query if left is empty.
    Otherwise,
}

/// Set quantifier for controlling duplicate handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetQuantifier {
    /// ALL - include duplicates.
    All,
    /// DISTINCT - remove duplicates (default when omitted).
    Distinct,
}

impl Default for SetQuantifier {
    fn default() -> Self {
        SetQuantifier::Distinct
    }
}

// ============================================================================
// Linear Query Types (Task 2)
// ============================================================================

/// A linear query statement with sequential primitive operations.
///
/// Linear queries chain primitive operations (MATCH, FILTER, LET, FOR, etc.)
/// in sequence, where each operation transforms the working table produced
/// by the previous operation.
#[derive(Debug, Clone, PartialEq)]
pub enum LinearQuery {
    /// Focused linear query with explicit USE GRAPH clause.
    Focused(FocusedLinearQuery),
    /// Ambient linear query using session default graph.
    Ambient(AmbientLinearQuery),
}

impl LinearQuery {
    /// Returns the span of this linear query.
    pub fn span(&self) -> &Span {
        match self {
            LinearQuery::Focused(q) => &q.span,
            LinearQuery::Ambient(q) => &q.span,
        }
    }
}

/// A focused linear query with explicit graph context.
///
/// # Example
///
/// ```text
/// USE myGraph
/// MATCH (n:Person)
/// RETURN n
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FocusedLinearQuery {
    /// USE GRAPH clause specifying the graph context.
    pub use_graph: UseGraphClause,
    /// Sequential primitive query statements.
    pub primitive_statements: Vec<PrimitiveQueryStatement>,
    /// Optional result statement (RETURN/FINISH).
    pub result_statement: Option<Box<PrimitiveResultStatement>>,
    /// Source span.
    pub span: Span,
}

/// An ambient linear query using the session default graph.
///
/// # Example
///
/// ```text
/// MATCH (n:Person)
/// FILTER n.age > 18
/// RETURN n.name
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientLinearQuery {
    /// Sequential primitive query statements.
    pub primitive_statements: Vec<PrimitiveQueryStatement>,
    /// Optional result statement (RETURN/FINISH).
    pub result_statement: Option<Box<PrimitiveResultStatement>>,
    /// Source span.
    pub span: Span,
}

/// A primitive query statement (individual operation in query pipeline).
///
/// Each primitive statement operates on the working table produced by
/// the previous statement in the pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveQueryStatement {
    /// MATCH statement for graph pattern matching.
    Match(MatchStatement),
    /// FILTER statement for filtering results.
    Filter(FilterStatement),
    /// LET statement for variable bindings.
    Let(LetStatement),
    /// FOR statement for iteration.
    For(ForStatement),
    /// ORDER BY and pagination (LIMIT/OFFSET).
    OrderByAndPage(OrderByAndPageStatement),
    /// SELECT statement with SQL-style syntax.
    Select(SelectStatement),
}

impl PrimitiveQueryStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            PrimitiveQueryStatement::Match(s) => s.span(),
            PrimitiveQueryStatement::Filter(s) => &s.span,
            PrimitiveQueryStatement::Let(s) => &s.span,
            PrimitiveQueryStatement::For(s) => &s.span,
            PrimitiveQueryStatement::OrderByAndPage(s) => &s.span,
            PrimitiveQueryStatement::Select(s) => &s.span,
        }
    }
}

/// A primitive result statement (query terminator).
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveResultStatement {
    /// RETURN statement for returning query results.
    Return(ReturnStatement),
    /// FINISH statement (placeholder for future implementation).
    Finish(Span),
}

impl PrimitiveResultStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            PrimitiveResultStatement::Return(s) => &s.span,
            PrimitiveResultStatement::Finish(span) => span,
        }
    }
}

// ============================================================================
// USE GRAPH Clause (Task 11)
// ============================================================================

/// USE GRAPH clause for specifying query graph context.
///
/// # Example
///
/// ```text
/// USE myGraph
/// USE GRAPH currentGraph
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UseGraphClause {
    /// Graph expression (can be a reference or computed expression).
    pub graph: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Match Statements (Task 3)
// ============================================================================

/// MATCH statement for graph pattern matching.
///
/// Note: Detailed pattern parsing is deferred to Sprint 8. This provides
/// the structural foundation.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchStatement {
    /// Simple MATCH statement.
    Simple(SimpleMatchStatement),
    /// OPTIONAL MATCH statement.
    Optional(OptionalMatchStatement),
}

impl MatchStatement {
    /// Returns the span of this match statement.
    pub fn span(&self) -> &Span {
        match self {
            MatchStatement::Simple(s) => &s.span,
            MatchStatement::Optional(s) => &s.span,
        }
    }
}

/// Simple MATCH statement for required pattern matching.
///
/// # Example
///
/// ```text
/// MATCH (n:Person)-[:KNOWS]->(m:Person)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleMatchStatement {
    /// Graph pattern to match (detailed structure in Sprint 8).
    pub pattern: GraphPattern,
    /// Source span.
    pub span: Span,
}

/// OPTIONAL MATCH statement for optional pattern matching.
///
/// # Example
///
/// ```text
/// OPTIONAL MATCH (n)-[:FRIEND]->(f)
/// OPTIONAL { MATCH (n)-[:KNOWS]->(k) }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalMatchStatement {
    /// What to optionally match.
    pub operand: OptionalOperand,
    /// Source span.
    pub span: Span,
}

/// Operand for OPTIONAL MATCH.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionalOperand {
    /// OPTIONAL MATCH <pattern>
    Match { pattern: GraphPattern },
    /// OPTIONAL { <match_block> }
    Block { statements: Vec<MatchStatement> },
    /// OPTIONAL ( <match_block> )
    ParenthesizedBlock { statements: Vec<MatchStatement> },
}

/// Graph pattern placeholder (detailed implementation in Sprint 8).
///
/// This placeholder provides integration points for Sprint 8's graph
/// pattern matching implementation. For Sprint 7, we parse the basic
/// structure only.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPattern {
    /// Source span.
    pub span: Span,
    // Future fields (Sprint 8):
    // - match_mode: MatchMode
    // - path_patterns: Vec<PathPattern>
    // - where_clause: Option<WhereClause>
    // - yield_clause: Option<YieldClause>
}

// ============================================================================
// Filter Statements (Task 4)
// ============================================================================

/// FILTER statement for filtering query results.
///
/// # Examples
///
/// ```text
/// FILTER n.age > 18
/// FILTER WHERE n.active = true
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FilterStatement {
    /// Whether WHERE keyword is present (optional in GQL).
    pub where_optional: bool,
    /// Search condition (boolean expression).
    pub condition: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Let Statements (Task 5)
// ============================================================================

/// LET statement for variable bindings.
///
/// # Example
///
/// ```text
/// LET full_name = n.firstName + ' ' + n.lastName,
///     age_in_months = n.age * 12
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement {
    /// Variable definitions (can bind multiple variables).
    pub bindings: Vec<LetVariableDefinition>,
    /// Source span.
    pub span: Span,
}

/// Variable definition in LET statement.
#[derive(Debug, Clone, PartialEq)]
pub struct LetVariableDefinition {
    /// Variable name.
    pub variable: BindingVariable,
    /// Optional type annotation.
    pub type_annotation: Option<ValueType>,
    /// Computed value expression.
    pub value: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// For Statements (Task 6)
// ============================================================================

/// FOR statement for iteration over collections.
///
/// # Examples
///
/// ```text
/// FOR item IN collection
/// FOR x IN [1, 2, 3] WITH ORDINALITY AS ord
/// FOR elem IN array WITH OFFSET AS idx
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    /// Iteration specification.
    pub item: ForItem,
    /// Optional WITH ORDINALITY or WITH OFFSET clause.
    pub ordinality_or_offset: Option<ForOrdinalityOrOffset>,
    /// Source span.
    pub span: Span,
}

/// Iteration specification for FOR statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ForItem {
    /// Loop variable binding.
    pub binding_variable: BindingVariable,
    /// Collection expression to iterate over.
    pub collection: Expression,
    /// Source span.
    pub span: Span,
}

/// WITH ORDINALITY or WITH OFFSET clause for FOR statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ForOrdinalityOrOffset {
    /// WITH ORDINALITY <variable> - 1-based position.
    Ordinality { variable: BindingVariable },
    /// WITH OFFSET <variable> - 0-based position.
    Offset { variable: BindingVariable },
}

// ============================================================================
// Select Statements (Task 7)
// ============================================================================

/// SELECT statement with SQL-style syntax.
///
/// # Example
///
/// ```text
/// SELECT DISTINCT n.name, n.age
/// FROM MATCH (n:Person)
/// WHERE n.active = true
/// GROUP BY n.department
/// HAVING COUNT(*) > 5
/// ORDER BY n.name ASC
/// LIMIT 10
/// OFFSET 5
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    /// Set quantifier (DISTINCT or ALL).
    pub quantifier: Option<SetQuantifier>,
    /// What to select.
    pub select_items: SelectItemList,
    /// Optional FROM clause.
    pub from_clause: Option<SelectFromClause>,
    /// Optional WHERE clause.
    pub where_clause: Option<WhereClause>,
    /// Optional GROUP BY clause.
    pub group_by: Option<GroupByClause>,
    /// Optional HAVING clause.
    pub having: Option<HavingClause>,
    /// Optional ORDER BY clause.
    pub order_by: Option<OrderByClause>,
    /// Optional OFFSET clause.
    pub offset: Option<OffsetClause>,
    /// Optional LIMIT clause.
    pub limit: Option<LimitClause>,
    /// Source span.
    pub span: Span,
}

/// Select item list (what to select).
#[derive(Debug, Clone, PartialEq)]
pub enum SelectItemList {
    /// SELECT *
    Star,
    /// SELECT item1, item2, ...
    Items { items: Vec<SelectItem> },
}

/// Individual item in SELECT clause.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    /// Expression to select.
    pub expression: Expression,
    /// Optional AS alias.
    pub alias: Option<SmolStr>,
    /// Source span.
    pub span: Span,
}

/// FROM clause in SELECT statement.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectFromClause {
    /// FROM graph match list.
    GraphMatchList { matches: Vec<GraphPattern> },
    /// FROM nested query.
    QuerySpecification { query: Box<Query> },
    /// FROM graph and query.
    GraphAndQuerySpecification {
        graph: Expression,
        query: Box<Query>,
    },
}

/// WHERE clause (filter condition).
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    /// Filter condition (boolean expression).
    pub condition: Expression,
    /// Source span.
    pub span: Span,
}

/// HAVING clause (filter condition on aggregates).
#[derive(Debug, Clone, PartialEq)]
pub struct HavingClause {
    /// Filter condition on aggregates (boolean expression).
    pub condition: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Return Statements (Task 8)
// ============================================================================

/// RETURN statement for returning query results.
///
/// # Examples
///
/// ```text
/// RETURN *
/// RETURN DISTINCT n.name, n.age
/// RETURN n.value AS val GROUP BY n.category
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    /// Set quantifier (DISTINCT or ALL).
    pub quantifier: Option<SetQuantifier>,
    /// What to return.
    pub items: ReturnItemList,
    /// Optional GROUP BY clause.
    pub group_by: Option<GroupByClause>,
    /// Source span.
    pub span: Span,
}

/// Return item list (what to return).
#[derive(Debug, Clone, PartialEq)]
pub enum ReturnItemList {
    /// RETURN *
    Star,
    /// RETURN item1, item2, ...
    Items { items: Vec<ReturnItem> },
}

/// Individual item in RETURN clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnItem {
    /// Expression to return.
    pub expression: Expression,
    /// Optional AS alias.
    pub alias: Option<SmolStr>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Ordering and Pagination (Task 9)
// ============================================================================

/// ORDER BY and pagination statement.
///
/// # Example
///
/// ```text
/// ORDER BY n.name ASC, n.age DESC NULLS LAST
/// LIMIT 10
/// OFFSET 5
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByAndPageStatement {
    /// Optional ORDER BY clause.
    pub order_by: Option<OrderByClause>,
    /// Optional OFFSET clause.
    pub offset: Option<OffsetClause>,
    /// Optional LIMIT clause.
    pub limit: Option<LimitClause>,
    /// Source span.
    pub span: Span,
}

/// ORDER BY clause with sort specifications.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByClause {
    /// Sort specifications (keys and directions).
    pub sort_specifications: Vec<SortSpecification>,
    /// Source span.
    pub span: Span,
}

/// Individual sort specification (key + direction + null ordering).
#[derive(Debug, Clone, PartialEq)]
pub struct SortSpecification {
    /// Expression to sort by.
    pub key: Expression,
    /// Optional ordering direction (ASC or DESC).
    pub ordering: Option<OrderingSpecification>,
    /// Optional null ordering (NULLS FIRST or NULLS LAST).
    pub null_ordering: Option<NullOrdering>,
    /// Source span.
    pub span: Span,
}

/// Ordering direction (ascending or descending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingSpecification {
    /// ASC or ASCENDING (default).
    Ascending,
    /// DESC or DESCENDING.
    Descending,
}

impl Default for OrderingSpecification {
    fn default() -> Self {
        OrderingSpecification::Ascending
    }
}

/// Null ordering specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullOrdering {
    /// NULLS FIRST - nulls sort before non-nulls.
    NullsFirst,
    /// NULLS LAST - nulls sort after non-nulls.
    NullsLast,
}

/// LIMIT clause (row count limit).
#[derive(Debug, Clone, PartialEq)]
pub struct LimitClause {
    /// Number of rows to return.
    pub count: Expression,
    /// Source span.
    pub span: Span,
}

/// OFFSET clause (skip rows).
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetClause {
    /// Number of rows to skip.
    pub count: Expression,
    /// Whether SKIP keyword was used instead of OFFSET.
    pub use_skip_keyword: bool,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Grouping (Task 10)
// ============================================================================

/// GROUP BY clause for grouping results.
///
/// # Examples
///
/// ```text
/// GROUP BY n.category
/// GROUP BY n.dept, n.team
/// GROUP BY ()  -- empty grouping set for full aggregation
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GroupByClause {
    /// Grouping elements (keys or empty set).
    pub elements: Vec<GroupingElement>,
    /// Source span.
    pub span: Span,
}

/// Grouping element (expression or empty grouping set).
#[derive(Debug, Clone, PartialEq)]
pub enum GroupingElement {
    /// Group by expression.
    Expression(Expression),
    /// Empty grouping set () for single aggregated result.
    EmptyGroupingSet,
}
