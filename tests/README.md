# Test Directory Structure

This document describes the organization and structure of the test suite for the GQL parser project.

## Directory Overview

The test suite is organized into component-based subdirectories, each containing tests for specific aspects of the parser and validator:

```
tests/
├── parser/                  # Parser-specific tests
├── semantic/                # Semantic validation tests
├── integration/             # Integration tests
├── conformance/             # Conformance & corpus tests
├── stress/                  # Stress & edge case tests
└── common/                  # Shared test utilities
```

## Test Categories

### Parser Tests (`tests/parser/`)

Tests for the GQL parser, covering syntax analysis and AST generation:

- **patterns.rs** - Pattern matching tests
- **queries.rs** - Query parsing tests
- **mutations.rs** - Mutation parsing tests
- **procedures.rs** - Procedure parsing tests
- **aggregates.rs** - Aggregate function parsing tests
- **graph_types.rs** - Graph type parsing tests
- **type_references.rs** - Type reference specification tests
- **case_insensitivity.rs** - Case-insensitive keyword tests

### Semantic Tests (`tests/semantic/`)

Tests for the semantic validator, type checking, and scope analysis:

- **validator.rs** - Core semantic validation tests
- **scoping_and_aggregation.rs** - Scope analysis and aggregation context tests
- **procedure_definitions.rs** - Procedure definition validation tests
- **schema_integration.rs** - Schema catalog integration tests
- **aggregate_validation.rs** - Aggregate function validation tests
- **callable_validation.rs** - Callable/function validation tests
- **type_inference.rs** - Type inference system tests

### Integration Tests (`tests/integration/`)

End-to-end tests that verify component interactions:

- **type_inference.rs** - Type inference integration tests

### Conformance Tests (`tests/conformance/`)

Tests that verify conformance to GQL standards and test against sample corpora:

- **matrix.rs** - Conformance matrix tests
- **iso_procedures.rs** - ISO/IEC procedure conformance tests
- **sample_corpus.rs** - Sample query corpus tests

### Stress Tests (`tests/stress/`)

Tests for edge cases, boundary conditions, and stress testing:

- **edge_cases.rs** - Comprehensive edge case tests (consolidated)
- **stress.rs** - Stress tests for parser and validator

### Common Utilities (`tests/common/`)

Shared test helpers, fixtures, and utilities used across multiple test modules. See [common/README.md](common/README.md) for detailed documentation.

**All test files now use these common utilities**, eliminating duplicate helper functions across the codebase.

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Tests by Category
```bash
# Run only parser tests
cargo test --test parser

# Run only semantic tests
cargo test --test semantic

# Run only integration tests
cargo test --test integration

# Run only conformance tests
cargo test --test conformance

# Run only stress tests
cargo test --test stress
```

### Run Specific Test Module
```bash
# Run pattern tests
cargo test parser::patterns

# Run type inference tests
cargo test semantic::type_inference
```

### List All Tests
```bash
cargo test -- --list
```

## Test Naming Conventions

### General Guidelines
- Use descriptive names that indicate what is being tested
- Avoid temporal markers (sprint, milestone, version numbers) in test names
- Test function names should follow the pattern: `test_<feature>_<scenario>`
- Use snake_case for test function names

### Module Organization
Each test module should include:
- Module-level documentation (`//!`) describing its purpose
- Clear test function names that describe the scenario being tested
- Helper functions and fixtures at the bottom of the file
- Grouped tests using `mod` blocks for related scenarios (optional)

### Example Test Module Structure
```rust
//! Tests for [specific feature]
//!
//! This module tests [detailed description].
//!
//! Related source: src/[relevant_module]

#[test]
fn test_basic_functionality() {
    // Test implementation
}

#[test]
fn test_error_handling() {
    // Test implementation
}

// Helper functions
fn create_test_fixture() -> TestData {
    // Helper implementation
}
```

## Adding New Tests

When adding new tests, follow these guidelines:

1. **Choose the Right Directory**: Place tests in the directory that best matches the component being tested
2. **Follow Naming Conventions**: Use descriptive names that indicate functionality, not chronology
3. **Add Module Documentation**: Include module-level documentation explaining the purpose
4. **Update mod.rs**: Add the new module to the appropriate `mod.rs` file
5. **Write Clear Test Names**: Test function names should clearly describe the scenario
6. **Group Related Tests**: Use nested `mod` blocks for related test scenarios if needed

## Test Statistics

Current test count: 281 tests passing

## Historical Context

This test structure was reorganized to improve maintainability and clarity. Previously, tests were named with milestone prefixes (e.g., `milestone3_*`, `milestone4_*`) which reflected when they were written rather than what they test. The current structure uses descriptive, component-based naming that makes it easier to locate and understand tests.

### Renamed Tests
- `milestone3_schema_catalog_tests.rs` → `semantic/schema_integration.rs`
- `milestone4_aggregate_validation_tests.rs` → `semantic/aggregate_validation.rs`
- `milestone4_callable_catalog_tests.rs` → `semantic/callable_validation.rs`
- `milestone5_type_inference_tests.rs` → `semantic/type_inference.rs`

## CI/CD Integration

The organized test structure enables:
- **Parallel execution**: Run test categories in parallel for faster CI
- **Selective testing**: Run only affected test categories based on changed files
- **Clear failure reporting**: Failures are organized by component
- **Better coverage analysis**: Track coverage by component

## Contributing

When contributing tests:
1. Follow the existing directory structure
2. Use descriptive names that describe functionality
3. Add appropriate documentation
4. Ensure tests pass locally before submitting: `cargo test`
5. Consider adding both positive and negative test cases
6. Include edge cases and boundary conditions where appropriate

## Related Documentation

- [Examples Directory](../examples/README.md) - Usage examples and demonstrations
- [Source Documentation](../src/README.md) - Source code organization
- [Contributing Guide](../CONTRIBUTING.md) - General contribution guidelines

---

**Last Updated**: 2026-02-20
**Test Count**: 281 tests
**Structure Version**: 2.0
