# Fixes Tracker (Active)

Last reviewed: 2026-02-18

## Scope
This file is intentionally short and tracks only active fix streams. Detailed implementation plans live in sprint-specific fix files.

## Verified Current Baseline
- `cargo test` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.

## Resolved Archive (Removed From Active List)
- Sprint 8 parser integration fixes (`FIX-001` through `FIX-015`) are complete.
- Historical details remain in git history and prior revisions of this file.

## Active Fix Streams

### S14-001: Semantic warnings are not surfaced on success
- Priority: P0
- Detailed plan: `SPRINT14_FIXES.md` (F1)

### S14-002: Scope resolution is unsound (leakage + non-reference-site lookup)
- Priority: P0
- Detailed plan: `SPRINT14_FIXES.md` (F2)

### S14-003: Full semantic enforcement is incomplete (TODO paths)
- Priority: P0
- Detailed plan: `SPRINT14_FIXES.md` (F3)

### S14-004: Type inference is not persisted in IR type table
- Priority: P1
- Detailed plan: `SPRINT14_FIXES.md` (F4)

### S14-005: Strict-mode aggregation/grouping checks are no-op
- Priority: P1
- Detailed plan: `SPRINT14_FIXES.md` (F5)

### S14-006: Semantic docs/examples are out of sync
- Priority: P1
- Detailed plan: `SPRINT14_FIXES.md` (F6)

## Definition of Done
- All `S14-*` streams closed with tests.
- Public API exposes semantic warnings without losing successful IR results.
- Scope/type/aggregation semantics align with Sprint 14 requirements.
- Semantic docs/examples match real behavior.
