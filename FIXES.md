# FIXES.md

## Purpose
This file tracks issues that are likely to cause regressions, rework, or architectural dead ends in upcoming sprints.

The items below are based on code inspection and runtime checks on the current workspace (Sprint 8 state).

## Quick Status
- `cargo test`: passes
- `cargo clippy --all-targets --all-features -- -D warnings`: passes
- Sprint 8 query/pattern parser breadth is fully implemented and integrated.

## Priority 0 (Blockers)

### FIX-001: Top-level parser still does not build real query AST
- Severity: Critical
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/ast/program.rs:32`
  - `src/ast/program.rs:34`
  - `src/parser/program.rs:133`
  - `src/parser/program.rs:135`
- Problem:
  - `QueryStatement` is still a placeholder with only `span`.
  - `parse_statement()` creates that placeholder directly and never calls `parser::query::parse_query()`.
- Impact:
  - Sprint 7/8 query and pattern AST are effectively disconnected from `parse()`.
  - Future semantic/lowering phases cannot rely on top-level parse results.
- Recommended fix:
  - Replace `QueryStatement` placeholder with a real query payload (for example `query: Query`).
  - In `src/parser/program.rs`, call `parse_query` for query statements and propagate diagnostics.
  - Add end-to-end assertions that parsed `Program` contains expected query AST shape.

### FIX-002: Statement classification/boundary logic breaks valid query forms
- Severity: Critical
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/program.rs:90`
  - `src/parser/program.rs:114`
  - `src/parser/program.rs:120`
- Problem:
  - Query starts are hardcoded to `MATCH | SELECT | FROM`.
  - Clause keywords are treated as top-level boundaries, so a single query can be split into multiple statements.
- Repro symptoms:
  - `SELECT * FROM MATCH (n) RETURN n` -> parsed as 3 statements with 0 diagnostics.
  - `USE GRAPH g MATCH (n) RETURN n` -> `unexpected token in statement`.
  - `RETURN 1` -> rejected as top-level statement.
  - `OPTIONAL MATCH (n) RETURN n` -> starts with error and then partial parse.
- Impact:
  - Public API behavior diverges from GQL query grammar.
  - Later sprint features may appear broken due to incorrect statement slicing.
- Recommended fix:
  - Move to grammar-driven query statement parsing in program parser (not token-class splitting heuristics).
  - If classification remains, include all valid query starts and make boundaries clause-aware.

### FIX-003: `parse_query` can succeed without consuming input or parsing anything
- Severity: Critical
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/query.rs:281`
  - `src/parser/query.rs:342`
  - `src/parser/query.rs:380`
- Problem:
  - Ambient query parsing succeeds even with zero primitive statements and zero result statement.
  - Invalid inputs return `Some(Query)` with no diagnostics and `pos` unchanged.
- Repro symptoms (direct `parse_query` call):
  - `foo` -> `q_some=true`, `pos=0`, `diags=0`
  - `;` -> `q_some=true`, `pos=0`, `diags=0`
- Impact:
  - Unsafe parser contract for callers (success without progress).
  - High risk of hidden parse acceptance bugs.
- Recommended fix:
  - Enforce progress: if no tokens consumed, return `None` + diagnostic.
  - Require at least one primitive statement or result statement in linear query.

## Priority 1 (High Risk)

### FIX-004: Sprint 8 pattern parser is still a placeholder and does not consume pattern tokens
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/patterns.rs:35`
  - `src/parser/patterns.rs:39`
  - `src/parser/patterns.rs:57`
  - `src/parser/patterns.rs:80`
  - `src/parser/query.rs:648`
- Problem:
  - `parse_graph_pattern()` returns a minimal empty `GraphPattern` and does not advance `pos`.
  - Helper `skip_to_statement_boundary()` exists but is never invoked.
- Impact:
  - MATCH parsing is structurally incomplete; token cursor handling is wrong.
  - Future real pattern parser integration will require refactoring call contracts and tests.
- Recommended fix:
  - Implement real pattern parsing or at minimum deterministic boundary consumption with diagnostics.
  - Ensure `parse_graph_pattern` always consumes valid pattern tokens or reports failure.
  - Add progress/assertion checks in query parser around pattern parse.

### FIX-005: `SELECT ... FROM ...` parser path is still stubbed and silently accepts invalid input
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/query.rs:1049`
  - `src/parser/query.rs:1204`
- Problem:
  - `parse_select_from_clause()` always returns `(None, vec![])`.
  - `FROM` token is consumed, but no clause AST/diagnostic is produced.
- Repro symptoms:
  - `SELECT * FROM` parses with no diagnostics.
  - `SELECT * FROM MATCH (n)` does not fail despite missing FROM parsing.
- Impact:
  - Query AST correctness is compromised.
  - Downstream semantic passes will receive malformed/partial AST without error signals.
- Recommended fix:
  - Implement `FROM` clause variants (`graph match list`, `query specification`, `graph + query`).
  - Emit diagnostics on malformed/missing FROM payload.

### FIX-006: `FOR ... WITH ...` malformed forms are accepted without diagnostics
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/query.rs:876`
  - `src/parser/query.rs:959`
  - `src/parser/query.rs:975`
  - `src/parser/query.rs:996`
- Problem:
  - `WITH` is consumed, but missing/invalid ORDINALITY/OFFSET target silently returns `None`.
- Repro symptoms:
  - `FOR x IN xs WITH`
  - `FOR x IN xs WITH OFFSET`
  - `FOR x IN xs WITH ORDINALITY`
  - All currently produce no diagnostics in direct query parser use.
- Impact:
  - Silent acceptance of malformed clauses.
- Recommended fix:
  - If `WITH` is present and parse fails, emit diagnostic and recover to clause boundary.

### FIX-007: Schema reference behavior regressed for plain identifiers (ecosystem inconsistency)
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/references.rs:193`
  - `examples/parser_demo.rs:8`
- Problem:
  - `SESSION SET SCHEMA myschema` now errors (`expected schema reference in schema reference`).
  - Example/demo still uses plain identifier and fails.
- Impact:
  - Confusing UX and potential backward compatibility break with existing usage/examples.
- Recommended fix:
  - Decide grammar policy explicitly:
    - If plain identifier is valid, support it in schema reference parser.
    - If invalid by design, update demos/tests/docs to only accepted forms.

## Priority 2 (Quality/Performance/Compliance)

### FIX-008: Strict clippy gate is failing
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/patterns.rs:78`
  - `src/ast/query.rs:113`
  - `src/ast/query.rs:191`
  - `src/ast/query.rs:318`
  - `src/ast/query.rs:368`
  - `src/ast/query.rs:480`
  - `src/ast/query.rs:656`
  - `src/ast/query.rs:806`
  - `src/ast/query.rs:1453`
  - `src/parser/query.rs:975`
  - `src/parser/query.rs:996`
- Problem:
  - 11 warnings-as-errors (`derivable_impls`, `large_enum_variant`, `collapsible_if`, `empty_line_after_outer_attr`).
- Impact:
  - Violates project quality gate and blocks CI if strict clippy is required.
- Recommended fix:
  - Address all reported items directly.
  - For large enums, introduce `Box` indirection on outlier variants.

### FIX-009: AST memory layout risk from very large enum variants
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/ast/query.rs:191`
  - `src/ast/query.rs:318`
  - `src/ast/query.rs:656`
  - `src/ast/query.rs:806`
- Problem:
  - Several enums carry very large variants by value.
- Impact:
  - Higher memory use/copy cost and potential perf degradation in parser and future semantic passes.
- Recommended fix:
  - Box large payload variants (`SelectStatement`, `GraphPattern`, `ElementPattern`, `FullEdgePattern`, etc.).

### FIX-010: New pattern keywords are not included in `TokenKind::is_keyword()`
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/lexer/token.rs:331`
  - Missing entries for: `Repeatable`, `Different`, `Keep`, `Shortest`, `Paths`, `Groups`, `Labels`
- Problem:
  - Keywords exist in lexer mapping/display but are omitted from keyword classification helper.
- Impact:
  - Inconsistent keyword behavior in utilities/tests/features that rely on `is_keyword()`.
- Recommended fix:
  - Add the new tokens to `is_keyword()` and extend tests.

### FIX-011: `ParseResult.ast` is always `Some(...)` in parser entrypoint
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/mod.rs:58`
- Problem:
  - API shape suggests parse can fail (`Option<Program>`), but implementation always returns `Some(program)`.
- Impact:
  - Ambiguous contract for callers and weak signal for catastrophic parse failure.
- Recommended fix:
  - Either make failures explicit (`None` on fatal parse) or change API to non-optional AST + diagnostics.

## Priority 3 (Test and Documentation Debt)

### FIX-012: Pattern tests validate type construction more than parser behavior
- Severity: High
- Status: ✅ Resolved February 17, 2026
- Files:
  - `tests/pattern_tests.rs:25`
  - `tests/pattern_tests.rs:45`
  - `tests/pattern_tests.rs:252`
  - `tests/pattern_tests.rs:264`
- Problem:
  - Most tests manually construct AST nodes and assert enum presence.
  - Very few assertions verify parsed AST content or token consumption semantics.
- Impact:
  - Major parser defects can pass tests (currently happening).
- Recommended fix:
  - Add behavior tests that parse source and assert exact query/pattern AST shape and diagnostics.

### FIX-013: Query parser tests still reflect Sprint 7 placeholder assumptions
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `src/parser/query.rs:1998`
  - `src/parser/query.rs:2024`
- Problem:
  - Comments/tests still note placeholder behavior and skip important assertions (for example result statement after MATCH).
- Impact:
  - Makes it easy for regressions to remain unnoticed post-Sprint 8.
- Recommended fix:
  - Replace placeholder comments with strict assertions once parser integration is fixed.

### FIX-014: Documentation/status overstates implementation completeness
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `SPRINT8.md:9`
  - `SPRINT8.md:24`
  - `SPRINTS.md:71`
- Problem:
  - Sprint status docs previously implied broader completion than current parser coverage.
- Impact:
  - Misleads sprint planning and masks true remaining work.
- Recommended fix:
  - Keep sprint docs explicit about completed integration work vs remaining Sprint 8 grammar breadth.

### FIX-015: Example behavior is currently misleading
- Severity: Medium
- Status: ✅ Resolved February 17, 2026
- Files:
  - `examples/parser_demo.rs:8`
- Problem:
  - Demo includes statements that now emit diagnostics due parser behavior inconsistencies.
- Impact:
  - New contributors may misinterpret parser health.
- Recommended fix:
  - Align demo inputs with supported grammar and include expected diagnostics in output checks.

## Suggested Fix Order
1. FIX-001, FIX-002, FIX-003 (restore trustworthy top-level query parsing contract).
2. FIX-004, FIX-005, FIX-006 (close major query/pattern correctness gaps).
3. FIX-008, FIX-009, FIX-010 (restore quality/perf baseline and lexer consistency).
4. FIX-012, FIX-013, FIX-014, FIX-015 (raise confidence and prevent regression drift).
5. FIX-007 and FIX-011 (resolve contract/grammar policy decisions explicitly).

## Definition of Done for this file
- `cargo test` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- Public `parse()` builds real query AST for representative Sprint 7/8 samples.
- Query/pattern parsing failures produce diagnostics and make parser progress.
- Integration tests assert AST structure (not only existence).
