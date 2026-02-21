# TokenStream Migration - Quick Reference for Next Session

## ✅ What's Been Done

### Fully Migrated Files (100%)
1. **query/pagination.rs** - 7 functions (ORDER BY, LIMIT, OFFSET, GROUP BY)
2. **query/primitive.rs** - 13 functions (MATCH, FILTER, LET, FOR statements)

### Partially Migrated (50%)
3. **query/result.rs** - 7 of 14 functions
   - ✅ Done: WITH, FROM, WHERE, HAVING clauses
   - ⏳ TODO: SELECT/RETURN items parsing, source items, graph match lists

## 🎯 Start Here Next Session

### Priority 1: Finish result.rs (~3-4 hours)
Migrate these remaining functions in result.rs:

1. `parse_select_items` + `parse_select_item`
2. `parse_select_source_item`
3. `parse_from_graph_match_list`
4. `parse_graph_pattern_checked`
5. `parse_return_items` + `parse_return_item`
6. `parse_optional_source_alias`

**Pattern to follow**: See `parse_where_clause` or `parse_having_clause` in result.rs

### Priority 2: Migrate linear.rs (~2-3 hours)
Three functions:
- `parse_linear_query_as_query`
- `parse_linear_query`
- `parse_query_statements`

### Priority 3: Migrate query/mod.rs helpers (~2-3 hours)
Four functions:
- `skip_to_query_clause_boundary`
- `parse_composite_query`
- `parse_set_quantifier_opt`

## 📋 Migration Checklist

For each function:
- [ ] Replace `tokens: &[Token], pos: &mut usize` with `stream: &mut TokenStream`
- [ ] Replace `*pos >= tokens.len()` checks → Not needed (stream handles this)
- [ ] Replace `tokens[*pos]` → `stream.current()`
- [ ] Replace `*pos += 1` → `stream.advance()`
- [ ] Replace `matches!(tokens[*pos].kind, K)` → `stream.check(&K)`
- [ ] Use `stream.previous_span()` for end spans
- [ ] Bridge to legacy functions with pattern:
  ```rust
  let tokens = stream.tokens();
  let mut pos = stream.position();
  let (result, diags) = legacy_fn(tokens, &mut pos);
  stream.set_position(pos);
  ```

## 🔧 Testing Commands

```bash
# Quick compile check
cargo build --lib

# Run tests (note: some pre-existing failures unrelated to migration)
cargo test --lib

# Check for warnings
cargo clippy
```

## 📁 Files Reference

Key files to look at for patterns:
- `src/parser/query/primitive.rs` - Perfect examples of pure TokenStream
- `src/parser/query/result.rs` - Examples of bridging to legacy
- `src/parser/query/pagination.rs` - Clean completed migration

## 💾 Current Status

**Compilation**: ✅ Clean
**Migration**: ~40% complete
**Estimate remaining**: 44-61 hours

## 🚀 Quick Start

1. Open `src/parser/query/result.rs`
2. Find `parse_select_items` function
3. Follow the pattern from `parse_where_clause` above it
4. Test with `cargo build --lib` after each function

Good luck! The patterns are established and working well. 🎉
