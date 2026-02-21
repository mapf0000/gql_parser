macro_rules! define_visit_api {
    ($trait_name:ident, [$($ref:tt)+], $agg_access:ident) => {
        macro_rules! try_visit {
            ($expr:expr) => {
                match $expr {
                    ControlFlow::Continue(()) => {}
                    ControlFlow::Break(b) => return ControlFlow::Break(b),
                }
            };
        }

pub trait $trait_name {
    /// Early-exit payload produced when traversal stops.
    type Break;

    fn visit_program(&mut self, program: $($ref)+ Program) -> VisitResult<Self::Break> {
        walk_program(self, program)
    }

    fn visit_statement(&mut self, statement: $($ref)+ Statement) -> VisitResult<Self::Break> {
        walk_statement(self, statement)
    }

    fn visit_query_statement(&mut self, statement: $($ref)+ QueryStatement) -> VisitResult<Self::Break> {
        walk_query_statement(self, statement)
    }

    fn visit_query(&mut self, query: $($ref)+ Query) -> VisitResult<Self::Break> {
        walk_query(self, query)
    }

    fn visit_linear_query(&mut self, query: $($ref)+ LinearQuery) -> VisitResult<Self::Break> {
        walk_linear_query(self, query)
    }

    fn visit_primitive_query_statement(
        &mut self,
        statement: $($ref)+ PrimitiveQueryStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_query_statement(self, statement)
    }

    fn visit_primitive_result_statement(
        &mut self,
        statement: $($ref)+ PrimitiveResultStatement,
    ) -> VisitResult<Self::Break> {
        walk_primitive_result_statement(self, statement)
    }

    fn visit_match_statement(&mut self, statement: $($ref)+ MatchStatement) -> VisitResult<Self::Break> {
        walk_match_statement(self, statement)
    }

    fn visit_graph_pattern(&mut self, pattern: $($ref)+ GraphPattern) -> VisitResult<Self::Break> {
        walk_graph_pattern(self, pattern)
    }

    fn visit_path_pattern(&mut self, pattern: $($ref)+ PathPattern) -> VisitResult<Self::Break> {
        walk_path_pattern(self, pattern)
    }

    fn visit_path_pattern_expression(
        &mut self,
        expression: $($ref)+ PathPatternExpression,
    ) -> VisitResult<Self::Break> {
        walk_path_pattern_expression(self, expression)
    }

    fn visit_path_factor(&mut self, factor: $($ref)+ PathFactor) -> VisitResult<Self::Break> {
        walk_path_factor(self, factor)
    }

    fn visit_path_primary(&mut self, primary: $($ref)+ PathPrimary) -> VisitResult<Self::Break> {
        walk_path_primary(self, primary)
    }

    fn visit_element_pattern(&mut self, pattern: $($ref)+ ElementPattern) -> VisitResult<Self::Break> {
        walk_element_pattern(self, pattern)
    }

    fn visit_node_pattern(&mut self, pattern: $($ref)+ NodePattern) -> VisitResult<Self::Break> {
        walk_node_pattern(self, pattern)
    }

    fn visit_edge_pattern(&mut self, pattern: $($ref)+ EdgePattern) -> VisitResult<Self::Break> {
        walk_edge_pattern(self, pattern)
    }

    fn visit_label_expression(&mut self, expression: $($ref)+ LabelExpression) -> VisitResult<Self::Break> {
        walk_label_expression(self, expression)
    }

    fn visit_filter_statement(&mut self, statement: $($ref)+ FilterStatement) -> VisitResult<Self::Break> {
        walk_filter_statement(self, statement)
    }

    fn visit_let_statement(&mut self, statement: $($ref)+ LetStatement) -> VisitResult<Self::Break> {
        walk_let_statement(self, statement)
    }

    fn visit_let_binding(&mut self, binding: $($ref)+ LetVariableDefinition) -> VisitResult<Self::Break> {
        walk_let_binding(self, binding)
    }

    fn visit_for_statement(&mut self, statement: $($ref)+ ForStatement) -> VisitResult<Self::Break> {
        walk_for_statement(self, statement)
    }

    fn visit_select_statement(&mut self, statement: $($ref)+ SelectStatement) -> VisitResult<Self::Break> {
        walk_select_statement(self, statement)
    }

    fn visit_return_statement(&mut self, statement: $($ref)+ ReturnStatement) -> VisitResult<Self::Break> {
        walk_return_statement(self, statement)
    }

    fn visit_return_item(&mut self, item: $($ref)+ ReturnItem) -> VisitResult<Self::Break> {
        walk_return_item(self, item)
    }

    fn visit_expression(&mut self, expression: $($ref)+ Expression) -> VisitResult<Self::Break> {
        walk_expression(self, expression)
    }
}

pub fn walk_program<V: $trait_name + ?Sized>(
    visitor: &mut V,
    program: $($ref)+ Program,
) -> VisitResult<V::Break> {
    for statement in $($ref)+ program.statements {
        try_visit!(visitor.visit_statement(statement));
    }
    ControlFlow::Continue(())
}

/// Walks a statement.
pub fn walk_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ Statement,
) -> VisitResult<V::Break> {
    if let Statement::Query(query_statement) = statement {
        return visitor.visit_query_statement(query_statement);
    }

    ControlFlow::Continue(())
}

/// Walks a query statement.
pub fn walk_query_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ QueryStatement,
) -> VisitResult<V::Break> {
    visitor.visit_query($($ref)+ statement.query)
}

/// Walks a query.
pub fn walk_query<V: $trait_name + ?Sized>(visitor: &mut V, query: $($ref)+ Query) -> VisitResult<V::Break> {
    match query {
        Query::Linear(linear) => visitor.visit_linear_query(linear),
        Query::Composite(composite) => {
            try_visit!(visitor.visit_query($($ref)+ composite.left));
            visitor.visit_query($($ref)+ composite.right)
        }
        Query::Parenthesized(inner, _) => visitor.visit_query(inner),
    }
}

/// Walks a linear query.
pub fn walk_linear_query<V: $trait_name + ?Sized>(
    visitor: &mut V,
    query: $($ref)+ LinearQuery,
) -> VisitResult<V::Break> {
    if let Some(use_graph) = $($ref)+ query.use_graph {
        try_visit!(visitor.visit_expression($($ref)+ use_graph.graph));
    }
    for statement in $($ref)+ query.primitive_statements {
        try_visit!(visitor.visit_primitive_query_statement(statement));
    }
    if let Some(result) = $($ref)+ query.result_statement {
        try_visit!(visitor.visit_primitive_result_statement(result));
    }

    ControlFlow::Continue(())
}

/// Walks a primitive query statement.
pub fn walk_primitive_query_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ PrimitiveQueryStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveQueryStatement::Match(match_statement) => {
            visitor.visit_match_statement(match_statement)
        }
        PrimitiveQueryStatement::Call(call) => walk_call_procedure_statement(visitor, call),
        PrimitiveQueryStatement::Filter(filter) => visitor.visit_filter_statement(filter),
        PrimitiveQueryStatement::Let(let_statement) => visitor.visit_let_statement(let_statement),
        PrimitiveQueryStatement::For(for_statement) => visitor.visit_for_statement(for_statement),
        PrimitiveQueryStatement::OrderByAndPage(order_by_and_page) => {
            if let Some(order_by) = $($ref)+ order_by_and_page.order_by {
                for sort in $($ref)+ order_by.sort_specifications {
                    try_visit!(visitor.visit_expression($($ref)+ sort.key));
                }
            }
            if let Some(offset) = $($ref)+ order_by_and_page.offset {
                try_visit!(visitor.visit_expression($($ref)+ offset.count));
            }
            if let Some(limit) = $($ref)+ order_by_and_page.limit {
                try_visit!(visitor.visit_expression($($ref)+ limit.count));
            }
            ControlFlow::Continue(())
        }
        PrimitiveQueryStatement::Select(select) => visitor.visit_select_statement(select),
    }
}

/// Walks a primitive result statement.
pub fn walk_primitive_result_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ PrimitiveResultStatement,
) -> VisitResult<V::Break> {
    match statement {
        PrimitiveResultStatement::Return(return_statement) => {
            visitor.visit_return_statement(return_statement)
        }
        PrimitiveResultStatement::Finish(_) => ControlFlow::Continue(()),
    }
}

/// Walks a match statement.
pub fn walk_match_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ MatchStatement,
) -> VisitResult<V::Break> {
    match statement {
        MatchStatement::Simple(simple) => visitor.visit_graph_pattern($($ref)+ simple.pattern),
        MatchStatement::Optional(optional) => match $($ref)+ optional.operand {
            crate::ast::query::OptionalOperand::Match { pattern } => {
                visitor.visit_graph_pattern(pattern)
            }
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

/// Walks a graph pattern.
pub fn walk_graph_pattern<V: $trait_name + ?Sized>(
    visitor: &mut V,
    pattern: $($ref)+ GraphPattern,
) -> VisitResult<V::Break> {
    for path_pattern in $($ref)+ pattern.paths.patterns {
        try_visit!(visitor.visit_path_pattern(path_pattern));
    }

    if let Some(where_clause) = $($ref)+ pattern.where_clause {
        try_visit!(visitor.visit_expression($($ref)+ where_clause.condition));
    }

    if let Some(yield_clause) = $($ref)+ pattern.yield_clause {
        for item in $($ref)+ yield_clause.items {
            try_visit!(visitor.visit_expression($($ref)+ item.expression));
        }
    }

    ControlFlow::Continue(())
}

/// Walks a path pattern.
pub fn walk_path_pattern<V: $trait_name + ?Sized>(
    visitor: &mut V,
    pattern: $($ref)+ PathPattern,
) -> VisitResult<V::Break> {
    visitor.visit_path_pattern_expression($($ref)+ pattern.expression)
}

/// Walks a path pattern expression.
pub fn walk_path_pattern_expression<V: $trait_name + ?Sized>(
    visitor: &mut V,
    expression: $($ref)+ PathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        PathPatternExpression::Union { left, right, .. } => {
            try_visit!(visitor.visit_path_pattern_expression(left));
            visitor.visit_path_pattern_expression(right)
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for alternative in alternatives {
                try_visit!(visitor.visit_path_pattern_expression(alternative));
            }
            ControlFlow::Continue(())
        }
        PathPatternExpression::Term(term) => {
            for factor in $($ref)+ term.factors {
                try_visit!(visitor.visit_path_factor(factor));
            }
            ControlFlow::Continue(())
        }
    }
}

/// Walks a path factor.
pub fn walk_path_factor<V: $trait_name + ?Sized>(
    visitor: &mut V,
    factor: $($ref)+ PathFactor,
) -> VisitResult<V::Break> {
    visitor.visit_path_primary($($ref)+ factor.primary)
}

/// Walks a path primary.
pub fn walk_path_primary<V: $trait_name + ?Sized>(
    visitor: &mut V,
    primary: $($ref)+ PathPrimary,
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

/// Walks an element pattern.
pub fn walk_element_pattern<V: $trait_name + ?Sized>(
    visitor: &mut V,
    pattern: $($ref)+ ElementPattern,
) -> VisitResult<V::Break> {
    match pattern {
        ElementPattern::Node(node_pattern) => visitor.visit_node_pattern(node_pattern),
        ElementPattern::Edge(edge_pattern) => visitor.visit_edge_pattern(edge_pattern),
    }
}

/// Walks a node pattern.
pub fn walk_node_pattern<V: $trait_name + ?Sized>(
    visitor: &mut V,
    pattern: $($ref)+ NodePattern,
) -> VisitResult<V::Break> {
    if let Some(label_expression) = $($ref)+ pattern.label_expression {
        try_visit!(visitor.visit_label_expression(label_expression));
    }
    if let Some(properties) = $($ref)+ pattern.properties {
        for property in $($ref)+ properties.properties {
            try_visit!(visitor.visit_expression($($ref)+ property.value));
        }
    }
    if let Some(where_clause) = $($ref)+ pattern.where_clause {
        try_visit!(visitor.visit_expression($($ref)+ where_clause.condition));
    }

    ControlFlow::Continue(())
}

/// Walks an edge pattern.
pub fn walk_edge_pattern<V: $trait_name + ?Sized>(
    visitor: &mut V,
    pattern: $($ref)+ EdgePattern,
) -> VisitResult<V::Break> {
    if let EdgePattern::Full(full) = pattern {
        if let Some(label_expression) = $($ref)+ full.filler.label_expression {
            try_visit!(visitor.visit_label_expression(label_expression));
        }
        if let Some(properties) = $($ref)+ full.filler.properties {
            for property in $($ref)+ properties.properties {
                try_visit!(visitor.visit_expression($($ref)+ property.value));
            }
        }
        if let Some(where_clause) = $($ref)+ full.filler.where_clause {
            try_visit!(visitor.visit_expression($($ref)+ where_clause.condition));
        }
    }

    ControlFlow::Continue(())
}

/// Walks a label expression.
pub fn walk_label_expression<V: $trait_name + ?Sized>(
    visitor: &mut V,
    expression: $($ref)+ LabelExpression,
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

/// Walks a filter statement.
pub fn walk_filter_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ FilterStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression($($ref)+ statement.condition)
}

/// Walks a let statement.
pub fn walk_let_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ LetStatement,
) -> VisitResult<V::Break> {
    for binding in $($ref)+ statement.bindings {
        try_visit!(visitor.visit_let_binding(binding));
    }

    ControlFlow::Continue(())
}

/// Walks a let binding.
pub fn walk_let_binding<V: $trait_name + ?Sized>(
    visitor: &mut V,
    binding: $($ref)+ LetVariableDefinition,
) -> VisitResult<V::Break> {
    visitor.visit_expression($($ref)+ binding.value)
}

/// Walks a FOR statement.
pub fn walk_for_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ ForStatement,
) -> VisitResult<V::Break> {
    visitor.visit_expression($($ref)+ statement.item.collection)
}

/// Walks a SELECT statement.
pub fn walk_select_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ SelectStatement,
) -> VisitResult<V::Break> {
    if let Some(with_clause) = $($ref)+ statement.with_clause {
        for cte in $($ref)+ with_clause.items {
            try_visit!(visitor.visit_query($($ref)+ cte.query));
        }
    }

    match $($ref)+ statement.select_items {
        SelectItemList::Star => {}
        SelectItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_expression($($ref)+ item.expression));
            }
        }
    }

    if let Some(from_clause) = $($ref)+ statement.from_clause {
        match from_clause {
            SelectFromClause::GraphMatchList { matches } => {
                for graph_pattern in matches {
                    try_visit!(visitor.visit_graph_pattern(graph_pattern));
                }
            }
            SelectFromClause::QuerySpecification { query, .. } => {
                try_visit!(visitor.visit_query(query));
            }
            SelectFromClause::GraphAndQuerySpecification { graph, query, .. } => {
                try_visit!(visitor.visit_expression(graph));
                try_visit!(visitor.visit_query(query));
            }
            SelectFromClause::SourceList { sources } => {
                for source in sources {
                    match source {
                        SelectSourceItem::Query { query, .. } => {
                            try_visit!(visitor.visit_query(query));
                        }
                        SelectSourceItem::GraphAndQuery { graph, query, .. } => {
                            try_visit!(visitor.visit_expression(graph));
                            try_visit!(visitor.visit_query(query));
                        }
                        SelectSourceItem::Expression { expression, .. } => {
                            try_visit!(visitor.visit_expression(expression));
                        }
                    }
                }
            }
        }
    }

    if let Some(where_clause) = $($ref)+ statement.where_clause {
        try_visit!(visitor.visit_expression($($ref)+ where_clause.condition));
    }

    if let Some(group_by) = $($ref)+ statement.group_by {
        for element in $($ref)+ group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression(expression));
            }
        }
    }

    if let Some(having) = $($ref)+ statement.having {
        try_visit!(visitor.visit_expression($($ref)+ having.condition));
    }

    if let Some(order_by) = $($ref)+ statement.order_by {
        for sort in $($ref)+ order_by.sort_specifications {
            try_visit!(visitor.visit_expression($($ref)+ sort.key));
        }
    }

    if let Some(offset) = $($ref)+ statement.offset {
        try_visit!(visitor.visit_expression($($ref)+ offset.count));
    }

    if let Some(limit) = $($ref)+ statement.limit {
        try_visit!(visitor.visit_expression($($ref)+ limit.count));
    }

    ControlFlow::Continue(())
}

/// Walks a RETURN statement.
pub fn walk_return_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    statement: $($ref)+ ReturnStatement,
) -> VisitResult<V::Break> {
    match $($ref)+ statement.items {
        ReturnItemList::Star => {}
        ReturnItemList::Items { items } => {
            for item in items {
                try_visit!(visitor.visit_return_item(item));
            }
        }
    }

    if let Some(group_by) = $($ref)+ statement.group_by {
        for element in $($ref)+ group_by.elements {
            if let GroupingElement::Expression(expression) = element {
                try_visit!(visitor.visit_expression(expression));
            }
        }
    }

    ControlFlow::Continue(())
}

/// Walks a RETURN item.
pub fn walk_return_item<V: $trait_name + ?Sized>(
    visitor: &mut V,
    item: $($ref)+ ReturnItem,
) -> VisitResult<V::Break> {
    visitor.visit_expression($($ref)+ item.expression)
}

/// Walks an expression.
pub fn walk_expression<V: $trait_name + ?Sized>(
    visitor: &mut V,
    expression: $($ref)+ Expression,
) -> VisitResult<V::Break> {
    match expression {
        Expression::Literal(literal, _) => walk_literal(visitor, literal),
        Expression::Unary(_, inner, _)
        | Expression::Parenthesized(inner, _)
        | Expression::GraphExpression(inner, _)
        | Expression::BindingTableExpression(inner, _) => visitor.visit_expression(inner),
        Expression::SubqueryExpression(_, _) => ControlFlow::Continue(()),
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
            for argument in $($ref)+ function_call.arguments {
                try_visit!(visitor.visit_expression(argument));
            }
            ControlFlow::Continue(())
        }
        Expression::Case(case_expression) => match case_expression {
            CaseExpression::Simple(simple) => {
                try_visit!(visitor.visit_expression($($ref)+ simple.operand));
                for when_clause in $($ref)+ simple.when_clauses {
                    try_visit!(visitor.visit_expression($($ref)+ when_clause.when_value));
                    try_visit!(visitor.visit_expression($($ref)+ when_clause.then_result));
                }
                if let Some(else_clause) = $($ref)+ simple.else_clause {
                    try_visit!(visitor.visit_expression(else_clause));
                }
                ControlFlow::Continue(())
            }
            CaseExpression::Searched(searched) => {
                for when_clause in $($ref)+ searched.when_clauses {
                    try_visit!(visitor.visit_expression($($ref)+ when_clause.condition));
                    try_visit!(visitor.visit_expression($($ref)+ when_clause.then_result));
                }
                if let Some(else_clause) = $($ref)+ searched.else_clause {
                    try_visit!(visitor.visit_expression(else_clause));
                }
                ControlFlow::Continue(())
            }
        },
        Expression::Cast(cast) => visitor.visit_expression($($ref)+ cast.operand),
        Expression::AggregateFunction(aggregate_function) => match aggregate_function.$agg_access() {
            crate::ast::expression::AggregateFunction::CountStar { .. } => {
                ControlFlow::Continue(())
            }
            crate::ast::expression::AggregateFunction::GeneralSetFunction(function) => {
                visitor.visit_expression($($ref)+ function.expression)
            }
            crate::ast::expression::AggregateFunction::BinarySetFunction(function) => {
                try_visit!(visitor.visit_expression($($ref)+ function.inverse_distribution_argument));
                visitor.visit_expression($($ref)+ function.expression)
            }
        },
        Expression::TypeAnnotation(inner, _, _) => visitor.visit_expression(inner),
        Expression::ListConstructor(expressions, _)
        | Expression::PathConstructor(expressions, _) => {
            for item in expressions {
                try_visit!(visitor.visit_expression(item));
            }
            ControlFlow::Continue(())
        }
        Expression::RecordConstructor(fields, _) => {
            for field in fields {
                try_visit!(visitor.visit_expression($($ref)+ field.value));
            }
            ControlFlow::Continue(())
        }
        Expression::Exists(exists_expression) => match $($ref)+ exists_expression.variant {
            ExistsVariant::GraphPattern(_) => ControlFlow::Continue(()),
            ExistsVariant::Subquery(subquery) => visitor.visit_expression(subquery),
        },
        Expression::Predicate(predicate) => walk_predicate(visitor, predicate),
    }
}

fn walk_predicate<V: $trait_name + ?Sized>(
    visitor: &mut V,
    predicate: $($ref)+ Predicate,
) -> VisitResult<V::Break> {
    match predicate {
        Predicate::IsNull(expression, _, _)
        | Predicate::IsTyped(expression, _, _, _)
        | Predicate::IsNormalized(expression, _, _)
        | Predicate::IsDirected(expression, _, _)
        | Predicate::IsTruthValue(expression, _, _, _)
        | Predicate::PropertyExists(expression, _, _) => visitor.visit_expression(expression),
        Predicate::IsLabeled(expression, _, _, _) => visitor.visit_expression(expression),
        Predicate::IsSource(source, target, _, _)
        | Predicate::IsDestination(source, target, _, _) => {
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

fn walk_literal<V: $trait_name + ?Sized>(
    visitor: &mut V,
    literal: $($ref)+ Literal,
) -> VisitResult<V::Break> {
    match literal {
        Literal::List(expressions) => {
            for expression in expressions {
                try_visit!(visitor.visit_expression(expression));
            }
            ControlFlow::Continue(())
        }
        Literal::Record(fields) => {
            for field in fields {
                try_visit!(visitor.visit_expression($($ref)+ field.value));
            }
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

fn walk_call_procedure_statement<V: $trait_name + ?Sized>(
    visitor: &mut V,
    call: $($ref)+ CallProcedureStatement,
) -> VisitResult<V::Break> {
    match $($ref)+ call.call {
        ProcedureCall::Inline(_) => ControlFlow::Continue(()),
        ProcedureCall::Named(named) => {
            if let Some(arguments) = $($ref)+ named.arguments {
                for argument in $($ref)+ arguments.arguments {
                    try_visit!(visitor.visit_expression($($ref)+ argument.expression));
                }
            }
            if let Some(yield_clause) = $($ref)+ named.yield_clause {
                for item in $($ref)+ yield_clause.items.items {
                    try_visit!(visitor.visit_expression($($ref)+ item.expression));
                }
            }
            ControlFlow::Continue(())
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn walk_simplified_expression<V: $trait_name + ?Sized>(
    visitor: &mut V,
    expression: $($ref)+ SimplifiedPathPatternExpression,
) -> VisitResult<V::Break> {
    match expression {
        SimplifiedPathPatternExpression::Contents(_) => ControlFlow::Continue(()),
        SimplifiedPathPatternExpression::Union(union) => {
            try_visit!(walk_simplified_expression(visitor, $($ref)+ union.left));
            walk_simplified_expression(visitor, $($ref)+ union.right)
        }
        SimplifiedPathPatternExpression::MultisetAlternation(alternation) => {
            for alternative in $($ref)+ alternation.alternatives {
                try_visit!(walk_simplified_expression(visitor, alternative));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Conjunction(conjunction) => {
            try_visit!(walk_simplified_expression(visitor, $($ref)+ conjunction.left));
            walk_simplified_expression(visitor, $($ref)+ conjunction.right)
        }
        SimplifiedPathPatternExpression::Concatenation(concatenation) => {
            for part in $($ref)+ concatenation.parts {
                try_visit!(walk_simplified_expression(visitor, part));
            }
            ControlFlow::Continue(())
        }
        SimplifiedPathPatternExpression::Quantified(quantified) => {
            walk_simplified_expression(visitor, $($ref)+ quantified.pattern)
        }
        SimplifiedPathPatternExpression::Questioned(questioned) => {
            walk_simplified_expression(visitor, $($ref)+ questioned.pattern)
        }
        SimplifiedPathPatternExpression::DirectionOverride(direction_override) => {
            walk_simplified_expression(visitor, $($ref)+ direction_override.pattern)
        }
        SimplifiedPathPatternExpression::Negation(negation) => {
            walk_simplified_expression(visitor, $($ref)+ negation.pattern)
        }
    }
}

    };
}

pub(crate) use define_visit_api;
