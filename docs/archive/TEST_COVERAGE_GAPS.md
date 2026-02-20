# Test Coverage Gap Analysis

**Date:** 2026-02-20
**Analysis Scope:** Mutation/Procedure parsing, Error recovery, Graph type specifications

---

## Executive Summary

This analysis identified **critical gaps** in test coverage across three key areas of the GQL parser:

1. **Mutation/Procedure Validation**: No semantic validation tests exist despite having comprehensive parsing tests
2. **Error Recovery**: Limited tests for malformed mutations/procedures and continuation after errors
3. **Graph Type Specifications**: Basic parsing tests only; no validation or error handling tests

**Impact:** These gaps leave significant portions of the codebase untested, particularly semantic validation logic and error recovery paths.

---

## 1. Mutation/Procedure Statement Parsing and Validation

### Current Coverage

#### Mutation Parsing Tests
**Location:** [tests/parser/mutations.rs](tests/parser/mutations.rs)
**Count:** 12 tests

**Covered Scenarios:**
- Basic INSERT, SET, REMOVE, DELETE parsing
- DETACH DELETE with detach options
- Focused mutations with USE GRAPH clause
- Mutation chains with query steps
- Inline CALL statements within mutations
- Error cases: unclosed braces, empty property maps

**Example Tests:**
```rust
#[test]
fn parse_set_statement_is_mutation_start() // Line 10
fn parse_detach_delete_statement_is_mutation_start() // Line 24
fn parse_focused_use_graph_mutation() // Line 43
fn parse_mutation_chain_with_query_step_stays_single_statement() // Line 63
```

#### Procedure Parsing Tests
**Location:** [tests/parser/procedures.rs](tests/parser/procedures.rs)
**Count:** 25 tests

**Covered Scenarios:**
- CALL statements (inline and named)
- OPTIONAL CALL statements
- Variable scope clauses
- YIELD clauses with aliases
- Variable definitions (GRAPH, TABLE, VALUE)
- AT schema clauses
- Procedure argument lists

**Example Tests:**
```rust
#[test]
fn test_simple_named_procedure_call() // Line 16
fn test_optional_procedure_call() // Line 109
fn test_inline_procedure_call_with_variables() // Line 148
fn test_yield_item_with_alias() // Line 326
```

### Critical Gaps Identified

#### 1.1 No Mutation Semantic Validation Tests ❌

**Gap:** Zero tests validating semantic correctness of mutation statements.

**Missing Test Scenarios:**

```rust
// Variable scoping validation
"INSERT (n:Person) SET m.age = 30"
// Expected: Error - variable 'm' not in scope
// Actual: No test verifies this

"MATCH (n) INSERT (n)-[:KNOWS]->(m) RETURN n, m"
// Expected: Both 'n' and 'm' should be in scope for RETURN
// Actual: No test verifies scoping across mutation boundaries

// Type checking for SET operations
"MATCH (n:Person) SET n.age = 'not a number'"
// Expected: Type mismatch error if schema defines age as INT
// Actual: No test verifies type compatibility

"SET n.nonexistent = 1"
// Expected: Warning or error if schema validation enabled
// Actual: No test verifies property existence

// DELETE operation validation
"MATCH (n) DELETE n.property"
// Expected: Can only delete nodes/edges, not properties
// Actual: No test verifies this constraint

"DETACH DELETE e"
// Expected: DETACH only applies to nodes, not edges
// Actual: No test verifies this semantic rule
```

**Impact:** Semantic validation code in mutation parsing ([src/parser/mutation.rs](src/parser/mutation.rs)) is untested.

#### 1.2 No Procedure Statement Validation Tests ❌

**Gap:** Procedure signature matching and YIELD validation not tested.

**Missing Test Scenarios:**

```rust
// Procedure existence validation
"CALL nonexistent_procedure()"
// Expected: Error if callable validation enabled
// Actual: No test with catalog validation

// Argument count/type validation
"CALL known_proc(1, 2, 3)"
// Expected: Arity mismatch error if proc expects 2 args
// Actual: No test verifies argument count

"CALL string_proc(123)"
// Expected: Type mismatch if proc expects STRING
// Actual: No test verifies argument types

// YIELD clause validation
"CALL myProc() YIELD nonexistent_field"
// Expected: Error - procedure doesn't yield 'nonexistent_field'
// Actual: No test verifies yield field existence

"CALL myProc() YIELD x, x"
// Expected: Error - duplicate yield alias
// Actual: No test verifies uniqueness

// Variable scope in inline procedures
"CALL (x, y) { MATCH (n) RETURN z }"
// Expected: Error - 'z' not in scope (should use x or y)
// Actual: No test verifies inline procedure scoping

// Variable definition validation
"GRAPH g = undefined_graph_var"
// Expected: Error - reference to undefined variable
// Actual: No test verifies initializer validity

"VALUE x = y + 1"
// Expected: Error if 'y' not in scope
// Actual: No test verifies expression validity in initializers
```

**Impact:** Callable validation logic is partially tested for functions ([tests/semantic/callable_validation.rs](tests/semantic/callable_validation.rs)) but procedure-specific validation is missing.

#### 1.3 Limited Mutation Edge Cases

**Missing Scenarios:**

```rust
// Complex mutation chains
"INSERT (n) MATCH (m) WHERE m.id = n.id SET m.updated = true DELETE m"
// No test verifies scoping across multiple mutation/query steps

// Conflicting operations
"MATCH (n) SET n:NewLabel REMOVE n:NewLabel"
// No test verifies handling of conflicting label operations

// Circular dependencies
"INSERT (n)-[:REF]->(n)"
// No test for self-referential patterns

// Multiple CALL statements
"CALL proc1() CALL proc2() RETURN 1"
// Limited tests for procedure composition
```

### Evidence

**Search Results:**
```bash
# Mutation validation tests
grep -r "mutation.*validation\|INSERT.*validation" tests/
# Result: No files found

# Procedure validation tests (excluding callable functions)
grep -r "procedure.*statement.*validation" tests/
# Result: Only callable_validation.rs (functions, not procedures)
```

---

## 2. Error Recovery Scenario Testing

### Current Coverage

**Location:** [tests/stress/edge_cases.rs](tests/stress/edge_cases.rs)
**Count:** ~23 malformed input tests (lines 104-183)

**Covered Scenarios:**
- Unclosed string literals
- Unclosed delimited identifiers
- Unclosed parentheses, brackets, braces
- Unexpected EOF in basic contexts
- Invalid token sequences

**Example Tests:**
```rust
#[test]
fn unclosed_string_literal() // Line 107
fn unclosed_parenthesis() // Line 129
fn unexpected_eof() // Line 154
fn invalid_token_sequences() // Line 174
```

### Critical Gaps Identified

#### 2.1 Mutation Error Recovery ❌

**Gap:** No tests for recovering from malformed mutation statements.

**Missing Test Scenarios:**

```rust
// Incomplete INSERT patterns
"INSERT (n) SET"
// Expected: Error on missing SET item, but parser should recognize statement type
// Actual: No test verifies recovery behavior

"INSERT (n:Person {name:}) RETURN n"
// Expected: Error on incomplete property spec, but continue to parse RETURN
// Actual: No test verifies continuation after error

// Malformed DELETE
"DELETE n, , m"
// Expected: Error on double comma, recover to parse both variables
// Actual: No test for recovery from extra delimiters

"DETACH"
// Expected: Error - missing DELETE keyword
// Actual: No test for incomplete DETACH DELETE

// Invalid SET syntax
"SET n.prop ="
// Expected: Error on missing value expression
// Actual: No error recovery test

"SET n = {incomplete: }"
// Expected: Error in property spec, but recognize SET statement
// Actual: No test for partial property maps

// REMOVE edge cases
"REMOVE n:"
// Expected: Error on missing label name
// Actual: No test for incomplete label removal

"REMOVE n."
// Expected: Error on missing property name
// Actual: No test for incomplete property removal
```

#### 2.2 Procedure Error Recovery ❌

**Gap:** No tests for recovering from malformed procedure statements.

**Missing Test Scenarios:**

```rust
// Unclosed inline procedures
"CALL { MATCH (n)"
// Expected: Error on missing closing brace
// Actual: Test exists in mutations.rs line 128, but limited coverage

"CALL (x, y) { RETURN x"
// Expected: Error on unclosed procedure body
// Actual: No comprehensive test

// Invalid YIELD clauses
"CALL proc() YIELD"
// Expected: Error on missing yield items
// Actual: No test for empty YIELD

"CALL proc() YIELD x,"
// Expected: Error on trailing comma in yield list
// Actual: Test exists (line 377) but for arguments, not yield items

// Malformed variable definitions
"GRAPH g ="
// Expected: Error on missing initializer
// Actual: Tests check this (lines 203-214) but only for validation, not recovery

"VALUE x ::INT = 'string'"
// Expected: Type annotation + value mismatch
// Actual: No test for type annotation errors

// Invalid procedure references
"CALL /"
// Expected: Error on incomplete procedure reference
// Actual: No test for malformed references

// Missing argument lists
"CALL myProc"
// Expected: Error - missing required parentheses
// Actual: Test exists (line 353), but limited recovery testing
```

#### 2.3 Multi-Error Recovery ❌

**Gap:** No tests verifying parser continues after finding first error.

**Missing Test Scenarios:**

```rust
// Multiple errors in single statement
"INSERT (n {bad: }) SET m.prop RETURN x"
// Expected: Errors on incomplete property, undefined 'm', undefined 'x'
// Should produce multiple diagnostics
// Actual: No test verifies multiple error detection

// Statement continuation after errors
"SET invalid syntax here ; MATCH (n) RETURN n"
// Expected: Error on first statement, but second statement should parse
// Actual: No test verifies parser resynchronization at semicolon

// Nested error recovery
"CALL { INSERT (n) bad_syntax SET n.prop = 1 }"
// Expected: Error in inline procedure, but recognize outer CALL structure
// Actual: No test for nested error contexts
```

### Evidence

**Test File Statistics:**
- [tests/stress/edge_cases.rs](tests/stress/edge_cases.rs): ~250 lines
- Malformed input section: Lines 104-183 (~80 lines)
- Focus: Lexer and basic parser errors
- **Missing**: Complex statement error recovery

---

## 3. Graph Type Specifications

### Current Coverage

**Location:** [tests/parser/graph_types.rs](tests/parser/graph_types.rs)
**Count:** 8 tests

**Covered Scenarios:**
- Basic compilation check
- Empty graph type specification
- Property type specifications (empty)
- Label set specifications (single and multiple labels)
- Edge endpoint aliases
- Abstract node types with inheritance and constraints
- Abstract edge types with constraints and inheritance

**Example Tests:**
```rust
#[test]
fn test_basic_compilation() // Line 7
fn test_graph_type_parser_module_exists() // Line 19
fn test_label_set_specification_multiple_labels() // Line 75
fn test_graph_type_parser_supports_abstract_inheritance_and_constraints() // Line 154
```

### Critical Gaps Identified

#### 3.1 No Graph Type Validation Tests ❌

**Gap:** Graph type statements are parsed but semantic correctness is not validated.

**Missing Test Scenarios:**

```rust
// Inheritance cycle detection
"CREATE GRAPH TYPE circular AS {
    NODE TYPE A INHERITS B
    NODE TYPE B INHERITS A
}"
// Expected: Error - circular inheritance detected
// Actual: No test verifies cycle detection

// Duplicate element type names
"CREATE GRAPH TYPE dup AS {
    NODE TYPE Person
    NODE TYPE Person
}"
// Expected: Error - duplicate type name
// Actual: No test verifies uniqueness

// Invalid parent types
"CREATE GRAPH TYPE invalid AS {
    NODE TYPE Employee INHERITS NonExistent
}"
// Expected: Error - parent type doesn't exist
// Actual: No test verifies parent existence

// Property type consistency
"CREATE GRAPH TYPE inconsistent AS {
    NODE TYPE Person
        LABEL Person { age :: INT }
    NODE TYPE Employee INHERITS Person
        LABEL Employee { age :: STRING }
}"
// Expected: Error or warning - property type conflict with parent
// Actual: No test verifies property compatibility

// Constraint conflicts
"CREATE GRAPH TYPE conflict AS {
    NODE TYPE Person {
        id :: INT,
        CONSTRAINT UNIQUE (id),
        CONSTRAINT UNIQUE (id)
    }
}"
// Expected: Warning - duplicate constraint
// Actual: No test verifies constraint uniqueness
```

#### 3.2 Missing Complex Graph Type Scenarios

**Missing Test Scenarios:**

```rust
// Nested element type definitions
"CREATE GRAPH TYPE nested AS {
    NODE TYPE Person { id :: INT }
    DIRECTED EDGE TYPE KNOWS CONNECTING (Person TO Person)
    DIRECTED EDGE TYPE MANAGES CONNECTING (Person TO Person)
    NODE TYPE Company { name :: STRING }
    DIRECTED EDGE TYPE WORKS_AT CONNECTING (Person TO Company)
}"
// Expected: Parse multiple element types with dependencies
// Actual: Only 2 comprehensive tests exist

// Multiple CONNECTING clauses
"CREATE GRAPH TYPE multi AS {
    EDGE TYPE RELATED
        CONNECTING (Person TO Person)
        CONNECTING (Person TO Company)
        CONNECTING (Company TO Company)
}"
// Expected: Parse multiple endpoint combinations
// Actual: No test verifies multiple CONNECTING clauses

// Graph type references in queries
"MATCH (n :: MyGraphType.Person) RETURN n"
// Expected: Parse graph type qualified references
// Actual: No test verifies type references in queries

// Property graph reference types
"VALUE myGraph :: PROPERTY GRAPH<MyGraphType> = CURRENT_GRAPH"
// Expected: Parse property graph reference with type parameter
// Actual: No test verifies PROPERTY GRAPH REFERENCE value types

// Complex property types
"CREATE GRAPH TYPE complex AS {
    NODE TYPE Data {
        metadata :: RECORD<
            name :: STRING,
            tags :: LIST<STRING>,
            scores :: MAP<STRING, FLOAT>
        >
    }
}"
// Expected: Parse nested structured property types
// Actual: Limited tests for complex type structures
```

#### 3.3 No Graph Type Error Recovery ❌

**Gap:** No tests for malformed graph type syntax and recovery.

**Missing Test Scenarios:**

```rust
// Malformed element type definitions
"CREATE GRAPH TYPE bad AS {
    NODE TYPE
}"
// Expected: Error - missing type name
// Actual: No error recovery test

"CREATE GRAPH TYPE bad AS {
    NODE TYPE Person {
        prop ::
    }
}"
// Expected: Error - incomplete property type
// Actual: No test for malformed property specs

// Unclosed graph type specification
"CREATE GRAPH TYPE incomplete AS {
    NODE TYPE Person"
// Expected: Error - missing closing brace
// Actual: No test verifies recovery

// Invalid constraint syntax
"CREATE GRAPH TYPE invalid AS {
    NODE TYPE Person {
        CONSTRAINT UNIQUE
    }
}"
// Expected: Error - UNIQUE constraint requires column list
// Actual: No test for incomplete constraints

// Malformed CONNECTING clause
"CREATE GRAPH TYPE bad AS {
    EDGE TYPE KNOWS CONNECTING (Person TO)
}"
// Expected: Error - incomplete endpoint specification
// Actual: No test for malformed CONNECTING
```

### Evidence

**Test Statistics:**
- Total graph type tests: 8
- Focus: AST structure verification
- **Missing**: Semantic validation, error recovery, complex scenarios

**File Analysis:**
```rust
// tests/parser/graph_types.rs line counts:
// - Basic parsing: 3 tests (lines 7-36)
// - Property/label specs: 3 tests (lines 38-94)
// - Comprehensive parsing: 2 tests (lines 96-299)
// - Validation: 0 tests ❌
// - Error recovery: 0 tests ❌
```

---

## Summary Matrix

| Test Area | Parsing | Validation | Error Recovery | Priority |
|-----------|---------|------------|----------------|----------|
| **INSERT statements** | ✅ 12 tests | ❌ None | ❌ None | **HIGH** |
| **SET statements** | ✅ Covered | ❌ None | ❌ None | **HIGH** |
| **REMOVE statements** | ✅ Covered | ❌ None | ❌ None | **HIGH** |
| **DELETE statements** | ✅ Covered | ❌ None | ❌ None | **HIGH** |
| **CALL statements** | ✅ 25 tests | ❌ None | ⚠️ Limited | **HIGH** |
| **YIELD clauses** | ✅ Covered | ❌ None | ❌ None | **MEDIUM** |
| **Variable definitions** | ✅ Covered | ⚠️ Partial | ❌ None | **MEDIUM** |
| **Graph type specs** | ⚠️ 8 tests | ❌ None | ❌ None | **CRITICAL** |
| **Inheritance** | ✅ 2 tests | ❌ None | ❌ None | **HIGH** |
| **Constraints** | ✅ 2 tests | ❌ None | ❌ None | **HIGH** |

### Legend
- ✅ **Good coverage** (>15 tests or comprehensive scenarios)
- ⚠️ **Partial coverage** (basic tests exist, gaps remain)
- ❌ **No coverage** (0 tests or critical gaps)

---

## Recommended Actions

### Immediate Priority (Critical Gaps)

1. **Add mutation semantic validation tests** ([tests/semantic/mutation_validation.rs](tests/semantic/mutation_validation.rs) - new file)
   - Variable scoping across mutation boundaries
   - Type checking for SET operations
   - DELETE operation constraints

2. **Add graph type validation tests** ([tests/semantic/graph_type_validation.rs](tests/semantic/graph_type_validation.rs) - new file)
   - Inheritance cycle detection
   - Duplicate type name validation
   - Property type consistency checks

3. **Expand error recovery tests** ([tests/stress/mutation_error_recovery.rs](tests/stress/mutation_error_recovery.rs) - new file)
   - Incomplete mutation statements
   - Multi-error detection
   - Statement continuation after errors

### High Priority

4. **Add procedure validation tests** ([tests/semantic/procedure_validation.rs](tests/semantic/procedure_validation.rs) - new file)
   - Procedure existence checks
   - Argument type/count validation
   - YIELD clause validation

5. **Expand graph type parsing tests** ([tests/parser/graph_types.rs](tests/parser/graph_types.rs))
   - Complex nested element types
   - Multiple CONNECTING clauses
   - Graph type references in queries

### Medium Priority

6. **Add integration tests** ([tests/integration/mutation_validation.rs](tests/integration/mutation_validation.rs))
   - End-to-end validation with schema catalog
   - Multi-statement semantic checks
   - Cross-reference validation

---

## Test File References

### Existing Files to Extend
- [tests/parser/mutations.rs](tests/parser/mutations.rs) - Add validation scenarios
- [tests/parser/procedures.rs](tests/parser/procedures.rs) - Add validation scenarios
- [tests/parser/graph_types.rs](tests/parser/graph_types.rs) - Add complex scenarios and validation
- [tests/stress/edge_cases.rs](tests/stress/edge_cases.rs) - Add mutation/procedure error recovery

### New Files to Create
- `tests/semantic/mutation_validation.rs` - Mutation semantic validation
- `tests/semantic/procedure_validation.rs` - Procedure statement validation
- `tests/semantic/graph_type_validation.rs` - Graph type validation
- `tests/stress/mutation_error_recovery.rs` - Mutation error recovery
- `tests/stress/procedure_error_recovery.rs` - Procedure error recovery
- `tests/integration/mutation_validation.rs` - Integration tests

### Source Files with Untested Logic
- [src/parser/mutation.rs](src/parser/mutation.rs) - 1665 lines, semantic validation untested
- [src/parser/procedure.rs](src/parser/procedure.rs) - 1954 lines, validation untested
- [src/parser/graph_type.rs](src/parser/graph_type.rs) - Validation logic untested

---

## Metrics

**Total Test Files Analyzed:** 27
**Mutation/Procedure Tests:** 37 (parsing only)
**Graph Type Tests:** 8 (parsing only)
**Error Recovery Tests:** 23 (basic scenarios)
**Validation Tests:** 0 for mutations/procedures, 0 for graph types

**Estimated Coverage:**
- Mutation/Procedure Parsing: 70%
- Mutation/Procedure Validation: **5%** ⚠️
- Error Recovery: **30%** ⚠️
- Graph Type Parsing: 40%
- Graph Type Validation: **0%** ❌

---

*Generated: 2026-02-20*
*Analysis Tool: Manual code review and grep-based search*
