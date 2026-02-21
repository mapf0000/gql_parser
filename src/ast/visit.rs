//! Immutable AST visitor infrastructure.

use std::ops::ControlFlow;

use crate::ast::Expression;
use crate::ast::expression::{CaseExpression, ExistsVariant, Literal, Predicate};
use crate::ast::procedure::{CallProcedureStatement, ProcedureCall};
use crate::ast::program::{Program, QueryStatement, Statement};
use crate::ast::query::{
    EdgePattern, ElementPattern, FilterStatement, ForStatement, GraphPattern, GroupingElement,
    LabelExpression, LetStatement, LetVariableDefinition, LinearQuery, MatchStatement, NodePattern,
    PathFactor, PathPattern, PathPatternExpression, PathPrimary, PrimitiveQueryStatement,
    PrimitiveResultStatement, Query, ReturnItem, ReturnItemList, ReturnStatement, SelectFromClause,
    SelectItemList, SelectSourceItem, SelectStatement, SimplifiedPathPatternExpression,
};

use super::visit_macros::define_visit_api;

/// Shared type alias for visitor traversal methods.
pub type VisitResult<B> = ControlFlow<B>;

define_visit_api!(Visit, [&], as_ref);

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::Visit;
    use crate::parse;

    #[derive(Default)]
    struct ExprCounter {
        count: usize,
    }

    impl Visit for ExprCounter {
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
        assert!(
            visitor.count >= 6,
            "expected multiple expressions to be visited"
        );
    }
}
