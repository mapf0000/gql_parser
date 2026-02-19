# Sprint 14 Remaining Implementation Plan

**Last Updated:** 2026-02-19
**Status:** F2 Complete, F3-F6 Remaining

## Executive Summary

Sprint 14 aims to complete semantic validation for the GQL parser. **F2 (Scope Resolution)** has been successfully implemented with reference-site-aware lookups. This document provides a detailed implementation plan for the remaining fixes: F3, F4, F5, and F6.

### Current Status

- ✅ **F2 Complete**: Reference-site-aware variable lookups implemented, 80 tests pass, 2 known limitations documented
- ❌ **F3 Pending**: Complete expression validation (CASE, null, subquery)
- ❌ **F4 Pending**: Type persistence consumption in downstream passes
- ❌ **F5 Pending**: Complete aggregation/GROUP BY validation
- ❌ **F6 Pending**: Documentation updates

### Test Suite Health

- **320 tests passing** (0 failures)
- **2 tests ignored** (known parser limitations for F2)
- All existing functionality preserved

---

## F2: Scope Resolution (COMPLETED) ✅

### What Was Implemented

1. **Infrastructure Added:**
   - `ExpressionContext` struct (lines 18-24): Tracks scope_id and statement_id per expression
   - `ScopeMetadata` struct (lines 28-33): Maps statement IDs to scope IDs
   - Modified `run_scope_analysis()` to return `(SymbolTable, ScopeMetadata)` tuple

2. **Reference-Site-Aware Lookups:**
   - Updated `validate_expression_variables()` (line 1468) to use `lookup_from(scope_id, var_name)`
   - Variables now resolved from correct statement scope instead of global `current_scope`
   - Implementation at lines 1480-1495

3. **Scope Tracking:**
   - Each statement gets unique `statement_id` during analysis
   - `analyze_linear_query()` creates new scope per statement (line 268)
   - Composite queries use offset statement IDs (statement_id + 1000) for isolation

4. **Test Coverage:**
   - 78 existing tests updated and passing
   - 2 new tests added:
     - `test_scope_proper_linear_flow()` - ✅ Passes
     - `test_composite_query_both_sides_valid()` - ✅ Passes
   - 2 tests documenting known limitations (ignored):
     - `test_scope_isolation_across_statements()` - Parser limitation
     - `test_composite_query_scope_isolation_union()` - Requires scope popping

### Known Limitations

1. **Parser Limitation**: Semicolon-separated queries not parsed as separate Statements
   - Example: `"MATCH (n); RETURN n"` is single Statement, not two
   - Variables leak across semicolons (expected but not fixable in semantic layer)

2. **Composite Query Scope Isolation**: Both sides of UNION share accumulated scopes
   - Need to implement scope popping or cloning for true isolation
   - Currently uses different statement_ids but doesn't prevent variable visibility

### Files Modified

- `src/semantic/validator.rs` (lines 1-4670):
  - Added structs (lines 18-33)
  - Modified 15+ method signatures to accept `scope_metadata` and `statement_id`
  - Implemented reference-site lookups (lines 1480-1495)
  - Added 4 new tests (lines 4595-4665)

---

## F5: Aggregation Validation (HIGH PRIORITY) ❌

### Problem Statement

Current aggregation validation (lines 1970-2110) has significant gaps:
- Only validates SELECT statements, not RETURN statements
- Missing: nested aggregation detection, WHERE clause checks, HAVING validation, ORDER BY validation
- Expression equivalence too simplistic (lines 2073-2087)
- Zero test coverage for GROUP BY scenarios

### Implementation Plan

#### Step 1: Add RETURN Statement Aggregation Validation

**Location:** `src/semantic/validator.rs` around line 1436

**Current Code:**
```rust
fn validate_result_statement_variables(...) {
    // Only validates variable references, not aggregation
}
```

**Add After Line 1457:**
```rust
fn validate_return_aggregation(
    &self,
    return_stmt: &crate::ast::query::ReturnStatement,
    diagnostics: &mut Vec<Diag>,
) {
    // Check if mixing aggregated and non-aggregated expressions
    let (has_aggregation, non_aggregated_expressions) = match &return_stmt.items {
        ReturnItemList::Star => (false, vec![]),
        ReturnItemList::Items { items } => {
            let mut has_agg = false;
            let mut non_agg_exprs = Vec::new();

            for item in items {
                if self.expression_contains_aggregation(&item.expression) {
                    has_agg = true;
                } else {
                    non_agg_exprs.push(&item.expression);
                }
            }
            (has_agg, non_agg_exprs)
        }
    };

    // In strict mode or with GROUP BY, mixing requires GROUP BY
    if has_aggregation && !non_aggregated_expressions.is_empty() {
        // RETURN doesn't have GROUP BY, so this is an error in strict mode
        if self.config.strict_mode {
            for expr in non_aggregated_expressions {
                let diag = SemanticDiagBuilder::aggregation_error(
                    "Cannot mix aggregated and non-aggregated expressions in RETURN without GROUP BY",
                    expr.span().clone()
                ).build();
                diagnostics.push(diag);
            }
        }
    }
}
```

**Call Site:** In `validate_result_statement_variables()` after line 1453:
```rust
// Validate aggregation rules for RETURN
self.validate_return_aggregation(return_stmt, diagnostics);
```

#### Step 2: Nested Aggregation Detection

**Location:** `src/semantic/validator.rs` around line 2090

**Add New Method:**
```rust
/// Checks for illegal nested aggregation functions.
fn check_nested_aggregation(
    &self,
    expr: &Expression,
    in_aggregate: bool,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::ast::expression::AggregateFunction;

    match expr {
        Expression::AggregateFunction(agg_func) => {
            if in_aggregate {
                // Nested aggregation detected!
                let diag = SemanticDiagBuilder::aggregation_error(
                    "Nested aggregation functions are not allowed",
                    expr.span().clone()
                ).build();
                diagnostics.push(diag);
                return; // Don't recurse further
            }

            // Check arguments with in_aggregate=true
            match &**agg_func {
                AggregateFunction::CountStar { .. } => {}
                AggregateFunction::GeneralSetFunction(gsf) => {
                    self.check_nested_aggregation(&gsf.expression, true, diagnostics);
                }
                AggregateFunction::BinarySetFunction(bsf) => {
                    self.check_nested_aggregation(&bsf.expression, true, diagnostics);
                    self.check_nested_aggregation(&bsf.inverse_distribution_argument, true, diagnostics);
                }
            }
        }
        Expression::Binary(_, left, right, _) => {
            self.check_nested_aggregation(left, in_aggregate, diagnostics);
            self.check_nested_aggregation(right, in_aggregate, diagnostics);
        }
        Expression::Unary(_, operand, _) => {
            self.check_nested_aggregation(operand, in_aggregate, diagnostics);
        }
        // Add other expression types as needed...
        _ => {
            // For other types, recursively check children
        }
    }
}
```

**Call Site:** In `validate_select_aggregation()` and `validate_return_aggregation()`:
```rust
// Check for nested aggregation
for item in items {
    self.check_nested_aggregation(&item.expression, false, diagnostics);
}
```

#### Step 3: WHERE Clause Aggregation Check

**Location:** `src/semantic/validator.rs` line 1308

**Modify `validate_primitive_statement_variables()` at Filter case:**
```rust
PrimitiveQueryStatement::Filter(filter) => {
    // Validate FILTER condition
    self.validate_expression_variables(&filter.condition, symbol_table, scope_metadata, statement_id, diagnostics);

    // NEW: Check for illegal aggregation in WHERE clause
    if self.expression_contains_aggregation(&filter.condition) {
        let diag = SemanticDiagBuilder::aggregation_error(
            "Aggregation functions not allowed in WHERE clause (use HAVING instead)",
            filter.condition.span().clone()
        ).build();
        diagnostics.push(diag);
    }
}
```

#### Step 4: HAVING Clause Validation

**Location:** `src/semantic/validator.rs` around line 1348

**Modify SELECT validation in `validate_primitive_statement_variables()`:**
```rust
// Validate HAVING if present
if let Some(having) = &select.having {
    self.validate_expression_variables(&having.condition, symbol_table, scope_metadata, statement_id, diagnostics);

    // NEW: Validate HAVING semantics
    self.validate_having_clause(&having.condition, &select.group_by, diagnostics);
}
```

**Add New Method:**
```rust
fn validate_having_clause(
    &self,
    condition: &Expression,
    group_by: &Option<crate::ast::query::GroupByClause>,
    diagnostics: &mut Vec<Diag>,
) {
    // Collect non-aggregated expressions in HAVING
    let non_agg_exprs = self.collect_non_aggregated_expressions(condition);

    if let Some(group_by) = group_by {
        // Check each non-aggregated expression appears in GROUP BY
        let group_by_exprs = self.collect_group_by_expressions(group_by);

        for expr in non_agg_exprs {
            let found_in_group_by = group_by_exprs.iter()
                .any(|gb_expr| self.expressions_equivalent(expr, gb_expr));

            if !found_in_group_by {
                let diag = SemanticDiagBuilder::aggregation_error(
                    "Non-aggregated expression in HAVING must appear in GROUP BY",
                    expr.span().clone()
                ).build();
                diagnostics.push(diag);
            }
        }
    } else {
        // HAVING without GROUP BY - only aggregates allowed
        if !non_agg_exprs.is_empty() && self.config.strict_mode {
            for expr in non_agg_exprs {
                let diag = SemanticDiagBuilder::aggregation_error(
                    "HAVING clause requires GROUP BY when using non-aggregated expressions",
                    expr.span().clone()
                ).build();
                diagnostics.push(diag);
            }
        }
    }
}

fn collect_non_aggregated_expressions(&self, expr: &Expression) -> Vec<&Expression> {
    let mut result = Vec::new();
    self.collect_non_aggregated_expressions_recursive(expr, false, &mut result);
    result
}

fn collect_non_aggregated_expressions_recursive(
    &self,
    expr: &Expression,
    in_aggregate: bool,
    result: &mut Vec<&Expression>,
) {
    match expr {
        Expression::AggregateFunction(agg) => {
            // Inside aggregate, check arguments with in_aggregate=true
            match &**agg {
                AggregateFunction::CountStar { .. } => {}
                AggregateFunction::GeneralSetFunction(gsf) => {
                    self.collect_non_aggregated_expressions_recursive(&gsf.expression, true, result);
                }
                AggregateFunction::BinarySetFunction(bsf) => {
                    self.collect_non_aggregated_expressions_recursive(&bsf.expression, true, result);
                    self.collect_non_aggregated_expressions_recursive(&bsf.inverse_distribution_argument, true, result);
                }
            }
        }
        Expression::VariableReference(_, _) | Expression::PropertyReference(_, _, _) => {
            if !in_aggregate {
                result.push(expr);
            }
        }
        Expression::Binary(_, left, right, _) => {
            self.collect_non_aggregated_expressions_recursive(left, in_aggregate, result);
            self.collect_non_aggregated_expressions_recursive(right, in_aggregate, result);
        }
        // Add other cases...
        _ => {}
    }
}
```

#### Step 5: ORDER BY with GROUP BY Validation

**Location:** `src/semantic/validator.rs` line 1312

**Modify `validate_primitive_statement_variables()` at OrderByAndPage case:**
```rust
PrimitiveQueryStatement::OrderByAndPage(order_by_page) => {
    // Validate ORDER BY expressions
    if let Some(order_by) = &order_by_page.order_by {
        for sort_spec in &order_by.sort_specifications {
            self.validate_expression_variables(&sort_spec.key, symbol_table, scope_metadata, statement_id, diagnostics);

            // NEW: If query has GROUP BY, validate ORDER BY expressions
            // Need to track current SELECT statement's GROUP BY clause
            // For now, this requires context passing or state tracking
            // See implementation note below
        }
    }
    // ... rest of validation
}
```

**Implementation Note:** ORDER BY validation needs access to the SELECT statement's GROUP BY clause. This requires either:
1. Passing GROUP BY context through validation methods, OR
2. Storing SELECT context in validator state, OR
3. Performing ORDER BY validation within SELECT validation context

**Recommended Approach:** Validate ORDER BY within `validate_primitive_statement_variables()` when processing SELECT:

```rust
PrimitiveQueryStatement::Select(select) => {
    // ... existing validation ...

    // NEW: Validate ORDER BY with GROUP BY context
    // This requires looking ahead to next statement if it's OrderByAndPage
    // Or, restructure to validate ORDER BY here if it logically follows SELECT
}
```

#### Step 6: Enhance Expression Equivalence

**Location:** `src/semantic/validator.rs` lines 2073-2087

**Current Implementation (TOO SIMPLISTIC):**
```rust
fn expressions_equivalent(&self, expr1: &Expression, expr2: &Expression) -> bool {
    match (expr1, expr2) {
        (Expression::VariableReference(v1, _), Expression::VariableReference(v2, _)) => v1 == v2,
        (Expression::PropertyReference(base1, prop1, _), Expression::PropertyReference(base2, prop2, _)) => {
            prop1 == prop2 && self.expressions_equivalent(base1, base2)
        }
        _ => false,
    }
}
```

**Enhanced Implementation:**
```rust
fn expressions_equivalent(&self, expr1: &Expression, expr2: &Expression) -> bool {
    match (expr1, expr2) {
        // Literals
        (Expression::Literal(l1, _), Expression::Literal(l2, _)) => l1 == l2,

        // Variables
        (Expression::VariableReference(v1, _), Expression::VariableReference(v2, _)) => v1 == v2,

        // Properties
        (Expression::PropertyReference(base1, prop1, _), Expression::PropertyReference(base2, prop2, _)) => {
            prop1 == prop2 && self.expressions_equivalent(base1, base2)
        }

        // Binary operations
        (Expression::Binary(op1, l1, r1, _), Expression::Binary(op2, l2, r2, _)) => {
            op1 == op2
                && self.expressions_equivalent(l1, l2)
                && self.expressions_equivalent(r1, r2)
        }

        // Unary operations
        (Expression::Unary(op1, e1, _), Expression::Unary(op2, e2, _)) => {
            op1 == op2 && self.expressions_equivalent(e1, e2)
        }

        // Function calls
        (Expression::FunctionCall(f1), Expression::FunctionCall(f2)) => {
            f1.name == f2.name
                && f1.arguments.len() == f2.arguments.len()
                && f1.arguments.iter().zip(&f2.arguments)
                    .all(|(a1, a2)| self.expressions_equivalent(a1, a2))
        }

        // Parenthesized (unwrap and compare)
        (Expression::Parenthesized(e1, _), e2) => self.expressions_equivalent(e1, e2),
        (e1, Expression::Parenthesized(e2, _)) => self.expressions_equivalent(e1, e2),

        // Type annotations (ignore annotation, compare base)
        (Expression::TypeAnnotation(e1, _, _), e2) => self.expressions_equivalent(e1, e2),
        (e1, Expression::TypeAnnotation(e2, _, _)) => self.expressions_equivalent(e1, e2),

        // Default: not equivalent
        _ => false,
    }
}
```

### Test Cases to Add

Add these tests after line 3920 in the existing aggregation test section:

```rust
#[test]
fn test_return_mixed_aggregation() {
    let source = "MATCH (n:Person) RETURN COUNT(n), n.name";
    let config = ValidationConfig { strict_mode: true, ..Default::default() };
    let validator = SemanticValidator::with_config(config);
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: mixing aggregated and non-aggregated in RETURN");
    }
}

#[test]
fn test_nested_aggregation_error() {
    let source = "MATCH (n:Person) RETURN COUNT(SUM(n.age))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: nested aggregation");

        let has_nested_error = outcome.diagnostics.iter()
            .any(|d| d.message.contains("Nested aggregation"));
        assert!(has_nested_error, "Should have nested aggregation error");
    }
}

#[test]
fn test_aggregation_in_where_error() {
    let source = "MATCH (n:Person) WHERE AVG(n.age) > 30 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: aggregation in WHERE");

        let has_where_error = outcome.diagnostics.iter()
            .any(|d| d.message.contains("WHERE"));
        assert!(has_where_error, "Should mention WHERE clause");
    }
}

#[test]
fn test_having_non_grouped_error() {
    let source = "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING n.name = 'Alice'";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        // Should fail: n.name not in GROUP BY
        assert!(!outcome.is_success(), "Should fail: non-grouped expression in HAVING");
    }
}

#[test]
fn test_valid_group_by() {
    let source = "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(), "Should succeed: valid GROUP BY");
    }
}

#[test]
fn test_valid_having_with_aggregate() {
    let source = "MATCH (n:Person) SELECT n.dept, AVG(n.age) GROUP BY n.dept HAVING AVG(n.age) > 30";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(outcome.is_success(), "Should succeed: HAVING uses aggregate");
    }
}
```

### Files to Modify

- `src/semantic/validator.rs`:
  - Lines 1436-1457: Add `validate_return_aggregation()`
  - Lines 1308-1310: Add WHERE clause aggregation check
  - Lines 1348-1350: Add HAVING validation
  - Lines 1970-2110: Enhance `validate_select_aggregation()` with new checks
  - Lines 2073-2087: Enhance `expressions_equivalent()`
  - Add helper methods: `check_nested_aggregation()`, `validate_having_clause()`, `collect_non_aggregated_expressions()`
  - Lines 3920+: Add 6 new test cases

### Verification Steps

1. Run `cargo test --lib semantic::validator::tests::test_return_mixed_aggregation`
2. Run `cargo test --lib semantic::validator::tests::test_nested_aggregation_error`
3. Run `cargo test --lib semantic::validator::tests::test_aggregation_in_where_error`
4. Run `cargo test --lib semantic::validator::tests::test_having_non_grouped_error`
5. Run `cargo test --lib semantic::validator::tests::test_valid_group_by`
6. Run `cargo test --lib semantic::validator::tests::test_valid_having_with_aggregate`
7. Run `cargo test --lib semantic` - should have 86 tests passing (80 current + 6 new)

### Estimated Effort

- **Time:** 4-6 hours
- **Complexity:** Medium (extends existing patterns)
- **Risk:** Low (isolated changes, well-tested)

---

## F4: Type Persistence Consumption (MEDIUM PRIORITY) ❌

### Problem Statement

Type inference stores types via `set_type_by_span()` (line 1116) but no downstream passes consume them:
- `run_type_checking()` receives `_type_table: &TypeTable` (underscore = unused)
- Passes use ad-hoc pattern matching instead of persisted types
- Cannot detect variable type errors (e.g., `LET x = 5 RETURN x + "hello"`)

### Implementation Plan

#### Step 1: Verify Type Persistence Coverage

**Location:** `src/semantic/validator.rs` lines 932-1117 (`infer_expression_type()`)

**Check:** Ensure all expression types persist their inferred types. Current implementation only persists at line 1116 (end of method).

**Verification:** Review each expression match arm and ensure type is persisted for ALL branches, not just at the end.

#### Step 2: Update Type Checking Pass to Use TypeTable

**Location:** `src/semantic/validator.rs` lines 2112-2437 (`run_type_checking()`)

**Current Implementation:**
```rust
fn run_type_checking(&self, program: &Program, _type_table: &TypeTable, diagnostics: &mut Vec<Diag>) {
    // Uses pattern matching on AST, not type table
    fn is_definitely_string(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Literal(Literal::String(_), _))
    }
}
```

**Enhanced Implementation:**
```rust
fn run_type_checking(&self, program: &Program, type_table: &TypeTable, diagnostics: &mut Vec<Diag>) {
    // ... walk program ...

    // For each expression, retrieve its type from TypeTable
    fn get_expression_type(&self, expr: &Expression, type_table: &TypeTable) -> Option<Type> {
        type_table.get_type_by_span(&expr.span()).cloned()
    }

    // Use retrieved types for validation
    fn check_binary_operation_types(
        &self,
        op: &BinaryOp,
        left: &Expression,
        right: &Expression,
        type_table: &TypeTable,
        diagnostics: &mut Vec<Diag>,
    ) {
        let left_type = self.get_expression_type(left, type_table);
        let right_type = self.get_expression_type(right, type_table);

        if let (Some(lt), Some(rt)) = (left_type, right_type) {
            // Check compatibility using Type::is_compatible_with()
            if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div) {
                if !lt.is_numeric() {
                    let diag = SemanticDiagBuilder::type_mismatch(
                        &lt.name(),
                        "numeric type",
                        left.span().clone()
                    ).with_note(&format!("Cannot perform arithmetic on {}", lt.name()))
                     .build();
                    diagnostics.push(diag);
                }
                if !rt.is_numeric() {
                    let diag = SemanticDiagBuilder::type_mismatch(
                        &rt.name(),
                        "numeric type",
                        right.span().clone()
                    ).with_note(&format!("Cannot perform arithmetic on {}", rt.name()))
                     .build();
                    diagnostics.push(diag);
                }
            }
        }
    }
}
```

#### Step 3: Enhance Error Messages with Type Names

**Location:** Throughout `run_type_checking()` methods

**Current:**
```rust
"Cannot add string to arithmetic operation"
```

**Enhanced:**
```rust
format!("Cannot add {} to numeric operation", actual_type.name())
// Example output: "Cannot add String to numeric operation"
```

**Implementation Pattern:**
```rust
if let Some(expr_type) = type_table.get_type_by_span(&expr.span()) {
    let diag = SemanticDiagBuilder::type_mismatch(
        &expr_type.name(),        // Actual type
        "expected type here",      // Expected type
        expr.span().clone()
    ).with_note(&format!("Expression has type {} but context requires ...", expr_type.name()))
     .build();
    diagnostics.push(diag);
}
```

### Test Cases to Add

```rust
#[test]
fn test_type_persistence_variable_error() {
    // Variable type should be persisted and checked
    let source = "MATCH (n:Person) LET x = 5 RETURN x + 'hello'";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // Should fail: x is Int, cannot add to String
        assert!(!outcome.is_success(), "Should fail: Int + String");

        // Error message should mention types
        let has_type_error = outcome.diagnostics.iter()
            .any(|d| d.message.contains("Int") || d.message.contains("String"));
        assert!(has_type_error, "Should mention types in error");
    }
}

#[test]
fn test_type_based_error_messages() {
    let source = "MATCH (n:Person) RETURN n.name + 5";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        // Error message should be specific about types
        let error_msg = outcome.diagnostics.iter()
            .find(|d| d.severity == DiagSeverity::Error)
            .map(|d| &d.message);

        if let Some(msg) = error_msg {
            assert!(msg.contains("String") || msg.contains("Integer") || msg.contains("Int"),
                "Error should mention specific types");
        }
    }
}
```

### Files to Modify

- `src/semantic/validator.rs`:
  - Lines 2112-2437: Update `run_type_checking()` to use TypeTable
  - Add helper: `get_expression_type(expr, type_table) -> Option<Type>`
  - Update all error messages to use `type.name()`
  - Lines 2112+: Add 2 new test cases

### Verification Steps

1. Run `cargo test test_type_persistence_variable_error`
2. Run `cargo test test_type_based_error_messages`
3. Verify error messages include specific type names (String, Integer, etc.)
4. Run full semantic test suite: `cargo test --lib semantic`

### Estimated Effort

- **Time:** 3-4 hours
- **Complexity:** Medium (refactoring existing code)
- **Risk:** Medium (changes affect existing error messages)

---

## F3: Complete Expression Validation (LOW PRIORITY) ❌

### Problem Statement

Expression validation pass (lines 2577-2750) has incomplete implementations:
- CASE expression type consistency not checked
- Null propagation rules not enforced
- Subquery result types not validated

### Implementation Plan

#### Step 1: CASE Expression Type Consistency

**Location:** `src/semantic/validator.rs` lines 2577-2750

**Add Method:**
```rust
fn validate_case_type_consistency(
    &self,
    program: &Program,
    type_table: &TypeTable,
    diagnostics: &mut Vec<Diag>,
) {
    // Walk program and find all CASE expressions
    self.walk_program_expressions(program, |expr| {
        if let Expression::Case(case_expr) = expr {
            match case_expr {
                CaseExpression::Searched(searched) => {
                    // Collect types of all WHEN branches
                    let mut branch_types = Vec::new();

                    for when in &searched.when_clauses {
                        if let Some(ty) = type_table.get_type_by_span(&when.then_result.span()) {
                            branch_types.push((ty.clone(), when.then_result.span()));
                        }
                    }

                    // Check ELSE clause
                    if let Some(else_expr) = &searched.else_clause {
                        if let Some(ty) = type_table.get_type_by_span(&else_expr.span()) {
                            branch_types.push((ty.clone(), else_expr.span()));
                        }
                    }

                    // Verify all branches have compatible types
                    if let Some((first_type, _)) = branch_types.first() {
                        for (branch_type, span) in branch_types.iter().skip(1) {
                            if !first_type.is_compatible_with(branch_type) {
                                let diag = SemanticDiagBuilder::type_mismatch(
                                    &branch_type.name(),
                                    &first_type.name(),
                                    span.clone()
                                ).with_note("All CASE branches must return compatible types")
                                 .build();
                                diagnostics.push(diag);
                            }
                        }
                    }
                }
                CaseExpression::Simple(simple) => {
                    // Similar logic for simple CASE
                }
            }
        }
    });
}

// Helper to walk all expressions in program
fn walk_program_expressions<F>(&self, program: &Program, mut visitor: F)
where
    F: FnMut(&Expression),
{
    for statement in &program.statements {
        // Walk statements recursively and call visitor on each expression
        // Implementation similar to existing traversal patterns
    }
}
```

#### Step 2: Null Propagation Validation

**Add Method:**
```rust
fn validate_null_handling(&self, program: &Program, diagnostics: &mut Vec<Diag>) {
    self.walk_program_expressions(program, |expr| {
        match expr {
            Expression::Binary(op, left, right, span) => {
                // Check for null in arithmetic
                if self.is_null_literal(left) || self.is_null_literal(right) {
                    if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div) {
                        let diag = Diag::new(
                            DiagSeverity::Warning,
                            "Arithmetic with NULL always returns NULL".to_string(),
                            span.clone()
                        );
                        diagnostics.push(diag);
                    }
                }
            }
            _ => {}
        }
    });
}

fn is_null_literal(&self, expr: &Expression) -> bool {
    matches!(expr, Expression::Literal(Literal::Null, _))
}
```

#### Step 3: Subquery Result Type Validation

**Add Method:**
```rust
fn validate_subquery_types(
    &self,
    program: &Program,
    type_table: &TypeTable,
    diagnostics: &mut Vec<Diag>,
) {
    // Validate that subquery expressions return appropriate types
    // for their context (e.g., EXISTS should work with graph patterns)

    self.walk_program_expressions(program, |expr| {
        match expr {
            Expression::Exists(exists_expr) => {
                // EXISTS should contain graph pattern or boolean-returning subquery
                // Validation logic here
            }
            Expression::SubqueryExpression(inner, _) => {
                // Validate subquery result type matches usage context
                // Validation logic here
            }
            _ => {}
        }
    });
}
```

#### Step 4: Update `run_expression_validation()`

**Location:** Line 2577

**Current:**
```rust
fn run_expression_validation(&self, program: &Program, _type_table: &TypeTable, diagnostics: &mut Vec<Diag>) {
    // Basic validation only
}
```

**Enhanced:**
```rust
fn run_expression_validation(&self, program: &Program, type_table: &TypeTable, diagnostics: &mut Vec<Diag>) {
    // Validate CASE type consistency
    self.validate_case_type_consistency(program, type_table, diagnostics);

    // Validate null propagation
    self.validate_null_handling(program, diagnostics);

    // Validate subquery types
    self.validate_subquery_types(program, type_table, diagnostics);

    // Existing validation...
    // (context validation, expression structure validation, etc.)
}
```

### Test Cases to Add

```rust
#[test]
fn test_case_type_consistency() {
    let source = "MATCH (n:Person) RETURN CASE WHEN true THEN 5 WHEN false THEN 'string' END";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: inconsistent CASE types");
    }
}

#[test]
fn test_null_propagation_warning() {
    let source = "MATCH (n:Person) WHERE n.age + NULL > 5 RETURN n";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);

        let has_null_warning = outcome.diagnostics.iter()
            .any(|d| d.severity == DiagSeverity::Warning && d.message.contains("NULL"));
        assert!(has_null_warning, "Should warn about NULL in arithmetic");
    }
}
```

### Files to Modify

- `src/semantic/validator.rs`:
  - Lines 2577-2750: Add validation methods
  - Update `run_expression_validation()` to call new methods
  - Add helper `walk_program_expressions()`
  - Add 2 new test cases

### Verification Steps

1. Run `cargo test test_case_type_consistency`
2. Run `cargo test test_null_propagation_warning`
3. Run full semantic test suite

### Estimated Effort

- **Time:** 2-3 hours
- **Complexity:** Low (adding new checks)
- **Risk:** Low (new functionality, doesn't change existing)

---

## F6: Documentation Updates (LOW PRIORITY) ❌

### Problem Statement

Documentation is outdated:
- `src/semantic/README.md` shows incomplete status (line 40: "⚠️ PASSES TODO")
- Doesn't reflect completed F2 implementation
- Missing information about known limitations

### Implementation Plan

#### Step 1: Update README Status Section

**Location:** `src/semantic/README.md` lines 40-56

**Current:**
```markdown
## Status

**Overall**: 🚧 Partial implementation - validation passes need full implementation

- ✅ **Complete**: Diagnostic system, Symbol table, Type table, IR structure, ValidationOutcome API
- ⏳ **Partially Implemented**: Scope analysis, variable validation, pattern validation, type inference
- ⏸️ **Pending**: Complete mutation support, CASE enforcement, reference validation, type persistence, aggregation/grouping semantics
```

**Updated:**
```markdown
## Status

**Overall**: ✅ Core validation complete with known limitations

- ✅ **Complete**:
  - Diagnostic system, Symbol table, Type table, IR structure
  - ValidationOutcome API with warning visibility (F1)
  - Reference-site-aware variable lookups (F2)
  - Scope analysis, variable validation, pattern validation, type inference
  - Context validation, type checking, expression validation
  - Schema and catalog validation (optional)

- 🔶 **Partially Complete**:
  - Statement isolation (F2) - limited by parser
  - Composite query scope isolation (F2) - limited by scope stack
  - Type persistence consumption (F4) - infrastructure exists
  - Aggregation validation (F5) - SELECT only, RETURN/HAVING incomplete
  - Expression validation (F3) - basic cases covered, advanced cases pending

- ⏸️ **Future Work**:
  - Full statement isolation (requires parser changes)
  - Complete aggregation semantics (nested, WHERE, HAVING, ORDER BY)
  - Enhanced type-based error messages
  - CASE type consistency validation
  - Null propagation warnings
```

#### Step 2: Add Known Limitations Section

**Add after Status section:**
```markdown
## Known Limitations (Sprint 14)

### F2: Scope Resolution
1. **Statement Isolation**: Parser doesn't create separate Statement objects for semicolon-separated queries (e.g., `"MATCH (n); RETURN n"` is one Statement)
   - Variables leak across semicolons
   - Fix requires parser changes (out of scope)

2. **Composite Query Isolation**: UNION/EXCEPT sides share accumulated scopes
   - Variables from left side visible on right side
   - Fix requires scope popping/cloning (future work)

### F4: Type Persistence
- Types inferred and stored but not fully consumed by all downstream passes
- Error messages use literal type detection instead of inferred types
- Infrastructure exists, consumption incomplete

### F5: Aggregation Validation
- Only validates SELECT statements with GROUP BY
- Missing: RETURN aggregation, nested aggregation detection, WHERE clause checks, HAVING validation, ORDER BY validation
- Expression equivalence uses structural comparison only

### F3: Expression Validation
- CASE type consistency not checked
- Null propagation rules not enforced
- Subquery result types not validated

See `SPRINT14.md` for detailed implementation status and `SPRINT14_REMAINING.md` for completion plan.
```

#### Step 3: Update Examples

**Location:** `src/semantic/README.md` lines 62-89

Verify all examples work with current implementation. Update if needed.

#### Step 4: Update Root README

**Location:** `README.md` (if exists)

Add section about semantic validation:
```markdown
## Semantic Validation

The parser includes a semantic validation layer that checks for:
- ✅ Variable scope and binding correctness
- ✅ Type inference and compatibility
- ✅ Pattern connectivity validation
- ✅ Context-appropriate clause usage
- 🔶 Aggregation and grouping rules (partial)
- ✅ Optional schema/catalog validation

See `src/semantic/README.md` for details.

**Status**: 320 tests passing, core validation complete with known limitations.
```

### Files to Modify

- `src/semantic/README.md`:
  - Lines 40-56: Update status section
  - Add known limitations section
  - Verify examples (lines 62-89)

- `README.md` (root):
  - Add/update semantic validation section

- `SPRINT14.md`:
  - Update status at top to reflect F2 completion
  - Note location of remaining work plan

### Verification Steps

1. Read through updated README.md and verify accuracy
2. Test all code examples in documentation
3. Verify links and references are correct
4. Run `cargo doc --open` and review generated documentation

### Estimated Effort

- **Time:** 1-2 hours
- **Complexity:** Low (documentation only)
- **Risk:** None

---

## Implementation Sequence

### Recommended Order

1. **F5 First** (4-6 hours) - Highest user value
   - Most visible to users
   - Extends existing patterns
   - Well-defined scope

2. **F4 Second** (3-4 hours) - Quality improvement
   - Better error messages
   - Uses existing infrastructure
   - Medium complexity

3. **F3 Third** (2-3 hours) - Completeness
   - New features
   - Low risk
   - Nice-to-have

4. **F6 Last** (1-2 hours) - Documentation
   - No code changes
   - Can be done anytime
   - Important for users

**Total Estimated Effort**: 10-15 hours

### Critical Success Factors

1. **Run tests frequently** - After each method addition, run relevant tests
2. **Commit often** - Small, focused commits for each feature
3. **Keep existing tests passing** - Currently 320 passing, maintain this
4. **Add tests before implementation** - TDD approach catches issues early
5. **Use existing patterns** - Follow established code style and structure

---

## Testing Strategy

### Test Organization

Tests are in `src/semantic/validator.rs` starting around line 3500.

**Sections:**
- Lines 3520-3590: Scope analysis tests
- Lines 3590-3670: Variable validation tests
- Lines 3670-3750: Pattern validation tests
- Lines 3750-3850: Context validation tests
- Lines 3850-3920: Aggregation tests (expand here for F5)
- Lines 3920-4120: Edge case tests
- Lines 4120-4300: Expression validation tests
- Lines 4595-4670: Scope isolation tests (F2)

**Add new tests in logical sections:**
- F5 tests: After line 3920 in aggregation section
- F4 tests: After line 4120 in expression section or create new "Type Persistence" section
- F3 tests: After line 4300 in expression validation section

### Test Naming Convention

Follow existing pattern:
```rust
#[test]
fn test_<category>_<specific_behavior>() {
    // Example: test_aggregation_nested_error
    // Example: test_type_persistence_variable_error
}
```

### Running Tests

- **Single test**: `cargo test test_name`
- **Category**: `cargo test --lib semantic::validator::tests::test_aggregation`
- **All semantic**: `cargo test --lib semantic`
- **Full suite**: `cargo test`
- **With output**: `cargo test test_name -- --nocapture`

---

## Code Quality Checks

Before completing each fix:

1. **Compile**: `cargo build --lib`
2. **Tests**: `cargo test --lib semantic`
3. **Full tests**: `cargo test`
4. **Clippy**: `cargo clippy --lib` (note: existing warnings unrelated to changes)
5. **Format**: `cargo fmt`

---

## Useful References

### Key Files

- `src/semantic/validator.rs` (4670 lines) - Main validation logic
- `src/semantic/diag.rs` - Diagnostic builders
- `src/ir/symbol_table.rs` - Symbol table operations
- `src/ir/type_table.rs` - Type storage and queries
- `src/ast/expression.rs` - Expression AST nodes
- `src/ast/query.rs` - Query AST nodes

### Key Structures

```rust
// Validation configuration
pub struct ValidationConfig {
    pub strict_mode: bool,
    pub schema_validation: bool,
    pub catalog_validation: bool,
    pub warn_on_shadowing: bool,
    pub warn_on_disconnected_patterns: bool,
}

// Type representation
pub enum Type {
    Int, Float, String, Boolean,
    Date, Time, Timestamp, Duration,
    Node(Option<Vec<String>>),
    Edge(Option<Vec<String>>),
    Path, List(Box<Type>),
    Record(Vec<(String, Type)>),
    Union(Vec<Type>),
    Null, Any,
}

// Diagnostic severity
pub enum DiagSeverity {
    Error,
    Warning,
    Note,
}
```

### Helper Methods

```rust
// Building diagnostics
SemanticDiagBuilder::undefined_variable(var_name, span).build()
SemanticDiagBuilder::type_mismatch(actual, expected, span).build()
SemanticDiagBuilder::aggregation_error(message, span).build()

// Type queries
type.is_numeric()
type.is_comparable()
type.is_compatible_with(other_type)
type.name() // Human-readable name

// Symbol table
symbol_table.lookup(name) // Global lookup
symbol_table.lookup_from(scope_id, name) // From specific scope
symbol_table.define(name, kind, span)
```

---

## Session Handoff Checklist

Before starting a new session to continue this work:

1. ✅ Read this document (`SPRINT14_REMAINING.md`)
2. ✅ Review `SPRINT14.md` for overall context
3. ✅ Check current branch: `git status`
4. ✅ Verify tests pass: `cargo test --lib semantic`
5. ✅ Review recent changes: `git log --oneline -10`
6. ✅ Check for any uncommitted changes: `git diff`
7. ✅ Choose which fix to implement (recommend F5 first)
8. ✅ Read the specific fix section above
9. ✅ Start with tests (TDD approach)
10. ✅ Implement incrementally, testing frequently

---

## Success Criteria

Sprint 14 is considered complete when:

- ✅ F1: Warning visibility (COMPLETE)
- ✅ F2: Scope resolution (COMPLETE with limitations)
- ✅ F3: Expression validation complete
- ✅ F4: Type persistence consumed
- ✅ F5: Aggregation validation complete
- ✅ F6: Documentation updated
- ✅ All tests passing (target: 330+ tests)
- ✅ No clippy warnings for new code
- ✅ `cargo test` passes
- ✅ `cargo clippy --lib` passes (for validator.rs)

---

## Contact/Questions

If continuing this work and need clarification:
- Review exploration agents' outputs from original session
- Check `SPRINT14.md` for historical context
- Review plan file at `/Users/d072013/.claude/plans/concurrent-juggling-wren.md`
- Refer to F2 implementation in `src/semantic/validator.rs` for patterns

**Last Implementation Session:** 2026-02-19
**Implementer:** Claude (Opus 4.6)
**Test Status at Handoff:** 320 passing, 0 failing, 2 ignored
