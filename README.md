# gql_parser

A pure-Rust ISO GQL (Graph Query Language) parser providing span-aware diagnostics, a typed AST, and semantic validation for building graph query engines.

## Quick Start

```toml
[dependencies]
gql_parser = "0.1"
```

### Simple Query Example

```rust
use gql_parser::parse;

let source = "MATCH (person:Person)-[:KNOWS]->(friend) WHERE person.age > 18 RETURN friend.name";
let result = parse(source);

if let Some(program) = result.ast {
    println!("Parsed {} statement(s)", program.statements.len());
}
for diagnostic in result.diagnostics {
    eprintln!("{diagnostic}");
}
```

### Schema Definition Example

```rust
use gql_parser::parse;

let schema = r#"
    CREATE GRAPH TYPE SocialNetwork AS {
        NODE TYPE Person {
            id :: INT,
            name :: STRING,
            age :: INT
        }
        CONSTRAINT UNIQUE (id),
        DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person) {
            since :: DATE
        }
    }
"#;

let result = parse(schema);
assert!(result.ast.is_some());
assert!(result.diagnostics.is_empty());
```

## Features

- **ISO GQL Compliant** - Implements ISO/IEC 39075 (GQL) standard
- **Rich Diagnostics** - Span-aware error messages via `miette`
- **Typed AST** - Strongly-typed abstract syntax tree
- **Zero-Copy Visitors** - Efficient AST traversal without cloning
- **Semantic Validation** - Optional validation with schema catalog integration
- **Query Analysis** - Compiler-facing metadata extraction

## Core APIs

### Parsing

```rust
use gql_parser::parse;

// Basic parsing
let result = parse("MATCH (n) RETURN n");

// With semantic validation
use gql_parser::parse_and_validate;
let result = parse_and_validate("MATCH (n:Person) RETURN n");
```

### AST Traversal

```rust
use gql_parser::ast::{VariableCollector, Visit};
use gql_parser::parse;

let program = parse("MATCH (n)-[:KNOWS]->(m) RETURN m").ast.unwrap();
let mut collector = VariableCollector::new();
let _ = collector.visit_program(&program);

println!("Variables defined: {:?}", collector.definitions());
println!("Variables used: {:?}", collector.references());
```

### Query Analysis

```rust
use gql_parser::{QueryInfo, VariableDependencyGraph, parse};

let statement = &parse("MATCH (n) LET x = n.age RETURN x")
    .ast
    .unwrap()
    .statements[0];

let query_info = QueryInfo::from_ast(statement);
let deps = VariableDependencyGraph::build(statement);

println!("Clause sequence: {:?}", query_info.clause_sequence);
println!("Variable dependencies: {:?}", deps.edges);
```

## Examples

Run the included examples to see the parser in action:

```bash
cargo run --example parser_demo
cargo run --example visitor_usage
cargo run --example query_analysis_usage
cargo run --example semantic_validation_demo
```

## Documentation

- [AST Guide](docs/AST_GUIDE.md) - Working with the abstract syntax tree
- [User Guide](docs/USER_GUIDE.md) - Detailed API documentation
- [Semantic Validation](docs/SEMANTIC_VALIDATION.md) - Schema integration and validation
- [Benchmark Baseline](docs/BENCHMARK_BASELINE.md) - Performance characteristics

## Testing

```bash
# Run all tests
cargo test

# Run benchmarks
cargo bench

# Run specific test suite
cargo test --test semantic
```

## Project Status

- Parser: ISO GQL compliant, 328/328 tests passing
- Semantic validation: Core features implemented
- Performance: <10ms for simple queries, <50ms for complex queries
- Documentation: Comprehensive API docs and guides

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)).
