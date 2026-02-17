//! Advanced lexer examples demonstrating edge cases and complex scenarios.

use gql_parser::lexer::token::TokenKind;
use gql_parser::tokenize;

fn main() {
    println!("=== Advanced GQL Lexer Examples ===\n");

    // Example 1: Complex nested query
    example_complex_query();

    // Example 2: Unicode in identifiers and strings
    example_unicode();

    // Example 3: Edge cases with operators
    example_operators();

    // Example 4: Multiple errors with recovery
    example_error_recovery();

    // Example 5: Temporal literals
    example_temporal();
}

fn example_complex_query() {
    println!("Example 1: Complex Nested Query");
    let source = r#"
MATCH (p:Person {name: 'Alice'})-[:KNOWS*1..3]->(friend:Person)
WHERE friend.age > $minAge AND friend.city IN ['NYC', 'SF']
WITH friend, count(*) AS connectionCount
ORDER BY connectionCount DESC
LIMIT 10
RETURN friend.name, friend.age, connectionCount
"#;

    let result = tokenize(source);
    println!("Source: {}", source.trim());
    println!("Tokens: {}", result.tokens.len());
    println!("Identifiers found:");

    for token in &result.tokens {
        if let TokenKind::Identifier(name) = &token.kind {
            println!("  - {}", name);
        }
    }

    println!("Parameters found:");
    for token in &result.tokens {
        if let TokenKind::Parameter(name) = &token.kind {
            println!("  - ${}", name);
        }
    }

    println!("Diagnostics: {}\n", result.diagnostics.len());
}

fn example_unicode() {
    println!("Example 2: Unicode Support");

    // Regular identifiers are ASCII-only
    // Unicode characters require delimited identifiers (backticks)
    let source = r#"
MATCH (user:`用户` {`名字`: 'Alice 爱丽丝'})
WHERE user.email = 'alice@example.com'
RETURN user.`名字`, user.`年龄`
"#;

    let result = tokenize(source);
    println!("Source: {}", source.trim());
    println!("Successfully tokenized: {} tokens", result.tokens.len());

    // Show delimited identifiers with Unicode
    println!("Delimited identifiers with Unicode:");
    for token in &result.tokens {
        if let TokenKind::DelimitedIdentifier(name) = &token.kind
            && name.chars().any(|c| c as u32 > 127)
        {
            println!("  - `{}` (contains non-ASCII)", name);
        }
    }

    // Show string literals with Unicode
    println!("String literals with Unicode:");
    for token in &result.tokens {
        if let TokenKind::StringLiteral(s) = &token.kind
            && s.chars().any(|c| c as u32 > 127)
        {
            println!("  - '{}' (contains non-ASCII)", s);
        }
    }

    println!("Diagnostics: {}\n", result.diagnostics.len());
}

fn example_operators() {
    println!("Example 3: Operator Edge Cases");

    // Test all multi-character operators
    let cases = vec![
        ("a->b", "Arrow operator"),
        ("a<-b", "Left arrow operator"),
        ("a<>b", "Not equal operator"),
        ("a!=b", "Alternative not equal"),
        ("a<=b", "Less than or equal"),
        ("a>=b", "Greater than or equal"),
        ("a||b", "Double pipe"),
        ("a::b", "Double colon"),
        ("a..b", "Range operator"),
        ("a<~b", "Left tilde"),
        ("a~>b", "Right tilde"),
    ];

    for (source, description) in cases {
        let result = tokenize(source);
        println!(
            "  {}: {} => {:?}",
            description,
            source,
            result
                .tokens
                .iter()
                .filter(|t| t.kind != TokenKind::Eof)
                .map(|t| format!("{}", t.kind))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!();
}

fn example_error_recovery() {
    println!("Example 4: Error Recovery");

    // Multiple errors in one input
    let source = r#"
MATCH (n) WHERE n.name = 'unclosed string
  AND n.age = @ // invalid character
  AND n.email = 'valid@example.com'
RETURN n
"#;

    let result = tokenize(source);
    println!("Source with errors: {}", source.trim());
    println!("Tokens produced: {}", result.tokens.len());
    println!("Errors found: {}", result.diagnostics.len());

    for (i, diag) in result.diagnostics.iter().enumerate() {
        println!("  Error {}: {}", i + 1, diag.message);
    }

    // Verify lexer recovered and continued
    println!("Lexer recovered and found:");
    for token in &result.tokens {
        if token.kind == TokenKind::Return {
            println!("  ✓ RETURN keyword found - recovery successful!");
            break;
        }
    }
    println!();
}

fn example_temporal() {
    println!("Example 5: Temporal Literals");

    let source = r#"
MATCH (event:Event)
WHERE event.start >= DATE '2024-01-01'
  AND event.start < DATE '2024-12-31'
  AND event.time >= TIME '09:00:00'
  AND event.time < TIME '17:00:00'
  AND event.created > TIMESTAMP '2024-01-01T00:00:00'
  AND event.duration < DURATION 'PT2H'
RETURN event
"#;

    let result = tokenize(source);
    println!("Source: {}", source.trim());
    println!("\nTemporal literals found:");

    for token in &result.tokens {
        match &token.kind {
            TokenKind::DateLiteral(d) => {
                println!("  DATE '{}' at position {}", d, token.span.start);
            }
            TokenKind::TimeLiteral(t) => {
                println!("  TIME '{}' at position {}", t, token.span.start);
            }
            TokenKind::TimestampLiteral(ts) => {
                println!("  TIMESTAMP '{}' at position {}", ts, token.span.start);
            }
            TokenKind::DurationLiteral(dur) => {
                println!("  DURATION '{}' at position {}", dur, token.span.start);
            }
            _ => {}
        }
    }

    println!("\nDiagnostics: {}", result.diagnostics.len());
}
