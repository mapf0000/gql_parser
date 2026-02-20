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

use crate::ast::references::BindingVariable;
use crate::ast::{Expression, Span, ValueType};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetQuantifier {
    /// ALL - include duplicates.
    All,
    /// DISTINCT - remove duplicates (default when omitted).
    #[default]
    Distinct,
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
    /// CALL procedure statement.
    Call(crate::ast::procedure::CallProcedureStatement),
    /// FILTER statement for filtering results.
    Filter(FilterStatement),
    /// LET statement for variable bindings.
    Let(LetStatement),
    /// FOR statement for iteration.
    For(ForStatement),
    /// ORDER BY and pagination (LIMIT/OFFSET).
    OrderByAndPage(OrderByAndPageStatement),
    /// SELECT statement with SQL-style syntax.
    Select(Box<SelectStatement>),
}

impl PrimitiveQueryStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            PrimitiveQueryStatement::Match(s) => s.span(),
            PrimitiveQueryStatement::Call(s) => &s.span,
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
    Simple(Box<SimpleMatchStatement>),
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
    Match { pattern: Box<GraphPattern> },
    /// OPTIONAL { <match_block> }
    Block { statements: Vec<MatchStatement> },
    /// OPTIONAL ( <match_block> )
    ParenthesizedBlock { statements: Vec<MatchStatement> },
}

// ============================================================================
// Graph Patterns (Sprint 8 - Tasks 1-9)
// ============================================================================

/// Graph pattern for graph matching in MATCH statements.
///
/// A graph pattern specifies nodes, edges, and paths to match in the graph,
/// with optional match modes, keep clauses, where clauses, and yield clauses.
///
/// # Example
///
/// ```text
/// MATCH REPEATABLE ELEMENTS (a)-[e:KNOWS]->(b)
/// KEEP TRAIL
/// WHERE a.age > 18
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPattern {
    /// Optional match mode (REPEATABLE ELEMENTS or DIFFERENT EDGES).
    pub match_mode: Option<MatchMode>,
    /// Path patterns to match.
    pub paths: PathPatternList,
    /// Optional KEEP clause specifying path mode to keep.
    pub keep_clause: Option<KeepClause>,
    /// Optional WHERE clause for filtering matches.
    pub where_clause: Option<GraphPatternWhereClause>,
    /// Optional YIELD clause for selecting output bindings from the pattern.
    pub yield_clause: Option<GraphPatternYieldClause>,
    /// Source span.
    pub span: Span,
}

/// Match mode for graph patterns.
///
/// Controls how pattern matching handles repeated elements (nodes/edges) in paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchMode {
    /// REPEATABLE ELEMENTS - allows repeated nodes and edges (default).
    #[default]
    RepeatableElements,
    /// DIFFERENT EDGES - requires edges to be different (nodes can repeat).
    DifferentEdges,
}

/// Path pattern list - comma-separated path patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPatternList {
    /// Path patterns in the list.
    pub patterns: Vec<PathPattern>,
    /// Source span.
    pub span: Span,
}

/// A single path pattern with optional prefix and variable declaration.
///
/// # Examples
///
/// ```text
/// (a)-[e]->(b)
/// TRAIL (a)-[e]->(b) AS myPath
/// ALL SHORTEST SIMPLE (a)-[*]->(b)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PathPattern {
    /// Optional path mode/search prefix.
    pub prefix: Option<PathPatternPrefix>,
    /// Path pattern expression.
    pub expression: PathPatternExpression,
    /// Optional path variable declaration (AS variable).
    pub variable_declaration: Option<PathVariableDeclaration>,
    /// Source span.
    pub span: Span,
}

/// Path variable declaration (AS variable) for binding path results.
#[derive(Debug, Clone, PartialEq)]
pub struct PathVariableDeclaration {
    /// Path variable name.
    pub variable: PathVariable,
    /// Source span.
    pub span: Span,
}

/// A path variable (identifier for binding paths).
pub type PathVariable = SmolStr;

/// KEEP clause specifying which path mode to preserve.
///
/// # Example
///
/// ```text
/// KEEP TRAIL
/// KEEP SIMPLE
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct KeepClause {
    /// Path pattern prefix to keep.
    pub prefix: PathPatternPrefix,
    /// Source span.
    pub span: Span,
}

/// Graph pattern WHERE clause for filtering matches.
///
/// # Example
///
/// ```text
/// WHERE a.age > 18 AND b.active = true
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatternWhereClause {
    /// Filter condition (boolean expression).
    pub condition: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Path Pattern Prefixes (Task 2)
// ============================================================================

/// Path pattern prefix - specifies path mode or path search strategy.
///
/// # Examples
///
/// ```text
/// WALK       -- path mode
/// ALL SHORTEST SIMPLE   -- path search
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PathPatternPrefix {
    /// Path mode prefix (WALK, TRAIL, SIMPLE, ACYCLIC).
    PathMode(PathMode),
    /// Path search prefix (ALL, ANY, SHORTEST variants).
    PathSearch(PathSearch),
}

/// Path mode specifying what kinds of paths to match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathMode {
    /// WALK - any path (default), allows repeated nodes and edges.
    #[default]
    Walk,
    /// TRAIL - no repeated edges (nodes can repeat).
    Trail,
    /// SIMPLE - no repeated nodes or edges.
    Simple,
    /// ACYCLIC - no repeated nodes (edges can repeat).
    Acyclic,
}

/// Path search strategy specifying which paths to find.
#[derive(Debug, Clone, PartialEq)]
pub enum PathSearch {
    /// ALL [path_mode] [PATHS] - find all matching paths.
    All(AllPathSearch),
    /// ANY [path_mode] - find any single matching path.
    Any(AnyPathSearch),
    /// SHORTEST [variants] - find shortest path(s).
    Shortest(ShortestPathSearch),
}

/// ALL path search - find all matching paths.
///
/// # Examples
///
/// ```text
/// ALL
/// ALL TRAIL
/// ALL SIMPLE PATHS
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AllPathSearch {
    /// Optional path mode.
    pub mode: Option<PathMode>,
    /// Whether PATHS keyword is present.
    pub use_paths_keyword: bool,
    /// Source span.
    pub span: Span,
}

/// ANY path search - find any single matching path.
///
/// # Examples
///
/// ```text
/// ANY
/// ANY TRAIL
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AnyPathSearch {
    /// Optional path mode.
    pub mode: Option<PathMode>,
    /// Source span.
    pub span: Span,
}

/// SHORTEST path search - find shortest path(s).
#[derive(Debug, Clone, PartialEq)]
pub enum ShortestPathSearch {
    /// ALL SHORTEST [path_mode] - find all shortest paths.
    AllShortest { mode: Option<PathMode>, span: Span },
    /// ANY SHORTEST [path_mode] - find any single shortest path.
    AnyShortest { mode: Option<PathMode>, span: Span },
    /// SHORTEST k [path_mode] [PATHS] - find k shortest paths.
    CountedShortest {
        count: Expression,
        mode: Option<PathMode>,
        use_paths_keyword: bool,
        span: Span,
    },
    /// SHORTEST k [path_mode] GROUPS - find k groups of equal-length shortest paths.
    CountedShortestGroups {
        count: Expression,
        mode: Option<PathMode>,
        span: Span,
    },
}

impl ShortestPathSearch {
    /// Returns the span of this shortest path search.
    pub fn span(&self) -> &Span {
        match self {
            ShortestPathSearch::AllShortest { span, .. }
            | ShortestPathSearch::AnyShortest { span, .. }
            | ShortestPathSearch::CountedShortest { span, .. }
            | ShortestPathSearch::CountedShortestGroups { span, .. } => span,
        }
    }
}

// ============================================================================
// Path Pattern Expressions (Task 3)
// ============================================================================

/// Path pattern expression with alternation and union operators.
///
/// # Examples
///
/// ```text
/// (a)-[e]->(b)                    -- single term
/// (a)-[:KNOWS]->(b) | (a)-[:LIKES]->(b)  -- alternation
/// (a)-[e]->(b) UNION (c)-[f]->(d)         -- union
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PathPatternExpression {
    /// Path union - combine path patterns.
    Union {
        left: Box<PathPatternExpression>,
        right: Box<PathPatternExpression>,
        span: Span,
    },
    /// Path alternation - multiple alternative patterns.
    Alternation {
        alternatives: Vec<PathTerm>,
        span: Span,
    },
    /// Single path term.
    Term(PathTerm),
}

impl PathPatternExpression {
    /// Returns the span of this expression.
    pub fn span(&self) -> &Span {
        match self {
            PathPatternExpression::Union { span, .. } => span,
            PathPatternExpression::Alternation { span, .. } => span,
            PathPatternExpression::Term(term) => &term.span,
        }
    }
}

/// Path term - sequential composition of path factors.
///
/// # Example
///
/// ```text
/// (a)-[e]->(b)-[f]->(c)   -- sequence of 2 factors
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PathTerm {
    /// Sequential path factors.
    pub factors: Vec<PathFactor>,
    /// Source span.
    pub span: Span,
}

/// Path factor - primary pattern with optional quantifier.
///
/// # Examples
///
/// ```text
/// (a)-[e]->(b)      -- primary without quantifier
/// (a)-[e]->(b)+     -- primary with quantifier
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PathFactor {
    /// Base path element (primary).
    pub primary: PathPrimary,
    /// Optional quantifier (*, +, ?, {n}, {n,m}).
    pub quantifier: Option<GraphPatternQuantifier>,
    /// Source span.
    pub span: Span,
}

/// Path primary - base element of path pattern.
///
/// # Examples
///
/// ```text
/// (a)                  -- node pattern
/// -[e]->               -- edge pattern
/// ((a)-[e]->(b))       -- parenthesized subpattern
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum PathPrimary {
    /// Element pattern (node or edge).
    ElementPattern(Box<ElementPattern>),
    /// Parenthesized path pattern expression.
    ParenthesizedExpression(Box<PathPatternExpression>),
    /// Simplified path pattern expression.
    SimplifiedExpression(Box<SimplifiedPathPatternExpression>),
}

/// Graph pattern quantifier for repeating patterns.
///
/// # Examples
///
/// ```text
/// *        -- zero or more
/// +        -- one or more
/// ?        -- zero or one
/// {3}      -- exactly 3
/// {2,5}    -- between 2 and 5
/// {3,}     -- at least 3
/// {,10}    -- at most 10
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphPatternQuantifier {
    /// * - zero or more (Kleene star).
    Star { span: Span },
    /// + - one or more (Kleene plus).
    Plus { span: Span },
    /// ? - zero or one (optional).
    QuestionMark { span: Span },
    /// {n} - exactly n times.
    Fixed { count: u32, span: Span },
    /// {n,m} - between n and m times; {n,} - at least n; {,m} - at most m.
    General {
        min: Option<u32>,
        max: Option<u32>,
        span: Span,
    },
}

impl GraphPatternQuantifier {
    /// Returns the span of this quantifier.
    pub fn span(&self) -> &Span {
        match self {
            GraphPatternQuantifier::Star { span }
            | GraphPatternQuantifier::Plus { span }
            | GraphPatternQuantifier::QuestionMark { span }
            | GraphPatternQuantifier::Fixed { span, .. }
            | GraphPatternQuantifier::General { span, .. } => span,
        }
    }
}

// ============================================================================
// Element Patterns - Nodes (Task 4)
// ============================================================================

/// Element pattern - node or edge pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementPattern {
    /// Node pattern: (variable :label {props} WHERE pred)
    Node(Box<NodePattern>),
    /// Edge pattern: -[edge]->
    Edge(EdgePattern),
}

/// Node pattern - matches nodes in the graph.
///
/// # Examples
///
/// ```text
/// ()                           -- anonymous node
/// (n)                          -- node with variable
/// (n:Person)                   -- node with label
/// (n:Person {age: 30})         -- node with properties
/// (n WHERE n.active = true)    -- node with predicate
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NodePattern {
    /// Optional element variable.
    pub variable: Option<ElementVariableDeclaration>,
    /// Optional label expression.
    pub label_expression: Option<LabelExpression>,
    /// Optional property specifications.
    pub properties: Option<ElementPropertySpecification>,
    /// Optional WHERE predicate.
    pub where_clause: Option<ElementPatternPredicate>,
    /// Source span.
    pub span: Span,
}

/// Element variable declaration - binds a variable to a matched element.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementVariableDeclaration {
    /// Element variable name.
    pub variable: ElementVariable,
    /// Source span.
    pub span: Span,
}

/// An element variable (identifier for binding nodes/edges).
pub type ElementVariable = SmolStr;

/// Element property specification - property constraints for matching.
///
/// # Example
///
/// ```text
/// {name: 'Alice', age: 30, active: true}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPropertySpecification {
    /// Property key-value pairs.
    pub properties: Vec<PropertyKeyValuePair>,
    /// Source span.
    pub span: Span,
}

/// Property key-value pair for element matching.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyKeyValuePair {
    /// Property name.
    pub key: SmolStr,
    /// Property value expression.
    pub value: Expression,
    /// Source span.
    pub span: Span,
}

/// Element pattern predicate - WHERE clause in element pattern.
///
/// # Example
///
/// ```text
/// WHERE n.age > 18 AND n.active = true
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPatternPredicate {
    /// Predicate condition (boolean expression).
    pub condition: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Element Patterns - Edges (Task 5)
// ============================================================================

/// Edge pattern - matches edges in the graph.
///
/// Edges can be full (with details) or abbreviated (arrows only).
#[derive(Debug, Clone, PartialEq)]
pub enum EdgePattern {
    /// Full edge pattern with variable, labels, properties, and predicates.
    Full(Box<FullEdgePattern>),
    /// Abbreviated edge pattern (arrow only).
    Abbreviated(AbbreviatedEdgePattern),
}

/// Full edge pattern with direction and filler.
///
/// # Examples
///
/// ```text
/// -[e:KNOWS]->
/// <-[r:FOLLOWS {since: 2020}]-
/// ~[s:SIMILAR]~
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FullEdgePattern {
    /// Edge direction.
    pub direction: EdgeDirection,
    /// Edge filler (variable, labels, properties, predicates).
    pub filler: FullEdgePointingFiller,
    /// Source span.
    pub span: Span,
}

/// Edge direction - 7 possible directions in GQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    /// <-[edge]- - pointing left.
    PointingLeft,
    /// -[edge]-> - pointing right.
    PointingRight,
    /// ~[edge]~ - undirected.
    Undirected,
    /// <-[edge]-> - any directed (bidirectional).
    AnyDirected,
    /// <~[edge]~ - left or undirected.
    LeftOrUndirected,
    /// -[edge]- - any direction.
    AnyDirection,
    /// ~[edge]-> - undirected or right.
    RightOrUndirected,
}

/// Full edge filler - details for full edge patterns.
///
/// Same structure as node pattern filler.
#[derive(Debug, Clone, PartialEq)]
pub struct FullEdgePointingFiller {
    /// Optional element variable.
    pub variable: Option<ElementVariableDeclaration>,
    /// Optional label expression.
    pub label_expression: Option<LabelExpression>,
    /// Optional property specifications.
    pub properties: Option<ElementPropertySpecification>,
    /// Optional WHERE predicate.
    pub where_clause: Option<ElementPatternPredicate>,
    /// Source span.
    pub span: Span,
}

/// Abbreviated edge pattern - arrow syntax without details.
///
/// # Examples
///
/// ```text
/// <-    -- left arrow
/// ->    -- right arrow
/// ~     -- undirected
/// -     -- any direction
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbbreviatedEdgePattern {
    /// <- - left arrow.
    LeftArrow { span: Span },
    /// -> - right arrow.
    RightArrow { span: Span },
    /// ~ - undirected.
    Undirected { span: Span },
    /// - - any direction.
    AnyDirection { span: Span },
}

impl AbbreviatedEdgePattern {
    /// Returns the span of this abbreviated edge.
    pub fn span(&self) -> &Span {
        match self {
            AbbreviatedEdgePattern::LeftArrow { span }
            | AbbreviatedEdgePattern::RightArrow { span }
            | AbbreviatedEdgePattern::Undirected { span }
            | AbbreviatedEdgePattern::AnyDirection { span } => span,
        }
    }
}

// ============================================================================
// Label Expressions (Task 9)
// ============================================================================

/// Label expression for matching node/edge labels with boolean algebra.
///
/// # Examples
///
/// ```text
/// :Person                    -- simple label
/// :Person|Company            -- disjunction (OR)
/// :Person&Employee           -- conjunction (AND)
/// :!Deleted                  -- negation (NOT)
/// :%                         -- wildcard (any label)
/// :(Person|Company)&Active   -- complex expression
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum LabelExpression {
    /// ! negation - NOT operator.
    Negation {
        operand: Box<LabelExpression>,
        span: Span,
    },
    /// & conjunction - AND operator.
    Conjunction {
        left: Box<LabelExpression>,
        right: Box<LabelExpression>,
        span: Span,
    },
    /// | disjunction - OR operator.
    Disjunction {
        left: Box<LabelExpression>,
        right: Box<LabelExpression>,
        span: Span,
    },
    /// Simple label name.
    LabelName { name: SmolStr, span: Span },
    /// % wildcard - matches any label.
    Wildcard { span: Span },
    /// Parenthesized label expression.
    Parenthesized {
        expression: Box<LabelExpression>,
        span: Span,
    },
}

impl LabelExpression {
    /// Returns the span of this label expression.
    pub fn span(&self) -> &Span {
        match self {
            LabelExpression::Negation { span, .. }
            | LabelExpression::Conjunction { span, .. }
            | LabelExpression::Disjunction { span, .. }
            | LabelExpression::LabelName { span, .. }
            | LabelExpression::Wildcard { span }
            | LabelExpression::Parenthesized { span, .. } => span,
        }
    }
}

/// IS label expression wrapper (: prefix in patterns).
#[derive(Debug, Clone, PartialEq)]
pub struct IsLabelExpression {
    /// Label expression.
    pub expression: LabelExpression,
    /// Source span.
    pub span: Span,
}

/// Label set specification - ampersand-separated label list.
///
/// # Example
///
/// ```text
/// LABEL Person&Employee&Active
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LabelSetSpecification {
    /// Ampersand-separated labels.
    pub labels: Vec<SmolStr>,
    /// Source span.
    pub span: Span,
}

/// Label set phrase - LABEL or LABELS keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelSetPhrase {
    /// LABEL keyword.
    Label,
    /// LABELS keyword.
    Labels,
}

// ============================================================================
// Simplified Path Patterns (Task 6)
// ============================================================================

/// Simplified path pattern - alternative abbreviated syntax.
///
/// Note: Simplified patterns are a complex alternative syntax for path patterns.
/// This implementation provides basic structure; full implementation may be deferred
/// based on specification details.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedPathPattern {
    /// Simplified pattern expression.
    pub expression: SimplifiedPathPatternExpression,
    /// Source span.
    pub span: Span,
}

/// Simplified path pattern expression.
///
/// Simplified syntax supports all 7 edge directions with alternative notation.
#[derive(Debug, Clone, PartialEq)]
pub enum SimplifiedPathPatternExpression {
    /// Simplified contents (base case).
    Contents(SimplifiedContents),
    /// Union of simplified paths.
    Union(SimplifiedPathUnion),
    /// |+| multiset alternation operator.
    MultisetAlternation(SimplifiedMultisetAlternation),
    /// & conjunction operator.
    Conjunction(SimplifiedConjunction),
    /// Concatenation of simplified factors.
    Concatenation(SimplifiedConcatenation),
    /// Quantified pattern.
    Quantified(SimplifiedQuantified),
    /// Questioned pattern (?).
    Questioned(SimplifiedQuestioned),
    /// Direction override.
    DirectionOverride(SimplifiedDirectionOverride),
    /// Negated pattern.
    Negation(SimplifiedNegation),
}

/// Simplified contents.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedContents {
    /// Label atoms seen in this simplified content segment.
    pub labels: Vec<SmolStr>,
    /// Source span.
    pub span: Span,
}

/// Simplified path union.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedPathUnion {
    /// Left operand.
    pub left: Box<SimplifiedPathPatternExpression>,
    /// Right operand.
    pub right: Box<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

/// Simplified multiset alternation (|+| operator).
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedMultisetAlternation {
    /// Alternatives.
    pub alternatives: Vec<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

/// Simplified conjunction (& operator).
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedConjunction {
    /// Left operand.
    pub left: Box<SimplifiedPathPatternExpression>,
    /// Right operand.
    pub right: Box<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

/// Simplified concatenation.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedConcatenation {
    /// Concatenated parts.
    pub parts: Vec<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

/// Simplified quantified pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedQuantified {
    /// Pattern to quantify.
    pub pattern: Box<SimplifiedPathPatternExpression>,
    /// Quantifier.
    pub quantifier: GraphPatternQuantifier,
    /// Source span.
    pub span: Span,
}

/// Simplified questioned pattern (?).
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedQuestioned {
    /// Pattern to make optional.
    pub pattern: Box<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

/// Simplified direction override.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedDirectionOverride {
    /// Pattern with overridden direction.
    pub pattern: Box<SimplifiedPathPatternExpression>,
    /// New direction.
    pub direction: EdgeDirection,
    /// Source span.
    pub span: Span,
}

/// Simplified negation.
#[derive(Debug, Clone, PartialEq)]
pub struct SimplifiedNegation {
    /// Pattern to negate.
    pub pattern: Box<SimplifiedPathPatternExpression>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Graph Pattern Binding and Yield (Task 7)
// ============================================================================

/// Graph pattern binding table.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatternBindingTable {
    /// Parsed graph pattern.
    pub pattern: Box<GraphPattern>,
    /// Optional YIELD clause associated with the pattern.
    pub yield_clause: Option<GraphPatternYieldClause>,
    /// Source span.
    pub span: Span,
}

/// Graph pattern YIELD clause - yields specific values from matches.
///
/// # Example
///
/// ```text
/// YIELD n.name AS name, n.age AS age
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatternYieldClause {
    /// Yield items.
    pub items: Vec<YieldItem>,
    /// Source span.
    pub span: Span,
}

/// Yield item - expression with optional alias.
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItem {
    /// Expression to yield.
    pub expression: Expression,
    /// Optional AS alias.
    pub alias: Option<SmolStr>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Parenthesized Path Patterns (Task 8)
// ============================================================================

/// Parenthesized path pattern expression for precedence control.
///
/// # Example
///
/// ```text
/// ((a)-[e]->(b) | (a)-[f]->(c))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParenthesizedPathPatternExpression {
    /// Nested path expression.
    pub expression: Box<PathPatternExpression>,
    /// Source span.
    pub span: Span,
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
    /// Optional WITH clause (CTE definitions).
    pub with_clause: Option<WithClause>,
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

/// WITH clause for SELECT statement.
#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    /// Whether RECURSIVE was specified.
    pub recursive: bool,
    /// Common table expression items.
    pub items: Vec<CommonTableExpression>,
    /// Source span.
    pub span: Span,
}

/// Common table expression item.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTableExpression {
    /// CTE identifier.
    pub name: SmolStr,
    /// Optional projected column names.
    pub columns: Vec<SmolStr>,
    /// CTE query payload.
    pub query: Box<Query>,
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
    QuerySpecification {
        query: Box<Query>,
        alias: Option<SmolStr>,
    },
    /// FROM graph and query.
    GraphAndQuerySpecification {
        graph: Expression,
        query: Box<Query>,
        alias: Option<SmolStr>,
    },
    /// FROM list of table/query sources.
    SourceList { sources: Vec<SelectSourceItem> },
}

/// Source item inside FROM source list.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectSourceItem {
    /// Parenthesized or direct query specification source.
    Query {
        query: Box<Query>,
        alias: Option<SmolStr>,
        span: Span,
    },
    /// Graph expression followed by query specification.
    GraphAndQuery {
        graph: Expression,
        query: Box<Query>,
        alias: Option<SmolStr>,
        span: Span,
    },
    /// Table expression source.
    Expression {
        expression: Expression,
        alias: Option<SmolStr>,
        span: Span,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderingSpecification {
    /// ASC or ASCENDING (default).
    #[default]
    Ascending,
    /// DESC or DESCENDING.
    Descending,
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
