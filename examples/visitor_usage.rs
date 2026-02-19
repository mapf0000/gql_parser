//! Visitor usage example.

use gql_parser::ast::{AstNode, AstVisitor, CollectingVisitor, SpanCollector, VariableCollector};
use gql_parser::parse;

fn main() {
    let source = "MATCH (n:Person)-[:KNOWS]->(m:Person) WHERE n.age > 21 RETURN m.name";
    let parse_result = parse(source);

    let Some(program) = parse_result.ast else {
        eprintln!("failed to parse input");
        for diagnostic in parse_result.diagnostics {
            eprintln!("{diagnostic}");
        }
        return;
    };

    let spans = SpanCollector::collect_program(&program);
    println!("collected {} spans", spans.len());

    let mut variable_collector = VariableCollector::new();
    let _ = variable_collector.visit_program(&program);
    println!("definitions: {:?}", variable_collector.definitions());
    println!("references: {:?}", variable_collector.references());

    let mut property_collector = CollectingVisitor::new(|node| match node {
        AstNode::Expression(gql_parser::ast::Expression::PropertyReference(_, property, _)) => {
            Some(property.clone())
        }
        _ => None,
    });

    let _ = property_collector.visit_program(&program);
    println!("property references: {:?}", property_collector.items());
}
