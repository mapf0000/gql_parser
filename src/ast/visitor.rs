//! Query-focused AST visitor infrastructure.
//!
//! These visitors intentionally focus on query syntax and expression trees,
//! which are the primary downstream integration surface for compilers.

use std::ops::ControlFlow;

use crate::ast::expression::{CaseExpression, ExistsVariant, Literal, Predicate};
use crate::ast::procedure::{CallProcedureStatement, ProcedureCall};
use crate::ast::program::{Program, QueryStatement, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, FilterStatement, ForStatement, GraphPattern, GroupingElement,
    LabelExpression, LetStatement, LetVariableDefinition, LinearQuery, MatchStatement,
    NodePattern, PathFactor, PathPattern, PathPatternExpression, PathPrimary,
    PrimitiveQueryStatement, PrimitiveResultStatement, Query, ReturnItem, ReturnItemList,
    ReturnStatement, SelectFromClause, SelectItemList, SelectStatement,
    SimplifiedPathPatternExpression,
};
use crate::ast::Expression;

macro_rules! try_visit {
    ($expr:expr) => {
        match $expr {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(b) => return ControlFlow::Break(b),
        }
    };
}

/// Shared type alias for visitor traversal methods.
pub type VisitResult<B> = ControlFlow<B>;

/// Immutable AST visitor for query and expression nodes.
pub trait AstVisitor {
    /// Early-exit payload produced when traversal stops.
    type Break;

    fn visit_program(&mut self, program: &Program) -> VisitResult<Self::Break> {
        walk_program(self, program)
    }

    fn visit_statement(&mut self, statement: &Statement) -> VisitResult<Self::Break> {
        walk_statement(self, statement)
    }

    fn visit_query_statement(&mut self, statement: &QueryStatement) -> VisitResult<Self::Break> {
        walk_query_statement(self, statement)
    }

    fn visit_query(&mut self, query: &Query) -> VisitResult<Self::Break> {
        walk_query(self, query)
    }

    fn visit_linear_query(&mut self, query: &LinearQuery) -> VisitResult<Self::Break> {
        walk_linear_query(self, query)
    }

    fn visit_primitive_query_statement(
        &mut self,
        statement: &PrimitiveQueryStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_query_statement(self, statement)
    }

    fn visit_primitive_result_statement(
        &mut self,
        statement: &PrimitiveResultStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_result_statement(self, statement)
    }

    fn visit_match_statement(&mut self, statement: &MatchStatement) -> VisitResult<Self::Break> {
        walk_match_statement(self, statement)
    }

    fn visit_graph_pattern(&mut self, pattern: &GraphPattern) -> VisitResult<Self::Break> {
        walk_graph_pattern(self, pattern)
    }

    fn visit_path_pattern(&mut self, pattern: &PathPattern) -> VisitResult<Self::Break> {
        walk_path_pattern(self, pattern)
    }

    fn visit_path_pattern_expression(
        &mut self,
        expression: &PathPatternExpression,
    ) -> VisitResult<Self::Break> {
        walk_path_pattern_expression(self, expression)
    }

    fn visit_path_factor(&mut self, factor: &PathFactor) -> VisitResult<Self::Break> {
        walk_path_factor(self, factor)
    }

    fn visit_path_primary(&mut self, primary: &PathPrimary) -> VisitResult<Self::Break> {
        walk_path_primary(self, primary)
    }

    fn visit_element_pattern(&mut self, pattern: &ElementPattern) -> VisitResult<Self::Break> {
        walk_element_pattern(self, pattern)
    }

    fn visit_node_pattern(&mut self, pattern: &NodePattern) -> VisitResult<Self::Break> {
        walk_node_pattern(self, pattern)
    }

    fn visit_edge_pattern(&mut self, pattern: &EdgePattern) -> VisitResult<Self::Break> {
        walk_edge_pattern(self, pattern)
    }

    fn visit_label_expression(
        &mut self,
        expression: &LabelExpression,
    ) -> VisitResult<Self::Break> {
        walk_label_expression(self, expression)
    }

    fn visit_filter_statement(&mut self, statement: &FilterStatement) -> VisitResult<Self::Break> {
        walk_filter_statement(self, statement)
    }

    fn visit_let_statement(&mut self, statement: &LetStatement) -> VisitResult<Self::Break> {
        walk_let_statement(self, statement)
    }

    fn visit_let_binding(&mut self, binding: &LetVariableDefinition) -> VisitResult<Self::Break> {
        walk_let_binding(self, binding)
    }

    fn visit_for_statement(&mut self, statement: &ForStatement) -> VisitResult<Self::Break> {
        walk_for_statement(self, statement)
    }

    fn visit_select_statement(&mut self, statement: &SelectStatement) -> VisitResult<Self::Break> {
        walk_select_statement(self, statement)
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement) -> VisitResult<Self::Break> {
        walk_return_statement(self, statement)
    }

    fn visit_return_item(&mut self, item: &ReturnItem) -> VisitResult<Self::Break> {
        walk_return_item(self, item)
    }

    fn visit_expression(&mut self, expression: &Expression) -> VisitResult<Self::Break> {
        walk_expression(self, expression)
    }
}

/// Mutable AST visitor for query and expression nodes.
pub trait AstVisitorMut {
    /// Early-exit payload produced when traversal stops.
    type Break;

    fn visit_program_mut(&mut self, program: &mut Program) -> VisitResult<Self::Break> {
        walk_program_mut(self, program)
    }

    fn visit_statement_mut(&mut self, statement: &mut Statement) -> VisitResult<Self::Break> {
        walk_statement_mut(self, statement)
    }

    fn visit_query_statement_mut(
        &mut self,
        statement: &mut QueryStatement,
    ) -> VisitResult<Self::Break> {
        walk_query_statement_mut(self, statement)
    }

    fn visit_query_mut(&mut self, query: &mut Query) -> VisitResult<Self::Break> {
        walk_query_mut(self, query)
    }

    fn visit_linear_query_mut(&mut self, query: &mut LinearQuery) -> VisitResult<Self::Break> {
        walk_linear_query_mut(self, query)
    }

    fn visit_primitive_query_statement_mut(
        &mut self,
        statement: &mut PrimitiveQueryStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_query_statement_mut(self, statement)
    }

    fn visit_primitive_result_statement_mut(
        &mut self,
        statement: &mut PrimitiveResultStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_result_statement_mut(self, statement)
    }

    fn visit_match_statement_mut(
        &mut self,
        statement: &mut MatchStatement,
    ) -> VisitResult<Self::Break> {
        walk_match_statement_mut(self, statement)
    }

    fn visit_graph_pattern_mut(
        &mut self,
        pattern: &mut GraphPattern,
    ) -> VisitResult<Self::Break> {
        walk_graph_pattern_mut(self, pattern)
    }

    fn visit_path_pattern_mut(&mut self, pattern: &mut PathPattern) -> VisitResult<Self::Break> {
        walk_path_pattern_mut(self, pattern)
    }

    fn visit_path_pattern_expression_mut(
        &mut self,
        expression: &mut PathPatternExpression,
    ) -> VisitResult<Self::Break> {
        walk_path_pattern_expression_mut(self, expression)
    }

    fn visit_path_factor_mut(&mut self, factor: &mut PathFactor) -> VisitResult<Self::Break> {
        walk_path_factor_mut(self, factor)
    }

    fn visit_path_primary_mut(&mut self, primary: &mut PathPrimary) -> VisitResult<Self::Break> {
        walk_path_primary_mut(self, primary)
    }

    fn visit_element_pattern_mut(
        &mut self,
        pattern: &mut ElementPattern,
    ) -> VisitResult<Self::Break> {
        walk_element_pattern_mut(self, pattern)
    }

    fn visit_node_pattern_mut(&mut self, pattern: &mut NodePattern) -> VisitResult<Self::Break> {
        walk_node_pattern_mut(self, pattern)
    }

    fn visit_edge_pattern_mut(&mut self, pattern: &mut EdgePattern) -> VisitResult<Self::Break> {
        walk_edge_pattern_mut(self, pattern)
    }

    fn visit_label_expression_mut(
        &mut self,
        expression: &mut LabelExpression,
    ) -> VisitResult<Self::Break> {
        walk_label_expression_mut(self, expression)
    }

    fn visit_filter_statement_mut(
        &mut self,
        statement: &mut FilterStatement,
    ) -> VisitResult<Self::Break> {
        walk_filter_statement_mut(self, statement)
    }

    fn visit_let_statement_mut(
        &mut self,
        statement: &mut LetStatement,
    ) -> VisitResult<Self::Break> {
        walk_let_statement_mut(self, statement)
    }

    fn visit_let_binding_mut(
        &mut self,
        binding: &mut LetVariableDefinition,
    ) -> VisitResult<Self::Break> {
        walk_let_binding_mut(self, binding)
    }

    fn visit_for_statement_mut(&mut self, statement: &mut ForStatement) -> VisitResult<Self::Break> {
        walk_for_statement_mut(self, statement)
    }

    fn visit_select_statement_mut(
        &mut self,
        statement: &mut SelectStatement,
    ) -> VisitResult<Self::Break> {
        walk_select_statement_mut(self, statement)
    }

    fn visit_return_statement_mut(
        &mut self,
        statement: &mut ReturnStatement,
    ) -> VisitResult<Self::Break> {
        walk_return_statement_mut(self, statement)
    }

    fn visit_return_item_mut(&mut self, item: &mut ReturnItem) -> VisitResult<Self::Break> {
        walk_return_item_mut(self, item)
    }

    fn visit_expression_mut(&mut self, expression: &mut Expression) -> VisitResult<Self::Break> {
        walk_expression_mut(self, expression)
    }
}

/// Walks a full program with an immutable visitor.
pub fn walk_program<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    program: &Program,
) -> VisitResult<V::Break> {
    for statement in &program.statements {
        try_visit!(visitor.visit_statement(statement));
    }
    ControlFlow::Continue(())
}

/// Walks a statement with an immutable visitor.
pub fn walk_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &Statement,
) -> VisitResult<V::Break> {
    if let Statement::Query(query_statement) = statement {
        return visitor.visit_query_statement(query_statement);
    }

    ControlFlow::Continue(())
}

/// Walks a query statement with an immutable visitor.
pub fn walk_query_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &QueryStatement,
) -> VisitResult<V::Break> {
    visitor.visit_query(&statement.query)
}

/// Walks a query with an immutable visitor.
pub fn walk_query<V: AstVisitor + ?Sized>(visitor: &mut V, query: &Query) -> VisitResult<V::Break> {
    match query {
        Query::Linear(linear) => visitor.visit_linear_query(linear),
        Query::Composite(composite) => {
            try_visit!(visitor.visit_query(&composite.left));
            visitor.visit_query(&composite.right)
        }
        Query::Parenthesized(inner, _) => visitor.visit_query(inner),
    }
}

/// Walks a linear query with an immutable visitor.
pub fn walk_linear_query<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    query: &LinearQuery,
) -> VisitResult<V::Break> {
    match query {
        LinearQuery::Focused(focused) => {
            try_visit!(visitor.visit_expression(&focused.use_graph.graph));
            for statement in &focused.primitive_statements {
                try_visit!(visitor.visit_primitive_query_statement(statement));
            }
            if let Some(result) = &focused.result_statement {
                try_visit!(visitor.visit_primitive_result_statement(result));
            }
        }
        LinearQuery::Ambient(ambient) => {
            for statement in &ambient.primitive_statements {
                try_visit!(visitor.visit_primitive_query_statement(statement));
            }
            if let Some(result) = &ambient.result_statement {
                try_visit!(visitor.visit_primitive_result_statement(result));
            }
        }
    }

    ControlFlow::Continue(())
}

/// Walks a primitive query statement with an immutable visitor.
pub fn walk_primitive_query_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &PrimitiveQueryStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveQueryStatement::Match(match_statement) => visitor.visit_match_statement(match_statement),
        PrimitiveQueryStatement::Call(call) => walk_call_procedure_statement(visitor, call),
        PrimitiveQueryStatement::Filter(filter) => visitor.visit_filter_statement(filter),
        PrimitiveQueryStatement::Let(let_statement) => visitor.visit_let_statement(let_statement),
        PrimitiveQueryStatement::For(for_statement) => visitor.visit_for_statement(for_statement),
        PrimitiveQueryStatement::OrderByAndPage(order_by_and_page) => {
            if let Some(order_by) = &order_by_and_page.order_by {
                for sort in &order_by.sort_specifications {
                    try_visit!(visitor.visit_expression(&sort.key));
                }
            }
            if let Some(offset) = &order_by_and_page.offset {
                try_visit!(visitor.visit_expression(&offset.count));
            }
            if let Some(limit) = &order_by_and_page.limit {
                try_visit!(visitor.visit_expression(&limit.count));
            }
            ControlFlow::Continue(())
        }
        PrimitiveQueryStatement::Select(select) => visitor.visit_select_statement(select),
    }
}

/// Walks a primitive result statement with an immutable visitor.
pub fn walk_primitive_result_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &PrimitiveResultStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveResultStatement::Return(return_statement) => {
            visitor.visit_return_statement(return_statement)
        }
        PrimitiveResultStatement::Finish(_) => ControlFlow::Continue(()),
    }
}

/// Walks a match statement with an immutable visitor.
pub fn walk_match_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &MatchStatement,
) -> VisitResult<V::Break> {
    match statement {
        MatchStatement::Simple(simple) => visitor.visit_graph_pattern(&simple.pattern),
        MatchStatement::Optional(optional) => match &optional.operand {
            crate::ast::query::OptionalOperand::Match { pattern } => visitor.visit_graph_pattern(pattern),
            crate::ast::query::OptionalOperand::Block { statements }
            | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                for statement in statements {
                    try_visit!(visitor.visit_match_statement(statement));
                }
                ControlFlow::Continue(())
            }
        },
    }
}

/// Walks a graph pattern with an immutable visitor.
pub fn walk_graph_pattern<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    pattern: &GraphPattern,
) -> VisitResult<V::Break> {
    for path_pattern in &pattern.paths.patterns {
        try_visit!(visitor.visit_path_pattern(path_pattern));
    }

    if let Some(where_clause) = &pattern.where_clause {
        try_visit!(visitor.visit_expression(&where_clause.condition));
    }

    if let Some(yield_clause) = &pattern.yield_clause {
        for item in &yield_clause.items {
            try_visit!(visitor.visit_expression(&item.expression));
        }
    }

    ControlFlow::Continue(())
}

/// Walks a path pattern with an immutable visitor.
pub fn walk_path_pattern<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    pattern: &PathPattern,
) -> VisitResult<V::Break> {
    visitor.visit_path_pattern_expression(&pattern.expression)
}

/// Walks a path pattern expression with an immutable visitor.
pub fn walk_path_pattern_expression<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    expression: &PathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        PathPatternExpression::Union { left, right, .. } => {
            try_visit!(visitor.visit_path_pattern_expression(left));
            visitor.visit_path_pattern_expression(right)
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for term in alternatives {
                for factor in &term.factors {
                    try_visit!(visitor.visit_path_factor(factor));
                }
            }
            ControlFlow::Continue(())
        }
        PathPatternExpression::Term(term) => {
            for factor in &term.factors {
                try_visit!(visitor.visit_path_factor(factor));
            }
            ControlFlow::Continue(())
        }
    }
}

/// Walks a path factor with an immutable visitor.
pub fn walk_path_factor<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    factor: &PathFactor,
) -> VisitResult<V::Break> {
    visitor.visit_path_primary(&factor.primary)
}

/// Walks a path primary with an immutable visitor.
pub fn walk_path_primary<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    primary: &PathPrimary,
) -> VisitResult<V::Break> {
    match primary {
        PathPrimary::ElementPattern(pattern) => visitor.visit_element_pattern(pattern),
        PathPrimary::ParenthesizedExpression(expression) => {
            visitor.visit_path_pattern_expression(expression)
        }
        PathPrimary::SimplifiedExpression(expression) => {
            walk_simplified_expression(visitor, expression)
        }
    }
}

/// Walks an element pattern with an immutable visitor.
pub fn walk_element_pattern<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    pattern: &ElementPattern,
) -> VisitResult<V::Break> {
    match pattern {
        ElementPattern::Node(node_pattern) => visitor.visit_node_pattern(node_pattern),
        ElementPattern::Edge(edge_pattern) => visitor.visit_edge_pattern(edge_pattern),
    }
}

/// Walks a node pattern with an immutable visitor.
pub fn walk_node_pattern<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    pattern: &NodePattern,
) -> VisitResult<V::Break> {
    if let Some(label_expression) = &pattern.label_expression {
        try_visit!(visitor.visit_label_expression(label_expression));
    }
    if let Some(properties) = &pattern.properties {
        for property in &properties.properties {
            try_visit!(visitor.visit_expression(&property.value));
        }
    }
    if let Some(where_clause) = &pattern.where_clause {
        try_visit!(visitor.visit_expression(&where_clause.condition));
    }

    ControlFlow::Continue(())
}

/// Walks an edge pattern with an immutable visitor.
pub fn walk_edge_pattern<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    pattern: &EdgePattern,
) -> VisitResult<V::Break> {
    if let EdgePattern::Full(full) = pattern {
        if let Some(label_expression) = &full.filler.label_expression {
            try_visit!(visitor.visit_label_expression(label_expression));
        }
        if let Some(properties) = &full.filler.properties {
            for property in &properties.properties {
                try_visit!(visitor.visit_expression(&property.value));
            }
        }
        if let Some(where_clause) = &full.filler.where_clause {
            try_visit!(visitor.visit_expression(&where_clause.condition));
        }
    }

    ControlFlow::Continue(())
}

/// Walks a label expression with an immutable visitor.
pub fn walk_label_expression<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    expression: &LabelExpression,
) -> VisitResult<V::Break> {
    match expression {
        LabelExpression::Negation { operand, .. } => visitor.visit_label_expression(operand),
        LabelExpression::Conjunction { left, right, .. }
        | LabelExpression::Disjunction { left, right, .. } => {
            try_visit!(visitor.visit_label_expression(left));
            visitor.visit_label_expression(right)
        }
        LabelExpression::LabelName { .. } | LabelExpression::Wildcard { .. } => {
            ControlFlow::Continue(())
        }
        LabelExpression::Parenthesized { expression, .. } => {
            visitor.visit_label_expression(expression)
        }
    }
}

/// Walks a filter statement with an immutable visitor.
pub fn walk_filter_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &FilterStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression(&statement.condition)
}

/// Walks a let statement with an immutable visitor.
pub fn walk_let_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &LetStatement,
) -> VisitResult<V::Break> {
    for binding in &statement.bindings {
        try_visit!(visitor.visit_let_binding(binding));
    }

    ControlFlow::Continue(())
}

/// Walks a let binding with an immutable visitor.
pub fn walk_let_binding<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    binding: &LetVariableDefinition,
) -> VisitResult<V::Break> {
    visitor.visit_expression(&binding.value)
}

/// Walks a FOR statement with an immutable visitor.
pub fn walk_for_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &ForStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression(&statement.item.collection)
}

/// Walks a SELECT statement with an immutable visitor.
pub fn walk_select_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &SelectStatement,
) -> VisitResult<V::Break> {
    match &statement.select_items {
        SelectItemList::Star => {}
        SelectItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_expression(&item.expression));
            }
        }
    }

    if let Some(from_clause) = &statement.from_clause {
        match from_clause {
            SelectFromClause::GraphMatchList { matches } => {
                for graph_pattern in matches {
                    try_visit!(visitor.visit_graph_pattern(graph_pattern));
                }
            }
            SelectFromClause::QuerySpecification { query } => {
                try_visit!(visitor.visit_query(query));
            }
            SelectFromClause::GraphAndQuerySpecification { graph, query } => {
                try_visit!(visitor.visit_expression(graph));
                try_visit!(visitor.visit_query(query));
            }
        }
    }

    if let Some(where_clause) = &statement.where_clause {
        try_visit!(visitor.visit_expression(&where_clause.condition));
    }

    if let Some(group_by) = &statement.group_by {
        for element in &group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression(expression));
            }
        }
    }

    if let Some(having) = &statement.having {
        try_visit!(visitor.visit_expression(&having.condition));
    }

    if let Some(order_by) = &statement.order_by {
        for sort in &order_by.sort_specifications {
            try_visit!(visitor.visit_expression(&sort.key));
        }
    }

    if let Some(offset) = &statement.offset {
        try_visit!(visitor.visit_expression(&offset.count));
    }

    if let Some(limit) = &statement.limit {
        try_visit!(visitor.visit_expression(&limit.count));
    }

    ControlFlow::Continue(())
}

/// Walks a RETURN statement with an immutable visitor.
pub fn walk_return_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    statement: &ReturnStatement,
) -> VisitResult<V::Break> {
    match &statement.items {
        ReturnItemList::Star => {}
        ReturnItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_return_item(item));
            }
        }
    }

    if let Some(group_by) = &statement.group_by {
        for element in &group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression(expression));
            }
        }
    }

    ControlFlow::Continue(())
}

/// Walks a RETURN item with an immutable visitor.
pub fn walk_return_item<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    item: &ReturnItem,
) -> VisitResult<V::Break> {
    visitor.visit_expression(&item.expression)
}

/// Walks an expression with an immutable visitor.
pub fn walk_expression<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    expression: &Expression,
) -> VisitResult<V::Break> {
    match expression {
        Expression::Literal(literal, _) => walk_literal(visitor, literal),
        Expression::Unary(_, inner, _)
        | Expression::Parenthesized(inner, _)
        | Expression::GraphExpression(inner, _)
        | Expression::BindingTableExpression(inner, _)
        | Expression::SubqueryExpression(inner, _) => visitor.visit_expression(inner),
        Expression::Binary(_, left, right, _)
        | Expression::Comparison(_, left, right, _)
        | Expression::Logical(_, left, right, _) => {
            try_visit!(visitor.visit_expression(left));
            visitor.visit_expression(right)
        }
        Expression::PropertyReference(target, _, _) => visitor.visit_expression(target),
        Expression::VariableReference(_, _) | Expression::ParameterReference(_, _) => {
            ControlFlow::Continue(())
        }
        Expression::FunctionCall(function_call) => {
            for argument in &function_call.arguments {
                try_visit!(visitor.visit_expression(argument));
            }
            ControlFlow::Continue(())
        }
        Expression::Case(case_expression) => match case_expression {
            CaseExpression::Simple(simple) => {
                try_visit!(visitor.visit_expression(&simple.operand));
                for when_clause in &simple.when_clauses {
                    try_visit!(visitor.visit_expression(&when_clause.when_value));
                    try_visit!(visitor.visit_expression(&when_clause.then_result));
                }
                if let Some(else_clause) = &simple.else_clause {
                    try_visit!(visitor.visit_expression(else_clause));
                }
                ControlFlow::Continue(())
            }
            CaseExpression::Searched(searched) => {
                for when_clause in &searched.when_clauses {
                    try_visit!(visitor.visit_expression(&when_clause.condition));
                    try_visit!(visitor.visit_expression(&when_clause.then_result));
                }
                if let Some(else_clause) = &searched.else_clause {
                    try_visit!(visitor.visit_expression(else_clause));
                }
                ControlFlow::Continue(())
            }
        },
        Expression::Cast(cast) => visitor.visit_expression(&cast.operand),
        Expression::AggregateFunction(aggregate_function) => match aggregate_function.as_ref() {
            crate::ast::expression::AggregateFunction::CountStar { .. } => ControlFlow::Continue(()),
            crate::ast::expression::AggregateFunction::GeneralSetFunction(function) => {
                visitor.visit_expression(&function.expression)
            }
            crate::ast::expression::AggregateFunction::BinarySetFunction(function) => {
                try_visit!(visitor.visit_expression(&function.inverse_distribution_argument));
                visitor.visit_expression(&function.expression)
            }
        },
        Expression::TypeAnnotation(inner, _, _) => visitor.visit_expression(inner),
        Expression::ListConstructor(expressions, _) | Expression::PathConstructor(expressions, _) => {
            for item in expressions {
                try_visit!(visitor.visit_expression(item));
            }
            ControlFlow::Continue(())
        }
        Expression::RecordConstructor(fields, _) => {
            for field in fields {
                try_visit!(visitor.visit_expression(&field.value));
            }
            ControlFlow::Continue(())
        }
        Expression::Exists(exists_expression) => match &exists_expression.variant {
            ExistsVariant::GraphPattern(_) => ControlFlow::Continue(()),
            ExistsVariant::Subquery(subquery) => visitor.visit_expression(subquery),
        },
        Expression::Predicate(predicate) => walk_predicate(visitor, predicate),
    }
}

fn walk_predicate<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    predicate: &Predicate,
) -> VisitResult<V::Break> {
    match predicate {
        Predicate::IsNull(expression, _, _)
        | Predicate::IsTyped(expression, _, _, _)
        | Predicate::IsNormalized(expression, _, _)
        | Predicate::IsDirected(expression, _, _)
        | Predicate::IsTruthValue(expression, _, _, _)
        | Predicate::PropertyExists(expression, _, _) => visitor.visit_expression(expression),
        Predicate::IsLabeled(expression, _, _, _) => visitor.visit_expression(expression),
        Predicate::IsSource(source, target, _, _) | Predicate::IsDestination(source, target, _, _) => {
            try_visit!(visitor.visit_expression(source));
            visitor.visit_expression(target)
        }
        Predicate::AllDifferent(expressions, _) => {
            for expression in expressions {
                try_visit!(visitor.visit_expression(expression));
            }
            ControlFlow::Continue(())
        }
        Predicate::Same(left, right, _) => {
            try_visit!(visitor.visit_expression(left));
            visitor.visit_expression(right)
        }
    }
}

fn walk_literal<V: AstVisitor + ?Sized>(visitor: &mut V, literal: &Literal) -> VisitResult<V::Break> {
    match literal {
        Literal::List(expressions) => {
            for expression in expressions {
                try_visit!(visitor.visit_expression(expression));
            }
            ControlFlow::Continue(())
        }
        Literal::Record(fields) => {
            for field in fields {
                try_visit!(visitor.visit_expression(&field.value));
            }
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

fn walk_call_procedure_statement<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    call: &CallProcedureStatement,
) -> VisitResult<V::Break> {
    match &call.call {
        ProcedureCall::Inline(_) => ControlFlow::Continue(()),
        ProcedureCall::Named(named) => {
            if let Some(arguments) = &named.arguments {
                for argument in &arguments.arguments {
                    try_visit!(visitor.visit_expression(&argument.expression));
                }
            }
            if let Some(yield_clause) = &named.yield_clause {
                for item in &yield_clause.items.items {
                    try_visit!(visitor.visit_expression(&item.expression));
                }
            }
            ControlFlow::Continue(())
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn walk_simplified_expression<V: AstVisitor + ?Sized>(
    visitor: &mut V,
    expression: &SimplifiedPathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        SimplifiedPathPatternExpression::Contents(_) => ControlFlow::Continue(()),
        SimplifiedPathPatternExpression::Union(union) => {
            try_visit!(walk_simplified_expression(visitor, &union.left));
            walk_simplified_expression(visitor, &union.right)
        }
        SimplifiedPathPatternExpression::MultisetAlternation(alternation) => {
            for alternative in &alternation.alternatives {
                try_visit!(walk_simplified_expression(visitor, alternative));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Conjunction(conjunction) => {
            try_visit!(walk_simplified_expression(visitor, &conjunction.left));
            walk_simplified_expression(visitor, &conjunction.right)
        }
        SimplifiedPathPatternExpression::Concatenation(concatenation) => {
            for part in &concatenation.parts {
                try_visit!(walk_simplified_expression(visitor, part));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Quantified(quantified) => {
            walk_simplified_expression(visitor, &quantified.pattern)
        }
        SimplifiedPathPatternExpression::Questioned(questioned) => {
            walk_simplified_expression(visitor, &questioned.pattern)
        }
        SimplifiedPathPatternExpression::DirectionOverride(direction_override) => {
            walk_simplified_expression(visitor, &direction_override.pattern)
        }
        SimplifiedPathPatternExpression::Negation(negation) => {
            walk_simplified_expression(visitor, &negation.pattern)
        }
    }
}

/// Mutable program walker.
pub fn walk_program_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    program: &mut Program,
) -> VisitResult<V::Break> {
    for statement in &mut program.statements {
        try_visit!(visitor.visit_statement_mut(statement));
    }
    ControlFlow::Continue(())
}

/// Mutable statement walker.
pub fn walk_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut Statement,
) -> VisitResult<V::Break> {
    if let Statement::Query(query_statement) = statement {
        return visitor.visit_query_statement_mut(query_statement);
    }

    ControlFlow::Continue(())
}

/// Mutable query statement walker.
pub fn walk_query_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut QueryStatement,
) -> VisitResult<V::Break> {
    visitor.visit_query_mut(&mut statement.query)
}

/// Mutable query walker.
pub fn walk_query_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    query: &mut Query,
) -> VisitResult<V::Break> {
    match query {
        Query::Linear(linear) => visitor.visit_linear_query_mut(linear),
        Query::Composite(composite) => {
            try_visit!(visitor.visit_query_mut(&mut composite.left));
            visitor.visit_query_mut(&mut composite.right)
        }
        Query::Parenthesized(inner, _) => visitor.visit_query_mut(inner),
    }
}

/// Mutable linear query walker.
pub fn walk_linear_query_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    query: &mut LinearQuery,
) -> VisitResult<V::Break> {
    match query {
        LinearQuery::Focused(focused) => {
            try_visit!(visitor.visit_expression_mut(&mut focused.use_graph.graph));
            for statement in &mut focused.primitive_statements {
                try_visit!(visitor.visit_primitive_query_statement_mut(statement));
            }
            if let Some(result) = &mut focused.result_statement {
                try_visit!(visitor.visit_primitive_result_statement_mut(result));
            }
        }
        LinearQuery::Ambient(ambient) => {
            for statement in &mut ambient.primitive_statements {
                try_visit!(visitor.visit_primitive_query_statement_mut(statement));
            }
            if let Some(result) = &mut ambient.result_statement {
                try_visit!(visitor.visit_primitive_result_statement_mut(result));
            }
        }
    }

    ControlFlow::Continue(())
}

/// Mutable primitive query statement walker.
pub fn walk_primitive_query_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut PrimitiveQueryStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveQueryStatement::Match(match_statement) => {
            visitor.visit_match_statement_mut(match_statement)
        }
        PrimitiveQueryStatement::Call(call) => walk_call_procedure_statement_mut(visitor, call),
        PrimitiveQueryStatement::Filter(filter) => visitor.visit_filter_statement_mut(filter),
        PrimitiveQueryStatement::Let(let_statement) => visitor.visit_let_statement_mut(let_statement),
        PrimitiveQueryStatement::For(for_statement) => visitor.visit_for_statement_mut(for_statement),
        PrimitiveQueryStatement::OrderByAndPage(order_by_and_page) => {
            if let Some(order_by) = &mut order_by_and_page.order_by {
                for sort in &mut order_by.sort_specifications {
                    try_visit!(visitor.visit_expression_mut(&mut sort.key));
                }
            }
            if let Some(offset) = &mut order_by_and_page.offset {
                try_visit!(visitor.visit_expression_mut(&mut offset.count));
            }
            if let Some(limit) = &mut order_by_and_page.limit {
                try_visit!(visitor.visit_expression_mut(&mut limit.count));
            }
            ControlFlow::Continue(())
        }
        PrimitiveQueryStatement::Select(select) => visitor.visit_select_statement_mut(select),
    }
}

/// Mutable primitive result statement walker.
pub fn walk_primitive_result_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut PrimitiveResultStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveResultStatement::Return(return_statement) => {
            visitor.visit_return_statement_mut(return_statement)
        }
        PrimitiveResultStatement::Finish(_) => ControlFlow::Continue(()),
    }
}

/// Mutable match statement walker.
pub fn walk_match_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut MatchStatement,
) -> VisitResult<V::Break> {
    match statement {
        MatchStatement::Simple(simple) => visitor.visit_graph_pattern_mut(&mut simple.pattern),
        MatchStatement::Optional(optional) => match &mut optional.operand {
            crate::ast::query::OptionalOperand::Match { pattern } => {
                visitor.visit_graph_pattern_mut(pattern)
            }
            crate::ast::query::OptionalOperand::Block { statements }
            | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                for statement in statements {
                    try_visit!(visitor.visit_match_statement_mut(statement));
                }
                ControlFlow::Continue(())
            }
        },
    }
}

/// Mutable graph pattern walker.
pub fn walk_graph_pattern_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    pattern: &mut GraphPattern,
) -> VisitResult<V::Break> {
    for path_pattern in &mut pattern.paths.patterns {
        try_visit!(visitor.visit_path_pattern_mut(path_pattern));
    }

    if let Some(where_clause) = &mut pattern.where_clause {
        try_visit!(visitor.visit_expression_mut(&mut where_clause.condition));
    }

    if let Some(yield_clause) = &mut pattern.yield_clause {
        for item in &mut yield_clause.items {
            try_visit!(visitor.visit_expression_mut(&mut item.expression));
        }
    }

    ControlFlow::Continue(())
}

/// Mutable path pattern walker.
pub fn walk_path_pattern_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    pattern: &mut PathPattern,
) -> VisitResult<V::Break> {
    visitor.visit_path_pattern_expression_mut(&mut pattern.expression)
}

/// Mutable path pattern expression walker.
pub fn walk_path_pattern_expression_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    expression: &mut PathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        PathPatternExpression::Union { left, right, .. } => {
            try_visit!(visitor.visit_path_pattern_expression_mut(left));
            visitor.visit_path_pattern_expression_mut(right)
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for term in alternatives {
                for factor in &mut term.factors {
                    try_visit!(visitor.visit_path_factor_mut(factor));
                }
            }
            ControlFlow::Continue(())
        }
        PathPatternExpression::Term(term) => {
            for factor in &mut term.factors {
                try_visit!(visitor.visit_path_factor_mut(factor));
            }
            ControlFlow::Continue(())
        }
    }
}

/// Mutable path factor walker.
pub fn walk_path_factor_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    factor: &mut PathFactor,
) -> VisitResult<V::Break> {
    visitor.visit_path_primary_mut(&mut factor.primary)
}

/// Mutable path primary walker.
pub fn walk_path_primary_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    primary: &mut PathPrimary,
) -> VisitResult<V::Break> {
    match primary {
        PathPrimary::ElementPattern(pattern) => visitor.visit_element_pattern_mut(pattern),
        PathPrimary::ParenthesizedExpression(expression) => {
            visitor.visit_path_pattern_expression_mut(expression)
        }
        PathPrimary::SimplifiedExpression(expression) => {
            walk_simplified_expression_mut(visitor, expression)
        }
    }
}

/// Mutable element pattern walker.
pub fn walk_element_pattern_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    pattern: &mut ElementPattern,
) -> VisitResult<V::Break> {
    match pattern {
        ElementPattern::Node(node_pattern) => visitor.visit_node_pattern_mut(node_pattern),
        ElementPattern::Edge(edge_pattern) => visitor.visit_edge_pattern_mut(edge_pattern),
    }
}

/// Mutable node pattern walker.
pub fn walk_node_pattern_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    pattern: &mut NodePattern,
) -> VisitResult<V::Break> {
    if let Some(label_expression) = &mut pattern.label_expression {
        try_visit!(visitor.visit_label_expression_mut(label_expression));
    }
    if let Some(properties) = &mut pattern.properties {
        for property in &mut properties.properties {
            try_visit!(visitor.visit_expression_mut(&mut property.value));
        }
    }
    if let Some(where_clause) = &mut pattern.where_clause {
        try_visit!(visitor.visit_expression_mut(&mut where_clause.condition));
    }

    ControlFlow::Continue(())
}

/// Mutable edge pattern walker.
pub fn walk_edge_pattern_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    pattern: &mut EdgePattern,
) -> VisitResult<V::Break> {
    if let EdgePattern::Full(full) = pattern {
        if let Some(label_expression) = &mut full.filler.label_expression {
            try_visit!(visitor.visit_label_expression_mut(label_expression));
        }
        if let Some(properties) = &mut full.filler.properties {
            for property in &mut properties.properties {
                try_visit!(visitor.visit_expression_mut(&mut property.value));
            }
        }
        if let Some(where_clause) = &mut full.filler.where_clause {
            try_visit!(visitor.visit_expression_mut(&mut where_clause.condition));
        }
    }

    ControlFlow::Continue(())
}

/// Mutable label expression walker.
pub fn walk_label_expression_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    expression: &mut LabelExpression,
) -> VisitResult<V::Break> {
    match expression {
        LabelExpression::Negation { operand, .. } => visitor.visit_label_expression_mut(operand),
        LabelExpression::Conjunction { left, right, .. }
        | LabelExpression::Disjunction { left, right, .. } => {
            try_visit!(visitor.visit_label_expression_mut(left));
            visitor.visit_label_expression_mut(right)
        }
        LabelExpression::LabelName { .. } | LabelExpression::Wildcard { .. } => {
            ControlFlow::Continue(())
        }
        LabelExpression::Parenthesized { expression, .. } => {
            visitor.visit_label_expression_mut(expression)
        }
    }
}

/// Mutable filter statement walker.
pub fn walk_filter_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut FilterStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression_mut(&mut statement.condition)
}

/// Mutable let statement walker.
pub fn walk_let_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut LetStatement,
) -> VisitResult<V::Break> {
    for binding in &mut statement.bindings {
        try_visit!(visitor.visit_let_binding_mut(binding));
    }

    ControlFlow::Continue(())
}

/// Mutable let binding walker.
pub fn walk_let_binding_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    binding: &mut LetVariableDefinition,
) -> VisitResult<V::Break> {
    visitor.visit_expression_mut(&mut binding.value)
}

/// Mutable FOR statement walker.
pub fn walk_for_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut ForStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression_mut(&mut statement.item.collection)
}

/// Mutable SELECT statement walker.
pub fn walk_select_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut SelectStatement,
) -> VisitResult<V::Break> {
    match &mut statement.select_items {
        SelectItemList::Star => {}
        SelectItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_expression_mut(&mut item.expression));
            }
        }
    }

    if let Some(from_clause) = &mut statement.from_clause {
        match from_clause {
            SelectFromClause::GraphMatchList { matches } => {
                for graph_pattern in matches {
                    try_visit!(visitor.visit_graph_pattern_mut(graph_pattern));
                }
            }
            SelectFromClause::QuerySpecification { query } => {
                try_visit!(visitor.visit_query_mut(query));
            }
            SelectFromClause::GraphAndQuerySpecification { graph, query } => {
                try_visit!(visitor.visit_expression_mut(graph));
                try_visit!(visitor.visit_query_mut(query));
            }
        }
    }

    if let Some(where_clause) = &mut statement.where_clause {
        try_visit!(visitor.visit_expression_mut(&mut where_clause.condition));
    }

    if let Some(group_by) = &mut statement.group_by {
        for element in &mut group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression_mut(expression));
            }
        }
    }

    if let Some(having) = &mut statement.having {
        try_visit!(visitor.visit_expression_mut(&mut having.condition));
    }

    if let Some(order_by) = &mut statement.order_by {
        for sort in &mut order_by.sort_specifications {
            try_visit!(visitor.visit_expression_mut(&mut sort.key));
        }
    }

    if let Some(offset) = &mut statement.offset {
        try_visit!(visitor.visit_expression_mut(&mut offset.count));
    }

    if let Some(limit) = &mut statement.limit {
        try_visit!(visitor.visit_expression_mut(&mut limit.count));
    }

    ControlFlow::Continue(())
}

/// Mutable RETURN statement walker.
pub fn walk_return_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    statement: &mut ReturnStatement,
) -> VisitResult<V::Break> {
    match &mut statement.items {
        ReturnItemList::Star => {}
        ReturnItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_return_item_mut(item));
            }
        }
    }

    if let Some(group_by) = &mut statement.group_by {
        for element in &mut group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression_mut(expression));
            }
        }
    }

    ControlFlow::Continue(())
}

/// Mutable RETURN item walker.
pub fn walk_return_item_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    item: &mut ReturnItem,
) -> VisitResult<V::Break> {
    visitor.visit_expression_mut(&mut item.expression)
}

/// Mutable expression walker.
pub fn walk_expression_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    expression: &mut Expression,
) -> VisitResult<V::Break> {
    match expression {
        Expression::Literal(literal, _) => walk_literal_mut(visitor, literal),
        Expression::Unary(_, inner, _)
        | Expression::Parenthesized(inner, _)
        | Expression::GraphExpression(inner, _)
        | Expression::BindingTableExpression(inner, _)
        | Expression::SubqueryExpression(inner, _) => visitor.visit_expression_mut(inner),
        Expression::Binary(_, left, right, _)
        | Expression::Comparison(_, left, right, _)
        | Expression::Logical(_, left, right, _) => {
            try_visit!(visitor.visit_expression_mut(left));
            visitor.visit_expression_mut(right)
        }
        Expression::PropertyReference(target, _, _) => visitor.visit_expression_mut(target),
        Expression::VariableReference(_, _) | Expression::ParameterReference(_, _) => {
            ControlFlow::Continue(())
        }
        Expression::FunctionCall(function_call) => {
            for argument in &mut function_call.arguments {
                try_visit!(visitor.visit_expression_mut(argument));
            }
            ControlFlow::Continue(())
        }
        Expression::Case(case_expression) => match case_expression {
            CaseExpression::Simple(simple) => {
                try_visit!(visitor.visit_expression_mut(&mut simple.operand));
                for when_clause in &mut simple.when_clauses {
                    try_visit!(visitor.visit_expression_mut(&mut when_clause.when_value));
                    try_visit!(visitor.visit_expression_mut(&mut when_clause.then_result));
                }
                if let Some(else_clause) = &mut simple.else_clause {
                    try_visit!(visitor.visit_expression_mut(else_clause));
                }
                ControlFlow::Continue(())
            }
            CaseExpression::Searched(searched) => {
                for when_clause in &mut searched.when_clauses {
                    try_visit!(visitor.visit_expression_mut(&mut when_clause.condition));
                    try_visit!(visitor.visit_expression_mut(&mut when_clause.then_result));
                }
                if let Some(else_clause) = &mut searched.else_clause {
                    try_visit!(visitor.visit_expression_mut(else_clause));
                }
                ControlFlow::Continue(())
            }
        },
        Expression::Cast(cast) => visitor.visit_expression_mut(&mut cast.operand),
        Expression::AggregateFunction(aggregate_function) => match aggregate_function.as_mut() {
            crate::ast::expression::AggregateFunction::CountStar { .. } => ControlFlow::Continue(()),
            crate::ast::expression::AggregateFunction::GeneralSetFunction(function) => {
                visitor.visit_expression_mut(&mut function.expression)
            }
            crate::ast::expression::AggregateFunction::BinarySetFunction(function) => {
                try_visit!(visitor.visit_expression_mut(
                    &mut function.inverse_distribution_argument,
                ));
                visitor.visit_expression_mut(&mut function.expression)
            }
        },
        Expression::TypeAnnotation(inner, _, _) => visitor.visit_expression_mut(inner),
        Expression::ListConstructor(expressions, _) | Expression::PathConstructor(expressions, _) => {
            for item in expressions {
                try_visit!(visitor.visit_expression_mut(item));
            }
            ControlFlow::Continue(())
        }
        Expression::RecordConstructor(fields, _) => {
            for field in fields {
                try_visit!(visitor.visit_expression_mut(&mut field.value));
            }
            ControlFlow::Continue(())
        }
        Expression::Exists(exists_expression) => match &mut exists_expression.variant {
            ExistsVariant::GraphPattern(_) => ControlFlow::Continue(()),
            ExistsVariant::Subquery(subquery) => visitor.visit_expression_mut(subquery),
        },
        Expression::Predicate(predicate) => walk_predicate_mut(visitor, predicate),
    }
}

fn walk_predicate_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    predicate: &mut Predicate,
) -> VisitResult<V::Break> {
    match predicate {
        Predicate::IsNull(expression, _, _)
        | Predicate::IsTyped(expression, _, _, _)
        | Predicate::IsNormalized(expression, _, _)
        | Predicate::IsDirected(expression, _, _)
        | Predicate::IsTruthValue(expression, _, _, _)
        | Predicate::PropertyExists(expression, _, _) => visitor.visit_expression_mut(expression),
        Predicate::IsLabeled(expression, _, _, _) => visitor.visit_expression_mut(expression),
        Predicate::IsSource(source, target, _, _) | Predicate::IsDestination(source, target, _, _) => {
            try_visit!(visitor.visit_expression_mut(source));
            visitor.visit_expression_mut(target)
        }
        Predicate::AllDifferent(expressions, _) => {
            for expression in expressions {
                try_visit!(visitor.visit_expression_mut(expression));
            }
            ControlFlow::Continue(())
        }
        Predicate::Same(left, right, _) => {
            try_visit!(visitor.visit_expression_mut(left));
            visitor.visit_expression_mut(right)
        }
    }
}

fn walk_literal_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    literal: &mut Literal,
) -> VisitResult<V::Break> {
    match literal {
        Literal::List(expressions) => {
            for expression in expressions {
                try_visit!(visitor.visit_expression_mut(expression));
            }
            ControlFlow::Continue(())
        }
        Literal::Record(fields) => {
            for field in fields {
                try_visit!(visitor.visit_expression_mut(&mut field.value));
            }
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

fn walk_call_procedure_statement_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    call: &mut CallProcedureStatement,
) -> VisitResult<V::Break> {
    match &mut call.call {
        ProcedureCall::Inline(_) => ControlFlow::Continue(()),
        ProcedureCall::Named(named) => {
            if let Some(arguments) = &mut named.arguments {
                for argument in &mut arguments.arguments {
                    try_visit!(visitor.visit_expression_mut(&mut argument.expression));
                }
            }
            if let Some(yield_clause) = &mut named.yield_clause {
                for item in &mut yield_clause.items.items {
                    try_visit!(visitor.visit_expression_mut(&mut item.expression));
                }
            }
            ControlFlow::Continue(())
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn walk_simplified_expression_mut<V: AstVisitorMut + ?Sized>(
    visitor: &mut V,
    expression: &mut SimplifiedPathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        SimplifiedPathPatternExpression::Contents(_) => ControlFlow::Continue(()),
        SimplifiedPathPatternExpression::Union(union) => {
            try_visit!(walk_simplified_expression_mut(visitor, &mut union.left));
            walk_simplified_expression_mut(visitor, &mut union.right)
        }
        SimplifiedPathPatternExpression::MultisetAlternation(alternation) => {
            for alternative in &mut alternation.alternatives {
                try_visit!(walk_simplified_expression_mut(visitor, alternative));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Conjunction(conjunction) => {
            try_visit!(walk_simplified_expression_mut(visitor, &mut conjunction.left));
            walk_simplified_expression_mut(visitor, &mut conjunction.right)
        }
        SimplifiedPathPatternExpression::Concatenation(concatenation) => {
            for part in &mut concatenation.parts {
                try_visit!(walk_simplified_expression_mut(visitor, part));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Quantified(quantified) => {
            walk_simplified_expression_mut(visitor, &mut quantified.pattern)
        }
        SimplifiedPathPatternExpression::Questioned(questioned) => {
            walk_simplified_expression_mut(visitor, &mut questioned.pattern)
        }
        SimplifiedPathPatternExpression::DirectionOverride(direction_override) => {
            walk_simplified_expression_mut(visitor, &mut direction_override.pattern)
        }
        SimplifiedPathPatternExpression::Negation(negation) => {
            walk_simplified_expression_mut(visitor, &mut negation.pattern)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::{AstVisitor, AstVisitorMut};
    use crate::parse;

    #[derive(Default)]
    struct ExprCounter {
        count: usize,
    }

    impl AstVisitor for ExprCounter {
        type Break = ();

        fn visit_expression(
            &mut self,
            expression: &crate::ast::Expression,
        ) -> ControlFlow<Self::Break> {
            self.count += 1;
            super::walk_expression(self, expression)
        }
    }

    #[test]
    fn visitor_walks_query_expressions() {
        let parse_result = parse(
            "MATCH (n:Person {age: 30}) WHERE n.age > 18 LET x = n.age RETURN x + 1 ORDER BY x",
        );
        let program = parse_result.ast.expect("expected AST");

        let mut visitor = ExprCounter::default();
        let flow = visitor.visit_program(&program);

        assert!(matches!(flow, ControlFlow::Continue(())));
        assert!(visitor.count >= 6, "expected multiple expressions to be visited");
    }

    #[derive(Default)]
    struct UppercaseMutator;

    impl AstVisitorMut for UppercaseMutator {
        type Break = ();

        fn visit_expression_mut(
            &mut self,
            expression: &mut crate::ast::Expression,
        ) -> ControlFlow<Self::Break> {
            if let crate::ast::Expression::VariableReference(name, _) = expression {
                *name = name.to_uppercase().into();
            }
            super::walk_expression_mut(self, expression)
        }
    }

    #[test]
    fn mutable_visitor_can_transform_variable_references() {
        let parse_result = parse("MATCH (n) RETURN n");
        let mut program = parse_result.ast.expect("expected AST");

        let mut visitor = UppercaseMutator;
        let flow = visitor.visit_program_mut(&mut program);

        assert!(matches!(flow, ControlFlow::Continue(())));

        let output = format!("{program:?}");
        assert!(output.contains("\"N\""));
    }
}
