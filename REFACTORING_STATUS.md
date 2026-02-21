# TokenStream Refactoring Status

## Executive Summary

This document tracks the progress of refactoring the entire GQL parser codebase to use `TokenStream` everywhere, eliminating the legacy `&[Token] + &mut usize` pattern.

**Overall Progress**: ~10% Complete (1 of 9 files)

## Completed Work

### ✅ src/parser/query/pagination.rs (100% Complete)

All 6 functions have been successfully migrated to use TokenStream internally:

1. **parse_order_by_and_page_statement** - Uses TokenStream internally
2. **parse_order_by_clause** - Wrapper + `parse_order_by_clause_impl`
3. **parse_sort_specification** - Wrapper + `parse_sort_specification_impl`
4. **parse_limit_clause** - Wrapper + `parse_limit_clause_impl`
5. **parse_offset_clause** - Wrapper + `parse_offset_clause_impl`
6. **parse_group_by_clause** - Wrapper + `parse_group_by_clause_impl`
7. **parse_grouping_element** - Wrapper + `parse_grouping_element_impl`

**Pattern Used**:
- Public functions maintain legacy signature `(tokens: &[Token], pos: &mut usize)`
- Internal `_impl` functions use `&mut TokenStream`
- Wrappers create TokenStream, call impl, sync position back

**Compilation Status**: ✅ Builds successfully with minor warnings about unused legacy wrapper functions

## Remaining Work

### Priority 1: Query Module (Medium Complexity)

#### 🔄 src/parser/query/primitive.rs
**Functions to Migrate**: 13 functions
- `parse_primitive_query_statement` - Entry point (delegates only)
- `parse_use_graph_clause` - Simple pattern
- `parse_match_statement` - Delegates to other functions
- `parse_simple_match_statement` - Calls parse_graph_pattern_checked
- `parse_optional_match_statement` - Complex with blocks
- `parse_optional_operand` - Match expressions
- `parse_match_statement_block` - Loop parsing match statements
- `parse_graph_pattern_checked` - Wrapper for parse_graph_pattern
- `parse_filter_statement` - Simple pattern
- `parse_let_statement` - Loop with bindings
- `parse_let_variable_definition` - Complex with type annotations
- `parse_for_statement` - Complex with ordinality/offset
- `parse_for_item` - Binding variable + collection expression
- `parse_for_ordinality_or_offset` - Two variants

**Estimated Effort**: 4-6 hours

#### 🔄 src/parser/query/result.rs
**Functions to Migrate**: 14 functions
- `parse_select_statement` - Large function with many clauses
- `parse_with_clause` - CTE definitions loop
- `parse_common_table_expression` - Complex with recursive query
- `parse_select_items` - List or star
- `parse_select_item` - Expression + optional AS
- `parse_select_from_clause` - Multiple variants (match list, query, sources)
- `parse_select_source_item` - Graph/query/expression variants
- `parse_optional_source_alias` - AS or bare identifier
- `parse_identifier_token` - Token kind checking
- `token_is_word` - Pure function (no changes needed)
- `is_from_clause_boundary_keyword` - Pure function (no changes needed)
- `parse_from_graph_match_list` - Loop parsing MATCH patterns
- `parse_graph_pattern_checked` - Wrapper for parse_graph_pattern
- `parse_where_clause` - Simple pattern
- `parse_having_clause` - Simple pattern
- `parse_return_statement` - Similar to SELECT
- `parse_return_items` - List or star
- `parse_return_item` - Expression + optional AS

**Estimated Effort**: 5-7 hours

#### 🔄 src/parser/query/linear.rs
**Functions to Migrate**: 3 functions
- `parse_linear_query_as_query` - Parenthesized query handling
- `parse_linear_query` - USE + primitives + result
- `parse_query_statements` - Loop parsing statements

**Estimated Effort**: 2-3 hours

#### 🔄 src/parser/query/mod.rs
**Functions to Migrate**: 4 functions
- `skip_to_query_clause_boundary` - Position advancement loop
- `parse_composite_query` - Set operators loop
- `parse_set_quantifier_opt` - Token checking
- `find_expression_boundary` - Keep as-is (pure analysis function)
- `parse_expression_at` - Keep as-is (calls expression parser)

**Estimated Effort**: 2-3 hours

**Total Query Module**: 13-19 hours

### Priority 2: Mutation & Procedure Modules (High Complexity)

#### 🔄 src/parser/mutation.rs (1636 lines)
**Functions to Migrate**: ~50 functions
- Uses legacy pattern extensively
- Complex error recovery
- Many nested loops and conditionals

**Challenge**: This file is very large. Consider breaking it into submodules before refactoring.

**Estimated Effort**: 10-14 hours

#### 🔄 src/parser/procedure.rs (1953 lines)
**Functions to Migrate**: ~80 functions
- Heavy use of `check_token()`, `consume_if()`, `expect_token()` bridge functions
- These must be replaced inline with TokenStream method calls
- Very complex with nested structures

**Challenge**: Largest file. Bridge function usage must be carefully replaced.

**Estimated Effort**: 12-16 hours

**Total Mutation/Procedure**: 22-30 hours

### Priority 3: Pattern Parser (Architectural Change)

#### 🔄 src/parser/patterns/mod.rs
**Architecture Change Required**:

Current structure:
```rust
struct PatternParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diags: Vec<Diag>,
}
```

New structure:
```rust
struct PatternParser<'a> {
    stream: TokenStream<'a>,
    diags: Vec<Diag>,
}
```

**Methods to Update**: ~40 methods need internal changes:
- Replace `self.tokens[self.pos]` → `self.stream.current()`
- Replace `self.pos += 1` → `self.stream.advance()`
- Replace `self.pos` → `self.stream.position()`
- Update `new()` to create TokenStream
- Update all position manipulation logic

**Estimated Effort**: 8-12 hours

### Priority 4: Cleanup & Testing

#### 🔄 src/parser/program.rs
**Work Required**: Update any usage of migrated functions
**Estimated Effort**: 2-3 hours

#### 🔄 src/parser/base.rs
**Work Required**: Remove legacy bridge functions once all usages are eliminated:
- `check_token()`
- `consume_if()`
- `expect_token()`

**Estimated Effort**: 1 hour

#### 🔄 Testing & Validation
**Work Required**:
- Run full test suite after each file
- Fix any regressions
- Update integration tests if needed
- Performance validation

**Estimated Effort**: 4-6 hours

**Total Cleanup**: 7-10 hours

## Total Estimated Effort

| Phase | Hours |
|-------|-------|
| Query Module | 13-19 |
| Mutation & Procedure | 22-30 |
| Pattern Parser | 8-12 |
| Cleanup & Testing | 7-10 |
| **TOTAL** | **50-71 hours** |

## Migration Methodology

### Pattern Applied (from pagination.rs)

```rust
// 1. Keep public function signature for compatibility
pub(super) fn parse_foo(tokens: &[Token], pos: &mut usize) -> ParseResult<Foo> {
    let mut stream = TokenStream::new(tokens);
    stream.set_position(*pos);

    let result = parse_foo_impl(&mut stream);
    *pos = stream.position();
    result
}

// 2. Create internal implementation with TokenStream
fn parse_foo_impl(stream: &mut TokenStream) -> ParseResult<Foo> {
    let mut diags = Vec::new();

    // Use stream methods instead of raw token/position access
    if !stream.check(&TokenKind::Bar) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    // For expression parsing (temporary until expression.rs is migrated)
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (expr_opt, mut expr_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
    diags.append(&mut expr_diags);

    let end = stream.previous_span().end;

    (Some(Foo { /* ... */ }), diags)
}
```

### Key Conversion Rules

| Old Pattern | New Pattern |
|------------|-------------|
| `*pos >= tokens.len()` | (Not needed - `current()` handles this) |
| `tokens[*pos]` | `stream.current()` |
| `tokens[*pos].kind` | `stream.current().kind` |
| `tokens[*pos].span` | `stream.current().span` |
| `*pos += 1` | `stream.advance()` |
| `tokens.get(*pos)` | `stream.current()` |
| `tokens.get(*pos - 1)` | Use `stream.previous_span()` |
| `matches!(tokens[*pos].kind, K)` | `stream.check(&K)` |

### Expression Parsing Workaround

Since expression parsing still uses legacy interface:

```rust
// Inside TokenStream-based function
let tokens = stream.tokens();
let mut pos = stream.position();
let (expr, diags) = parse_expression_with_diags(tokens, &mut pos);
stream.set_position(pos);
```

## Compilation Checkpoints

After migrating each file:
```bash
cargo build --lib
```

After completing each module:
```bash
cargo test --lib
```

Full validation:
```bash
cargo test --all-features
cargo clippy -- -D warnings
```

## Rollback Strategy

Each file migration should be in its own commit:
- If issues arise, can revert specific file
- Easier to bisect if tests fail
- Allows incremental code review

## Benefits of Completion

1. **Unified Architecture**: Single pattern throughout codebase
2. **Better Safety**: TokenStream prevents out-of-bounds access
3. **Easier Maintenance**: Consistent API for all parsers
4. **Better Error Messages**: TokenStream methods provide better context
5. **Preparation for Future Work**: Clean foundation for future parser improvements

## Next Steps

1. Complete primitive.rs (13 functions)
2. Complete result.rs (14 functions)
3. Complete linear.rs (3 functions)
4. Complete query/mod.rs (4 functions)
5. Tackle mutation.rs (consider breaking into submodules first)
6. Tackle procedure.rs (largest file)
7. Refactor PatternParser architecture
8. Update program.rs
9. Remove bridge functions from base.rs
10. Final testing and validation

## Notes

- **No Behavior Changes**: This is purely mechanical refactoring
- **Backward Compatibility**: Maintained via wrapper functions
- **Test Coverage**: Existing tests validate correctness
- **Performance**: Should be neutral (same operations, different API)
