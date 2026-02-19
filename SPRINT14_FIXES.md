# Sprint 14 Remaining Fixes Plan

Last reviewed: 2026-02-19

## Scope
This document tracks only unresolved Sprint 14 work. Implemented items were removed.

## Verified Implemented
- ISO-conformant disconnected comma-separated patterns are accepted (warning-level only), not errors.
- Semantic validation runs only when parsing succeeds (`parse_and_validate*` returns early on parse diagnostics).
- Semantic diagnostics preserve full structure when converted to `miette::Report`.
- **F1 COMPLETED (2026-02-19)**: Warning diagnostics are now visible on successful semantic validation via `ValidationOutcome`.
- Quality gate currently passes:
  - `cargo test` - 313 tests passing
  - `cargo clippy --all-targets --all-features -- -D warnings`

## Remaining / New Issues

### ✅ P0-1: Warning diagnostics are lost on success paths - COMPLETED
- **Status**: COMPLETED 2026-02-19
- **Implementation**:
  - Added `ValidationOutcome` struct in `src/ir/mod.rs` with `ir: Option<IR>` and `diagnostics: Vec<Diag>`
  - Updated `SemanticValidator::validate()` to return `ValidationOutcome`
  - Updated `parse_and_validate*` APIs to always include diagnostics
  - Updated examples (`semantic_validation_demo.rs`, `custom_validation_config.rs`)
  - Added 5 new tests for warning visibility
- **Result**: Config knobs like `warn_on_disconnected_patterns` and `warn_on_shadowing` now surface warnings even when IR is successfully produced.

### P0-2: Scope model remains semantically unsound - PARTIAL PROGRESS
- Files:
  - `src/semantic/validator.rs:127`
  - `src/semantic/validator.rs:162`
  - `src/lib.rs:108`
  - `src/lib.rs:178`
- Problem:
  - `validate()` returns `Ok(IR)` when there are no errors, but warnings are not returned.
  - `parse_and_validate*` returns `diagnostics: []` for `Ok`, so warning-only diagnostics are invisible.
- Impact:
  - Config knobs like `warn_on_disconnected_patterns` and `warn_on_shadowing` are effectively hidden in success cases.

### P0-2: Scope model remains semantically unsound - PARTIAL PROGRESS
- Files:
  - `src/semantic/validator.rs:176-236`
  - `src/semantic/validator.rs:802-891`
  - `src/ir/symbol_table.rs:195-213`
- **Partial Implementation (2026-02-19)**:
  - Added `lookup_from(starting_scope, name)` method to `SymbolTable` for scope-aware lookups
  - Current scope tracking infrastructure is in place
- **Remaining Problems**:
  - Query scopes are pushed but not popped, causing cross-statement visibility leakage
  - Variable validation uses a single global `current_scope` from final scope state, not the actual scope at each reference site
  - Requires architectural refactor: validation passes need to track active scope context during AST traversal
  - 44 tests break when attempting to add statement-level scope boundaries (symbols can't be found after scopes are popped)
- **Impact**:
  - False negatives for undefined variables across statements
  - Potential forward-reference acceptance and incorrect visibility behavior
- **Recommendation**:
  - This requires a significant architectural change to thread scope context through all validation passes
  - Consider deferring to Sprint 15 with proper design phase
  - Alternative: Add statement index tracking to symbols as a simpler intermediate solution

### P0-3: “Full semantic enforcement” is still incomplete (active TODO paths)
- Files:
  - `src/semantic/validator.rs:190`
  - `src/semantic/validator.rs:258`
  - `src/semantic/validator.rs:531`
  - `src/semantic/validator.rs:866`
  - `src/semantic/validator.rs:923`
  - `src/semantic/validator.rs:1067`
  - `src/semantic/validator.rs:1143`
  - `src/semantic/validator.rs:1491`
  - `src/semantic/validator.rs:1811`
  - `src/semantic/validator.rs:2011`
  - `src/semantic/validator.rs:2026`
  - `src/semantic/validator.rs:2054`
  - `src/semantic/validator.rs:2060`
  - `src/semantic/validator.rs:2130`
- Problem:
  - Multiple semantic paths remain stubbed or partial (mutation passes, CALL args, nested EXISTS/subquery checks, CASE consistency, reference validation, optional MATCH schema traversal).

### P1-1: Type inference is not materialized into `TypeTable`
- Files:
  - `src/semantic/validator.rs:604`
  - `src/semantic/validator.rs:789`
  - `src/semantic/validator.rs:793`
- Problem:
  - Types are inferred locally but never persisted as `ExprId -> Type` entries.

### P1-2: Strict-mode aggregation/grouping checks are currently no-op
- Files:
  - `src/semantic/validator.rs:1451`
  - `src/semantic/validator.rs:1457`
- Problem:
  - Mixed aggregate/non-aggregate detection exists, but strict-mode branch emits no diagnostics and performs no `GROUP BY` legality checks.

### P1-3: Semantic docs/examples are out of sync with implementation - PARTIAL PROGRESS
- Files:
  - `docs/SEMANTIC_VALIDATION.md:164`
  - `docs/SEMANTIC_ERROR_CATALOG.md:57`
  - `src/semantic/README.md:50`
  - `src/semantic/README.md:56`
  - `src/semantic/README.md:258`
- **Partial Implementation (2026-02-19)**:
  - ✅ Updated examples to show warnings and current API behavior
    - `examples/semantic_validation_demo.rs` now uses `ValidationOutcome`
    - `examples/custom_validation_config.rs` now uses `ValidationOutcome`
- **Remaining Problems**:
  - Disconnected patterns still documented as errors in docs
  - Status text claims outdated task state
  - Stale links/reference to non-current task docs
- **Impact**:
  - Users may be confused by documentation not matching actual behavior

---

## Implementation Status Summary (2026-02-19)

### ✅ Completed
- **F1: Return Warnings Alongside Successful IR** (P0-1)
  - Added `ValidationOutcome` struct with helper methods
  - Updated all validator APIs and examples
  - Added comprehensive test coverage (5 new tests)
  - All 313 tests passing

### 🔶 Partially Completed
- **F6: Align Docs and Examples** (P1-3)
  - ✅ Examples updated (`semantic_validation_demo.rs`, `custom_validation_config.rs`)
  - ❌ Documentation files not yet updated

### ⚠️ Blocked / Needs Design
- **F2: Make Scope Resolution Correct** (P0-2)
  - Partial progress: Added `lookup_from()` method
  - Blocked: Requires architectural refactor to thread scope context through validation passes
  - 44 tests break with naive implementation
  - Recommend: Defer to Sprint 15 with proper design phase

### ❌ Not Started
- **F3: Close All Sprint-14 Semantic TODO Paths** (P0-3)
- **F4: Persist Inferred Types into TypeTable** (P1-1)
- **F5: Implement Real Aggregation/Grouping Semantics** (P1-2)

---

## Detailed Fix Plan

## ✅ F1. Return Warnings Alongside Successful IR (P0-1) - COMPLETED
### Design
Introduce a semantic outcome object that always carries diagnostics:
- `ValidationOutcome { ir: Option<IR>, diagnostics: Vec<Diag> }`

### Implementation (COMPLETED 2026-02-19)
1. ✅ Added `ValidationOutcome` type in `src/ir/mod.rs`
2. ✅ Changed `SemanticValidator::validate` signature to return `ValidationOutcome`
3. ✅ Keep `ir: Some` when only warnings/notes exist; `ir: None` when errors exist
4. ✅ Updated `parse_and_validate*` to always convert semantic diagnostics to reports
5. ✅ Updated examples to print warnings even when validation succeeds
6. ✅ Updated all 59 validator tests to use new API
7. ✅ Added 5 new test cases for warning visibility

### Tests (ALL PASSING)
1. ✅ `test_warning_visibility_disconnected_patterns` - Disconnected-pattern query returns `ir: Some` and warning diagnostics
2. ✅ `test_warning_visibility_shadowing` - Shadowing query returns `ir: Some` and warning diagnostics
3. ✅ `test_warning_with_error_both_returned` - Mixed warning+error query returns `ir: None` and full diagnostic set
4. ✅ `test_no_warnings_when_disabled` - Warnings can be disabled via config
5. ✅ `test_successful_validation_with_no_diagnostics` - Clean queries have no diagnostics

### Done Criteria
✅ Warning diagnostics are visible through public APIs when IR is produced.

---

## 🔶 F2. Make Scope Resolution Correct and Reference-Site Aware (P0-2) - BLOCKED
### Design
Enforce explicit scope lifecycle and resolve references from the scope active at each AST location.

### Partial Implementation (2026-02-19)
1. ✅ Added `lookup_from(scope_id, name)` API that accepts explicit starting scope in `SymbolTable`
2. ❌ Statement-level scope boundaries - NOT IMPLEMENTED (breaks 44 tests)
3. ❌ Push/pop query scopes symmetrically - NOT IMPLEMENTED (breaks lookups)
4. ❌ During variable validation, track active scope - NOT IMPLEMENTED (architectural limitation)
5. ❌ Validate references against active scope - NOT IMPLEMENTED
6. ❌ Add declaration-order checks - NOT IMPLEMENTED

### Blocking Issue
**Root Cause**: The current two-pass architecture (scope analysis, then variable validation) doesn't maintain scope context during the second pass. When scopes are properly popped after analysis, the validation pass can't find symbols because it doesn't know which scope each AST node belongs to.

**Failed Approach**: Attempted to add statement-level scope boundaries and symmetric push/pop, but this causes all variables to become unreachable during validation (current_scope is back at root after all analysis completes).

**Required Architecture Change**: Need to either:
1. Thread scope context through all validation passes (major refactor)
2. Build scope-to-AST mapping during analysis phase
3. Add statement/query index tracking to symbols and check during validation
4. Redesign as single-pass validation that maintains scope state throughout

**Test Impact**: 44 tests fail with scope isolation changes (symbols can't be resolved).

### Recommendation
- Defer to **Sprint 15** with proper architectural design phase
- Alternative: Implement minimal fix using statement index tracking on symbols
- Current behavior (cross-statement leakage) is a semantic bug but doesn't cause crashes

### Tests (NOT IMPLEMENTED)
1. ❌ `MATCH (n) RETURN n; RETURN n` fails in second statement
2. ❌ In-statement pipeline references still resolve correctly
3. ❌ Reference before declaration in same statement is rejected
4. ❌ Nested scope shadowing resolves to nearest binding

### Done Criteria
❌ No cross-statement leakage and no scope-resolution false negatives.

---

## ❌ F3. Close All Sprint-14 Semantic TODO Paths (P0-3) - NOT STARTED
### Design
Replace placeholder TODO branches with concrete semantic checks for the declared Sprint 14 scope.

### Implementation Steps
1. Mutation support: scope, variable, pattern, type, and expression validation for mutation AST forms.
2. Variable validation: MATCH `WHERE`, CALL arguments/yields, nested EXISTS/subquery references.
3. CASE enforcement: searched WHEN conditions boolean; branch result compatibility.
4. Reference validation: catalog-backed checks for graph/schema/procedure/type references.
5. Schema traversal: include optional MATCH blocks/parenthesized operands.

### Tests
1. Add positive/negative tests for each previously stubbed branch.
2. Add integration suites for mixed query + mutation programs.
3. Add nested EXISTS/OPTIONAL scenarios with schema/catalog mocks.

### Done Criteria
- No active Sprint 14 semantic TODO stubs in validator logic.

---

## ❌ F4. Persist Inferred Types into `TypeTable` (P1-1) - NOT STARTED
### Design
Assign stable `ExprId`s during traversal and persist inferred types/constraints for downstream use.

### Implementation Steps
1. Add deterministic ExprId allocation during expression walk.
2. Persist each inferred type via `type_table.set_type(expr_id, ty)`.
3. Record constraints (`Numeric`, `Boolean`, etc.) as checks are applied.
4. Consume `TypeTable` in type-checking and expression validation instead of literal-only heuristics.

### Tests
1. Assert inferred types for literals/arithmetic/aggregates/CASE.
2. Assert constraint failures are driven by recorded types/constraints.

### Done Criteria
- `IR.type_table` contains actionable type data used by later checks.

---

## ❌ F5. Implement Real Aggregation/Grouping Semantics (P1-2) - NOT STARTED
### Design
Build explicit grouping context per SELECT/RETURN and enforce aggregate legality rules.

### Implementation Steps
1. Collect grouping keys from `GROUP BY`.
2. Classify each expression as aggregate or non-aggregate.
3. Enforce: non-aggregate expressions must be grouped (or equivalent).
4. Validate ORDER BY/HAVING in grouped context.
5. Wire strict-mode diagnostics with concrete actionable messages.

### Tests
1. Mixed aggregate/non-aggregate without grouping -> error.
2. Correct grouped query -> pass.
3. `GROUP BY ()` aggregate-only semantics -> pass.

### Done Criteria
- Strict mode actually enforces grouping/aggregation legality.

---

## 🔶 F6. Align Docs and Examples With Actual Behavior (P1-3) - PARTIALLY COMPLETED
### Implementation Steps
1. ❌ Update disconnected-pattern documentation to warning-level ISO-conformant behavior
2. ❌ Update semantic status sections to current implementation state
3. ❌ Remove stale TODO references and invalid links
4. ✅ Update examples to show warnings and current API behavior (COMPLETED)
5. ❌ Add a brief “known limitations” section until F1-F5 complete

### Completed (2026-02-19)
- ✅ Updated `examples/semantic_validation_demo.rs` to use `ValidationOutcome` and display warnings
- ✅ Updated `examples/custom_validation_config.rs` to use `ValidationOutcome` and display warnings

### Remaining Work
- ❌ Update `docs/SEMANTIC_VALIDATION.md`
- ❌ Update `docs/SEMANTIC_ERROR_CATALOG.md`
- ❌ Update `src/semantic/README.md`

### Tests
- Manual verification: Run examples to ensure outputs align with documented behavior

### Done Criteria
❌ Documentation and examples match implementation and active roadmap (examples done, docs pending).

---

## Execution Order (Updated 2026-02-19)
1. ✅ F1 - COMPLETED
2. 🔶 F2 - BLOCKED (architectural issue, defer to Sprint 15)
3. ❌ F3 - NOT STARTED
4. ❌ F4 - NOT STARTED
5. ❌ F5 - NOT STARTED
6. 🔶 F6 - PARTIALLY COMPLETED (examples done, docs pending)

## Sprint 14 Exit Criteria (Updated Status)
- ✅ Warning diagnostics are visible on successful semantic validation
- ❌ Scope and variable resolution are reference-site accurate and statement-isolated (BLOCKED - architectural issue)
- ❌ No active Sprint-14 semantic TODO stubs remain (NOT STARTED)
- ❌ Type inference is persisted and consumed (NOT STARTED)
- ❌ Aggregation/grouping semantics are fully enforced (NOT STARTED)
- 🔶 Docs/examples are accurate and current (PARTIALLY - examples done, docs pending)

## Next Steps
1. **Immediate**: Complete F6 documentation updates
2. **Sprint 15 Planning**: Design proper scope resolution architecture for F2
3. **Sprint 15**: Implement F3, F4, F5
4. **Consider**: Whether F3-F5 can proceed without F2 being resolved
