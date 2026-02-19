# Quick Start

## 1. Add the crate

```toml
[dependencies]
gql_parser = "0.1"
```

## 2. Parse a query

```rust
use gql_parser::parse;

let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n.name";
let result = parse(source);

assert!(result.ast.is_some());
assert!(result.diagnostics.is_empty());
```

## 3. Traverse the AST

```rust
use gql_parser::{AstVisitor, VariableCollector, parse};

let program = parse("MATCH (n)-[:KNOWS]->(m) RETURN m").ast.unwrap();
let mut collector = VariableCollector::new();
let _ = collector.visit_program(&program);

assert!(collector.definitions().contains("n"));
assert!(collector.definitions().contains("m"));
```

## 4. Run compiler-facing analysis

```rust
use gql_parser::{QueryInfo, VariableDependencyGraph, parse};

let statement = &parse("MATCH (n) LET x = n.age RETURN x")
    .ast
    .unwrap()
    .statements[0];

let info = QueryInfo::from_ast(statement);
let deps = VariableDependencyGraph::build(statement);

assert_eq!(info.clause_sequence.len(), 3);
assert!(!deps.edges.is_empty());
```

## Examples

- `cargo run --example parser_demo`
- `cargo run --example visitor_usage`
- `cargo run --example query_analysis_usage`
