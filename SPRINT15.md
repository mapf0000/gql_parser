# Sprint 15: 0.1.0 Release Execution Plan (Final)

## Sprint Overview

**Sprint Goal**: Ship a production-ready, parser-only Rust crate that is:
1. Self-contained for open source users.
2. Immediately consumable by the downstream compiler project defined in `DB_PLAN.md`.

**Status**: 🚧 **EXECUTION MODE (SCOPE LOCKED)**

**Last Updated**: 2026-02-19

## Context

Sprint 15 is the handoff point from implementation-complete parser internals to a released library with stable consumption surfaces.

The parser remains intentionally decoupled from execution engines and storage backends. It provides syntax, diagnostics, and static analysis only.

---

## Hard Boundaries

### In Scope
- Lexer + parser + diagnostics + semantic validation as a standalone crate.
- Query-focused AST traversal APIs.
- Query analysis APIs for compiler planning inputs.
- Documentation and examples for open source adoption.
- Release packaging and publish-readiness.

### Out of Scope
- DataFusion logical/physical plan generation.
- Iceberg catalog wiring, snapshot orchestration, CSR index build/use.
- Query runtime execution or traversal operators.
- Backend-specific assumptions in parser APIs.

---

## Downstream Contract to `DB_PLAN.md`

Sprint 15 must deliver parser outputs that support compiler requirements in `DB_PLAN.md` section 8:

1. **Demand Analysis Inputs** (`DB_PLAN.md` §8.2, lines 241-253)
   - Extract variable/property demand from `RETURN`, `WHERE`, and related expressions.
2. **Clause/Pipeline Structure** (`DB_PLAN.md` §8.5)
   - Preserve clause order and query pipeline structure for lowering.
3. **Pattern Structure Metadata** (`DB_PLAN.md` §8.4-8.5)
   - Provide path/node/edge counts and label-expression characteristics.
4. **Variable Dependency Information** (`DB_PLAN.md` §8)
   - Provide per-clause define/use data for planning and validation.

Parser outputs are analysis-only. The downstream compiler owns all execution planning and backend behavior.

---

## Feature Set (Final)

### F1: AST Visitor Framework (MUST HAVE)

**Objective**: Provide stable, zero-copy traversal for query and expression ASTs.

**Deliverables**
1. `AstVisitor` (immutable) and `AstVisitorMut` (mutable) traits.
2. Query-focused walk helpers with short-circuit (`ControlFlow`) support.
3. Concrete visitors for common downstream tasks:
   - `CollectingVisitor<T>`
   - `SpanCollector`
   - `VariableCollector`

**Success Criteria**
- Traversal handles full query ASTs without panics.
- Read-only traversal does not require AST cloning.
- Visitor behavior covered by focused unit tests.

---

### F2: Query Analysis API (MUST HAVE)

**Objective**: Expose deterministic, parser-native metadata needed for compiler lowering.

**Deliverables**
1. `QueryInfo::from_ast(&Statement)`
   - Clause sequence.
   - Per-clause variable definitions and uses.
   - Graph pattern count.
   - Aggregation presence flags.
2. `VariableDependencyGraph::build(&Statement)`
   - Definition points.
   - Usage points.
   - Define/use edges across clause boundaries.
3. `PatternInfo::analyze(&GraphPattern)`
   - Node/edge/path counts.
   - Label-expression complexity classification.
   - Basic connectivity metadata.
4. `ExpressionInfo::analyze(&Expression)`
   - Variable references.
   - Property references.
   - Function calls.
   - Literals.

**Success Criteria**
- APIs are read-only and deterministic.
- Results are compiler-friendly and cache-safe.
- Analysis modules include focused tests and examples.

---

### F3: Documentation and Examples (MUST HAVE)

**Objective**: Make the crate usable by external developers without project-specific context.

**Deliverables**
1. Crate-level getting-started docs and rustdoc coverage for public APIs.
2. User guides:
   - `docs/USER_GUIDE.md`
   - `docs/AST_GUIDE.md`
   - `docs/QUICK_START.md`
3. Working examples:
   - Parse and report diagnostics.
   - Visitor usage.
   - Query analysis usage.

**Success Criteria**
- `cargo doc --no-deps` renders cleanly.
- Examples compile and run.
- New user can parse + analyze a query quickly.

---

### F4: Release Engineering (MUST HAVE)

**Objective**: Publish-ready crate packaging.

**Deliverables**
1. `Cargo.toml` metadata complete:
   - description, license, repository, keywords, categories.
2. OSS release assets:
   - `CHANGELOG.md`
   - `LICENSE-MIT`
   - `LICENSE-APACHE`
   - release-ready `README.md`
3. CI checks expected for release branch:
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `cargo doc --no-deps`

**Success Criteria**
- `cargo publish --dry-run --allow-dirty` succeeds.
- No warnings in required release checks.

---

### F5: Baseline Benchmarks (SHOULD HAVE)

**Objective**: Capture baseline parser/validation performance for regression tracking.

**Deliverables**
1. Maintain criterion benchmark suite for representative query profiles.
2. Document baseline numbers and benchmark commands.

**Success Criteria**
- Benchmarks run cleanly with current crate.
- Baseline retained in project docs.

---

## Execution Plan

### Phase 1: Consumption APIs
1. Implement and stabilize F1 visitor layer.
2. Implement and stabilize F2 analysis layer.
3. Add tests and examples for both.

### Phase 2: Documentation
1. Publish usage guides and rustdoc updates.
2. Validate all examples compile/run.

### Phase 3: Release Readiness
1. Finalize manifest metadata and release files.
2. Run full verification suite.
3. Prepare 0.1.0 release artifacts.

---

## Exit Criteria (Release Gate)

Sprint 15 is done when all gates below pass:

1. ✅ F1 complete and tested.
2. ✅ F2 complete and tested.
3. ✅ F3 docs/examples complete and verified.
4. ✅ F4 release packaging complete.
5. ✅ `cargo test` passes.
6. ✅ `cargo clippy -- -D warnings` passes.
7. ✅ `cargo doc --no-deps` passes.
8. ✅ `cargo publish --dry-run --allow-dirty` passes.
9. ✅ `README.md` clearly shows parse + traversal + analysis usage.
10. ✅ Downstream compiler team confirms APIs satisfy `DB_PLAN.md` section 8 needs.

---

## Non-Goals (Explicit)

1. No AST-to-DataFusion lowering in this sprint.
2. No Iceberg/CSR runtime or index integration in this sprint.
3. No query runtime execution or optimizer implementation in this sprint.
4. No backend-coupled parser APIs in this sprint.

---

## Summary

Sprint 15 now targets a **single outcome**: release a **self-contained parser library** with robust diagnostics, stable traversal APIs, and compiler-oriented analysis outputs that directly satisfy the parser-side inputs required by `DB_PLAN.md`.
