# Sprint 1 Plan: Diagnostics and Span Infrastructure

## Status
- Completed on February 17, 2026.
- Exit checks passed: `cargo test` and `cargo clippy --all-targets --all-features -- -D warnings`.
- Unresolved technical debt from Sprint 1: none.

## Sprint Intent
Build the reusable diagnostics and span foundation that all later lexer/parser work will depend on.

## Target Outcome
At sprint end, the project has a stable internal diagnostic model, span types used consistently across syntax artifacts, and an API-level conversion path to `miette::Report` with source snippets.

## Scope
- Define shared span primitives (`Span`, `Spanned<T>`) for cross-module use.
- Define internal diagnostic domain model (`Diag`, labels, severity, help, notes, code/category).
- Implement conversion from internal diagnostics to rendered `miette::Report`.
- Establish source-text handling strategy for diagnostic rendering.
- Add test coverage for span math, label behavior, and report rendering.
- Wire initial module structure expected by future sprints (`src/diag.rs`, `src/ast.rs`, `src/lib.rs` exports).

## Out of Scope
- Full lexer implementation.
- Full parser implementation.
- Semantic validation rules.
- Performance optimization beyond obvious correctness issues.

## Deliverables
- `src/ast.rs` with foundational span types:
  - `pub type Span = std::ops::Range<usize>` (or equivalent stable alias).
  - `pub struct Spanned<T> { pub node: T, pub span: Span }`.
- `src/diag.rs` with diagnostic model:
  - `DiagSeverity` enum.
  - `DiagLabel` struct with span + label text + primary/secondary role.
  - `Diag` struct including message, labels, help, notes, severity, optional code.
  - Constructor helpers for common patterns.
- Miette bridge:
  - Deterministic conversion from `Diag` to `miette::Report`.
  - Support for source snippet attachment.
- Public API scaffolding:
  - `src/lib.rs` exports span/diag types and conversion helpers needed by next sprints.
- Test suite for diagnostics layer.

## Work Breakdown

### Workstream A: Core Span Types
- Define canonical span type and ownership semantics.
- Define `Spanned<T>` and basic helpers (`map`, `into_inner`, etc. if useful).
- Add unit tests for span behavior and boundary assumptions.

### Workstream B: Internal Diagnostic Model
- Define diagnostic severity levels for syntax phase (`Error`, `Warning`, optional `Note`).
- Define label model supporting multiple labeled spans.
- Define optional metadata fields (`code`, `help`, `notes`).
- Ensure model is parser-agnostic and lexer-agnostic.

### Workstream C: Miette Conversion Layer
- Decide conversion boundary API shape.
- Implement conversion that preserves:
  - Primary/secondary labels.
  - Help text.
  - Notes.
  - Severity intent.
- Validate rendering on representative multi-label errors.

### Workstream D: Source Management for Reports
- Define how source text is carried into report conversion (borrowed/owned strategy).
- Provide helper for converting `Vec<Diag>` into `Vec<miette::Report>` for one source string.
- Ensure no panics on invalid/out-of-range spans; degrade gracefully.

### Workstream E: Quality Gates
- Unit tests for each struct/helper.
- Snapshot-style tests (or stable string assertions) for rendered diagnostics.
- `cargo test` and strict `cargo clippy` clean for sprint-added code.

## Suggested Execution Order
1. Span types and tests.
2. Diagnostic structs and constructors.
3. Miette conversion with minimal happy-path tests.
4. Multi-label, help, notes, and severity behavior tests.
5. Public exports and cleanup.

## Acceptance Criteria
- Internal diagnostics can express at least one primary span and multiple secondary spans.
- Conversion to `miette::Report` works for:
  - Single-label syntax error.
  - Multi-label relationship error.
  - Error with help and notes.
- Bad spans do not panic the conversion pipeline.
- Test suite includes positive and negative cases for diagnostics rendering.
- `src/lib.rs` exposes a stable surface for later lexer/parser integration.

## Definition of Done
- All deliverables implemented.
- Tests pass locally.
- Clippy strict mode passes for sprint-touched files.
- Sprint notes updated with unresolved technical debt (if any).

## Risks and Mitigations
- Risk: Overfitting diagnostics model to current assumptions.
  - Mitigation: Keep model syntax-phase-generic and avoid parser-specific enum variants.
- Risk: Early API churn across upcoming sprints.
  - Mitigation: Keep public exports minimal; keep most constructors/internal details crate-private where possible.
- Risk: Miette conversion edge cases (span/source mismatch).
  - Mitigation: Defensive checks and fallback rendering paths.

## Dependencies for Sprint 2
- Stable `Diag` model and conversion helpers.
- Stable `Span`/`Spanned<T>` types used by lexer tokens.
- Clear severity taxonomy for lexer diagnostics.

## Locked Decisions
1. **Severity Model Scope**: Option B
- Include `Error`, `Warning`, and `Note`/`Advice` in Sprint 1.

2. **Source Ownership in Diagnostic Conversion**: Option B
- Introduce a `SourceFile` wrapper in Sprint 1 and pass it through diagnostics conversion.

3. **Public API Exposure in Sprint 1**: Option B
- Keep low-level diagnostics types crate-private for now.
- Expose only high-level conversion/API surface needed by current crate users.

4. **Test Style for Rendered Diagnostics**: Option A
- Use structural assertions (message/labels/help/notes/severity) and avoid snapshot tests in Sprint 1.

## Decision Rationale
- Project is greenfield with no external consumers yet.
- Favor strong internal architecture now (`Severity` breadth + `SourceFile` wrapper).
- Avoid premature public API lock-in while diagnostics internals are still evolving.
- Keep tests robust and low-maintenance until rendering format stabilizes.
