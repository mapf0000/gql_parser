# Quick Start Checklist for Next Session

## Pre-Flight Check ✈️

```bash
# 1. Verify current status
cargo test --lib semantic
# Expected: 80 passed; 0 failed; 2 ignored

# 2. Check for uncommitted changes
git status
git diff

# 3. Review recent work
git log --oneline -5
```

## Pick Your Task 🎯

### Option 1: F5 - Aggregation Validation (RECOMMENDED)
- **Priority:** HIGH
- **Time:** 4-6 hours
- **Impact:** User-facing query validation
- **Plan:** `SPRINT14_REMAINING.md` lines 77-380

### Option 2: F4 - Type Persistence
- **Priority:** MEDIUM
- **Time:** 3-4 hours
- **Impact:** Better error messages
- **Plan:** `SPRINT14_REMAINING.md` lines 382-537

### Option 3: F3 - Expression Validation
- **Priority:** LOW
- **Time:** 2-3 hours
- **Impact:** Completeness
- **Plan:** `SPRINT14_REMAINING.md` lines 539-663

### Option 4: F6 - Documentation
- **Priority:** LOW
- **Time:** 1-2 hours
- **Impact:** User understanding
- **Plan:** `SPRINT14_REMAINING.md` lines 665-789

## Implementation Steps 📝

### 1. Write Tests First (TDD)
```rust
// Example for F5
#[test]
fn test_nested_aggregation_error() {
    let source = "MATCH (n) RETURN COUNT(SUM(n.age))";
    let validator = SemanticValidator::new();
    let parse_result = parse(source);

    if let Some(program) = parse_result.ast {
        let outcome = validator.validate(&program);
        assert!(!outcome.is_success(), "Should fail: nested aggregation");
    }
}
```

### 2. Run Test (Should Fail)
```bash
cargo test test_nested_aggregation_error
# Should fail with "test passed" error
```

### 3. Implement Feature
- Open `src/semantic/validator.rs`
- Find the section mentioned in plan
- Add the method/logic
- Follow existing patterns

### 4. Run Test Again (Should Pass)
```bash
cargo test test_nested_aggregation_error
# Should now pass
```

### 5. Run Full Suite
```bash
cargo test --lib semantic
# Should show 81+ tests passing (80 + your new test)
```

### 6. Repeat for Each Test Case

## Key Code Patterns 🔑

### Adding Validation Method
```rust
fn validate_something(
    &self,
    thing: &Thing,
    diagnostics: &mut Vec<Diag>,
) {
    // Your validation logic
    if condition_fails {
        let diag = SemanticDiagBuilder::error_type(
            "Error message",
            span.clone()
        ).build();
        diagnostics.push(diag);
    }
}
```

### Walking Expressions
```rust
match expression {
    Expression::Binary(_, left, right, _) => {
        self.validate_something(left, diagnostics);
        self.validate_something(right, diagnostics);
    }
    Expression::AggregateFunction(agg) => {
        // Handle aggregate
    }
    _ => {}
}
```

### Calling New Validation
```rust
// In existing validation method, add:
self.validate_something(thing, diagnostics);
```

## File Locations 📂

```
src/semantic/validator.rs  # Main file (4670 lines)
├── Lines 18-33: Structs (ExpressionContext, ScopeMetadata)
├── Lines 140-230: Scope analysis
├── Lines 1179-1660: Variable validation (F2 DONE)
├── Lines 1970-2110: Aggregation validation (F5 TODO)
├── Lines 2112-2437: Type checking (F4 TODO)
├── Lines 2577-2750: Expression validation (F3 TODO)
└── Lines 3500+: Tests

src/semantic/README.md     # Documentation (F6 TODO)
SPRINT14_REMAINING.md      # Detailed plan
SESSION_SUMMARY.md         # What was done
SPRINT14.md               # Overall status
```

## Common Commands 💻

```bash
# Single test
cargo test test_name

# Test with output
cargo test test_name -- --nocapture

# All semantic tests
cargo test --lib semantic

# Full suite
cargo test

# Build only
cargo build --lib

# Format code
cargo fmt

# Lint (note: existing warnings unrelated to your changes)
cargo clippy --lib
```

## Success Criteria ✅

After implementing each fix:

- [ ] Tests added (1-6 new tests depending on fix)
- [ ] Tests passing (all new + all existing)
- [ ] `cargo test --lib semantic` shows increase in passing tests
- [ ] `cargo test` full suite passes
- [ ] Code formatted (`cargo fmt`)
- [ ] Git committed with clear message

## Getting Unstuck 🆘

If you're stuck:

1. **Review F2 implementation** - It's a good pattern to follow
   - Lines 1179-1660 in `validator.rs`
   - Shows how to add validation methods
   - Shows testing patterns

2. **Check existing tests** - They show what works
   - Lines 3500+ in `validator.rs`
   - Copy patterns from similar tests

3. **Review plan again** - Step-by-step instructions
   - `SPRINT14_REMAINING.md` has detailed code samples
   - Shows exactly where to add code

4. **Start simple** - Implement one test case at a time
   - Don't try to do everything at once
   - Get one test passing, then move to next

5. **Use existing helpers** - Don't reinvent
   - `SemanticDiagBuilder::*` for errors
   - `expression_contains_aggregation()` already exists
   - `expressions_equivalent()` already exists (but needs enhancement)

## Quick Reference 📚

**Diagnostic Builders:**
```rust
SemanticDiagBuilder::undefined_variable(var, span)
SemanticDiagBuilder::type_mismatch(actual, expected, span)
SemanticDiagBuilder::aggregation_error(msg, span)
```

**Type Queries:**
```rust
type.is_numeric()
type.is_compatible_with(other)
type.name() // "Integer", "String", etc.
```

**Symbol Table:**
```rust
symbol_table.lookup(name)
symbol_table.lookup_from(scope_id, name)
```

## Estimated Timeline ⏱️

- **F5 only:** 4-6 hours
- **F5 + F4:** 7-10 hours
- **F5 + F4 + F3:** 9-13 hours
- **All (F5 + F4 + F3 + F6):** 10-15 hours

**Recommended for one session:** F5 only, or F5 + F4

---

Ready to start? Pick a fix above and dive in! 🚀

**Pro tip:** Start with F5, test case #1 (nested aggregation). It's small, well-defined, and follows existing patterns.
