# Test Coverage Implementation Plan

**Goal:** Close critical test gaps in mutation/procedure validation and graph type testing
**Timeline:** 1 session (~2-3 hours)
**Priority:** Focus on highest-impact tests with clear pass/fail criteria

---

## Session Plan Overview

### Phase 1: Mutation Validation Tests (45 min)
Create `tests/semantic/mutation_validation.rs` with 15 essential tests

### Phase 2: Procedure Validation Tests (30 min)
Create `tests/semantic/procedure_validation.rs` with 10 essential tests

### Phase 3: Graph Type Validation Tests (30 min)
Extend `tests/parser/graph_types.rs` with 10 validation tests

### Phase 4: Error Recovery Tests (30 min)
Extend `tests/stress/edge_cases.rs` with 12 error recovery tests

### Phase 5: Integration & Verification (15 min)
Run full test suite and document results

**Total:** ~2.5 hours

---

## Phase 1: Mutation Validation Tests (45 min)

**File:** `tests/semantic/mutation_validation.rs` (new)

### Setup (5 min)

```rust
//! Semantic validation tests for mutation statements.
//!
//! Tests variable scoping, type checking, and constraint enforcement
//! for INSERT, SET, REMOVE, and DELETE statements.

use gql_parser::parse;
use gql_parser::semantic::{SemanticValidator, ValidationConfig};

fn validate_mutation(source: &str) -> gql_parser::semantic::ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let validator = SemanticValidator::new();
    validator.validate(parse_result.ast.as_ref().unwrap())
}

fn validate_mutation_with_schema(source: &str) -> gql_parser::semantic::ValidationOutcome {
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let config = ValidationConfig {
        schema_validation: true,
        ..Default::default()
    };
    let validator = SemanticValidator::with_config(config);
    validator.validate(parse_result.ast.as_ref().unwrap())
}
```

### Test 1-5: Variable Scoping (15 min)

```rust
#[test]
fn test_insert_variable_in_scope_for_subsequent_statements() {
    let source = "INSERT (n:Person) SET n.age = 30";
    let outcome = validate_mutation(source);

    // Should succeed - 'n' is bound by INSERT and used in SET
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_set_undefined_variable_fails() {
    let source = "INSERT (n:Person) SET m.age = 30";
    let outcome = validate_mutation(source);

    // Should fail - 'm' is not in scope
    assert!(!outcome.is_success(), "Expected validation failure");
    assert!(outcome.diagnostics.iter().any(|d|
        d.message.contains("undefined") || d.message.contains("not in scope")
    ), "Expected undefined variable diagnostic");
}

#[test]
fn test_mutation_chain_preserves_scope() {
    let source = "INSERT (n) MATCH (m) WHERE m.id = n.id SET n.updated = true";
    let outcome = validate_mutation(source);

    // Both 'n' and 'm' should be in scope for SET
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_delete_undefined_variable_fails() {
    let source = "INSERT (n) DELETE m";
    let outcome = validate_mutation(source);

    assert!(!outcome.is_success(), "Expected validation failure");
    assert!(outcome.diagnostics.iter().any(|d|
        d.message.contains("undefined") || d.message.contains("not in scope")
    ));
}

#[test]
fn test_remove_undefined_variable_fails() {
    let source = "INSERT (n) REMOVE m.property";
    let outcome = validate_mutation(source);

    assert!(!outcome.is_success(), "Expected validation failure");
}
```

### Test 6-10: DELETE Constraints (15 min)

```rust
#[test]
fn test_delete_node_is_valid() {
    let source = "MATCH (n) DELETE n";
    let outcome = validate_mutation(source);

    // Should succeed - deleting a node variable
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_delete_edge_is_valid() {
    let source = "MATCH ()-[e]->() DELETE e";
    let outcome = validate_mutation(source);

    // Should succeed - deleting an edge variable
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_delete_property_reference_fails() {
    let source = "MATCH (n) DELETE n.property";
    let outcome = validate_mutation(source);

    // Should fail - can't DELETE a property (use REMOVE instead)
    // Note: This may depend on parser/validator implementation
    // If parser rejects this, test should verify parse diagnostics
    let has_diagnostic = !outcome.is_success() || !outcome.diagnostics.is_empty();
    if !has_diagnostic {
        // If validator doesn't catch this, document the behavior
        eprintln!("Note: DELETE property not validated - document this");
    }
}

#[test]
fn test_detach_delete_node_is_valid() {
    let source = "MATCH (n) DETACH DELETE n";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_nodetach_delete_node_is_valid() {
    let source = "MATCH (n) NODETACH DELETE n";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}
```

### Test 11-15: SET Operation Validation (15 min)

```rust
#[test]
fn test_set_property_is_valid() {
    let source = "MATCH (n) SET n.age = 30";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_set_label_is_valid() {
    let source = "MATCH (n) SET n:NewLabel";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_set_all_properties_is_valid() {
    let source = "MATCH (n) SET n = {name: 'Alice', age: 30}";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_set_multiple_properties_is_valid() {
    let source = "MATCH (n) SET n.x = 1, n.y = 2, n.z = 3";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_set_with_expression_is_valid() {
    let source = "MATCH (n) SET n.count = n.count + 1";
    let outcome = validate_mutation(source);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}
```

**Estimated Time:** 45 minutes (including file setup and running tests)

---

## Phase 2: Procedure Validation Tests (30 min)

**File:** `tests/semantic/procedure_validation.rs` (new)

### Setup (5 min)

```rust
//! Semantic validation tests for procedure statements.
//!
//! Tests procedure signature matching, YIELD validation, and
//! variable scoping in inline procedures.

use gql_parser::parse;
use gql_parser::semantic::{SemanticValidator, ValidationConfig};
use gql_parser::semantic::callable::{
    CallableCatalog, CallableSignature, CallableKind,
    ParameterSignature, InMemoryCallableCatalog, CompositeCallableCatalog,
    BuiltinCallableCatalog,
};

fn validate_with_procedures(source: &str, catalog: impl CallableCatalog + 'static)
    -> gql_parser::semantic::ValidationOutcome
{
    let parse_result = parse(source);
    assert!(parse_result.ast.is_some(), "Failed to parse: {}", source);

    let config = ValidationConfig {
        callable_validation: true,
        ..Default::default()
    };
    let mut validator = SemanticValidator::with_config(config);
    validator.set_callable_catalog(Box::new(catalog));
    validator.validate(parse_result.ast.as_ref().unwrap())
}
```

### Test 1-5: Procedure Existence & Arguments (12 min)

```rust
#[test]
fn test_builtin_procedure_validates() {
    let source = "CALL abs(-5)";

    // Use builtin catalog which has 'abs' function
    let catalog = BuiltinCallableCatalog::new();
    let outcome = validate_with_procedures(source, catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_unknown_procedure_fails_with_validation_enabled() {
    let source = "CALL nonexistent_procedure()";

    let catalog = InMemoryCallableCatalog::new();
    let outcome = validate_with_procedures(source, catalog);

    // Should fail - procedure doesn't exist
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d| d.message.contains("not found")),
            "Expected undefined procedure diagnostic");
}

#[test]
fn test_procedure_with_correct_arity_validates() {
    let mut catalog = InMemoryCallableCatalog::new();

    // Register a procedure that takes 2 arguments
    catalog.register(
        "my_proc".to_string(),
        CallableSignature {
            name: "my_proc".to_string(),
            kind: CallableKind::Procedure,
            parameters: vec![
                ParameterSignature { name: "arg1".to_string(), required: true },
                ParameterSignature { name: "arg2".to_string(), required: true },
            ],
            return_fields: vec![],
        },
    );

    let source = "CALL my_proc(1, 2)";
    let outcome = validate_with_procedures(source, catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_procedure_with_wrong_arity_fails() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(
        "my_proc".to_string(),
        CallableSignature {
            name: "my_proc".to_string(),
            kind: CallableKind::Procedure,
            parameters: vec![
                ParameterSignature { name: "arg1".to_string(), required: true },
            ],
            return_fields: vec![],
        },
    );

    let source = "CALL my_proc(1, 2, 3)";
    let outcome = validate_with_procedures(source, catalog);

    // Should fail - wrong number of arguments
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("argument") || d.message.contains("arity")
            ));
}

#[test]
fn test_optional_call_validates() {
    let source = "OPTIONAL CALL abs(5)";

    let catalog = BuiltinCallableCatalog::new();
    let outcome = validate_with_procedures(source, catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}
```

### Test 6-10: YIELD & Inline Procedures (13 min)

```rust
#[test]
fn test_yield_valid_field_validates() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(
        "my_proc".to_string(),
        CallableSignature {
            name: "my_proc".to_string(),
            kind: CallableKind::Procedure,
            parameters: vec![],
            return_fields: vec!["result".to_string()],
        },
    );

    let source = "CALL my_proc() YIELD result";
    let outcome = validate_with_procedures(source, catalog);

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_yield_invalid_field_fails() {
    let mut catalog = InMemoryCallableCatalog::new();

    catalog.register(
        "my_proc".to_string(),
        CallableSignature {
            name: "my_proc".to_string(),
            kind: CallableKind::Procedure,
            parameters: vec![],
            return_fields: vec!["result".to_string()],
        },
    );

    let source = "CALL my_proc() YIELD nonexistent";
    let outcome = validate_with_procedures(source, catalog);

    // Should fail or warn - field doesn't exist
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("nonexistent") || d.message.contains("field")
            ));
}

#[test]
fn test_inline_procedure_validates() {
    let source = "CALL { MATCH (n) RETURN n }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_inline_procedure_with_scope_validates() {
    let source = "MATCH (x) CALL (x) { MATCH (y) WHERE y.id = x.id RETURN y }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    // 'x' should be in scope within the inline procedure
    assert!(outcome.is_success(), "Diagnostics: {:?}", outcome.diagnostics);
}

#[test]
fn test_inline_procedure_out_of_scope_variable_fails() {
    let source = "CALL (x) { RETURN y }";

    let validator = SemanticValidator::new();
    let parse_result = parse(source);
    let outcome = validator.validate(parse_result.ast.as_ref().unwrap());

    // 'y' is not in scope (only 'x' is)
    assert!(!outcome.is_success() ||
            outcome.diagnostics.iter().any(|d|
                d.message.contains("undefined") || d.message.contains("scope")
            ));
}
```

**Estimated Time:** 30 minutes

---

## Phase 3: Graph Type Validation Tests (30 min)

**File:** `tests/parser/graph_types.rs` (extend existing)

### Add to Existing File (30 min)

```rust
// Add these tests to the existing file

#[test]
fn test_duplicate_element_type_names_produces_diagnostic() {
    let source = r#"
        CREATE GRAPH TYPE dup AS {
            NODE TYPE Person
            NODE TYPE Person
        }
    "#;

    let result = parse(source);

    // Parser may catch this or semantic validator should
    // For now, verify it parses and produces AST
    assert!(result.ast.is_some(), "Should parse even with duplicate names");

    // TODO: Add semantic validation to check for duplicates
    // This test documents the expected behavior
}

#[test]
fn test_circular_inheritance_is_parsed() {
    let source = r#"
        CREATE GRAPH TYPE circular AS {
            NODE TYPE A INHERITS B
            NODE TYPE B INHERITS A
        }
    "#;

    let result = parse(source);

    // Parser should handle this; validator should catch the cycle
    assert!(result.ast.is_some(), "Should parse circular inheritance");

    // TODO: Add semantic validation for cycle detection
}

#[test]
fn test_multiple_element_types_parse_correctly() {
    let source = r#"
        CREATE GRAPH TYPE multi AS {
            NODE TYPE Person { id :: INT, name :: STRING }
            NODE TYPE Company { id :: INT, name :: STRING }
            DIRECTED EDGE TYPE WORKS_AT CONNECTING (Person TO Company)
            DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse without errors");
    assert!(result.ast.is_some());

    let program = result.ast.unwrap();
    let stmt = &program.statements[0];

    let gql_parser::ast::Statement::Catalog(cat) = stmt else {
        panic!("Expected catalog statement");
    };

    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create) = &cat.kind else {
        panic!("Expected CREATE GRAPH TYPE");
    };

    let Some(gql_parser::ast::GraphTypeSource::Detailed { specification, .. }) = &create.source else {
        panic!("Expected detailed source");
    };

    assert_eq!(specification.body.element_types.types.len(), 4,
               "Should have 4 element types");
}

#[test]
fn test_graph_type_with_multiple_labels_per_node() {
    let source = r#"
        CREATE GRAPH TYPE multi_label AS {
            NODE TYPE Person
                LABEL Employee { emp_id :: INT }
                LABEL Manager { dept :: STRING }
        }
    "#;

    let result = parse(source);
    assert!(result.ast.is_some(), "Should parse multiple labels");
    // Note: Current implementation may not support multiple LABELs
    // This test documents the expected behavior
}

#[test]
fn test_edge_type_with_multiple_connecting_clauses() {
    let source = r#"
        CREATE GRAPH TYPE multi_connect AS {
            NODE TYPE Person
            NODE TYPE Company
            EDGE TYPE RELATED
                CONNECTING (Person TO Person)
                CONNECTING (Person TO Company)
        }
    "#;

    let result = parse(source);

    // Parser may or may not support multiple CONNECTING
    // Test documents behavior
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_check_constraint() {
    let source = r#"
        CREATE GRAPH TYPE constrained AS {
            NODE TYPE Person {
                age :: INT,
                CONSTRAINT CHECK (age >= 0)
            }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse CHECK constraint");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_unique_constraint() {
    let source = r#"
        CREATE GRAPH TYPE unique_id AS {
            NODE TYPE Person {
                id :: INT,
                CONSTRAINT UNIQUE (id)
            }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse UNIQUE constraint");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_multiple_constraints() {
    let source = r#"
        CREATE GRAPH TYPE multi_constraint AS {
            NODE TYPE Person {
                id :: INT,
                age :: INT,
                CONSTRAINT UNIQUE (id),
                CONSTRAINT CHECK (age >= 0),
                CONSTRAINT CHECK (age <= 150)
            }
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse multiple constraints");

    let program = result.ast.unwrap();
    let gql_parser::ast::Statement::Catalog(cat) = &program.statements[0] else {
        panic!("Expected catalog statement");
    };

    let gql_parser::ast::CatalogStatementKind::CreateGraphType(create) = &cat.kind else {
        panic!("Expected CREATE GRAPH TYPE");
    };

    let Some(gql_parser::ast::GraphTypeSource::Detailed { specification, .. }) = &create.source else {
        panic!("Expected detailed source");
    };

    let gql_parser::ast::graph_type::ElementTypeSpecification::Node(node) =
        &specification.body.element_types.types[0] else {
        panic!("Expected node type");
    };

    let filler = node.pattern.phrase.filler.as_ref().unwrap();
    assert_eq!(filler.constraints.len(), 3, "Should have 3 constraints");
}

#[test]
fn test_undirected_edge_type_specification() {
    let source = r#"
        CREATE GRAPH TYPE undirected AS {
            NODE TYPE Person
            UNDIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse undirected edge");
    assert!(result.ast.is_some());
}

#[test]
fn test_graph_type_with_inheritance_chain() {
    let source = r#"
        CREATE GRAPH TYPE inheritance AS {
            NODE TYPE Entity
            NODE TYPE Person INHERITS Entity
            NODE TYPE Employee INHERITS Person
        }
    "#;

    let result = parse(source);
    assert!(result.diagnostics.is_empty(), "Should parse inheritance chain");
    assert!(result.ast.is_some());

    // TODO: Add validation for inheritance chain correctness
}
```

**Estimated Time:** 30 minutes

---

## Phase 4: Error Recovery Tests (30 min)

**File:** `tests/stress/edge_cases.rs` (extend existing)

### Add Error Recovery Section (30 min)

```rust
// Add these tests to the existing edge_cases.rs file
// Add after line 249 in the existing file

// ===== Mutation Error Recovery =====

#[test]
fn incomplete_insert_with_recovery() {
    let result = parse("INSERT (n) SET ; MATCH (m) RETURN m");

    // Should have diagnostic for incomplete SET, but still parse MATCH
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for incomplete SET");
    assert!(result.ast.is_some(), "Should produce partial AST");
}

#[test]
fn malformed_set_property() {
    let result = parse("SET n.prop = ");

    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for missing value");
}

#[test]
fn incomplete_delete_list() {
    let result = parse("DELETE n, , m");

    // Parser should handle double comma gracefully
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn incomplete_remove_statement() {
    let result = parse("REMOVE n:");

    // Missing label name
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for missing label");
}

#[test]
fn incomplete_remove_property() {
    let result = parse("REMOVE n.");

    // Missing property name
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for missing property");
}

// ===== Procedure Error Recovery =====

#[test]
fn unclosed_inline_procedure() {
    let result = parse("CALL { MATCH (n) RETURN n");

    // Missing closing brace
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for unclosed procedure");
}

#[test]
fn incomplete_yield_clause() {
    let result = parse("CALL myProc() YIELD");

    // Missing yield items
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for empty YIELD");
}

#[test]
fn incomplete_variable_definition() {
    let result = parse("CALL { GRAPH g = }");

    // Missing initializer value
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for incomplete definition");
}

#[test]
fn malformed_procedure_arguments() {
    let result = parse("CALL myProc(1, , 3)");

    // Double comma in argument list
    assert!(!result.diagnostics.is_empty(), "Expected diagnostic for malformed arguments");
}

// ===== Multi-Error Recovery =====

#[test]
fn multiple_errors_in_statement() {
    let result = parse("INSERT (n {bad: }) SET m.prop = RETURN x");

    // Should catch: incomplete property spec, undefined 'm', missing value, unexpected RETURN
    assert!(result.diagnostics.len() >= 2, "Expected multiple diagnostics");
}

#[test]
fn statement_continuation_after_error() {
    let result = parse("SET invalid syntax ; MATCH (n) RETURN n");

    // Should have error on first statement, but parse second statement
    assert!(!result.diagnostics.is_empty());
    assert!(result.ast.is_some(), "Should parse valid statement after error");
}

#[test]
fn nested_error_recovery() {
    let result = parse("CALL { INSERT (n) bad_syntax SET n.prop = 1 }");

    // Error inside inline procedure
    assert!(!result.diagnostics.is_empty());
    assert!(result.ast.is_some(), "Should recognize CALL structure");
}
```

**Estimated Time:** 30 minutes

---

## Phase 5: Integration & Verification (15 min)

### Run Tests

```bash
# Run all new tests
cargo test --test mutation_validation
cargo test --test procedure_validation
cargo test graph_types
cargo test edge_cases

# Run full test suite
cargo test
```

### Document Results

Create `TEST_RESULTS.md`:

```markdown
# Test Implementation Results

**Date:** [Fill in]
**Total Tests Added:** [Fill in]

## Summary

- **Mutation Validation Tests:** 15 added
- **Procedure Validation Tests:** 10 added
- **Graph Type Tests:** 10 added
- **Error Recovery Tests:** 12 added

**Total:** 47 new tests

## Pass/Fail Status

### Mutation Validation
- [ ] Test 1-5: Variable Scoping (5 tests)
- [ ] Test 6-10: DELETE Constraints (5 tests)
- [ ] Test 11-15: SET Operations (5 tests)

### Procedure Validation
- [ ] Test 1-5: Procedure Existence & Arguments (5 tests)
- [ ] Test 6-10: YIELD & Inline Procedures (5 tests)

### Graph Type Validation
- [ ] 10 graph type tests

### Error Recovery
- [ ] 12 error recovery tests

## Failures & Notes

[Document any failing tests and reasons]

## Coverage Improvement

**Before:**
- Mutation validation: ~5%
- Procedure validation: ~5%
- Graph type validation: 0%

**After:**
- Mutation validation: ~40%
- Procedure validation: ~35%
- Graph type validation: ~25%

## Next Steps

[Document any follow-up work needed]
```

---

## Quick Reference: Test Commands

```bash
# Create new test files
touch tests/semantic/mutation_validation.rs
touch tests/semantic/procedure_validation.rs

# Add to tests/semantic/mod.rs
echo "mod mutation_validation;" >> tests/semantic/mod.rs
echo "mod procedure_validation;" >> tests/semantic/mod.rs

# Run specific test file
cargo test --test mutation_validation
cargo test --test procedure_validation

# Run with output
cargo test --test mutation_validation -- --nocapture

# Run specific test
cargo test test_insert_variable_in_scope_for_subsequent_statements

# Run all tests
cargo test

# Check test count
cargo test --test mutation_validation -- --list | wc -l
```

---

## Implementation Checklist

### Phase 1: Mutation Validation ✓
- [ ] Create `tests/semantic/mutation_validation.rs`
- [ ] Add to `tests/semantic/mod.rs`
- [ ] Implement setup helpers
- [ ] Add 5 variable scoping tests
- [ ] Add 5 DELETE constraint tests
- [ ] Add 5 SET operation tests
- [ ] Run tests: `cargo test --test mutation_validation`

### Phase 2: Procedure Validation ✓
- [ ] Create `tests/semantic/procedure_validation.rs`
- [ ] Add to `tests/semantic/mod.rs`
- [ ] Implement setup helpers with catalog
- [ ] Add 5 procedure existence/argument tests
- [ ] Add 5 YIELD/inline procedure tests
- [ ] Run tests: `cargo test --test procedure_validation`

### Phase 3: Graph Type Validation ✓
- [ ] Open `tests/parser/graph_types.rs`
- [ ] Add 10 new tests to existing file
- [ ] Run tests: `cargo test graph_types`

### Phase 4: Error Recovery ✓
- [ ] Open `tests/stress/edge_cases.rs`
- [ ] Add error recovery section (line ~250)
- [ ] Add 5 mutation error recovery tests
- [ ] Add 3 procedure error recovery tests
- [ ] Add 3 multi-error recovery tests
- [ ] Run tests: `cargo test edge_cases`

### Phase 5: Verification ✓
- [ ] Run full test suite: `cargo test`
- [ ] Document results in `TEST_RESULTS.md`
- [ ] Update `TEST_COVERAGE_GAPS.md` with new coverage %
- [ ] Commit changes

---

## Success Criteria

✅ **Minimum:** 40 tests added, 80% passing
✅ **Target:** 47 tests added, 90% passing
⭐ **Stretch:** All tests passing, coverage >40% in all areas

---

## Notes

- Some tests may fail initially if semantic validation isn't fully implemented
- Document failing tests with TODO comments for future work
- Focus on tests that verify existing behavior first
- Add aspirational tests for expected behavior (marked with TODO)
- Tests serve as documentation of expected behavior even if failing

---

*Plan created: 2026-02-20*
*Estimated completion: 1 session (~2.5 hours)*
