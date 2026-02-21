# TokenStream Migration Guide

## Overview

This document outlines the complete refactoring of the GQL parser codebase to use `TokenStream` everywhere, eliminating the legacy `&[Token] + &mut usize` pattern completely.

## Migration Status

### ✅ Completed
- **pagination.rs**: All functions now use TokenStream internally
  - `parse_order_by_clause` → `parse_order_by_clause_impl`
  - `parse_sort_specification` → `parse_sort_specification_impl`
  - `parse_limit_clause` → `parse_limit_clause_impl`
  - `parse_offset_clause` → `parse_offset_clause_impl`
  - `parse_group_by_clause` → `parse_group_by_clause_impl`
  - `parse_grouping_element` → `parse_grouping_element_impl`

### 🔄 In Progress
- **primitive.rs**: Needs migration of 13 functions
- **result.rs**: Needs migration of 14 functions
- **linear.rs**: Needs migration of 3 functions
- **query/mod.rs**: Needs migration of helper functions

### ⏳ Pending
- **mutation.rs**: Large file with 50+ functions
- **procedure.rs**: Large file with 80+ functions using legacy bridge functions
- **patterns/mod.rs**: PatternParser struct needs field-level refactoring
- **program.rs**: Updates after dependencies are migrated

## Migration Pattern

### Step 1: Create Internal Implementation Function

For each function using the legacy pattern:

```rust
// OLD: Legacy pattern
pub(super) fn parse_foo(tokens: &[Token], pos: &mut usize) -> ParseResult<Foo> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Bar) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // ... rest of parsing logic
}

// NEW: TokenStream wrapper + internal implementation
pub(super) fn parse_foo(tokens: &[Token], pos: &mut usize) -> ParseResult<Foo> {
    let mut stream = TokenStream::new(tokens);
    stream.set_position(*pos);

    let result = parse_foo_impl(&mut stream);
    *pos = stream.position();
    result
}

fn parse_foo_impl(stream: &mut TokenStream) -> ParseResult<Foo> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Bar) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // ... rest of parsing logic using stream methods
}
```

### Step 2: Replace Legacy Patterns with TokenStream Methods

| Legacy Pattern | TokenStream Equivalent |
|---------------|----------------------|
| `*pos >= tokens.len()` | Not needed (handled by `current()`) |
| `tokens[*pos]` | `stream.current()` |
| `tokens.get(*pos)` | `stream.current()` (always safe) |
| `*pos += 1` | `stream.advance()` |
| `matches!(tokens[*pos].kind, K)` | `stream.check(&K)` |
| `tokens[*pos].span` | `stream.current().span` |
| `tokens.get(*pos - 1)` | `stream.previous_span()` |

### Step 3: Handle Expression Parsing

Expression parsing still uses the legacy interface temporarily. Use this pattern:

```rust
fn parse_foo_impl(stream: &mut TokenStream) -> ParseResult<Foo> {
    let mut diags = Vec::new();

    // Need to parse expression
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (expr_opt, mut expr_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut expr_diags);

    let expr = match expr_opt {
        Some(e) => e,
        None => return (None, diags),
    };

    // Continue with stream methods...
}
```

### Step 4: Handle Span Calculation

```rust
// Start span
let start = stream.current().span.start;

// End span after consuming tokens
let end = stream.previous_span().end;

// Or using last element
let end = elements.last().map(|e| e.span.end).unwrap_or(start);
```

## File-by-File Migration Plan

### Phase 1: Query Module (Low Risk)
1. ✅ **pagination.rs** - COMPLETED
2. **primitive.rs** - 13 functions
   - `parse_use_graph_clause`
   - `parse_match_statement` + helpers (7 functions)
   - `parse_filter_statement`
   - `parse_let_statement` + helpers (2 functions)
   - `parse_for_statement` + helpers (3 functions)

3. **result.rs** - 14 functions
   - `parse_select_statement` + helpers (10 functions)
   - `parse_return_statement` + helpers (3 functions)
   - Utility functions (3 functions)

4. **linear.rs** - 3 functions
   - `parse_linear_query_as_query`
   - `parse_linear_query`
   - `parse_query_statements`

5. **query/mod.rs** - 4 functions
   - `parse_expression_at` (keep as-is, already clean)
   - `skip_to_query_clause_boundary` (needs conversion)
   - `parse_composite_query`
   - `parse_set_quantifier_opt`

### Phase 2: Mutation & Procedure Modules (High Complexity)
6. **mutation.rs** - Large file
   - 50+ functions using legacy pattern
   - Consider breaking into submodules first

7. **procedure.rs** - Large file
   - 80+ functions
   - Heavy use of `check_token`, `consume_if`, `expect_token`
   - These bridge functions must be replaced inline

### Phase 3: Pattern Parser (Architectural Change)
8. **patterns/mod.rs** - PatternParser struct
   ```rust
   // OLD
   struct PatternParser<'a> {
       tokens: &'a [Token],
       pos: usize,
       diags: Vec<Diag>,
   }

   // NEW
   struct PatternParser<'a> {
       stream: TokenStream<'a>,
       diags: Vec<Diag>,
   }

   // Update all methods from:
   // self.tokens[self.pos] → self.stream.current()
   // self.pos += 1 → self.stream.advance()
   // etc.
   ```

### Phase 4: Cleanup
9. **program.rs** - Update any usage of migrated functions
10. **base.rs** - Remove legacy bridge functions:
    - `check_token()`
    - `consume_if()`
    - `expect_token()`

## Testing Strategy

After each file migration:
1. Run `cargo build --lib` to check for compilation errors
2. Run `cargo test` to ensure all tests pass
3. Fix any test failures before proceeding to next file

Full test suite run after complete migration:
```bash
cargo test --lib
cargo test --all-features
```

## Key Challenges

### 1. Expression Parsing Dependency
Expression parsing still uses legacy pattern, so we need the temporary workaround shown above until expression.rs is also migrated.

### 2. Recursive Calls Between Modules
Many functions call functions in other modules. Keep wrapper functions with legacy signatures until all callers are migrated.

### 3. Error Recovery Logic
Some parsers have complex error recovery that relies on position manipulation. Need to preserve this logic carefully when converting to TokenStream.

### 4. PatternParser Architecture
PatternParser has custom error recovery and complex state. This requires careful refactoring to avoid breaking existing behavior.

## Example: Complete Migration of parse_use_graph_clause

```rust
// BEFORE
pub(crate) fn parse_use_graph_clause(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<UseGraphClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Use) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let (graph_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let graph = match graph_opt {
        Some(g) => g,
        None => {
            diags.push(
                Diag::error("Expected graph expression after USE").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(UseGraphClause {
            graph,
            span: start..end,
        }),
        diags,
    )
}

// AFTER
pub(crate) fn parse_use_graph_clause(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<UseGraphClause> {
    let mut stream = TokenStream::new(tokens);
    stream.set_position(*pos);

    let result = parse_use_graph_clause_impl(&mut stream);
    *pos = stream.position();
    result
}

fn parse_use_graph_clause_impl(stream: &mut TokenStream) -> ParseResult<UseGraphClause> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Use) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // Expression parsing - temporary legacy interface usage
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (graph_opt, mut expr_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut expr_diags);

    let graph = match graph_opt {
        Some(g) => g,
        None => {
            diags.push(
                Diag::error("Expected graph expression after USE").with_primary_label(
                    stream.current().span.clone(),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = stream.previous_span().end;

    (
        Some(UseGraphClause {
            graph,
            span: start..end,
        }),
        diags,
    )
}
```

## Timeline Estimate

- Phase 1 (Query Module): 4-6 hours
- Phase 2 (Mutation & Procedure): 8-12 hours
- Phase 3 (Pattern Parser): 4-6 hours
- Phase 4 (Cleanup & Testing): 2-4 hours

**Total: 18-28 hours of focused development work**

## Notes

- This is a mechanical refactoring that should not change parser behavior
- Maintain backward compatibility during migration via wrapper functions
- Clean up wrapper functions only after all callers are migrated
- Keep commits small and focused on one file at a time
- Run tests after each file to catch regressions early
