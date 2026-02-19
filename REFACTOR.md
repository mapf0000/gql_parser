# GQL Parser Refactoring Plan

## Overview
This document outlines structural improvements to the codebase following Rust best practices. Focus: splitting large files, eliminating duplication, proper module boundaries.

---

## Phase 1: Split validator.rs (PRIORITY: HIGH)

**Problem:** `src/semantic/validator.rs` (6,065 lines) violates Single-Responsibility Principle with 9 validation passes in one file.

**Action:** Create `src/semantic/validator/` submodule:

```
src/semantic/validator/
├── mod.rs                    (Lines 1-203: config, structs, main coordinator)
├── scope_analysis.rs         (Lines 204-827: Pass 1)
├── type_inference.rs         (Lines 828-1,252: Pass 2)
├── variable_validation.rs    (Lines 1,253-2,170: Pass 3) ← START HERE (918 lines)
├── pattern_validation.rs     (Lines 2,171-2,534: Pass 4)
├── context_validation.rs     (Lines 2,535-3,107: Pass 5)
├── type_checking.rs          (Lines 3,108-3,623: Pass 6)
├── expression_validation.rs  (Lines 3,624-4,131: Pass 7)
├── reference_validation.rs   (Lines 4,132-4,232: Pass 8)
└── schema_validation.rs      (Lines 4,233-4,498: Pass 9)
```

**Tests:** Move lines 4,498-6,065 to `tests/semantic_validator_tests.rs` or split by pass.

**Key Methods to Extract:**

- **variable_validation.rs:** `run_variable_validation`, `validate_query_variables`, `validate_linear_query_variables`, `validate_primitive_statement_variables`, `validate_return_aggregation`, `validate_expression_variables`
- **scope_analysis.rs:** `run_scope_analysis`, `analyze_query`, `analyze_linear_query`, `push_scope`, `pop_scope`, `register_variable`
- **context_validation.rs:** `run_context_validation`, `validate_aggregation_context`, `validate_group_by`

**Approach:**
1. Create `validator/mod.rs` with `SemanticValidator` struct and public API
2. Extract each pass into separate file with private helper methods
3. Import pass modules in `mod.rs` and delegate from main `validate()` method
4. Run full test suite after each extraction
5. Update `src/semantic/mod.rs` to re-export from `validator::mod`

---

## Phase 2: Split Parser Files

### 2.1 query.rs (2,844 lines, 36 methods)

**Action:** Create `src/parser/query/` submodule:

```
src/parser/query/
├── mod.rs              (Entry point, composite queries)
├── linear.rs           (Lines ~400-800: linear query parsing)
├── primitive.rs        (Lines ~800-1,400: MATCH, FILTER, LET, FOR)
├── result.rs           (Lines ~1,400-2,000: RETURN, SELECT)
└── pagination.rs       (Lines ~2,000-2,844: ORDER BY, LIMIT, OFFSET)
```

**Key Methods:**
- `mod.rs`: `parse_query`, `parse_composite_query`
- `linear.rs`: `parse_linear_query`
- `primitive.rs`: `parse_primitive_statement`, `parse_match_statement`, `parse_filter_statement`
- `result.rs`: `parse_result_statement`, `parse_return_statement`

### 2.2 patterns.rs (2,311 lines, 67 methods)

**Action:** Create `src/parser/patterns/` submodule:

```
src/parser/patterns/
├── mod.rs              (Entry point, graph pattern coordination)
├── path.rs             (Path pattern parsing, quantifiers)
├── element.rs          (Node and edge patterns, WHERE clauses)
└── label.rs            (Label expression parsing)
```

**Key Methods:**
- `mod.rs`: `parse_graph_pattern`, `parse_pattern_part`
- `path.rs`: `parse_path_pattern`, `parse_path_mode`, `parse_quantifier`
- `element.rs`: `parse_node_pattern`, `parse_edge_pattern`
- `label.rs`: `parse_label_expression`

### 2.3 types.rs (1,849 lines, 53 methods)

**Action:** Create `src/parser/types/` submodule:

```
src/parser/types/
├── mod.rs              (Entry point, type parser coordination)
├── predefined.rs       (Boolean, numeric, string, temporal types)
├── reference.rs        (Graph, node, edge, binding table types)
└── constructed.rs      (Path, list, record types)
```

**Key Methods:**
- `mod.rs`: `parse_type`, `parse_type_reference`
- `predefined.rs`: `parse_predefined_type`
- `reference.rs`: `parse_graph_reference_value_type`
- `constructed.rs`: `parse_path_value_type`, `parse_list_value_type`

---

## Phase 3: Eliminate Code Duplication

**Problem:** `procedure.rs` and `program.rs` reimplement helpers available in `base.rs`.

**Files:**
- `src/parser/procedure.rs:40-82` (duplicate helpers)
- `src/parser/program.rs:1538-1551` (duplicate helpers)

**Action:**
1. Remove local implementations of `expect_token`, `check_token`, `consume_if`
2. Refactor to use `TokenStream` from `src/parser/base.rs`
3. Update parser instantiation to use `TokenStream::new(tokens)`

**Benefits:** ~100 lines removed, consistent API across all parsers.

---

## Phase 4: Test Organization

**Move inline tests to dedicated test files:**

- `src/semantic/validator.rs` tests (1,566 lines) → `tests/semantic_validator_tests.rs` + `tests/semantic_validator_scope_and_agg_tests.rs`
- Consider splitting by validation pass: `tests/semantic/scope_tests.rs`, `tests/semantic/variable_tests.rs`, etc.

**Benefits:** Faster compilation, clearer organization, easier to run specific test suites.

---

## Implementation Checklist

### Phase 1: validator.rs
- [x] Create `src/semantic/validator/` directory
- [x] Extract `variable_validation.rs` (largest section first)
- [x] Create `mod.rs` with main coordinator
- [x] Extract remaining 8 passes
- [x] Move tests to `tests/semantic_validator_tests.rs`
- [x] Update re-exports in `src/semantic/mod.rs`
- [x] Run full test suite

### Phase 2: Parser files
- [x] Split `query.rs` into submodule
- [x] Split `patterns.rs` into submodule
- [x] Split `types.rs` into submodule
- [x] Update re-exports in `src/parser/mod.rs`
- [x] Run full test suite

### Phase 3: Duplication
- [x] Migrate `procedure.rs` to use shared parser helpers backed by `TokenStream`
- [x] Migrate `program.rs` to use shared parser helpers backed by `TokenStream`
- [x] Remove duplicate helper functions
- [x] Run parser tests

### Phase 4: Tests
- [x] Move validator tests to `tests/semantic_validator_tests.rs`
- [x] Split overflow semantic tests into `tests/semantic_validator_scope_and_agg_tests.rs`
- [x] Update test imports to use `gql_parser::` instead of `crate::`
- [x] Keep validator internals encapsulated; test via public `validate()` API
- [x] All 76 tests passing

---

## Success Metrics

| Metric | Before | Current | Target |
|--------|--------|---------|--------|
| Largest file | 6,065 lines | 1,986 lines | <1,000 lines |
| Files >2,000 lines | 3 | 0 | 0 |
| Parser submodules | 0 | 3 | 3 |
| Semantic submodules | 0 | 1 (validator/) | 1 (validator/) |
| Code duplication | ~100 lines | eliminated in parser helper layer | 0 |

---

## Risk Mitigation

- **Run tests after each extraction** to catch regressions early
- **Keep git history clean** with atomic commits per extraction
- **Start with largest files** to maximize impact
- **Preserve public API** - only internal structure changes
- **Document module boundaries** in each `mod.rs` file

---

## References

- Original analysis: See investigation notes from 2026-02-19
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Module organization: https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
