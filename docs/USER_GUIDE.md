# User Guide

## Overview

`gql_parser` is a parser-only Rust crate for ISO GQL. It provides:

- lexing and parsing
- rich span-aware diagnostics via `miette`
- a typed AST
- AST visitors
- query analysis metadata for compiler/lowering pipelines

## Parsing APIs

### High-level API

Use `parse(&str)` for standard usage:

```rust
use gql_parser::parse;

let result = parse("MATCH (n:Person) RETURN n.name");
if let Some(program) = result.ast {
    println!("parsed {} statement(s)", program.statements.len());
}
for diag in result.diagnostics {
    eprintln!("{diag}");
}
```

### Parse + semantic validation

Use `parse_and_validate(&str)` if you also want semantic diagnostics and validated IR:

```rust
use gql_parser::parse_and_validate;

let result = parse_and_validate("MATCH (n:Person) RETURN n");
if result.ir.is_none() {
    for diag in result.diagnostics {
        eprintln!("{diag}");
    }
}
```

## Visitor APIs

Use visitor APIs when you need custom AST traversal without cloning:

- `AstVisitor`: immutable traversal
- `AstVisitorMut`: mutable traversal
- `CollectingVisitor`: generic collector
- `SpanCollector`: collects visited spans
- `VariableCollector`: collects definitions and references

Example:

```rust
use gql_parser::{AstVisitor, VariableCollector, parse};

let program = parse("MATCH (n)-[:KNOWS]->(m) RETURN m").ast.unwrap();
let mut collector = VariableCollector::new();
let _ = collector.visit_program(&program);

println!("defs: {:?}", collector.definitions());
println!("uses: {:?}", collector.references());
```

## Analysis APIs

Use analysis APIs for lowering/planning inputs:

- `ExpressionInfo::analyze(&Expression)`
- `PatternInfo::analyze(&GraphPattern)`
- `QueryInfo::from_ast(&Statement)`
- `VariableDependencyGraph::build(&Statement)`

These are deterministic, read-only views over parser AST structures.

## Diagnostics

Diagnostics are emitted as `miette::Report` values from public APIs.
The parser never panics on malformed input; it returns diagnostics and partial ASTs when possible.

## Performance

Benchmark suite is included via Criterion:

- `cargo bench`
- `cargo bench simple_queries`
- `cargo bench parse_and_validate`

See `benches/README.md` and `docs/BENCHMARK_BASELINE.md`.
