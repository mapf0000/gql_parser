# gql_parser

Pure-Rust ISO GQL parser with:

- span-aware diagnostics (`miette`)
- typed AST
- zero-copy AST visitors
- compiler-facing query analysis metadata

This crate is parser/analysis-only. It does not execute queries.

## Install

```toml
[dependencies]
gql_parser = "0.1"
```

## Parse + Diagnostics

```rust
use gql_parser::parse;

let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n.name";
let result = parse(source);

assert!(result.ast.is_some());
for diag in &result.diagnostics {
    eprintln!("{diag}");
}
```

## AST Traversal

```rust
use gql_parser::ast::{AstVisitor, VariableCollector};
use gql_parser::parse;

let program = parse("MATCH (n)-[:KNOWS]->(m) RETURN m").ast.unwrap();
let mut collector = VariableCollector::new();
let _ = collector.visit_program(&program);

println!("definitions: {:?}", collector.definitions());
println!("references: {:?}", collector.references());
```

## Query Analysis

```rust
use gql_parser::{QueryInfo, VariableDependencyGraph, parse};

let statement = &parse("MATCH (n) LET x = n.age RETURN x")
    .ast
    .unwrap()
    .statements[0];

let query_info = QueryInfo::from_ast(statement);
let deps = VariableDependencyGraph::build(statement);

assert_eq!(query_info.clause_sequence.len(), 3);
assert!(!deps.edges.is_empty());
```

## Public Analysis APIs

- `ExpressionInfo::analyze(&Expression)`
- `PatternInfo::analyze(&GraphPattern)`
- `QueryInfo::from_ast(&Statement)`
- `VariableDependencyGraph::build(&Statement)`

## Examples

- `cargo run --example parser_demo`
- `cargo run --example visitor_usage`
- `cargo run --example query_analysis_usage`
- `cargo run --example semantic_validation_demo`

## Docs

- `docs/QUICK_START.md`
- `docs/USER_GUIDE.md`
- `docs/AST_GUIDE.md`
- `docs/BENCHMARK_BASELINE.md`

## License

Apache License, Version 2.0 (`LICENSE-APACHE`).
