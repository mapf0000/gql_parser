//! Mutation statement AST nodes for GQL data modification.
//!
//! This module models ISO GQL chapter 13 data-modifying statements.

use crate::ast::expression::Expression;
use crate::ast::query::{
    ElementPropertySpecification, ElementVariableDeclaration, LabelSetSpecification,
    PrimitiveQueryStatement, PrimitiveResultStatement, UseGraphClause,
};
use crate::ast::references::ProcedureReference;
use crate::ast::Span;
use smol_str::SmolStr;

// ============================================================================
// Linear Data Modifying Statements
// ============================================================================

/// A linear data modifying statement.
#[derive(Debug, Clone, PartialEq)]
pub enum LinearDataModifyingStatement {
    /// Focused statement with explicit `USE` clause.
    Focused(FocusedLinearDataModifyingStatement),
    /// Ambient statement using session graph context.
    Ambient(AmbientLinearDataModifyingStatement),
}

impl LinearDataModifyingStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            LinearDataModifyingStatement::Focused(stmt) => &stmt.span,
            LinearDataModifyingStatement::Ambient(stmt) => &stmt.span,
        }
    }
}

/// Focused linear data modifying statement body.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusedLinearDataModifyingStatement {
    pub use_graph_clause: UseGraphClause,
    pub statements: Vec<SimpleDataAccessingStatement>,
    pub primitive_result_statement: Option<PrimitiveResultStatement>,
    pub span: Span,
}

/// Ambient linear data modifying statement body.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientLinearDataModifyingStatement {
    pub statements: Vec<SimpleDataAccessingStatement>,
    pub primitive_result_statement: Option<PrimitiveResultStatement>,
    pub span: Span,
}

/// A simple data-accessing statement inside linear mutation flow.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleDataAccessingStatement {
    /// A simple query statement (MATCH, FILTER, LET, FOR, ORDER/LIMIT/OFFSET, SELECT).
    Query(PrimitiveQueryStatement),
    /// A simple data modifying statement (INSERT/SET/REMOVE/DELETE/CALL).
    Modifying(SimpleDataModifyingStatement),
}

impl SimpleDataAccessingStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            SimpleDataAccessingStatement::Query(stmt) => stmt.span(),
            SimpleDataAccessingStatement::Modifying(stmt) => stmt.span(),
        }
    }
}

/// A simple data modifying statement.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleDataModifyingStatement {
    /// Primitive mutation statement.
    Primitive(PrimitiveDataModifyingStatement),
    /// Procedure call that may modify data.
    Call(CallDataModifyingProcedureStatement),
}

impl SimpleDataModifyingStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            SimpleDataModifyingStatement::Primitive(stmt) => stmt.span(),
            SimpleDataModifyingStatement::Call(stmt) => &stmt.span,
        }
    }
}

// ============================================================================
// Primitive Data Modifying Statements
// ============================================================================

/// Primitive data-modifying statement.
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveDataModifyingStatement {
    Insert(InsertStatement),
    Set(SetStatement),
    Remove(RemoveStatement),
    Delete(DeleteStatement),
}

impl PrimitiveDataModifyingStatement {
    /// Returns the span of this statement.
    pub fn span(&self) -> &Span {
        match self {
            PrimitiveDataModifyingStatement::Insert(stmt) => &stmt.span,
            PrimitiveDataModifyingStatement::Set(stmt) => &stmt.span,
            PrimitiveDataModifyingStatement::Remove(stmt) => &stmt.span,
            PrimitiveDataModifyingStatement::Delete(stmt) => &stmt.span,
        }
    }
}

// ============================================================================
// INSERT
// ============================================================================

/// `INSERT` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub pattern: InsertGraphPattern,
    pub span: Span,
}

/// Insert graph pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertGraphPattern {
    pub paths: Vec<InsertPathPattern>,
    pub span: Span,
}

/// Insert path pattern: `node (edge node)*`.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertPathPattern {
    pub elements: Vec<InsertElementPattern>,
    pub span: Span,
}

/// Insert element pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertElementPattern {
    Node(InsertNodePattern),
    Edge(InsertEdgePattern),
}

impl InsertElementPattern {
    /// Returns the span of this pattern.
    pub fn span(&self) -> &Span {
        match self {
            InsertElementPattern::Node(node) => &node.span,
            InsertElementPattern::Edge(edge) => edge.span(),
        }
    }
}

/// Insert node pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertNodePattern {
    /// Optional filler between parentheses.
    pub filler: Option<InsertElementPatternFiller>,
    pub span: Span,
}

/// Insert edge pattern direction variants.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertEdgePattern {
    PointingLeft(InsertEdgePointingLeft),
    PointingRight(InsertEdgePointingRight),
    Undirected(InsertEdgeUndirected),
}

impl InsertEdgePattern {
    /// Returns the span of this pattern.
    pub fn span(&self) -> &Span {
        match self {
            InsertEdgePattern::PointingLeft(edge) => &edge.span,
            InsertEdgePattern::PointingRight(edge) => &edge.span,
            InsertEdgePattern::Undirected(edge) => &edge.span,
        }
    }
}

/// `<-[ ... ]-`
#[derive(Debug, Clone, PartialEq)]
pub struct InsertEdgePointingLeft {
    /// Optional filler between brackets.
    pub filler: Option<InsertElementPatternFiller>,
    pub span: Span,
}

/// `-[ ... ]->`
#[derive(Debug, Clone, PartialEq)]
pub struct InsertEdgePointingRight {
    /// Optional filler between brackets.
    pub filler: Option<InsertElementPatternFiller>,
    pub span: Span,
}

/// `~[ ... ]~`
#[derive(Debug, Clone, PartialEq)]
pub struct InsertEdgeUndirected {
    /// Optional filler between brackets.
    pub filler: Option<InsertElementPatternFiller>,
    pub span: Span,
}

/// Insert element filler inside `()` or `[]`.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertElementPatternFiller {
    pub variable: Option<ElementVariableDeclaration>,
    pub label_set: Option<LabelSetSpecification>,
    /// True when labels were introduced via `IS`, false when `:`.
    pub use_is_keyword: bool,
    pub properties: Option<ElementPropertySpecification>,
    pub span: Span,
}

// ============================================================================
// SET
// ============================================================================

/// `SET` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct SetStatement {
    pub items: SetItemList,
    pub span: Span,
}

/// Comma-separated SET item list.
#[derive(Debug, Clone, PartialEq)]
pub struct SetItemList {
    pub items: Vec<SetItem>,
    pub span: Span,
}

/// SET item variants.
#[derive(Debug, Clone, PartialEq)]
pub enum SetItem {
    Property(SetPropertyItem),
    AllProperties(SetAllPropertiesItem),
    Label(SetLabelItem),
}

impl SetItem {
    /// Returns the span of this item.
    pub fn span(&self) -> &Span {
        match self {
            SetItem::Property(item) => &item.span,
            SetItem::AllProperties(item) => &item.span,
            SetItem::Label(item) => &item.span,
        }
    }
}

/// `bindingVariableReference . propertyName = valueExpression`
#[derive(Debug, Clone, PartialEq)]
pub struct SetPropertyItem {
    pub element: SmolStr,
    pub property: SmolStr,
    pub value: Expression,
    pub span: Span,
}

/// `bindingVariableReference = { propertyKeyValuePairList? }`
#[derive(Debug, Clone, PartialEq)]
pub struct SetAllPropertiesItem {
    pub element: SmolStr,
    pub properties: ElementPropertySpecification,
    pub span: Span,
}

/// `bindingVariableReference (IS|:) labelName`
#[derive(Debug, Clone, PartialEq)]
pub struct SetLabelItem {
    pub element: SmolStr,
    pub label: SmolStr,
    pub use_is_keyword: bool,
    pub span: Span,
}

// ============================================================================
// REMOVE
// ============================================================================

/// `REMOVE` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveStatement {
    pub items: RemoveItemList,
    pub span: Span,
}

/// Comma-separated REMOVE item list.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveItemList {
    pub items: Vec<RemoveItem>,
    pub span: Span,
}

/// REMOVE item variants.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoveItem {
    Property(RemovePropertyItem),
    Label(RemoveLabelItem),
}

impl RemoveItem {
    /// Returns the span of this item.
    pub fn span(&self) -> &Span {
        match self {
            RemoveItem::Property(item) => &item.span,
            RemoveItem::Label(item) => &item.span,
        }
    }
}

/// `bindingVariableReference . propertyName`
#[derive(Debug, Clone, PartialEq)]
pub struct RemovePropertyItem {
    pub element: SmolStr,
    pub property: SmolStr,
    pub span: Span,
}

/// `bindingVariableReference (IS|:) labelName`
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveLabelItem {
    pub element: SmolStr,
    pub label: SmolStr,
    pub use_is_keyword: bool,
    pub span: Span,
}

// ============================================================================
// DELETE
// ============================================================================

/// `DELETE` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub detach_option: DetachOption,
    pub items: DeleteItemList,
    pub span: Span,
}

/// Optional DETACH mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetachOption {
    Detach,
    NoDetach,
    #[default]
    Default,
}

/// Comma-separated DELETE item list.
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteItemList {
    pub items: Vec<DeleteItem>,
    pub span: Span,
}

/// `valueExpression`
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteItem {
    pub expression: Expression,
    pub span: Span,
}

// ============================================================================
// CALL Data-Modifying Procedure
// ============================================================================

/// `OPTIONAL? CALL` statement in data-modifying contexts.
#[derive(Debug, Clone, PartialEq)]
pub struct CallDataModifyingProcedureStatement {
    pub optional: bool,
    pub procedure: ProcedureReference,
    pub arguments: Vec<Expression>,
    pub yield_items: Option<Vec<SmolStr>>,
    pub span: Span,
}
