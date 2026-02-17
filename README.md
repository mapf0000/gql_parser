# gql_parser

Pure-Rust GQL parser with span-aware diagnostics.

## Recommended API

Use the crate-level `parse(&str)` function. It performs lexing + parsing and
automatically merges diagnostics from both phases.

```rust
use gql_parser::parse;

let source = "MATCH (n:Person) RETURN n";
let result = parse(source);

assert!(result.ast.is_some());
```

## Advanced API

If you need lower-level control, use `tokenize` + `Parser` directly and merge
lexer diagnostics explicitly:

```rust
use gql_parser::{tokenize, Parser};

let source = "MATCH (n) RETURN n";
let lex_result = tokenize(source);
let result = Parser::new(lex_result.tokens, source)
    .with_lexer_diagnostics(lex_result.diagnostics)
    .parse();
```

## Examples

- `examples/parser_demo.rs`: end-to-end parsing and diagnostics printing
- `examples/lexer_demo.rs`: lexer usage examples
- `examples/advanced_lexer.rs`: advanced lexer scenarios
