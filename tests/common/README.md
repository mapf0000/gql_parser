# Common Test Utilities

This module provides shared test helpers and utilities to reduce code duplication across test files.

## Quick Reference

### Diagnostic Helpers

```rust
use crate::common::*;

// Format diagnostics for display
let diag_text = format_diagnostics(&result.diagnostics);

// Assert no parse errors
assert_no_parse_errors(&result, source);

// Assert validation succeeded
assert_no_validation_errors(&outcome);

// Assert specific error message
assert_has_error_containing(&outcome, "undefined variable");

// Assert any error exists
assert_has_any_error(&outcome);
```

### Parsing Helpers

```rust
use crate::common::*;

// Assert parsing succeeds (don't need AST)
assert_parses_cleanly("MATCH (n) RETURN n");

// Parse and get AST (panics on errors)
let program = parse_cleanly("MATCH (n) RETURN n");

// Tokenize source (panics on errors)
let tokens = tokenize_cleanly("MATCH (n)");
```

### Combined Parse + Validate Helpers

```rust
use crate::common::*;

// Parse and validate in one step
let outcome = parse_and_validate("MATCH (n:Person) RETURN n");
assert_no_validation_errors(&outcome);

// Parse and validate with custom validator
let validator = SemanticValidator::new().with_strict_mode(true);
let outcome = parse_and_validate_with(source, &validator);

// Assert validation fails (for negative tests)
parse_and_expect_failure("RETURN undefined_var");

// Assert validation succeeds (convenience)
parse_and_expect_success("MATCH (n) RETURN n");
```

## Migration Examples

### Before: Duplicated Helper Functions

```rust
// OLD: Each test file had its own copy
fn diagnostics_text(diags: &[miette::Report]) -> String {
    diags
        .iter()
        .map(|diag| format!("{diag:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_ok(source: &str) -> Program {
    let result = parse(source);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result.ast.expect("expected AST")
}
```

### After: Use Common Utilities

```rust
// NEW: Import from common module
use crate::common::*;

// Then use directly:
let program = parse_cleanly(source);
let diag_text = format_diagnostics(&diagnostics);
```

## Benefits

1. **Less Duplication**: Common patterns defined once
2. **Consistency**: All tests use the same helpers
3. **Better Errors**: Helpers provide clear error messages
4. **Easier Maintenance**: Fix bugs in one place
5. **Clearer Intent**: Descriptive function names

## Available Functions

### Diagnostic Functions

| Function | Purpose |
|----------|---------|
| `format_diagnostics()` | Format diagnostics for display |
| `assert_no_parse_errors()` | Assert parsing succeeded |
| `assert_no_validation_errors()` | Assert validation succeeded |
| `assert_has_error_containing()` | Assert specific error message |
| `assert_has_any_error()` | Assert any error exists |

### Parsing Functions

| Function | Purpose |
|----------|---------|
| `assert_parses_cleanly()` | Assert parse succeeds (no AST needed) |
| `parse_cleanly()` | Parse and return AST (panic on error) |
| `tokenize_cleanly()` | Tokenize and return tokens (panic on error) |

### Combined Functions

| Function | Purpose |
|----------|---------|
| `parse_and_validate()` | Parse + validate in one step |
| `parse_and_validate_with()` | Parse + validate with custom validator |
| `parse_and_expect_failure()` | Parse + validate, assert failure |
| `parse_and_expect_success()` | Parse + validate, assert success |

## Usage Statistics

Based on analysis of the test suite:

- **`diagnostics_text()` pattern**: Found in 6 files → Use `format_diagnostics()`
- **`parse_ok()` pattern**: Found in 3 files, 149+ uses → Use `parse_cleanly()`
- **Tokenization pattern**: Found in 4 files, 45+ uses → Use `tokenize_cleanly()`
- **Validation pattern**: Found in 7 files, 87+ uses → Use `parse_and_validate()`

## Future Enhancements

Potential additions based on patterns identified:

1. **Validator Fixtures**: Pre-configured validators with common catalogs
2. **Schema Fixtures**: Standard test schemas (Person, Company, etc.)
3. **Sample File Loading**: Helpers for loading test corpus files
4. **Pattern Extraction**: Helpers for extracting specific AST nodes

## See Also

- [Test Directory Structure](../README.md) - Overview of test organization
- Module documentation: `cargo doc --open --document-private-items`
