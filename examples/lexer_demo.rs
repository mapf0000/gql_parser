//! Example demonstrating the GQL lexer.

use gql_parser::{TokenKind, tokenize};

fn main() {
    // Example 1: Simple query
    println!("=== Example 1: Simple Query ===");
    let source = "MATCH (n:Person) WHERE n.age > 18 RETURN n";
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Tokens:");
    for token in &result.tokens {
        println!(
            "  {:?} at {}..{} => '{}'",
            token.kind,
            token.span.start,
            token.span.end,
            token.slice(source)
        );
    }
    println!("Diagnostics: {}", result.diagnostics.len());
    println!();

    // Example 2: Query with parameters
    println!("=== Example 2: Parameters ===");
    let source = "MATCH (n) WHERE n.id = $id RETURN n";
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Parameters found:");
    for token in &result.tokens {
        if let TokenKind::Parameter(name) = &token.kind {
            println!("  ${} at {}..{}", name, token.span.start, token.span.end);
        }
    }
    println!();

    // Example 3: String literals with escapes
    println!("=== Example 3: String Literals ===");
    let source = r#"MATCH (n {name: 'Alice\nBob', title: 'It\'s great'})"#;
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("String literals found:");
    for token in &result.tokens {
        if let TokenKind::StringLiteral(content) = &token.kind {
            println!("  '{}' => \"{}\"", token.slice(source), content);
        }
    }
    println!();

    // Example 4: Temporal literals
    println!("=== Example 4: Temporal Literals ===");
    let source = "MATCH (e:Event) WHERE e.start = DATE '2024-01-15' AND e.time = TIME '14:30:00'";
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Temporal literals found:");
    for window in result.tokens.windows(2) {
        if let [kw, value] = window {
            match (&kw.kind, &value.kind) {
                (TokenKind::Date, TokenKind::StringLiteral(d)) => {
                    println!("  DATE '{}' at {}..{}", d, kw.span.start, value.span.end)
                }
                (TokenKind::Time, TokenKind::StringLiteral(t)) => {
                    println!("  TIME '{}' at {}..{}", t, kw.span.start, value.span.end)
                }
                (TokenKind::Timestamp, TokenKind::StringLiteral(ts)) => {
                    println!(
                        "  TIMESTAMP '{}' at {}..{}",
                        ts, kw.span.start, value.span.end
                    )
                }
                (TokenKind::Duration, TokenKind::StringLiteral(dur)) => {
                    println!(
                        "  DURATION '{}' at {}..{}",
                        dur, kw.span.start, value.span.end
                    )
                }
                _ => {}
            }
        }
    }
    println!();

    // Example 5: Error recovery
    println!("=== Example 5: Error Recovery ===");
    let source = "MATCH (n) WHERE n.name = 'unclosed string";
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Tokens: {}", result.tokens.len());
    println!("Diagnostics: {}", result.diagnostics.len());
    for diag in &result.diagnostics {
        println!("  Error: {} at {:?}", diag.message, diag.labels);
    }
    println!();

    // Example 6: Comments
    println!("=== Example 6: Comments ===");
    let source = r#"
// Single-line comment
MATCH (n) /* block comment */ RETURN n
/* nested /* comment */ test */
"#;
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Tokens (comments are stripped):");
    for token in &result.tokens {
        if token.kind != TokenKind::Eof {
            println!("  {:?} => '{}'", token.kind, token.slice(source));
        }
    }
    println!();

    // Example 7: Numeric literals
    println!("=== Example 7: Numeric Literals ===");
    let source = "42 3.14 1.0e10 2.5E-3 1_000_000";
    let result = tokenize(source);

    println!("Source: {}", source);
    println!("Numeric literals:");
    for token in &result.tokens {
        match &token.kind {
            TokenKind::IntegerLiteral(n) => println!("  Integer: {}", n),
            TokenKind::FloatLiteral(n) => println!("  Float: {}", n),
            _ => {}
        }
    }
}
