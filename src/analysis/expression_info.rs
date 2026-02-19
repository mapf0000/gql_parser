//! Expression metadata extraction.

use std::collections::BTreeSet;
use std::ops::ControlFlow;

use smol_str::SmolStr;

use crate::ast::expression::{AggregateFunction, BooleanValue, FunctionName, Literal};
use crate::ast::visitor::{AstVisitor, walk_expression};
use crate::ast::{Expression, Span};

/// A property reference extracted from an expression tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyReference {
    /// Optional variable owning the property (if the target is a variable reference).
    pub variable: Option<SmolStr>,
    /// Property key.
    pub property: SmolStr,
    /// Span of the full property reference.
    pub span: Span,
}

/// A literal encountered in an expression tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralInfo {
    Null,
    Boolean(BooleanValue),
    Integer(SmolStr),
    Float(SmolStr),
    String(SmolStr),
    ByteString(SmolStr),
    Date(SmolStr),
    Time(SmolStr),
    Datetime(SmolStr),
    Duration(SmolStr),
    List,
    Record,
}

/// Query-planning-oriented metadata for a single expression.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpressionInfo {
    /// Variable names referenced by this expression.
    pub variable_references: BTreeSet<SmolStr>,
    /// Property references discovered in the tree.
    pub property_references: Vec<PropertyReference>,
    /// Function names encountered (including aggregates).
    pub function_calls: Vec<SmolStr>,
    /// Literal values encountered.
    pub literals: Vec<LiteralInfo>,
    /// Whether the expression contains an aggregate function.
    pub contains_aggregate: bool,
}

impl ExpressionInfo {
    /// Extracts expression metadata.
    pub fn analyze(expression: &Expression) -> Self {
        let mut collector = ExpressionInfoCollector::default();
        let _ = collector.visit_expression(expression);
        collector.into_info()
    }
}

#[derive(Debug, Default)]
struct ExpressionInfoCollector {
    info: ExpressionInfo,
}

impl ExpressionInfoCollector {
    fn into_info(self) -> ExpressionInfo {
        self.info
    }

    fn push_literal(&mut self, literal: &Literal) {
        let info = match literal {
            Literal::Boolean(value) => LiteralInfo::Boolean(*value),
            Literal::Null => LiteralInfo::Null,
            Literal::Integer(value) => LiteralInfo::Integer(value.clone()),
            Literal::Float(value) => LiteralInfo::Float(value.clone()),
            Literal::String(value) => LiteralInfo::String(value.clone()),
            Literal::ByteString(value) => LiteralInfo::ByteString(value.clone()),
            Literal::Date(value) => LiteralInfo::Date(value.clone()),
            Literal::Time(value) => LiteralInfo::Time(value.clone()),
            Literal::Datetime(value) => LiteralInfo::Datetime(value.clone()),
            Literal::Duration(value) => LiteralInfo::Duration(value.clone()),
            Literal::List(_) => LiteralInfo::List,
            Literal::Record(_) => LiteralInfo::Record,
        };

        self.info.literals.push(info);
    }

    fn push_function_name(&mut self, name: SmolStr) {
        self.info.function_calls.push(name);
    }
}

impl AstVisitor for ExpressionInfoCollector {
    type Break = ();

    fn visit_expression(&mut self, expression: &Expression) -> ControlFlow<Self::Break> {
        match expression {
            Expression::VariableReference(name, _) => {
                self.info.variable_references.insert(name.clone());
            }
            Expression::PropertyReference(target, key, span) => {
                let variable = match target.as_ref() {
                    Expression::VariableReference(name, _) => Some(name.clone()),
                    _ => None,
                };
                self.info.property_references.push(PropertyReference {
                    variable,
                    property: key.clone(),
                    span: span.clone(),
                });
            }
            Expression::FunctionCall(function_call) => {
                self.push_function_name(function_name_to_smol(&function_call.name));
            }
            Expression::AggregateFunction(function) => {
                self.info.contains_aggregate = true;
                self.push_function_name(aggregate_function_name(function.as_ref()));
            }
            Expression::Literal(literal, _) => {
                self.push_literal(literal);
            }
            _ => {}
        }

        walk_expression(self, expression)
    }
}

fn function_name_to_smol(name: &FunctionName) -> SmolStr {
    match name {
        FunctionName::Custom(name) => name.clone(),
        other => SmolStr::new(other.to_string()),
    }
}

fn aggregate_function_name(function: &AggregateFunction) -> SmolStr {
    match function {
        AggregateFunction::CountStar { .. } => SmolStr::new("COUNT"),
        AggregateFunction::GeneralSetFunction(function) => {
            SmolStr::new(function.function_type.to_string())
        }
        AggregateFunction::BinarySetFunction(function) => {
            SmolStr::new(function.function_type.to_string())
        }
    }
}

impl std::fmt::Display for FunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionName::Custom(name) => write!(f, "{name}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl std::fmt::Display for crate::ast::expression::GeneralSetFunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            crate::ast::expression::GeneralSetFunctionType::Avg => "AVG",
            crate::ast::expression::GeneralSetFunctionType::Count => "COUNT",
            crate::ast::expression::GeneralSetFunctionType::Max => "MAX",
            crate::ast::expression::GeneralSetFunctionType::Min => "MIN",
            crate::ast::expression::GeneralSetFunctionType::Sum => "SUM",
            crate::ast::expression::GeneralSetFunctionType::CollectList => "COLLECT_LIST",
            crate::ast::expression::GeneralSetFunctionType::StddevSamp => "STDDEV_SAMP",
            crate::ast::expression::GeneralSetFunctionType::StddevPop => "STDDEV_POP",
        };
        write!(f, "{name}")
    }
}

impl std::fmt::Display for crate::ast::expression::BinarySetFunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            crate::ast::expression::BinarySetFunctionType::PercentileCont => "PERCENTILE_CONT",
            crate::ast::expression::BinarySetFunctionType::PercentileDisc => "PERCENTILE_DISC",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::expression_info::ExpressionInfo;
    use crate::parse;

    #[test]
    fn expression_info_collects_variables_properties_and_functions() {
        let parse_result = parse("RETURN SUM(n.score) + helper(n.age), n.name");
        let program = parse_result.ast.expect("expected AST");

        let expression = match &program.statements[0] {
            crate::ast::Statement::Query(statement) => {
                let crate::ast::query::Query::Linear(crate::ast::query::LinearQuery::Ambient(
                    ambient,
                )) = &statement.query
                else {
                    panic!("expected ambient query");
                };

                let crate::ast::query::PrimitiveResultStatement::Return(return_statement) =
                    ambient.result_statement.as_ref().expect("expected return statement").as_ref()
                else {
                    panic!("expected RETURN");
                };

                let crate::ast::query::ReturnItemList::Items { items } = &return_statement.items
                else {
                    panic!("expected RETURN items");
                };

                &items[0].expression
            }
            _ => panic!("expected query statement"),
        };

        let info = ExpressionInfo::analyze(expression);

        assert!(info.contains_aggregate);
        assert!(info.variable_references.contains("n"));
        assert!(info.function_calls.iter().any(|name| name == "SUM"));
        assert!(
            info.property_references
                .iter()
                .any(|reference| reference.property == "score")
        );
    }
}
