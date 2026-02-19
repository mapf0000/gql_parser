//! AST foundation types and node structures.

mod catalog;
pub mod expression;
pub mod graph_type;
pub mod mutation;
pub mod procedure;
pub mod program;
pub mod query;
pub mod references;
pub mod visitor;
pub mod visitors;
mod session;
mod span;
mod transaction;
pub mod types;

// Re-export span types
pub use span::{Span, Spanned};

// Re-export program structure
pub use program::{
    CatalogStatement, MutationStatement, Program, QueryStatement, SessionStatement, Statement,
    TransactionStatement,
};

// Re-export session types
pub use session::{
    SessionCloseCommand, SessionCommand, SessionResetCommand, SessionResetTarget,
    SessionSetCommand, SessionSetGraphClause, SessionSetParameterClause, SessionSetSchemaClause,
    SessionSetTimeZoneClause,
};

// Re-export transaction types
pub use transaction::{
    CommitCommand, RollbackCommand, StartTransactionCommand, TransactionAccessMode,
    TransactionCharacteristics, TransactionCommand, TransactionMode,
};

// Re-export catalog types
pub use catalog::{
    CallCatalogModifyingProcedureStatement, CatalogStatementKind, CreateGraphStatement,
    CreateGraphTypeStatement, CreateSchemaStatement, DropGraphStatement, DropGraphTypeStatement,
    DropSchemaStatement, GraphTypeSource, GraphTypeSpec,
};

// Re-export expression types
pub use expression::{
    AggregateFunction, BinaryOperator, BinarySetFunction, BinarySetFunctionType, BooleanValue,
    CaseExpression, CastExpression, ComparisonOperator, ExistsExpression, ExistsVariant,
    Expression, FunctionCall, FunctionName, GeneralSetFunction, GeneralSetFunctionType,
    GraphPatternPlaceholder, LabelExpression, Literal, LogicalOperator, Predicate, RecordField,
    SearchedCaseExpression, SearchedWhenClause, SimpleCaseExpression, SimpleWhenClause,
    TrimSpecification, TruthValue, UnaryOperator,
};

// Re-export type system types
pub use types::{
    ApproximateNumericType, BindingTableReferenceValueType, BooleanType, ByteStringType,
    CharacterStringType, DecimalExactNumericType, DecimalKind, EdgeReferenceValueType,
    EdgeTypeSpecification, ExactNumericType, FieldType, FieldTypesSpecification,
    GraphReferenceValueType, ImmaterialValueType, ListSyntaxForm, ListValueType,
    NestedGraphTypeSpecification, NodeReferenceValueType, NodeTypeSpecification, NotNullConstraint,
    NumericType, PathValueType, PredefinedType, RecordType, ReferenceValueType,
    SignedBinaryExactNumericType, TemporalDurationType, TemporalInstantType, TemporalType,
    TypeAnnotation, TypeAnnotationOperator, UnsignedBinaryExactNumericType, ValueType,
};

// Re-export reference types
pub use references::{
    BindingTableReference, BindingVariable, CatalogObjectParentReference, CatalogQualifiedName,
    GraphReference, GraphTypeReference, ProcedureReference, SchemaReference,
};

// Re-export query types
pub use query::{
    AmbientLinearQuery, CompositeQuery, FilterStatement, FocusedLinearQuery, ForItem,
    ForOrdinalityOrOffset, ForStatement, GraphPattern, GroupByClause, GroupingElement,
    HavingClause, LetStatement, LetVariableDefinition, LimitClause, LinearQuery, MatchStatement,
    NullOrdering, OffsetClause, OptionalMatchStatement, OptionalOperand, OrderByAndPageStatement,
    OrderByClause, OrderingSpecification, PrimitiveQueryStatement, PrimitiveResultStatement, Query,
    ReturnItem, ReturnItemList, ReturnStatement, SelectFromClause, SelectItem, SelectItemList,
    SelectStatement, SetOperator, SetQuantifier, SimpleMatchStatement, SortSpecification,
    UseGraphClause, WhereClause,
};

// Re-export visitor infrastructure and concrete visitors.
pub use visitor::{AstVisitor, AstVisitorMut, VisitResult};
pub use visitors::{AstNode, CollectingVisitor, SpanCollector, VariableCollector};

// Re-export mutation types
pub use mutation::{
    AmbientLinearDataModifyingStatement, CallDataModifyingProcedureStatement, DeleteItem,
    DeleteItemList, DeleteStatement, DetachOption, FocusedLinearDataModifyingStatement,
    InsertEdgePattern, InsertEdgePointingLeft, InsertEdgePointingRight, InsertEdgeUndirected,
    InsertElementPattern, InsertElementPatternFiller, InsertGraphPattern, InsertNodePattern,
    InsertPathPattern, InsertStatement, LinearDataModifyingStatement,
    PrimitiveDataModifyingStatement, RemoveItem, RemoveItemList, RemoveLabelItem,
    RemovePropertyItem, RemoveStatement, SetAllPropertiesItem, SetItem, SetItemList, SetLabelItem,
    SetPropertyItem, SetStatement, SimpleDataAccessingStatement, SimpleDataModifyingStatement,
};

// Re-export procedure types
pub use procedure::{
    AtSchemaClause, BindingTableExpression, BindingTableInitializer,
    BindingTableVariableDefinition, BindingVariableDefinition, BindingVariableDefinitionBlock,
    CallProcedureStatement, GraphExpression, GraphInitializer, GraphVariableDefinition,
    InlineProcedureCall, LinearCatalogModifyingStatement, NamedProcedureCall,
    NestedDataModifyingProcedureSpecification, NestedProcedureSpecification,
    NestedQuerySpecification, NextStatement, ProcedureArgument, ProcedureArgumentList,
    ProcedureBody, ProcedureCall, Statement as ProcedureStatement, StatementBlock,
    ValueInitializer, ValueVariableDefinition, VariableScopeClause, YieldClause, YieldItem,
    YieldItemAlias, YieldItemList,
};

// Re-export graph type specification types
pub use graph_type::{
    ArcTypePointingLeft, ArcTypePointingRight, ArcTypeUndirected, DirectedArcType, EdgeKind,
    EdgeTypeFiller, EdgeTypeLabelSet, EdgeTypePattern, EdgeTypePatternDirected,
    EdgeTypePatternUndirected, EdgeTypePhrase, EdgeTypePhraseContent, EdgeTypePropertyTypes,
    EdgeTypeSpecification as EdgeTypeSpec, ElementTypeList, ElementTypeSpecification, EndpointPair,
    EndpointPairPhrase, GraphTypeSpecificationBody, LabelName, LabelSetPhrase,
    LabelSetSpecification, LocalNodeTypeAlias, NestedGraphTypeSpecification as NestedGraphTypeSpec,
    NodeTypeFiller, NodeTypeImpliedContent, NodeTypeKeyLabelSet, NodeTypeLabelSet, NodeTypePattern,
    NodeTypePhrase, NodeTypePropertyTypes, NodeTypeReference,
    NodeTypeSpecification as NodeTypeSpec, PropertyName, PropertyType, PropertyTypeList,
    PropertyTypesSpecification, PropertyValueType,
};
