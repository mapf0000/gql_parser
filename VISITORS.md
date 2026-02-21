# Visitor Pattern Refactoring: Consolidate Expression Validation Logic

## Context

The validator module has a **module explosion problem**: expression validation logic is scattered across 4+ validators, each reimplementing its own AST traversal instead of using the existing visitor pattern:

- [variable_validation.rs](src/semantic/validator/variable_validation.rs) - 1,895 lines (largest file!)
- [expression_validation.rs](src/semantic/validator/expression_validation.rs) - 529 lines
- [type_checking.rs](src/semantic/validator/type_checking.rs) - 501 lines
- [type_inference.rs](src/semantic/validator/type_inference.rs) - 748 lines

**Problem**: Each validator reimplements near-identical expression traversal boilerplate (~1,100 lines total across 4 modules). This makes the codebase harder to maintain when adding new AST nodes - you must update 4+ places instead of one.

**Solution**: Leverage the existing visitor pattern in [src/ast/visitor.rs](src/ast/visitor.rs), which is already proven by [callable_validation.rs](src/semantic/validator/callable_validation.rs).

**Expected Outcome**: Eliminate ~970 lines of duplicated traversal code (26.5% reduction), improve maintainability, and separate traversal logic from validation logic.

---

## Implementation Approach

### Phase 1: Simple Validators (Quick Wins)

**Targets**: expression_validation.rs (529 → ~300 lines) and type_checking.rs (501 → ~300 lines)

**Strategy**: Direct migration to `AstVisitor` trait following the pattern in callable_validation.rs.

**Implementation Pattern**:

```rust
// Example: expression_validation.rs
use crate::ast::visitor::{AstVisitor, VisitResult, walk_expression, walk_program};

struct ExpressionValidator<'v> {
    validator: &'v SemanticValidator<'v>,
    diagnostics: &'v mut Vec<Diag>,
}

impl<'v> AstVisitor for ExpressionValidator<'v> {
    type Break = ();

    fn visit_expression(&mut self, expr: &Expression) -> VisitResult<Self::Break> {
        // Only handle cases that need special validation
        match expr {
            Expression::Case(case_expr) => {
                // Validate CASE expression type consistency
                validate_case_expression(case_expr, self.diagnostics);
                walk_expression(self, expr)  // Continue traversal
            }
            _ => walk_expression(self, expr)  // Default traversal
        }
    }
}

pub(super) fn run_expression_validation(
    validator: &SemanticValidator,
    program: &Program,
    _type_table: &TypeTable,
    diagnostics: &mut Vec<Diag>,
) {
    let mut visitor = ExpressionValidator { validator, diagnostics };
    let _ = walk_program(&mut visitor, program);
}
```

**Benefits**: Eliminates ~400 lines of manual traversal, automatic coverage of new AST nodes.

---

### Phase 2: Contextual Visitor Infrastructure

**Goal**: Add support for scope-sensitive validators that need context during traversal.

**Changes to [src/ast/visitor.rs](src/ast/visitor.rs)** (+300 lines):

```rust
/// Visitor with traversal context for scope-sensitive validation.
pub trait ContextualAstVisitor {
    type Break;
    type Context;

    fn visit_expression_with_context(
        &mut self,
        expression: &Expression,
        context: &Self::Context
    ) -> VisitResult<Self::Break> {
        walk_expression_with_context(self, expression, context)
    }

    // Similar methods for other node types
}

/// Walker that threads context through expression tree
pub fn walk_expression_with_context<V: ContextualAstVisitor + ?Sized>(
    visitor: &mut V,
    expression: &Expression,
    context: &V::Context,
) -> VisitResult<V::Break> {
    match expression {
        Expression::Binary(_, left, right, _) => {
            try_visit!(visitor.visit_expression_with_context(left, context));
            visitor.visit_expression_with_context(right, context)
        }
        Expression::Unary(_, operand, _) => {
            visitor.visit_expression_with_context(operand, context)
        }
        // ... all expression variants with context threading
        _ => ControlFlow::Continue(())
    }
}

/// Context for scope-sensitive validation
#[derive(Clone)]
pub struct ValidationContext {
    pub scope_id: ScopeId,
    pub statement_id: usize,
}
```

**Rationale**: variable_validation.rs needs to track scope_id and statement_id during traversal to perform scope-aware symbol table lookups.

---

### Phase 3: Scope-Sensitive Validator Migration

**Target**: variable_validation.rs (1,895 → ~1,100 lines, -795 lines)

**Implementation**:

```rust
struct VariableValidator<'v> {
    validator: &'v SemanticValidator<'v>,
    symbol_table: &'v SymbolTable,
    scope_metadata: &'v ScopeMetadata,
    diagnostics: &'v mut Vec<Diag>,
}

impl<'v> ContextualAstVisitor for VariableValidator<'v> {
    type Break = ();
    type Context = ValidationContext;

    fn visit_expression_with_context(
        &mut self,
        expr: &Expression,
        context: &ValidationContext
    ) -> VisitResult<Self::Break> {
        match expr {
            Expression::VariableReference(name, span) => {
                // Use context for scope-aware lookup
                if self.symbol_table.lookup_from(context.scope_id, name).is_none() {
                    let diag = SemanticDiagBuilder::undefined_variable(name, span.clone()).build();
                    self.diagnostics.push(diag);
                }
                ControlFlow::Continue(())
            }
            Expression::AggregateFunction(agg) => {
                check_nested_aggregation(agg, self.diagnostics);
                walk_expression_with_context(self, expr, context)
            }
            _ => walk_expression_with_context(self, expr, context)
        }
    }
}
```

**Challenge**: Must properly initialize context at statement boundaries and thread through nested scopes.

---

### Phase 4: Type Inference Refinement (Hybrid Approach)

**Target**: type_inference.rs (748 → ~600 lines, -148 lines)

**Strategy**: Use visitor for statement-level traversal, keep functional style for expressions.

**Rationale**: Type inference is fundamentally different (returns values during traversal). Forcing visitor pattern adds little value. Better refactoring: extract pure computation functions.

**Implementation**:

```rust
// Use visitor for statement-level traversal
struct TypeInferenceVisitor<'v> {
    validator: &'v SemanticValidator<'v>,
    type_table: &'v mut TypeTable,
}

impl<'v> AstVisitor for TypeInferenceVisitor<'v> {
    type Break = ();

    fn visit_let_statement(&mut self, stmt: &LetStatement) -> VisitResult<Self::Break> {
        for binding in &stmt.bindings {
            infer_expression_type(self.validator, &binding.value, self.type_table);
        }
        ControlFlow::Continue(())
    }
}

// Keep functional style for expressions (already clean!)
fn infer_expression_type(
    validator: &SemanticValidator,
    expr: &Expression,
    type_table: &mut TypeTable,
) -> Type {
    let inferred_type = match expr {
        Expression::Binary(op, left, right, _) => {
            let left_type = infer_expression_type(validator, left, type_table);
            let right_type = infer_expression_type(validator, right, type_table);
            infer_binary_operation_type(op, &left_type, &right_type) // Pure function!
        }
        // ... rest
    };

    type_table.insert(expr.span(), inferred_type.clone());
    inferred_type
}

// Extract pure computation (easy to unit test!)
fn infer_binary_operation_type(
    op: &BinaryOperator,
    left_type: &Type,
    right_type: &Type,
) -> Type {
    match op {
        BinaryOperator::Add | BinaryOperator::Subtract
        | BinaryOperator::Multiply | BinaryOperator::Divide => {
            match (left_type, right_type) {
                (Type::Int, Type::Int) if *op != BinaryOperator::Divide => Type::Int,
                _ if left_type.is_numeric() && right_type.is_numeric() => Type::Float,
                _ => Type::Float,
            }
        }
        BinaryOperator::Concatenate => Type::String,
    }
}
```

**Benefits**: Improves testability by extracting pure computation functions; uses visitor only where it adds value.

---

## Expected Outcomes

### Code Reduction

| Module | Before | After | Reduction |
|--------|--------|-------|-----------|
| variable_validation.rs | 1,895 | ~1,100 | -795 (-42%) |
| expression_validation.rs | 529 | ~300 | -229 (-43%) |
| type_checking.rs | 501 | ~300 | -201 (-40%) |
| type_inference.rs | 748 | ~600 | -148 (-20%) |
| visitor.rs (infrastructure) | 1,691 | 1,991 | +300 |
| **Total** | **5,364** | **4,291** | **-973 (-26.5%)** |

### Qualitative Benefits

- **Maintainability**: Add new AST node → update walker once, not 4 times
- **Testability**: Pure type computation functions easy to unit test
- **Clarity**: Traversal vs. validation logic cleanly separated
- **Consistency**: All validators use same traversal pattern
- **Proven Pattern**: Used by rustc, syn, swc

---

## Critical Files

### Phase 1 (Expression & Type Checking)
- [src/semantic/validator/expression_validation.rs](src/semantic/validator/expression_validation.rs) - First migration target
- [src/semantic/validator/type_checking.rs](src/semantic/validator/type_checking.rs) - Second migration target
- [src/ast/visitor.rs](src/ast/visitor.rs) - Reference implementation
- [src/semantic/validator/callable_validation.rs](src/semantic/validator/callable_validation.rs) - Pattern to follow

### Phase 2 (Infrastructure)
- [src/ast/visitor.rs](src/ast/visitor.rs) - Add `ContextualAstVisitor` trait (+300 lines)

### Phase 3 (Variable Validation)
- [src/semantic/validator/variable_validation.rs](src/semantic/validator/variable_validation.rs) - Complex migration with context
- [src/semantic/validator/mod.rs](src/semantic/validator/mod.rs) - Update coordinator

### Phase 4 (Type Inference)
- [src/semantic/validator/type_inference.rs](src/semantic/validator/type_inference.rs) - Hybrid approach

---

## Verification

### Testing Strategy

1. **Regression Testing**: All existing validator tests must pass without changes
2. **Diagnostic Equivalence**: Validator output must be byte-for-byte identical (same errors, same spans)
3. **Performance**: Benchmark before/after to ensure no regression (visitor is zero-cost in release builds)
4. **Integration Tests**: Run full test suite after each phase

### How to Test

```bash
# Run all semantic validator tests
cargo test --package gql_parser --lib semantic::validator

# Run specific validator tests
cargo test --package gql_parser --lib semantic::validator::expression_validation
cargo test --package gql_parser --lib semantic::validator::type_checking
cargo test --package gql_parser --lib semantic::validator::variable_validation

# Check for diagnostic equivalence
cargo test --package gql_parser --test semantic -- --nocapture

# Performance benchmarking (if benchmarks exist)
cargo bench --bench validator_bench
```

---

## Risk Mitigation

### Identified Risks

1. **Scope Context Incorrect** - Mitigation: Comprehensive tests, incremental migration
2. **Performance Regression** - Mitigation: Benchmark before/after (visitor is zero-cost abstraction)
3. **Breaking Tests** - Mitigation: Phase-by-phase validation with identical diagnostics
4. **Increased Complexity** - Mitigation: Clear documentation and examples

### Rollback Plan

Each phase is independently mergeable:
- Phase 1 standalone (simple validators)
- Phase 2 optional (infrastructure for Phase 3)
- Phase 3 depends on 2, but can revert independently
- Phase 4 optional enhancement

**Rollback Triggers**:
- >5% performance regression
- Test failures unresolved in 2 days
- Fundamental design flaw in code review

---

## Summary

This refactoring consolidates ~1,100 lines of duplicated expression traversal code using the proven visitor pattern from callable_validation.rs. The approach is pragmatic and incremental:

- **Simple validators** → Full migration (40% reduction each)
- **Scope-sensitive validators** → Contextual visitor with explicit context threading (42% reduction)
- **Type inference** → Hybrid approach, extract pure computation functions (20% reduction)

**Key Principles**:
- Preserve multi-pass validation architecture
- Eliminate structural boilerplate, not validation logic
- Use proven Rust patterns (same as rustc, syn, swc)
- Migrate incrementally with rollback points
- Maintain identical diagnostic behavior

**Expected Result**: 26.5% reduction in validator code (973 lines), easier maintenance when adding new AST nodes, better testability with pure functions, and clearer separation between traversal and validation logic.
