//! Procedure call AST nodes for GQL.
//!
//! This module implements the procedural composition features of GQL, enabling:
//! - Procedure calls (inline and named)
//! - Variable scoping and definitions
//! - Nested procedure specifications with bodies
//! - Statement blocks with NEXT chaining
//! - OPTIONAL execution semantics
//! - YIELD clauses for result projection
//!
//! Procedures provide powerful abstractions for modular query organization and
//! reusable query/mutation units with parameter passing and result yielding.
//!
//! # Procedure Call Hierarchy
//!
//! - **CallProcedureStatement**: Top-level CALL statement (with OPTIONAL)
//! - **ProcedureCall**: Inline or named procedure invocation
//! - **InlineProcedureCall**: Inline procedure with variable scope and nested spec
//! - **NamedProcedureCall**: Named procedure with arguments and yield
//! - **NestedProcedureSpecification**: Procedure body with braces
//! - **ProcedureBody**: AT schema, variable definitions, and statement block
//! - **StatementBlock**: Sequential statements with NEXT chaining
//!
//! # Examples
//!
//! ```text
//! // Named procedure call
//! CALL my_procedure(arg1, arg2) YIELD result
//!
//! // Optional procedure call (continues on failure)
//! OPTIONAL CALL risky_operation()
//!
//! // Inline procedure call with variable scope
//! CALL (x, y) {
//!   MATCH (n WHERE n.id = x)
//!   RETURN n
//! }
//!
//! // Nested procedure specification with variable definitions
//! {
//!   AT my_schema
//!   GRAPH g :: GRAPH REFERENCE = CURRENT_GRAPH
//!   VALUE counter :: INT = 0
//!   MATCH (n) RETURN n
//!   NEXT
//!   MATCH (m) RETURN m
//! }
//! ```

use crate::ast::references::{BindingVariable, ProcedureReference, SchemaReference};
use crate::ast::types::{BindingTableReferenceValueType, GraphReferenceValueType, ValueType};
use crate::ast::{Expression, Span};
use smol_str::SmolStr;

// ============================================================================
// Call Procedure Statement (Task 1)
// ============================================================================

/// A CALL procedure statement.
///
/// CALL statements invoke procedures, either inline (with nested procedure specs)
/// or named (with procedure references and arguments). OPTIONAL keyword changes
/// the execution semantics to continue on procedure failure.
///
/// # Examples
///
/// ```text
/// CALL my_procedure(arg1, arg2)
/// OPTIONAL CALL risky_operation()
/// CALL (x, y) { MATCH (n) RETURN n }
/// ```
///
/// # Grammar References
///
/// - `callProcedureStatement` (Line 728)
#[derive(Debug, Clone, PartialEq)]
pub struct CallProcedureStatement {
    /// Whether OPTIONAL keyword is present (continues execution on procedure failure).
    pub optional: bool,
    /// The procedure call (inline or named).
    pub call: ProcedureCall,
    /// Source span.
    pub span: Span,
}

/// Procedure call dispatch (inline or named).
///
/// Procedure calls can be either inline (with nested procedure specifications)
/// or named (referencing an existing procedure by name with arguments).
///
/// # Grammar References
///
/// - `procedureCall` (Line 732)
#[derive(Debug, Clone, PartialEq)]
pub enum ProcedureCall {
    /// Inline procedure call with variable scope and nested specification.
    Inline(InlineProcedureCall),
    /// Named procedure call with arguments and yield clause.
    Named(NamedProcedureCall),
}

// ============================================================================
// Inline Procedure Calls (Task 2)
// ============================================================================

/// An inline procedure call with optional variable scope and nested specification.
///
/// Inline procedure calls execute a nested procedure specification (procedure body
/// in braces) with an optional variable scope clause that specifies input/output
/// variables.
///
/// # Examples
///
/// ```text
/// CALL () { MATCH (n) RETURN n }                    -- Empty variable scope
/// CALL (x, y) { MATCH (n WHERE n.id = x) RETURN n } -- With variables
/// CALL { MATCH (n) RETURN n }                       -- No variable scope
/// ```
///
/// # Grammar References
///
/// - `inlineProcedureCall` (Line 739)
#[derive(Debug, Clone, PartialEq)]
pub struct InlineProcedureCall {
    /// Optional variable scope clause (specifies input/output variables).
    pub variable_scope: Option<VariableScopeClause>,
    /// Nested procedure specification (procedure body).
    pub specification: NestedProcedureSpecification,
    /// Source span.
    pub span: Span,
}

/// Variable scope clause for inline procedure calls.
///
/// Specifies the set of binding variables that are visible as input/output
/// variables for the procedure. An empty list `()` means no variables are
/// in scope.
///
/// # Examples
///
/// ```text
/// ()         -- No variables in scope
/// (x)        -- Single variable
/// (x, y, z)  -- Multiple variables
/// ```
///
/// # Grammar References
///
/// - `variableScopeClause` (Line 743)
/// - `bindingVariableReferenceList` (Line 747)
#[derive(Debug, Clone, PartialEq)]
pub struct VariableScopeClause {
    /// List of binding variable references (from Sprint 5).
    pub variables: Vec<BindingVariable>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Named Procedure Calls (Task 3)
// ============================================================================

/// A named procedure call with arguments and optional yield clause.
///
/// Named procedure calls reference an existing procedure by name, pass arguments,
/// and optionally yield specific result items.
///
/// # Examples
///
/// ```text
/// CALL my_procedure()
/// CALL my_procedure(arg1, arg2)
/// CALL my_procedure(arg1) YIELD result1, result2
/// CALL my_procedure(x, y) YIELD result AS alias
/// ```
///
/// # Grammar References
///
/// - `namedProcedureCall` (Line 753)
#[derive(Debug, Clone, PartialEq)]
pub struct NamedProcedureCall {
    /// Procedure reference (from Sprint 6).
    pub procedure: ProcedureReference,
    /// Optional procedure arguments.
    pub arguments: Option<ProcedureArgumentList>,
    /// Optional YIELD clause.
    pub yield_clause: Option<YieldClause>,
    /// Source span.
    pub span: Span,
}

/// Procedure argument list (comma-separated arguments).
///
/// # Examples
///
/// ```text
/// ()           -- Empty argument list
/// (arg1)       -- Single argument
/// (arg1, arg2) -- Multiple arguments
/// ```
///
/// # Grammar References
///
/// - `procedureArgumentList` (Line 757)
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureArgumentList {
    /// List of procedure arguments.
    pub arguments: Vec<ProcedureArgument>,
    /// Source span.
    pub span: Span,
}

/// A single procedure argument (expression).
///
/// # Grammar References
///
/// - `procedureArgument` (Line 761)
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureArgument {
    /// Argument expression (from Sprint 5).
    pub expression: Expression,
    /// Source span.
    pub span: Span,
}

/// YIELD clause for procedure calls.
///
/// Specifies which result items to yield from the procedure call, with optional
/// aliases.
///
/// # Examples
///
/// ```text
/// YIELD result1
/// YIELD result1, result2
/// YIELD result1 AS alias1, result2 AS alias2
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct YieldClause {
    /// List of yield items.
    pub items: YieldItemList,
    /// Source span.
    pub span: Span,
}

/// Yield item list (comma-separated yield items).
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItemList {
    /// List of yield items.
    pub items: Vec<YieldItem>,
    /// Source span.
    pub span: Span,
}

/// A single yield item (expression with optional alias).
///
/// # Examples
///
/// ```text
/// result1
/// result1 AS alias1
/// expr + 10 AS computed_value
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItem {
    /// Expression to yield (from Sprint 5).
    pub expression: Expression,
    /// Optional alias for the yielded item.
    pub alias: Option<YieldItemAlias>,
    /// Source span.
    pub span: Span,
}

/// Yield item alias (AS name).
#[derive(Debug, Clone, PartialEq)]
pub struct YieldItemAlias {
    /// Alias name.
    pub name: SmolStr,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Nested Procedure Specifications (Task 4)
// ============================================================================

/// A nested procedure specification (procedure body in braces).
///
/// Nested procedure specifications define inline procedure bodies with optional
/// AT schema clause, variable definitions, and statement blocks.
///
/// # Examples
///
/// ```text
/// {
///   MATCH (n) RETURN n
/// }
/// {
///   AT my_schema
///   GRAPH g = CURRENT_GRAPH
///   MATCH (n) RETURN n
/// }
/// ```
///
/// # Grammar References
///
/// - `nestedProcedureSpecification` (Line 138)
#[derive(Debug, Clone, PartialEq)]
pub struct NestedProcedureSpecification {
    /// Procedure body content.
    pub body: ProcedureBody,
    /// Source span.
    pub span: Span,
}

/// A nested data-modifying procedure specification.
///
/// Similar to nested procedure specification but specifically for data-modifying
/// operations.
///
/// # Grammar References
///
/// - `nestedDataModifyingProcedureSpecification` (Line 156)
#[derive(Debug, Clone, PartialEq)]
pub struct NestedDataModifyingProcedureSpecification {
    /// Data-modifying procedure body.
    pub body: ProcedureBody,
    /// Source span.
    pub span: Span,
}

/// A nested query specification.
///
/// Similar to nested procedure specification but specifically for query operations.
///
/// # Grammar References
///
/// - `nestedQuerySpecification` (Line 164)
#[derive(Debug, Clone, PartialEq)]
pub struct NestedQuerySpecification {
    /// Query procedure body.
    pub body: ProcedureBody,
    /// Source span.
    pub span: Span,
}

/// Procedure body with optional AT schema clause, variable definitions, and statements.
///
/// The procedure body is the core of procedure execution, containing:
/// - Optional AT schema clause to set schema context
/// - Optional variable definition block for local variables
/// - Statement block with sequential statements and NEXT chaining
///
/// # Examples
///
/// ```text
/// {
///   AT my_schema
///   GRAPH g :: GRAPH REFERENCE = CURRENT_GRAPH
///   VALUE counter :: INT = 0
///   MATCH (n) RETURN n
///   NEXT
///   MATCH (m) RETURN m
/// }
/// ```
///
/// # Grammar References
///
/// - `procedureBody` (Line 174)
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureBody {
    /// Optional AT schema clause (sets schema context).
    pub at_schema: Option<AtSchemaClause>,
    /// Optional variable definition block.
    pub variable_definitions: Option<BindingVariableDefinitionBlock>,
    /// Statement block (sequential statements).
    pub statements: StatementBlock,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Variable Definition Blocks (Task 5)
// ============================================================================

/// Binding variable definition block.
///
/// Contains one or more variable definitions (graph, binding table, or value variables).
///
/// # Examples
///
/// ```text
/// GRAPH g = CURRENT_GRAPH
/// BINDING TABLE t = some_table
/// VALUE counter = 0
/// ```
///
/// # Grammar References
///
/// - `bindingVariableDefinitionBlock` (Line 178)
#[derive(Debug, Clone, PartialEq)]
pub struct BindingVariableDefinitionBlock {
    /// List of variable definitions.
    pub definitions: Vec<BindingVariableDefinition>,
    /// Source span.
    pub span: Span,
}

/// Binding variable definition types.
///
/// Variables can be graph variables, binding table variables, or value variables.
///
/// # Grammar References
///
/// - `bindingVariableDefinition` (Line 182)
#[derive(Debug, Clone, PartialEq)]
pub enum BindingVariableDefinition {
    /// Graph variable definition.
    Graph(GraphVariableDefinition),
    /// Binding table variable definition.
    BindingTable(BindingTableVariableDefinition),
    /// Value variable definition.
    Value(ValueVariableDefinition),
}

/// Graph variable definition.
///
/// Defines a graph variable with optional PROPERTY keyword, type annotation, and
/// initializer.
///
/// # Examples
///
/// ```text
/// GRAPH g
/// GRAPH g :: GRAPH REFERENCE
/// GRAPH g = CURRENT_GRAPH
/// PROPERTY GRAPH g = some_graph
/// GRAPH g :: GRAPH REFERENCE = CURRENT_GRAPH
/// ```
///
/// # Grammar References
///
/// - `graphVariableDefinition` (Line 204)
/// - `optTypedGraphInitializer` (Line 208)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphVariableDefinition {
    /// Whether PROPERTY keyword is present.
    pub is_property: bool,
    /// Variable name (from Sprint 5).
    pub variable: BindingVariable,
    /// Optional type annotation (from Sprint 6).
    pub type_annotation: Option<GraphReferenceValueType>,
    /// Optional initializer.
    pub initializer: Option<GraphInitializer>,
    /// Source span.
    pub span: Span,
}

/// Graph initializer (= graph_expression).
///
/// # Grammar References
///
/// - `graphInitializer` (Line 212)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphInitializer {
    /// Graph expression (from Sprint 5).
    pub expression: GraphExpression,
    /// Source span.
    pub span: Span,
}

/// Graph expression placeholder.
///
/// For Sprint 11, we use a simplified representation. This will be expanded in
/// future sprints if needed.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphExpression {
    /// Variable reference to a graph.
    VariableReference(SmolStr, Span),
    /// CURRENT_GRAPH reference.
    CurrentGraph(Span),
    /// Expression placeholder for complex graph expressions.
    Expression(Box<Expression>),
}

/// Binding table variable definition.
///
/// Defines a binding table variable with optional BINDING keyword, type annotation,
/// and initializer.
///
/// # Examples
///
/// ```text
/// TABLE t
/// TABLE t :: BINDING TABLE
/// TABLE t = some_table
/// BINDING TABLE t = results
/// TABLE t :: BINDING TABLE = source_table
/// ```
///
/// # Grammar References
///
/// - `bindingTableVariableDefinition` (Line 218)
#[derive(Debug, Clone, PartialEq)]
pub struct BindingTableVariableDefinition {
    /// Whether BINDING keyword is present.
    pub is_binding: bool,
    /// Variable name (from Sprint 5).
    pub variable: BindingVariable,
    /// Optional type annotation (from Sprint 6).
    pub type_annotation: Option<BindingTableReferenceValueType>,
    /// Optional initializer.
    pub initializer: Option<BindingTableInitializer>,
    /// Source span.
    pub span: Span,
}

/// Binding table initializer (= binding_table_expression).
///
/// # Grammar References
///
/// - Similar to graph initializer
#[derive(Debug, Clone, PartialEq)]
pub struct BindingTableInitializer {
    /// Binding table expression (from Sprint 5).
    pub expression: BindingTableExpression,
    /// Source span.
    pub span: Span,
}

/// Binding table expression placeholder.
///
/// For Sprint 11, we use a simplified representation. This will be expanded in
/// future sprints if needed.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingTableExpression {
    /// Variable reference to a binding table.
    VariableReference(SmolStr, Span),
    /// Expression placeholder for complex binding table expressions.
    Expression(Box<Expression>),
}

/// Value variable definition.
///
/// Defines a value variable with optional type annotation and initializer.
///
/// # Examples
///
/// ```text
/// VALUE counter
/// VALUE counter :: INT
/// VALUE counter = 0
/// VALUE name :: STRING = 'default'
/// VALUE counter :: INT = 10
/// ```
///
/// # Grammar References
///
/// - `valueVariableDefinition` (Line 232)
#[derive(Debug, Clone, PartialEq)]
pub struct ValueVariableDefinition {
    /// Variable name (from Sprint 5).
    pub variable: BindingVariable,
    /// Optional type annotation (from Sprint 6).
    pub type_annotation: Option<ValueType>,
    /// Optional initializer.
    pub initializer: Option<ValueInitializer>,
    /// Source span.
    pub span: Span,
}

/// Value initializer (= value_expression).
#[derive(Debug, Clone, PartialEq)]
pub struct ValueInitializer {
    /// Value expression (from Sprint 5).
    pub expression: Expression,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Statement Blocks and NEXT Chaining (Task 6)
// ============================================================================

/// Statement block with sequential statements and NEXT chaining.
///
/// Statement blocks contain one or more statements that execute sequentially.
/// Statements can be chained using NEXT for explicit sequential composition.
///
/// # Examples
///
/// ```text
/// MATCH (n) RETURN n
///
/// MATCH (n) RETURN n
/// NEXT
/// MATCH (m) RETURN m
///
/// MATCH (n) RETURN n
/// NEXT YIELD result
/// MATCH (m) RETURN m
/// ```
///
/// # Grammar References
///
/// - `statementBlock` (Line 188)
#[derive(Debug, Clone, PartialEq)]
pub struct StatementBlock {
    /// Sequential statements.
    pub statements: Vec<Statement>,
    /// NEXT statement chains.
    pub next_statements: Vec<NextStatement>,
    /// Source span.
    pub span: Span,
}

/// Statement types that can appear in procedure bodies.
///
/// Statements can be composite queries, catalog-modifying operations, or
/// data-modifying operations.
///
/// # Grammar References
///
/// - `statement` (Line 192)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Composite query statement (from Sprint 7).
    CompositeQuery(Box<crate::ast::query::Query>),
    /// Linear catalog-modifying statement (from Sprint 4).
    LinearCatalogModifying(Box<LinearCatalogModifyingStatement>),
    /// Linear data-modifying statement (from Sprint 10).
    LinearDataModifying(Box<LinearDataModifyingStatement>),
}

impl Statement {
    /// Returns the source span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            Statement::CompositeQuery(query) => query.span(),
            Statement::LinearCatalogModifying(stmt) => stmt.span(),
            Statement::LinearDataModifying(stmt) => stmt.span(),
        }
    }
}

/// Linear catalog-modifying statement type.
pub type LinearCatalogModifyingStatement = crate::ast::catalog::CatalogStatementKind;

/// Linear data-modifying statement type.
pub type LinearDataModifyingStatement = crate::ast::mutation::LinearDataModifyingStatement;

/// NEXT statement for sequential statement chaining.
///
/// NEXT chains statements together in sequence, optionally yielding intermediate
/// results.
///
/// # Examples
///
/// ```text
/// NEXT MATCH (n) RETURN n
/// NEXT YIELD result MATCH (m) RETURN m
/// ```
///
/// # Grammar References
///
/// - `nextStatement` (Line 198)
#[derive(Debug, Clone, PartialEq)]
pub struct NextStatement {
    /// Optional YIELD clause for intermediate results.
    pub yield_clause: Option<YieldClause>,
    /// Next statement to execute.
    pub statement: Box<Statement>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// AT Schema and USE Graph Clauses (Task 7)
// ============================================================================

/// AT schema clause for setting schema context.
///
/// The AT clause sets the schema context for the procedure body, allowing
/// procedure execution in a specific schema.
///
/// # Examples
///
/// ```text
/// AT my_schema
/// AT /absolute/path/schema
/// AT CURRENT_SCHEMA
/// ```
///
/// # Grammar References
///
/// - `atSchemaClause` (Line 767)
#[derive(Debug, Clone, PartialEq)]
pub struct AtSchemaClause {
    /// Schema reference (from Sprint 6).
    pub schema: SchemaReference,
    /// Source span.
    pub span: Span,
}

// Note: UseGraphClause is already defined in Sprint 7 (src/ast/query.rs).
// We'll reuse that definition for procedure contexts.
