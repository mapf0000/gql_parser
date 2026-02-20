# Test Directory Restructuring Plan

## Executive Summary

This plan restructures the test directory to improve organization by:
1. Creating component-based subdirectories
2. Renaming milestone-based tests to descriptive names
3. Moving examples to appropriate locations
4. Establishing clear naming conventions

## Current State Analysis

### Existing Test Files (24 files)
- **Parser tests**: pattern_tests.rs, query_tests.rs, mutation_tests.rs, procedure_tests.rs, aggregate_tests.rs, graph_type_tests.rs, type_reference_spec_tests.rs, case_insensitive_tests.rs
- **Semantic/Validator tests**: semantic_validator_tests.rs, semantic_validator_scope_and_agg_tests.rs, procedure_definition_tests.rs
- **Milestone tests**: milestone3_schema_catalog_tests.rs, milestone4_aggregate_validation_tests.rs, milestone4_callable_catalog_tests.rs, milestone5_type_inference_tests.rs
- **Integration tests**: integration_type_inference.rs
- **Conformance tests**: conformance_matrix_tests.rs, procedure_iso_conformance_tests.rs, sample_corpus_tests.rs
- **Edge case & stress tests**: edge_case_tests.rs, edge_case_tests_extended.rs, stress_tests.rs

### Existing Example Files with "milestone" prefix
- examples/milestone3_schema_catalog.rs
- examples/milestone3_advanced_features.rs
- examples/milestone4_callable_catalog.rs
- examples/milestone5_type_inference.rs

## Proposed Directory Structure

```
tests/
├── parser/                          # Parser-specific tests
│   ├── patterns.rs                  # pattern_tests.rs
│   ├── queries.rs                   # query_tests.rs
│   ├── mutations.rs                 # mutation_tests.rs
│   ├── procedures.rs                # procedure_tests.rs
│   ├── aggregates.rs                # aggregate_tests.rs
│   ├── graph_types.rs               # graph_type_tests.rs
│   ├── type_references.rs           # type_reference_spec_tests.rs
│   ├── case_insensitivity.rs        # case_insensitive_tests.rs
│   └── mod.rs
│
├── semantic/                        # Semantic validator tests
│   ├── validator.rs                 # semantic_validator_tests.rs
│   ├── scoping_and_aggregation.rs   # semantic_validator_scope_and_agg_tests.rs
│   ├── procedure_definitions.rs     # procedure_definition_tests.rs
│   ├── schema_integration.rs        # milestone3_schema_catalog_tests.rs (RENAMED)
│   ├── aggregate_validation.rs      # milestone4_aggregate_validation_tests.rs (RENAMED)
│   ├── callable_validation.rs       # milestone4_callable_catalog_tests.rs (RENAMED)
│   ├── type_inference.rs            # milestone5_type_inference_tests.rs (RENAMED)
│   └── mod.rs
│
├── integration/                     # Integration tests
│   ├── type_inference.rs            # integration_type_inference.rs
│   └── mod.rs
│
├── conformance/                     # Conformance & corpus tests
│   ├── matrix.rs                    # conformance_matrix_tests.rs
│   ├── iso_procedures.rs            # procedure_iso_conformance_tests.rs
│   ├── sample_corpus.rs             # sample_corpus_tests.rs
│   └── mod.rs
│
├── stress/                          # Stress & edge case tests
│   ├── edge_cases.rs                # edge_case_tests.rs
│   ├── edge_cases_extended.rs       # edge_case_tests_extended.rs
│   ├── stress.rs                    # stress_tests.rs
│   └── mod.rs
│
└── common/                          # Shared test utilities (if needed)
    └── mod.rs
```

## Renaming Strategy

### Tests: Milestone → Descriptive Names

| Old Name | New Name | Rationale |
|----------|----------|-----------|
| milestone3_schema_catalog_tests.rs | semantic/schema_integration.rs | Describes what it tests (schema catalog integration), not when it was written |
| milestone4_aggregate_validation_tests.rs | semantic/aggregate_validation.rs | Focuses on aggregate validation functionality |
| milestone4_callable_catalog_tests.rs | semantic/callable_validation.rs | Focuses on callable/function validation |
| milestone5_type_inference_tests.rs | semantic/type_inference.rs | Describes type inference testing |
| integration_type_inference.rs | integration/type_inference.rs | Already descriptive, just moves to proper folder |

### Examples: Milestone → Feature Names

| Old Name | New Name | Rationale |
|----------|----------|-----------|
| milestone3_schema_catalog.rs | schema_catalog_usage.rs | Shows how to use schema catalog |
| milestone3_advanced_features.rs | advanced_graph_features.rs | Demonstrates advanced graph pattern features |
| milestone4_callable_catalog.rs | callable_catalog_usage.rs | Shows how to use callable catalog |
| milestone5_type_inference.rs | type_inference_usage.rs | Demonstrates type inference API |

## Implementation Steps

### Phase 1: Create Directory Structure
1. Create subdirectories: `tests/{parser,semantic,integration,conformance,stress,common}`
2. Create `mod.rs` files in each subdirectory

### Phase 2: Move and Rename Parser Tests
3. Move parser tests to `tests/parser/` with shortened names
4. Update `tests/parser/mod.rs` to declare modules

### Phase 3: Move and Rename Semantic Tests
5. Move semantic tests to `tests/semantic/`
6. Rename milestone tests to descriptive names
7. Update `tests/semantic/mod.rs` to declare modules

### Phase 4: Move Integration, Conformance, and Stress Tests
8. Move integration tests to `tests/integration/`
9. Move conformance tests to `tests/conformance/`
10. Move stress/edge tests to `tests/stress/`
11. Update respective `mod.rs` files

### Phase 5: Update Examples
12. Rename milestone examples to descriptive names in `examples/`

### Phase 6: Verification
13. Run `cargo test` to ensure all tests still pass
14. Verify test discovery with `cargo test --list`
15. Check for any import issues

### Phase 7: Documentation
16. Update any documentation referencing old test names
17. Add a README.md in tests/ explaining the structure

## Additional Improvements

### 1. Consistent Test Naming Convention
- Use descriptive names that indicate what is being tested
- Avoid temporal markers (sprint, milestone, version numbers)
- Format: `{component}_{feature}` or `{feature}_tests.rs`

### 2. Test Module Organization
Each subdirectory should have a `mod.rs` that:
- Declares all test modules
- Provides shared test utilities (if needed)
- Documents the purpose of tests in that directory

Example `tests/semantic/mod.rs`:
```rust
//! Semantic validation and analysis tests
//!
//! This module contains tests for the semantic validator,
//! including type checking, scope analysis, and catalog integration.

mod validator;
mod scoping_and_aggregation;
mod procedure_definitions;
mod schema_integration;
mod aggregate_validation;
mod callable_validation;
mod type_inference;
```

### 3. Shared Test Utilities
If common test helpers exist, centralize them in `tests/common/mod.rs`:
- Test fixture builders
- Mock catalog implementations
- Assertion helpers
- Common test data

### 4. Test Documentation
Add module-level documentation to each test file:
```rust
//! Tests for [specific feature]
//!
//! This module tests [detailed description].
//!
//! Related source: src/[relevant_module]
```

### 5. Integration Test Strategy
Clarify the distinction:
- **Unit tests** (in `src/*/tests.rs` or `#[cfg(test)]` modules): Test individual functions/modules in isolation
- **Integration tests** (in `tests/`): Test public API and component interactions
- **Examples** (in `examples/`): Demonstrate usage patterns, not tests

### 6. CI/Test Organization Benefits
With this structure, CI can:
- Run specific test categories: `cargo test --test parser`
- Parallelize better with smaller test files
- Provide clearer failure reporting

## Migration Safety

### Pre-migration Checklist
- [ ] Ensure all tests currently pass: `cargo test`
- [ ] Create a backup/branch: `git checkout -b test-restructure`
- [ ] Document current test count: `cargo test -- --list | wc -l`

### Post-migration Verification
- [ ] All tests still pass: `cargo test`
- [ ] Test count matches: `cargo test -- --list | wc -l`
- [ ] No compilation errors
- [ ] Examples still compile: `cargo build --examples`
- [ ] Check for any dead code warnings indicating broken imports

## Timeline

This restructuring can be completed in a single session (~1-2 hours):
- Phase 1-2: 15 minutes
- Phase 3: 20 minutes
- Phase 4: 15 minutes
- Phase 5: 10 minutes
- Phase 6: 15 minutes
- Phase 7: 15 minutes

## Rollback Plan

If issues arise:
1. The changes are purely organizational (file moves/renames)
2. Git can easily revert: `git checkout main -- tests/`
3. No functional code changes, only structural

## Expected Benefits

1. **Clarity**: Developers can quickly locate relevant tests
2. **Maintainability**: New tests have clear places to go
3. **Descriptive naming**: Test names describe functionality, not history
4. **Scalability**: Structure supports growing test suite
5. **CI optimization**: Easier to run specific test categories
6. **Onboarding**: New contributors understand test organization

## Open Questions for User

1. Should `integration_type_inference.rs` stay as integration test or move to semantic tests?
2. Are there any additional test categories that should be created?
3. Should we consolidate `edge_case_tests.rs` and `edge_case_tests_extended.rs`?
4. Should example files also be organized into subdirectories?
5. Any specific naming conventions preferred for the project?

---

**Status**: Ready for implementation pending user approval
**Risk Level**: Low (purely organizational changes)
**Reversibility**: High (easily reverted via git)
