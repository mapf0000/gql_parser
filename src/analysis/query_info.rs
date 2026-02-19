//! Query-level compiler metadata extraction.

use std::collections::BTreeSet;

use smol_str::SmolStr;

use crate::analysis::expression_info::{ExpressionInfo, PropertyReference};
use crate::analysis::pattern_info::collect_pattern_expression_roots;
use crate::ast::procedure::{CallProcedureStatement, ProcedureCall};
use crate::ast::query::{
    GroupingElement, LinearQuery, MatchStatement, PrimitiveQueryStatement,
    PrimitiveResultStatement, Query, SelectFromClause, SelectItemList, SetOperator,
};
use crate::ast::visitor::AstVisitor;
use crate::ast::{Expression, Span, Statement, VariableCollector};

/// Stable clause identifier in a linear query pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClauseId {
    /// Monotonic pipeline id assigned while traversing query structure.
    pub pipeline_id: usize,
    /// Clause position within the linear pipeline.
    pub position: usize,
}

/// Kind of query clause encountered in a linear pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClauseKind {
    UseGraph,
    Match { optional: bool },
    Call { optional: bool, inline: bool },
    Filter,
    Let,
    For,
    OrderByAndPage,
    Select,
    Return,
    Finish,
}

/// Per-clause metadata used by downstream planning/lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseInfo {
    /// Stable clause id.
    pub clause_id: ClauseId,
    /// Clause type.
    pub kind: ClauseKind,
    /// Clause source span.
    pub span: Span,
    /// Variables defined by the clause.
    pub definitions: BTreeSet<SmolStr>,
    /// Variables used by the clause.
    pub uses: BTreeSet<SmolStr>,
    /// Property references discovered in clause expressions.
    pub property_references: Vec<PropertyReference>,
    /// Whether the clause contains aggregate expressions.
    pub contains_aggregate: bool,
    /// Number of graph patterns structurally owned by this clause.
    pub graph_pattern_count: usize,
}

/// High-level shape of the parsed query statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryShape {
    /// A single linear pipeline.
    Linear { pipeline_id: usize },
    /// Composite query combining two query shapes with a set operator.
    Composite {
        operator: SetOperator,
        left: Box<QueryShape>,
        right: Box<QueryShape>,
    },
}

/// Deterministic planning metadata extracted from a query statement AST.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryInfo {
    /// Flattened clause sequence in deterministic traversal order.
    pub clause_sequence: Vec<ClauseInfo>,
    /// Query shape including composite set operators and pipeline identity.
    pub query_shape: Option<QueryShape>,
    /// Total graph-pattern count across the statement.
    pub graph_pattern_count: usize,
    /// Whether any clause contains aggregation.
    pub contains_aggregation: bool,
}

impl QueryInfo {
    /// Builds query metadata from a top-level statement.
    pub fn from_ast(statement: &Statement) -> Self {
        let mut analyzer = QueryInfoAnalyzer::default();
        let query_shape = analyzer.analyze_statement(statement);

        Self {
            clause_sequence: analyzer.clause_sequence,
            query_shape,
            graph_pattern_count: analyzer.graph_pattern_count,
            contains_aggregation: analyzer.contains_aggregation,
        }
    }
}

#[derive(Debug, Default)]
struct QueryInfoAnalyzer {
    next_pipeline_id: usize,
    clause_sequence: Vec<ClauseInfo>,
    graph_pattern_count: usize,
    contains_aggregation: bool,
}

impl QueryInfoAnalyzer {
    fn analyze_statement(&mut self, statement: &Statement) -> Option<QueryShape> {
        match statement {
            Statement::Query(query_statement) => Some(self.analyze_query(&query_statement.query)),
            _ => None,
        }
    }

    fn analyze_query(&mut self, query: &Query) -> QueryShape {
        match query {
            Query::Linear(linear) => {
                let pipeline_id = self.next_pipeline_id;
                self.next_pipeline_id += 1;
                self.analyze_linear_query(linear, pipeline_id);
                QueryShape::Linear { pipeline_id }
            }
            Query::Composite(composite) => {
                let left = self.analyze_query(&composite.left);
                let right = self.analyze_query(&composite.right);

                QueryShape::Composite {
                    operator: composite.operator.clone(),
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
            Query::Parenthesized(inner, _) => self.analyze_query(inner),
        }
    }

    fn analyze_linear_query(&mut self, query: &LinearQuery, pipeline_id: usize) {
        let mut position = 0;

        match query {
            LinearQuery::Focused(focused) => {
                let metadata = ExpressionMetadata::from_expressions([&focused.use_graph.graph]);
                self.push_clause(
                    pipeline_id,
                    &mut position,
                    ClauseKind::UseGraph,
                    focused.use_graph.span.clone(),
                    BTreeSet::new(),
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );

                for primitive in &focused.primitive_statements {
                    self.analyze_primitive_clause(primitive, pipeline_id, &mut position);
                }

                if let Some(result) = &focused.result_statement {
                    self.analyze_result_clause(result, pipeline_id, &mut position);
                }
            }
            LinearQuery::Ambient(ambient) => {
                for primitive in &ambient.primitive_statements {
                    self.analyze_primitive_clause(primitive, pipeline_id, &mut position);
                }

                if let Some(result) = &ambient.result_statement {
                    self.analyze_result_clause(result, pipeline_id, &mut position);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_clause(
        &mut self,
        pipeline_id: usize,
        position: &mut usize,
        kind: ClauseKind,
        span: Span,
        definitions: BTreeSet<SmolStr>,
        uses: BTreeSet<SmolStr>,
        property_references: Vec<PropertyReference>,
        contains_aggregate: bool,
        graph_pattern_count: usize,
    ) {
        self.graph_pattern_count += graph_pattern_count;
        self.contains_aggregation |= contains_aggregate;

        self.clause_sequence.push(ClauseInfo {
            clause_id: ClauseId {
                pipeline_id,
                position: *position,
            },
            kind,
            span,
            definitions,
            uses,
            property_references,
            contains_aggregate,
            graph_pattern_count,
        });

        *position += 1;
    }

    fn analyze_primitive_clause(
        &mut self,
        statement: &PrimitiveQueryStatement,
        pipeline_id: usize,
        position: &mut usize,
    ) {
        match statement {
            PrimitiveQueryStatement::Match(match_statement) => {
                let (definitions, mut uses) = variable_sets_from_match(match_statement);
                let mut roots = Vec::new();
                collect_match_expression_roots(match_statement, &mut roots);
                let metadata = ExpressionMetadata::from_expressions(roots);
                uses.extend(metadata.uses);

                let optional = matches!(match_statement, MatchStatement::Optional(_));
                let graph_pattern_count = count_graph_patterns_in_match(match_statement);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Match { optional },
                    match_statement.span().clone(),
                    definitions,
                    uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    graph_pattern_count,
                );
            }
            PrimitiveQueryStatement::Call(call_statement) => {
                let (definitions, mut uses) = variable_sets_from_call(call_statement);
                let mut roots = Vec::new();
                collect_call_expression_roots(call_statement, &mut roots);
                let metadata = ExpressionMetadata::from_expressions(roots);
                uses.extend(metadata.uses);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Call {
                        optional: call_statement.optional,
                        inline: matches!(call_statement.call, ProcedureCall::Inline(_)),
                    },
                    call_statement.span.clone(),
                    definitions,
                    uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveQueryStatement::Filter(filter_statement) => {
                let metadata =
                    ExpressionMetadata::from_expressions([&filter_statement.condition]);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Filter,
                    filter_statement.span.clone(),
                    BTreeSet::new(),
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveQueryStatement::Let(let_statement) => {
                let definitions = let_statement
                    .bindings
                    .iter()
                    .map(|binding| binding.variable.name.clone())
                    .collect::<BTreeSet<_>>();

                let metadata = ExpressionMetadata::from_expressions(
                    let_statement.bindings.iter().map(|binding| &binding.value),
                );

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Let,
                    let_statement.span.clone(),
                    definitions,
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveQueryStatement::For(for_statement) => {
                let mut definitions = BTreeSet::new();
                definitions.insert(for_statement.item.binding_variable.name.clone());

                if let Some(for_suffix) = &for_statement.ordinality_or_offset {
                    match for_suffix {
                        crate::ast::ForOrdinalityOrOffset::Ordinality { variable }
                        | crate::ast::ForOrdinalityOrOffset::Offset { variable } => {
                            definitions.insert(variable.name.clone());
                        }
                    }
                }

                let metadata = ExpressionMetadata::from_expressions([&for_statement.item.collection]);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::For,
                    for_statement.span.clone(),
                    definitions,
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveQueryStatement::OrderByAndPage(order_statement) => {
                let mut expressions = Vec::new();

                if let Some(order_by) = &order_statement.order_by {
                    for sort in &order_by.sort_specifications {
                        expressions.push(&sort.key);
                    }
                }

                if let Some(offset) = &order_statement.offset {
                    expressions.push(&offset.count);
                }

                if let Some(limit) = &order_statement.limit {
                    expressions.push(&limit.count);
                }

                let metadata = ExpressionMetadata::from_expressions(expressions);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::OrderByAndPage,
                    order_statement.span.clone(),
                    BTreeSet::new(),
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveQueryStatement::Select(select_statement) => {
                let mut collector = VariableCollector::new();
                let _ = collector.visit_select_statement(select_statement);
                let mut definitions = collector.definitions().clone();
                let mut uses = collector.references().clone();

                if let SelectItemList::Items { items } = &select_statement.select_items {
                    for item in items {
                        if let Some(alias) = &item.alias {
                            definitions.insert(alias.clone());
                        }
                    }
                }

                let mut roots = Vec::new();
                let graph_pattern_count = collect_select_expression_roots(select_statement, &mut roots);
                let metadata = ExpressionMetadata::from_expressions(roots);
                uses.extend(metadata.uses);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Select,
                    select_statement.span.clone(),
                    definitions,
                    uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    graph_pattern_count,
                );
            }
        }
    }

    fn analyze_result_clause(
        &mut self,
        result_statement: &PrimitiveResultStatement,
        pipeline_id: usize,
        position: &mut usize,
    ) {
        match result_statement {
            PrimitiveResultStatement::Return(return_statement) => {
                let mut definitions = BTreeSet::new();

                if let crate::ast::ReturnItemList::Items { items } = &return_statement.items {
                    for item in items {
                        if let Some(alias) = &item.alias {
                            definitions.insert(alias.clone());
                        }
                    }
                }

                let mut roots = Vec::new();
                collect_return_expression_roots(return_statement, &mut roots);
                let metadata = ExpressionMetadata::from_expressions(roots);

                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Return,
                    return_statement.span.clone(),
                    definitions,
                    metadata.uses,
                    metadata.property_references,
                    metadata.contains_aggregate,
                    0,
                );
            }
            PrimitiveResultStatement::Finish(span) => {
                self.push_clause(
                    pipeline_id,
                    position,
                    ClauseKind::Finish,
                    span.clone(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    Vec::new(),
                    false,
                    0,
                );
            }
        }
    }
}

#[derive(Debug, Default)]
struct ExpressionMetadata {
    uses: BTreeSet<SmolStr>,
    property_references: Vec<PropertyReference>,
    contains_aggregate: bool,
}

impl ExpressionMetadata {
    fn from_expressions<'a>(expressions: impl IntoIterator<Item = &'a Expression>) -> Self {
        let mut metadata = Self::default();

        for expression in expressions {
            let info = ExpressionInfo::analyze(expression);
            metadata.uses.extend(info.variable_references);
            metadata.property_references.extend(info.property_references);
            metadata.contains_aggregate |= info.contains_aggregate;
        }

        metadata
    }
}

fn variable_sets_from_match(statement: &MatchStatement) -> (BTreeSet<SmolStr>, BTreeSet<SmolStr>) {
    let mut collector = VariableCollector::new();
    let _ = collector.visit_match_statement(statement);
    (collector.definitions().clone(), collector.references().clone())
}

fn variable_sets_from_call(
    statement: &CallProcedureStatement,
) -> (BTreeSet<SmolStr>, BTreeSet<SmolStr>) {
    let mut definitions = BTreeSet::new();
    let mut uses = BTreeSet::new();

    match &statement.call {
        ProcedureCall::Inline(inline) => {
            if let Some(scope_clause) = &inline.variable_scope {
                for variable in &scope_clause.variables {
                    uses.insert(variable.name.clone());
                }
            }
        }
        ProcedureCall::Named(named) => {
            if let Some(yield_clause) = &named.yield_clause {
                for item in &yield_clause.items.items {
                    if let Some(alias) = &item.alias {
                        definitions.insert(alias.name.clone());
                    } else if let Expression::VariableReference(name, _) = &item.expression {
                        definitions.insert(name.clone());
                    }
                }
            }
        }
    }

    (definitions, uses)
}

fn collect_match_expression_roots<'a>(statement: &'a MatchStatement, roots: &mut Vec<&'a Expression>) {
    match statement {
        MatchStatement::Simple(simple) => {
            collect_pattern_expression_roots(&simple.pattern, roots);
        }
        MatchStatement::Optional(optional) => match &optional.operand {
            crate::ast::OptionalOperand::Match { pattern } => {
                collect_pattern_expression_roots(pattern, roots);
            }
            crate::ast::OptionalOperand::Block { statements }
            | crate::ast::OptionalOperand::ParenthesizedBlock { statements } => {
                for statement in statements {
                    collect_match_expression_roots(statement, roots);
                }
            }
        },
    }
}

fn collect_call_expression_roots<'a>(statement: &'a CallProcedureStatement, roots: &mut Vec<&'a Expression>) {
    match &statement.call {
        ProcedureCall::Inline(_) => {}
        ProcedureCall::Named(named) => {
            if let Some(arguments) = &named.arguments {
                for argument in &arguments.arguments {
                    roots.push(&argument.expression);
                }
            }

            if let Some(yield_clause) = &named.yield_clause {
                for item in &yield_clause.items.items {
                    roots.push(&item.expression);
                }
            }
        }
    }
}

fn collect_select_expression_roots<'a>(
    statement: &'a crate::ast::SelectStatement,
    roots: &mut Vec<&'a Expression>,
) -> usize {
    let mut graph_pattern_count = 0;

    if let SelectItemList::Items { items } = &statement.select_items {
        for item in items {
            roots.push(&item.expression);
        }
    }

    if let Some(from_clause) = &statement.from_clause {
        match from_clause {
            SelectFromClause::GraphMatchList { matches } => {
                for graph_pattern in matches {
                    graph_pattern_count += 1;
                    collect_pattern_expression_roots(graph_pattern, roots);
                }
            }
            SelectFromClause::QuerySpecification { .. } => {}
            SelectFromClause::GraphAndQuerySpecification { graph, .. } => {
                roots.push(graph);
            }
        }
    }

    if let Some(where_clause) = &statement.where_clause {
        roots.push(&where_clause.condition);
    }

    if let Some(group_by) = &statement.group_by {
        for element in &group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                roots.push(expression);
            }
        }
    }

    if let Some(having_clause) = &statement.having {
        roots.push(&having_clause.condition);
    }

    if let Some(order_by) = &statement.order_by {
        for sort in &order_by.sort_specifications {
            roots.push(&sort.key);
        }
    }

    if let Some(offset_clause) = &statement.offset {
        roots.push(&offset_clause.count);
    }

    if let Some(limit_clause) = &statement.limit {
        roots.push(&limit_clause.count);
    }

    graph_pattern_count
}

fn collect_return_expression_roots<'a>(
    statement: &'a crate::ast::ReturnStatement,
    roots: &mut Vec<&'a Expression>,
) {
    if let crate::ast::ReturnItemList::Items { items } = &statement.items {
        for item in items {
            roots.push(&item.expression);
        }
    }

    if let Some(group_by) = &statement.group_by {
        for element in &group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                roots.push(expression);
            }
        }
    }
}

fn count_graph_patterns_in_match(statement: &MatchStatement) -> usize {
    match statement {
        MatchStatement::Simple(_) => 1,
        MatchStatement::Optional(optional) => match &optional.operand {
            crate::ast::OptionalOperand::Match { .. } => 1,
            crate::ast::OptionalOperand::Block { statements }
            | crate::ast::OptionalOperand::ParenthesizedBlock { statements } => {
                statements.iter().map(count_graph_patterns_in_match).sum()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::query_info::{ClauseKind, QueryInfo, QueryShape};
    use crate::ast::Statement;
    use crate::parse;

    fn first_statement(source: &str) -> Statement {
        let parse_result = parse(source);
        let program = parse_result.ast.expect("expected AST");
        program.statements[0].clone()
    }

    #[test]
    fn query_info_extracts_clause_sequence_and_define_use_sets() {
        let statement = first_statement(
            "USE myGraph MATCH (n:Person)-[r:KNOWS]->(m) WHERE n.age > 30 LET total = n.score + m.score RETURN m.name AS friend_name, COUNT(r) AS rels",
        );

        let info = QueryInfo::from_ast(&statement);

        assert_eq!(info.clause_sequence.len(), 4);
        assert_eq!(info.graph_pattern_count, 1);
        assert!(info.contains_aggregation);

        assert!(matches!(info.clause_sequence[0].kind, ClauseKind::UseGraph));
        assert!(matches!(
            info.clause_sequence[1].kind,
            ClauseKind::Match { optional: false }
        ));
        assert!(matches!(info.clause_sequence[2].kind, ClauseKind::Let));
        assert!(matches!(info.clause_sequence[3].kind, ClauseKind::Return));

        let match_clause = &info.clause_sequence[1];
        assert!(match_clause.definitions.contains("n"));
        assert!(match_clause.definitions.contains("r"));
        assert!(match_clause.definitions.contains("m"));
        assert!(match_clause.uses.contains("n"));

        let let_clause = &info.clause_sequence[2];
        assert!(let_clause.definitions.contains("total"));
        assert!(let_clause.uses.contains("n"));
        assert!(let_clause.uses.contains("m"));

        let return_clause = &info.clause_sequence[3];
        assert!(return_clause.definitions.contains("friend_name"));
        assert!(return_clause.definitions.contains("rels"));
        assert!(return_clause.uses.contains("m"));
        assert!(return_clause.uses.contains("r"));
    }

    #[test]
    fn query_info_preserves_composite_pipeline_structure() {
        let statement = first_statement("MATCH (a) RETURN a UNION MATCH (b) RETURN b");

        let info = QueryInfo::from_ast(&statement);

        assert_eq!(info.clause_sequence.len(), 4);

        match info.query_shape {
            Some(QueryShape::Composite { left, right, .. }) => {
                assert!(matches!(*left, QueryShape::Linear { pipeline_id: 0 }));
                assert!(matches!(*right, QueryShape::Linear { pipeline_id: 1 }));
            }
            _ => panic!("expected composite query shape"),
        }

        assert_eq!(info.clause_sequence[0].clause_id.pipeline_id, 0);
        assert_eq!(info.clause_sequence[1].clause_id.pipeline_id, 0);
        assert_eq!(info.clause_sequence[2].clause_id.pipeline_id, 1);
        assert_eq!(info.clause_sequence[3].clause_id.pipeline_id, 1);
    }

    #[test]
    fn query_info_is_empty_for_non_query_statements() {
        let statement = first_statement("SESSION CLOSE");

        let info = QueryInfo::from_ast(&statement);

        assert!(info.clause_sequence.is_empty());
        assert!(info.query_shape.is_none());
        assert_eq!(info.graph_pattern_count, 0);
        assert!(!info.contains_aggregation);
    }
}
