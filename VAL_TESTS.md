# Additional Validator Tests Plan

## Context

The GQL parser has a mature semantic validator with comprehensive coverage across 8+ validation passes. However, after analyzing the existing test suite, the GQL grammar specification, and the validator implementation, I've identified significant gaps in test coverage that should be addressed.

**Current test coverage** includes:
- Basic validator tests (79 tests in `validator.rs`)
- Aggregate validation (7 tests)
- Callable validation (19 tests)
- Procedure validation (10 tests)
- Mutation validation (15 tests)

**Known gaps** from documentation:
- Type table persistence not tested
- Incomplete mutation validation paths
- CALL argument/yield validation incomplete
- Nested EXISTS/subquery validation gaps
- CASE expression type consistency partially tested
- Optional MATCH schema traversal not implemented

## Recommended Additional Tests

Based on the GQL grammar analysis and validator implementation gaps, here are the additional test categories that should be implemented:

---

### 1. **Path Pattern Validation Tests** (HIGH PRIORITY)

**Rationale**: The GQL grammar defines extensive path pattern syntax including quantifiers, path modes, and path search, but current tests only cover basic connectivity.

**Test Coverage Needed**:

#### A. Path Quantifiers
- Fixed quantifier: `(a)-[e:KNOWS]->{3}(b)` - exactly 3 hops
- Range quantifier: `(a)-[e:KNOWS]->{2,5}(b)` - 2 to 5 hops
- Star quantifier: `(a)-[e:KNOWS]->*(b)` - zero or more
- Plus quantifier: `(a)-[e:KNOWS]->+(b)` - one or more
- Question quantifier: `(a)-[e:KNOWS]->?(b)` - zero or one
- Invalid quantifier ranges: `{5,2}` (upper < lower) should error
- Negative quantifiers: `{-1}` should error

#### B. Path Modes
- WALK mode: `(a) -[WALK :KNOWS]->+ (b)` - allows repeated edges/nodes
- TRAIL mode: `(a) -[TRAIL :KNOWS]->+ (b)` - no repeated edges
- SIMPLE mode: `(a) -[SIMPLE :KNOWS]->+ (b)` - no repeated nodes (except endpoints)
- ACYCLIC mode: `(a) -[ACYCLIC :KNOWS]->+ (b)` - no repeated nodes at all

#### C. Path Search
- ALL paths: `(a) -[ALL :KNOWS]->+ (b)`
- ANY path: `(a) -[ANY :KNOWS]->+ (b)`
- SHORTEST path: `(a) -[SHORTEST :KNOWS]->+ (b)`
- Validation of mode/search combinations

#### D. Path Variables
- Path binding: `p = (a)-[:KNOWS]->(b)`
- Path length function: `PATH_LENGTH(p)`
- Elements extraction: `ELEMENTS(p)`
- Path in WHERE: `WHERE PATH_LENGTH(p) > 2`

---

### 2. **Label Expression Validation Tests** (HIGH PRIORITY)

**Rationale**: GQL supports complex label expressions with logical operators, but tests don't cover these scenarios.

**Test Coverage Needed**:

#### A. Label Disjunction (OR)
- `(n:Person|Company)` - node with Person OR Company label
- `(n:Person|Employee|Student)` - multiple OR
- Schema validation: at least one label should exist

#### B. Label Conjunction (AND)
- `(n:Person&Employee)` - node with BOTH labels
- Schema validation: all labels should exist

#### C. Label Negation (NOT)
- `(n:!Robot)` - node without Robot label
- `(n:Person&!Employee)` - Person but not Employee

#### D. Wildcard
- `(n:%)` - any label
- `-[e:%]->` - any edge type

#### E. Complex Combinations
- `(n:Person|Company&Active)` - precedence testing
- Invalid expressions: empty labels, malformed syntax

---

### 3. **Subquery and EXISTS Validation Tests** (HIGH PRIORITY)

**Rationale**: Documentation explicitly mentions "Nested EXISTS/subquery reference checks missing". Current tests are minimal.

**Test Coverage Needed**:

#### A. EXISTS Predicate with Patterns
- `EXISTS { (n)-[:KNOWS]->(m) }` - basic EXISTS
- `EXISTS { (n)-[:KNOWS]->(m) WHERE m.age > 30 }` - with filter
- Variable scope: inner pattern should see outer variables
- Variable isolation: outer query shouldn't see EXISTS-local variables

#### B. Nested EXISTS
- `EXISTS { (n)-[:KNOWS]->(m) WHERE EXISTS { (m)-[:LIKES]->(x) } }`
- Deep nesting validation
- Variable scope across nesting levels

#### C. Scalar Subqueries
- `LET count = (SELECT COUNT(*) FROM (n)-[:KNOWS]->(m))`
- Subquery returning single value
- Type inference for subquery results

#### D. List Subqueries
- `LET friends = [SELECT m FROM (n)-[:KNOWS]->(m)]`
- Subquery returning list
- List type inference

#### E. Cross-References
- Outer variables referenced in subquery
- Undefined variables in subquery
- Type compatibility between outer and inner scopes

---

### 4. **Type System Validation Tests** (HIGH PRIORITY)

**Rationale**: GQL has an extensive type system (60+ types) but type checking tests are limited.

**Test Coverage Needed**:

#### A. Temporal Types
- DATE literals and operations
- TIME with/without timezone
- DATETIME, TIMESTAMP
- DURATION (YEAR TO MONTH, DAY TO SECOND)
- CURRENT_DATE, CURRENT_TIME, CURRENT_TIMESTAMP functions
- DURATION_BETWEEN function
- Invalid temporal operations (e.g., DATE + INT)

#### B. Numeric Type Compatibility
- INT8/16/32/64/128/256 operations
- UINT8/16/32/64/128/256 (unsigned)
- FLOAT16/32/64/128/256, REAL, DOUBLE
- Mixed signed/unsigned arithmetic
- Overflow detection (if applicable)
- Type promotion rules (e.g., INT32 + FLOAT64)

#### C. String Types
- STRING, CHAR, VARCHAR operations
- String concatenation `||`
- String functions: UPPER, LOWER, TRIM, LTRIM, RTRIM, BTRIM
- CHAR_LENGTH, BYTE_LENGTH
- NORMALIZE (NFC, NFD, NFKC, NFKD)
- Invalid operations on strings

#### D. Collection Types
- LIST/ARRAY construction: `[1, 2, 3]`
- ARRAY type with max length
- Nested lists: `[[1, 2], [3, 4]]`
- CARDINALITY, SIZE functions
- List element access
- Type homogeneity in lists

#### E. RECORD Types
- Record construction: `{name: 'John', age: 30}`
- Field access
- Nested records
- Field type validation

#### F. CAST Validation
- Valid casts: `CAST(x AS INT64)`
- Invalid casts: `CAST('hello' AS INT)` should fail at runtime or validate
- Null handling in casts
- Cast between numeric types

#### G. Dynamic Types
- ANY VALUE type
- PROPERTY VALUE type
- Closed dynamic unions with `|`
- IS TYPED predicate

---

### 5. **Aggregation and GROUP BY Validation Tests** (MEDIUM PRIORITY)

**Rationale**: Documentation mentions "Aggregation/Grouping (P1-2) - strict-mode checks not fully enforced".

**Test Coverage Needed**:

#### A. GROUP BY Semantics
- Non-aggregated expressions must be in GROUP BY
  ```gql
  SELECT n.country, AVG(n.age)  -- Error: n.country not in GROUP BY
  ```
- Valid GROUP BY:
  ```gql
  SELECT n.country, AVG(n.age) GROUP BY n.country
  ```
- Multiple GROUP BY columns
- Expression in GROUP BY

#### B. HAVING Clause
- HAVING with GROUP BY
- HAVING without GROUP BY (should error for non-aggregated)
- Aggregates in HAVING: `HAVING COUNT(*) > 5`
- Non-aggregated expressions in HAVING

#### C. Nested Aggregation Detection
- `COUNT(SUM(x))` - nested aggregation should error
- `AVG(MAX(y))` - nested aggregation should error
- Valid: `AVG(x) + SUM(y)` - multiple aggregates at same level

#### D. Aggregation Context
- WHERE clause: aggregates not allowed
  ```gql
  WHERE COUNT(*) > 5  -- Error: use HAVING instead
  ```
- SELECT/RETURN: aggregates allowed
- ORDER BY: aggregates allowed with GROUP BY
- GROUP BY: aggregates not allowed

#### E. Aggregate Function Variations
- COUNT(*) vs COUNT(expr)
- COUNT(DISTINCT expr)
- Statistical aggregates: STDDEV_SAMP, STDDEV_POP
- Percentiles: PERCENTILE_CONT, PERCENTILE_DISC
- COLLECT_LIST for list aggregation

---

### 6. **Set Operations Validation Tests** (MEDIUM PRIORITY)

**Rationale**: GQL supports UNION, EXCEPT, INTERSECT but no tests exist for these.

**Test Coverage Needed**:

#### A. UNION Operations
- Basic UNION: two queries with same schema
- UNION ALL vs UNION (distinct)
- Column count mismatch: should error
- Column type mismatch: should error or coerce
- UNION of 3+ queries

#### B. EXCEPT Operations
- Basic EXCEPT
- Column compatibility validation
- Multiple EXCEPT operations

#### C. INTERSECT Operations
- Basic INTERSECT
- Column compatibility validation

#### D. Combination
- `(A UNION B) EXCEPT C`
- Precedence and associativity
- Parenthesized set operations

---

### 7. **Mutation Validation Tests** (MEDIUM PRIORITY)

**Rationale**: Documentation mentions "Mutation support partial". Current tests have gaps.

**Test Coverage Needed**:

#### A. INSERT Validation
- INSERT with pattern connectivity (already tested)
- INSERT with property expressions: `INSERT (n {name: 'John', age: 30})`
- INSERT with computed properties: `INSERT (n {score: m.score * 2})`
- INSERT with undefined variables in properties
- INSERT with type mismatches in properties
- INSERT edges with valid endpoints
- INSERT edges with undefined endpoints

#### B. SET Validation (expand current)
- SET property: `SET n.name = 'John'`
- SET multiple properties: `SET n.name = 'John', n.age = 30`
- SET all properties: `SET n = {name: 'John', age: 30}`
- SET label: `SET n:NewLabel`
- SET with undefined variable
- SET with type mismatch
- SET with invalid property reference

#### C. REMOVE Validation
- REMOVE property: `REMOVE n.name`
- REMOVE label: `REMOVE n:Label`
- REMOVE multiple: `REMOVE n.name, n.age`
- REMOVE with undefined variable
- REMOVE non-existent property (with schema)

#### D. DELETE Validation (expand current)
- DELETE node: `DELETE n`
- DELETE edge: `DELETE e`
- DETACH DELETE: `DETACH DELETE n` (deletes with relationships)
- NODETACH DELETE: `NODETACH DELETE n` (fails if has relationships)
- DELETE with undefined variable
- DELETE with property reference (should error)

#### E. Mutation Chaining
- Multiple mutations in sequence
- Variable scope across mutations
- INSERT then SET: `INSERT (n) SET n.name = 'John'`

---

### 8. **CASE Expression Validation Tests** (MEDIUM PRIORITY)

**Rationale**: Current tests are basic. GQL supports simple and searched CASE with type consistency requirements.

**Test Coverage Needed**:

#### A. Simple CASE
- `CASE n.status WHEN 'active' THEN 1 WHEN 'inactive' THEN 0 END`
- Type consistency: all THEN branches same type
- Type mismatch: `WHEN 'active' THEN 1 WHEN 'inactive' THEN 'zero'` (mixed INT/STRING)
- ELSE clause type compatibility

#### B. Searched CASE
- `CASE WHEN n.age < 18 THEN 'minor' WHEN n.age < 65 THEN 'adult' ELSE 'senior' END`
- Boolean conditions validation
- Type consistency across branches

#### C. Nested CASE
- CASE within CASE
- CASE in different contexts (SELECT, WHERE, SET)

#### D. NULL Handling
- CASE with NULL values
- NULL propagation in CASE

---

### 9. **Procedure Call Validation Tests** (MEDIUM PRIORITY)

**Rationale**: Documentation mentions "CALL argument and yield validation incomplete".

**Test Coverage Needed**:

#### A. Named Procedure Calls
- Correct arity
- Wrong arity
- Undefined procedure
- Argument type validation (with metadata)

#### B. YIELD Clause
- YIELD valid field
- YIELD invalid field
- YIELD with renaming: `YIELD field AS alias`
- YIELD multiple fields
- YIELD * (all fields)

#### C. Inline Procedure Calls
- Variable scope within inline procedure
- Out-of-scope variable reference
- Return value handling

#### D. Optional CALL
- Optional procedure success
- Optional procedure failure handling
- Variable scope after optional call

---

### 10. **Predicate Validation Tests** (LOW PRIORITY)

**Rationale**: GQL has many specialized predicates beyond basic comparisons.

**Test Coverage Needed**:

#### A. NULL Predicates
- `IS NULL`, `IS NOT NULL`
- NULL in expressions

#### B. Type Predicates
- `IS TYPED` predicate
- Type checking in predicates

#### C. Graph-Specific Predicates
- `DIRECTED(e)` - check if edge is directed
- `LABELED(n, 'Person')` - check label
- `PROPERTY_EXISTS(n, 'name')` - check property
- `SOURCE(e)`, `DESTINATION(e)` - edge endpoints
- `ALL_DIFFERENT(a, b, c)` - all distinct
- `SAME(a, b)` - same element

---

### 11. **Catalog and Session Validation Tests** (LOW PRIORITY)

**Rationale**: GQL has catalog operations but limited validation tests.

**Test Coverage Needed**:

#### A. CREATE SCHEMA
- Valid schema creation
- IF NOT EXISTS semantics
- Duplicate schema error

#### B. DROP SCHEMA
- Valid schema drop
- IF EXISTS semantics
- Non-existent schema error

#### C. CREATE GRAPH
- Open vs closed graph types
- Graph type definitions
- AS COPY OF validation
- LIKE clause validation

#### D. DROP GRAPH
- Valid graph drop
- IF EXISTS semantics

#### E. Session Commands
- SET SCHEMA
- SET GRAPH
- SET TIME ZONE
- SET VALUE parameters
- Session parameter references `$param`, `$$param`

---

### 12. **Edge Case and Regression Tests** (LOW PRIORITY)

**Test Coverage Needed**:

#### A. Complex Nesting
- Deeply nested expressions (10+ levels)
- Deeply nested subqueries
- Performance with large query trees

#### B. Unicode and Special Characters
- Unicode identifiers
- Unicode in strings
- Delimited identifiers with special chars
- Accent-quoted identifiers

#### C. Numeric Edge Cases
- Very large integers
- Scientific notation: `1.5e10`
- Hexadecimal: `0xFF`
- Octal: `0o77`
- Binary: `0b1010`
- Negative zero handling

#### D. Empty Constructs
- Empty patterns (already tested)
- Empty lists: `[]`
- Empty records: `{}`
- Empty string: `''`

---

## Priority Summary

**HIGH PRIORITY** (most impactful gaps):
1. Path pattern validation (quantifiers, modes, search)
2. Label expression validation (OR, AND, NOT, wildcards)
3. Subquery and EXISTS validation (scope, nesting)
4. Type system validation (temporal, numeric, collections, CAST)

**MEDIUM PRIORITY** (important for completeness):
5. Aggregation and GROUP BY validation (strict mode)
6. Set operations validation (UNION, EXCEPT, INTERSECT)
7. Mutation validation (expanded coverage)
8. CASE expression validation (type consistency)
9. Procedure call validation (arguments, YIELD)

**LOW PRIORITY** (nice to have):
10. Predicate validation (graph-specific predicates)
11. Catalog and session validation
12. Edge cases and regression tests

---

## Implementation Strategy

For each test category:

1. **Create dedicated test module** or expand existing module
2. **Positive tests**: Valid constructs that should pass
3. **Negative tests**: Invalid constructs that should fail with specific errors
4. **Edge cases**: Boundary conditions and unusual valid constructs
5. **Error message validation**: Check diagnostic messages are clear and actionable

**Test Structure Pattern**:
```rust
#[test]
fn test_feature_valid_case() {
    let source = "VALID GQL QUERY";
    let result = parse_and_validate(source);
    assert!(result.is_success());
}

#[test]
fn test_feature_invalid_case() {
    let source = "INVALID GQL QUERY";
    let result = parse_and_validate(source);
    assert!(result.is_failure());
    assert!(result.has_error_code("EXPECTED_ERROR_CODE"));
}
```

---

## Verification

After implementing tests, verify:

1. **Coverage metrics**: Aim for >90% line coverage in validator modules
2. **Grammar coverage**: Ensure all grammar constructs have validator tests
3. **Error catalog completeness**: All error codes documented and tested
4. **Documentation updates**: Update SEMANTIC_VALIDATION.md with new capabilities

---

## Critical Files

**Test Files**:
- [tests/semantic/validator.rs](tests/semantic/validator.rs) - Main integration tests
- [tests/semantic/aggregate_validation.rs](tests/semantic/aggregate_validation.rs)
- [tests/semantic/callable_validation.rs](tests/semantic/callable_validation.rs)
- [tests/semantic/procedure_validation.rs](tests/semantic/procedure_validation.rs)
- [tests/semantic/mutation_validation.rs](tests/semantic/mutation_validation.rs)

**Validator Implementation**:
- [src/semantic/validator/mod.rs](src/semantic/validator/mod.rs)
- Individual validator passes in [src/semantic/validator/](src/semantic/validator/)

**Reference Documentation**:
- [third_party/opengql-grammar/GQL.g4](third_party/opengql-grammar/GQL.g4) - Grammar specification
- [docs/SEMANTIC_VALIDATION.md](docs/SEMANTIC_VALIDATION.md) - Architecture docs
- [docs/SEMANTIC_ERROR_CATALOG.md](docs/SEMANTIC_ERROR_CATALOG.md) - Error catalog
