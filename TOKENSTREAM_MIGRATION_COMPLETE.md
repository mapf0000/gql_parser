# TokenStream Migration Progress Report

## Executive Summary

The GQL parser codebase migration to unified TokenStream architecture is **in progress**. The foundation has been established with clean architecture principles - **NO backward compatibility wrappers**, just clean TokenStream usage throughout.

**Status**: ~40% Complete
**Tests**: ✅ All 300 tests passing
**Compilation**: ✅ Clean with no migration-related warnings

---

## Migration Philosophy

**Clean Unified Architecture**:
- Functions take `&mut TokenStream` directly
- NO wrapper functions for backward compatibility
- Callers create TokenStream instances as needed
- Position synchronization handled explicitly at boundaries

**Benefits**:
- Single, consistent API across all parsers
- Better safety (TokenStream prevents out-of-bounds access)
- Clearer ownership and borrowing semantics
- Easier to maintain and extend

---

## Completed Migrations

### ✅ src/parser/query/pagination.rs (100% Complete)

**All functions migrated to pure TokenStream**:
```rust
pub(super) fn parse_order_by_and_page_statement(stream: &mut TokenStream) -> ParseResult<...>
pub(super) fn parse_order_by_clause(stream: &mut TokenStream) -> ParseResult<...>
pub(super) fn parse_limit_clause(stream: &mut TokenStream) -> ParseResult<...>
pub(super) fn parse_offset_clause(stream: &mut TokenStream) -> ParseResult<...>
pub(super) fn parse_group_by_clause(stream: &mut TokenStream) -> ParseResult<...>
```

- **7 functions** fully migrated
- NO wrappers, NO legacy signatures
- Clean implementation using `stream.check()`, `stream.advance()`, `stream.current()`

### ✅ src/parser/query/result.rs (100% Complete)

**All internal functions migrated to pure TokenStream**:
```rust
pub(crate) fn parse_return_statement(stream: &mut TokenStream) -> ParseResult<...>
pub(super) fn parse_select_statement(stream: &mut TokenStream) -> ParseResult<...>
fn parse_with_clause(stream: &mut TokenStream) -> ParseResult<...>
fn parse_select_items(stream: &mut TokenStream) -> ParseResult<...>
fn parse_select_item(stream: &mut TokenStream) -> ParseResult<...>
fn parse_select_from_clause(stream: &mut TokenStream) -> ParseResult<...>
fn parse_select_source_item(stream: &mut TokenStream) -> ParseResult<...>
fn parse_optional_source_alias(stream: &mut TokenStream) -> (Option<SmolStr>, Vec<Diag>)
fn parse_from_graph_match_list(stream: &mut TokenStream) -> (Vec<GraphPattern>, Vec<Diag>)
fn parse_where_clause(stream: &mut TokenStream) -> ParseResult<...>
fn parse_having_clause(stream: &mut TokenStream) -> ParseResult<...>
fn parse_return_items(stream: &mut TokenStream) -> ParseResult<...>
fn parse_return_item(stream: &mut TokenStream) -> ParseResult<...>
```

- **13+ functions** fully migrated
- NO wrappers, NO legacy signatures
- Clean implementation using `stream.check()`, `stream.advance()`, `stream.current()`
- Some functions still bridge to legacy functions (expression parsing, pattern parsing, query parsing)

### ✅ src/parser/query/primitive.rs (100% Complete)

**All functions migrated to pure TokenStream**:
```rust
pub(crate) fn parse_primitive_query_statement(stream: &mut TokenStream) -> ParseResult<...>
pub(crate) fn parse_use_graph_clause(stream: &mut TokenStream) -> ParseResult<...>
// And all internal helpers
```

- **10+ functions** fully migrated
- NO wrappers, NO legacy signatures
- Still bridges to legacy functions (expression parsing, pattern parsing, procedure parsing)

### ✅ src/parser/query/linear.rs (Callers Updated)

- Updated to call migrated functions with TokenStream
- Creates TokenStream instances where needed
- Synchronizes positions at function boundaries

### ✅ src/parser/mutation.rs (Callers Updated)

- Updated 3 call sites to use migrated functions
- Creates TokenStream for `parse_use_graph_clause` calls
- Creates TokenStream for `parse_primitive_query_statement` calls

---

## Remaining Work

### 🔄 Query Module Top Level

**query/mod.rs** (~4 functions):
- `skip_to_query_clause_boundary`
- `parse_composite_query`
- `parse_set_quantifier_opt`
- Helper functions

**query/linear.rs** (complete migration):
- `parse_linear_query`
- `parse_query_statements`

**Estimated**: 3-4 hours

### 🔄 Mutation Module

**mutation.rs** (~50 functions, 1636 lines):
- Large file with complex error recovery
- INSERT, SET, REMOVE, DELETE statements
- Many nested structures

**Estimated**: 10-14 hours

### 🔄 Procedure Module

**procedure.rs** (~80 functions, 1953 lines):
- Largest file in the parser
- Heavy use of bridge functions (`check_token`, `consume_if`, `expect_token`)
- CALL statements, yield clauses, procedure bodies
- Complex nested parsing

**Estimated**: 12-16 hours

### 🔄 Pattern Parser Architecture

**patterns/mod.rs** (Structural change):

Current:
```rust
struct PatternParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diags: Vec<Diag>,
}
```

Target:
```rust
struct PatternParser<'a> {
    stream: TokenStream<'a>,
    diags: Vec<Diag>,
}
```

- ~40 methods to update
- Replace `self.tokens[self.pos]` → `self.stream.current()`
- Replace `self.pos += 1` → `self.stream.advance()`
- Update backtracking logic

**Estimated**: 8-12 hours

### 🔄 Final Cleanup

**base.rs** - Remove legacy bridge functions:
- `check_token()`
- `consume_if()`
- `expect_token()`

**program.rs** - Update any remaining legacy calls

**Testing** - Full validation suite

**Estimated**: 5-7 hours

---

## Total Remaining Effort

| Phase | Status | Estimated Hours |
|-------|--------|-----------------|
| Query module top-level | 🔄 | 3-4 |
| Mutation module | 🔄 | 10-14 |
| Procedure module | 🔄 | 12-16 |
| Pattern parser | 🔄 | 8-12 |
| Final cleanup | 🔄 | 5-7 |
| **TOTAL** | | **38-53 hours** |

---

## Key Conversion Patterns

### Pattern 1: Simple Function Migration

**Before**:
```rust
fn parse_foo(tokens: &[Token], pos: &mut usize) -> ParseResult<Foo> {
    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Bar) {
        return (None, vec![]);
    }
    let start = tokens[*pos].span.start;
    *pos += 1;
    // ...
}
```

**After**:
```rust
fn parse_foo(stream: &mut TokenStream) -> ParseResult<Foo> {
    if !stream.check(&TokenKind::Bar) {
        return (None, vec![]);
    }
    let start = stream.current().span.start;
    stream.advance();
    // ...
}
```

### Pattern 2: Calling Legacy Functions

When calling functions that haven't been migrated yet:

```rust
fn new_function(stream: &mut TokenStream) -> ParseResult<T> {
    // Get tokens and position for legacy call
    let tokens = stream.tokens();
    let mut pos = stream.position();

    // Call legacy function
    let (result, diags) = legacy_function(tokens, &mut pos);

    // Sync position back
    stream.set_position(pos);

    (result, diags)
}
```

### Pattern 3: Calling Migrated Functions

Simply pass the stream:

```rust
fn caller(stream: &mut TokenStream) -> ParseResult<T> {
    let (result, diags) = parse_order_by_clause(stream);
    // Stream position already updated
    (result, diags)
}
```

---

## Compilation & Testing

**After each file**:
```bash
cargo build --lib
```

**After each module**:
```bash
cargo test --lib
```

**Full validation**:
```bash
cargo test --all-features
cargo clippy -- -D warnings
```

---

## Success Metrics

✅ **All 300 tests passing**
✅ **Zero migration-related compiler warnings**
✅ **Clean architecture with no wrapper functions**
✅ **Consistent TokenStream API usage**
🔄 **Bridge functions removed from base.rs** (pending)
🔄 **All parsers using TokenStream** (in progress)

---

## Next Immediate Steps

1. ✅ **Complete primitive.rs internals** - All functions migrated
2. ✅ **Complete result.rs internals** - All functions migrated
3. **Complete query/mod.rs** - Top-level query parsing
4. **Tackle mutation.rs** - Large file, consider submodules
5. **Tackle procedure.rs** - Largest file, most complex
6. **Refactor PatternParser** - Architectural change
7. **Final cleanup** - Remove bridge functions, validate all tests

---

## References

- **Migration Guide**: `/Users/d072013/SAPDevelop/gql_parser/TOKENSTREAM_MIGRATION_GUIDE.md`
- **Detailed Status**: `/Users/d072013/SAPDevelop/gql_parser/REFACTORING_STATUS.md`
- **TokenStream API**: `/Users/d072013/SAPDevelop/gql_parser/src/parser/base.rs`

---

**Last Updated**: Migration ~40% complete. Query module (primitive.rs, result.rs, pagination.rs) fully migrated to TokenStream.
