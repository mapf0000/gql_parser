//! Mutable AST visitor infrastructure.

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

use super::visit::VisitResult;
use super::visit_macros::define_visit_api;

define_visit_api!(VisitMut, [&mut], as_mut);

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::VisitMut;
    use crate::parse;

    #[derive(Default)]
    struct UppercaseMutator;

    impl VisitMut for UppercaseMutator {
        type Break = ();

        fn visit_expression(
            &mut self,
            expression: &mut crate::ast::Expression,
        ) -> ControlFlow<Self::Break> {
            if let crate::ast::Expression::VariableReference(name, _) = expression {
                *name = name.to_uppercase().into();
            }
            super::walk_expression(self, expression)
        }
    }

    #[test]
    fn mutable_visitor_can_transform_variable_references() {
        let parse_result = parse("MATCH (n) RETURN n");
        let mut program = parse_result.ast.expect("expected AST");

        let mut visitor = UppercaseMutator;
        let flow = visitor.visit_program(&mut program);

        assert!(matches!(flow, ControlFlow::Continue(())));

        let output = format!("{program:?}");
        assert!(output.contains("\"N\""));
    }
}
