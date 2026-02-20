# Remaining GQL Milestones (Open Only)

Last Updated: 2026-02-20

This file now tracks only milestones that are still open after the latest self-contained parser implementation pass.

## Milestone 1 (Open): Re-baseline and Conformance Matrix

Remaining work:

- Build and maintain a full feature matrix mapping each gap to grammar productions, AST nodes, parser entrypoints, semantic passes, and test IDs.
- Add generated/maintained conformance status output that classifies every tracked row.
- Ensure every remaining row has explicit ownership and test traceability.

## Milestone 3 (Open): Advanced Graph Type + Schema Semantics

### Workstream 5 (Open subset): Advanced graph type grammar/modeling

Remaining work:

- Implement `ABSTRACT` type grammar forms.
- Implement graph type inheritance grammar forms.
- Implement full graph-type constraint grammar forms.
- Replace coarse `GraphTypeSource::Detailed { span }` payload with full-fidelity embedded graph type model in catalog AST.

### Workstream 4 (Open): Schema-dependent semantic enforcement

Remaining work:

- Expand schema model for property existence/type metadata and constraints.
- Add schema-aware context propagation into scope/type inference.
- Enforce property/type/constraint checks against schema metadata.
- Add schema semantics fixtures and regression tests.

## Milestone 4 (Partially Open): Partial-feature hardening

### Workstream 6.1 (Open): Graph pattern hardening

Remaining work:

- Close deep nested quantifier parsing edge cases.
- Close union/alternation precedence/association gaps in all pattern contexts.
- Add stress and regression coverage for these cases.

### Workstream 6.2 (Open subset): Built-in function completeness

Remaining work:

- Complete any remaining specialized string/datetime/list variants still missing from parser/AST inventory alignment.
- Complete function arity/argument semantic validation coverage where still missing.

## Milestone 5 (Open): Recovery and Type-Quality Polish

### Workstream 7.1 (Open): Error recovery improvements

Remaining work:

- Standardize synchronization sets by parser context.
- Reduce cascading diagnostics for malformed regions.
- Add explicit parser recovery invariants and guard tests.

### Workstream 7.2 (Open subset): Nested quantifier robustness

Remaining work:

- Expand fuzz/stress coverage and remove rollback ambiguity edge paths still present.

### Workstream 7.3 (Open): Type inference quality

Remaining work:

- Reduce `Type::Any` fallback in complex expressions.
- Improve cast/function-return/result inference quality.
- Integrate inference consistently into downstream type checks.

## Definition of Done (Remaining Scope)

The plan is complete when all open milestones above are closed with direct tests, diagnostics behavior is stable, and docs/changelog are synchronized with final parser + semantic behavior.
