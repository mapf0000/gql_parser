//! Query analysis API usage example.

use gql_parser::ast::Statement;
use gql_parser::{PatternInfo, QueryInfo, VariableDependencyGraph, parse};

fn main() {
    let source = "MATCH (a:Person)-[:KNOWS]->(b:Person) LET score = a.rank + b.rank RETURN b.name AS friend, score";
    let parse_result = parse(source);

    let Some(program) = parse_result.ast else {
        eprintln!("failed to parse input");
        for diagnostic in parse_result.diagnostics {
            eprintln!("{diagnostic}");
        }
        return;
    };

    let Some(statement) = program.statements.first() else {
        eprintln!("program has no statements");
        return;
    };

    let query_info = QueryInfo::from_ast(statement);
    println!("clause count: {}", query_info.clause_sequence.len());
    println!("graph pattern count: {}", query_info.graph_pattern_count);
    println!("contains aggregation: {}", query_info.contains_aggregation);

    for clause in &query_info.clause_sequence {
        println!(
            "pipeline={} pos={} kind={:?} defs={:?} uses={:?}",
            clause.clause_id.pipeline_id,
            clause.clause_id.position,
            clause.kind,
            clause.definitions,
            clause.uses,
        );
    }

    let dependency_graph = VariableDependencyGraph::build(statement);
    println!("definition points: {}", dependency_graph.definition_points.len());
    println!("usage points: {}", dependency_graph.usage_points.len());
    println!("define/use edges: {}", dependency_graph.edges.len());

    if let Statement::Query(query_statement) = statement
        && let gql_parser::ast::Query::Linear(linear_query) = &query_statement.query
        && linear_query.use_graph.is_none()
        && let Some(gql_parser::ast::PrimitiveQueryStatement::Match(match_statement)) =
            linear_query.primitive_statements.first()
        && let gql_parser::ast::MatchStatement::Simple(simple) = match_statement
    {
        let pattern_info = PatternInfo::analyze(&simple.pattern);
        println!("pattern nodes: {}", pattern_info.node_count);
        println!("pattern edges: {}", pattern_info.edge_count);
        println!("pattern connected: {}", pattern_info.is_fully_connected);
    }
}
