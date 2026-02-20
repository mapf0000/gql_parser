# ISO GQL Gaps and Fix Plan

## Gap Inventory (Current State)

**Parser-only ISO compatibility: NOT 100%**
**Estimated parser grammar coverage: ~80-85% (non-validated estimate)**

Last Updated: 2026-02-20

### Implemented (Parser, Recently Closed)

#### 1. Nested Expression Core Forms
- [x] `VALUE { ... }` nested query specification parsing
- [x] `PROPERTY GRAPH <expr>` parsing (including `CURRENT GRAPH`)
- [x] `BINDING TABLE <expr>` parsing, including nested query form `BINDING TABLE { ... }`

#### 2. Typed Reference-Value Spec Parsing
- [x] `PROPERTY GRAPH <nested_spec>` now parsed through real graph type parser
- [x] Typed `NODE` reference values parsed via real node type specification parser
- [x] Typed `EDGE` reference values parsed via real edge type specification parser

#### 3. Graph Type Edge-Phrase Fidelity
- [x] `CONNECTING (source TO destination)` endpoints are preserved in AST (no synthesized placeholder endpoints in that path)

### Not Implemented (Parser Scope)

#### 1. Top-Level Procedure Definition Grammar
- [ ] Full top-level procedure definition statement forms (catalog lifecycle for procedure definitions)
- [ ] Remaining ISO profile-specific procedure-definition variants not currently recognized by program-level parser

#### 2. Advanced SELECT Features
- [ ] Window functions
- [ ] `WITH` clause (Common Table Expressions)
- [ ] Complex nested table expressions in FROM

#### 3. Advanced Graph Type Features (Grammar §33, Lines 2320-2600)
- [ ] `ABSTRACT` types
- [ ] Type inheritance
- [ ] Full constraint specifications

### Deferred (Not Inferable From Input Text Alone)

- [ ] Schema-driven property existence and property-type checks
- [ ] Schema constraint enforcement
- [ ] Schema-aware semantic validation beyond syntax
- [ ] Runtime/cross-statement semantics requiring external catalog/runtime state

### Partially Implemented

#### 1. Graph Pattern Expressions
- [ ] Complex parenthesized path patterns with deeply nested quantifiers
- [ ] Simplified path pattern syntax (AST exists, parser partial)
- [ ] Pattern union/alternation in all contexts

#### 2. Built-in Functions
- [ ] Some specialized string manipulation variants
- [ ] Some datetime manipulation functions
- [ ] Advanced list/collection operations
- Coverage: ~90% of standard functions

#### 3. Statement Isolation
- [ ] Semicolon-separated statements share scope in some cases
- [ ] Parser does not create fully isolated `Statement` objects in every case
- Documented in tests

### Known Limitations

1. Error recovery: basic synchronization works; could be more sophisticated.
2. Procedure definitions: invocation and nested bodies are supported; full top-level definition grammar remains incomplete.
3. Nested quantifiers: some deeply nested cases may not parse optimally.
4. Type inference: returns `Type::Any` for complex expressions without schema.

### Priority Order

#### High Priority (95%+ compliance)
1. Top-level procedure definition grammar completion
2. Advanced SELECT (`WITH` / window / nested FROM)
3. Advanced graph type grammar completion (`ABSTRACT` / inheritance / constraints)

#### Medium Priority (completeness)
1. Schema integration for semantic typing/constraints
2. Improved error recovery
3. Statement isolation hardening

#### Low Priority (edge cases)
1. Complex pattern composition edge cases
2. Specialized function variants
3. Type inference quality improvements

### References

- Grammar: `third_party/opengql-grammar/GQL.g4`
- ISO Standard: ISO/IEC 39075:2024

## Scope and Goal

This plan addresses every gap item captured in the inventory below and drives the parser from feature-partial to high-confidence ISO GQL conformance with production-grade diagnostics and semantic validation.

Primary targets:

- Raise parser grammar coverage from ~80-85% to >=95%.
- Remove known placeholders in parser/AST paths that currently hide missing functionality.
- Keep practical usability >=95% while increasing correctness and predictability.
- Preserve current stability baseline (`cargo test -q` passing) during all phases.

## Baseline (Current State)

Current repository behavior relevant to this plan:

- Program-level parsing and statement splitting are active in `src/parser/program.rs`.
- Procedure invocation and nested procedure body parsing are active in `src/parser/procedure.rs`; top-level procedure-definition statement grammar remains incomplete.
- Nested expressions now include parser support for `VALUE { ... }`, `PROPERTY GRAPH ...`, and `BINDING TABLE { ... }` in `src/parser/expression.rs`.
- Typed graph/node/edge reference value parsing now uses real graph type parsers (no placeholder synthetic typed-spec path in `src/parser/types/reference.rs`).
- Edge phrase parsing in graph type parser now preserves endpoint aliases from `CONNECTING (source TO destination)` in `src/parser/graph_type.rs`.
- Catalog graph type source handling in `src/ast/catalog.rs` still retains `GraphTypeSource::Detailed { span }` as a coarse-grained representation (not full-fidelity model embedding).
- Schema validation pass currently focuses mostly on label validation (`src/semantic/validator/schema_validation.rs`) and does not fully enforce property/type/constraint semantics.

## Engineering Principles (Non-Negotiable)

- Grammar-driven implementation: every parser change must map to explicit `GQL.g4` productions.
- No silent placeholders for "done" features: placeholder nodes are allowed only behind clearly marked temporary flags and must be tracked with closure tickets.
- Parser determinism and progress guarantees: every successful parse branch must consume tokens; ambiguous branches require ordered fallback strategy with explicit tie-break rules.
- Recovery without corruption: error recovery must synchronize cleanly and avoid creating semantically misleading AST shapes.
- Incremental delivery: ship vertical slices (AST + parser + semantic + tests + docs) per feature group.
- Compatibility discipline: if AST changes are breaking, gate behind explicit changelog and migration notes.
- Test-first for edge cases: add failing conformance tests before implementation where practical.

## Delivery Structure

Use workstreams mapped 1:1 to the gap categories in this document, with cross-cutting hardening tracks.

Planned order:

1. Gap re-validation and conformance harness hardening
2. Procedure definitions and nested query/value expression correctness
3. Advanced SELECT extensions
4. Schema-dependent semantic enforcement
5. Advanced graph type completion
6. Partial-feature hardening (patterns/functions/isolation)
7. Error recovery and type inference upgrades

## Workstream 0: Gap Re-Validation and Measurement

Objective: establish an auditable, automated source of truth for remaining gaps.

Tasks:

- Build a feature matrix that maps each gap item in this document to:
  - Grammar production(s)
  - AST node(s)
  - Parser entrypoint(s)
  - Semantic pass(es)
  - Existing tests
- Add a conformance status section in this document or a generated artifact that distinguishes:
  - Implemented
  - Partially implemented
  - Placeholder implementation
  - Not implemented
- Add targeted golden tests for each currently failing/partial grammar production before refactors.

Code areas:

- `third_party/opengql-grammar/GQL.g4`
- `tests/*` (new conformance-focused files grouped by feature)
- `GAPS_FIXED.md` (status updates as work lands)

Acceptance criteria:

- Every row in the gap inventory has traceable test IDs and code ownership.
- No "unknown status" rows remain.

## Workstream 1: Top-Level Procedure Definitions (Open)

Objective: complete parser support for top-level procedure-definition statements.

Status (2026-02-20):

- [x] Nested procedure body pieces are parser-supported (`procedureBody`, `bindingVariableDefinitionBlock`, `atSchemaClause`, `nextStatement`) for inline/nested procedure specs.
- [ ] Top-level procedure definition statement grammar remains incomplete.

Remaining tasks:

- Add explicit AST nodes for top-level procedure definition/catalog lifecycle statements.
- Extend `src/parser/program.rs` to recognize top-level procedure definition forms.
- Add conformance tests covering positive and negative top-level definition cases.

Code areas:

- `src/ast/procedure.rs`
- `src/ast/program.rs`
- `src/ast/catalog.rs`
- `src/parser/program.rs`
- `tests/procedure_definition_tests.rs` (new)

## Workstream 2: Nested Expressions (Parser Core Closed)

Objective: implement grammar semantics for nested query/graph/binding-table expressions.

Status (2026-02-20):

- [x] `VALUE` now parses `nestedQuerySpecification` payload (`VALUE { ... }`).
- [x] `PROPERTY GRAPH` expression parsing updated and covered by tests.
- [x] `BINDING TABLE` expression supports nested query payload form (`BINDING TABLE { ... }`).
- [x] Parser/AST/test updates landed in:
  - `src/ast/expression.rs`
  - `src/parser/expression.rs`
  - `tests/type_reference_spec_tests.rs`
  - `src/parser/expression.rs` test module

Remaining (outside parser-only closure):

- [ ] Semantic scope/type evaluation for nested-query result propagation remains limited and depends on broader semantic pass upgrades.

## Workstream 3: Advanced SELECT Features (Not Implemented #3)

Objective: support advanced SELECT capabilities while keeping grammar conformance explicit.

Note:

- Since upstream `GQL.g4` may not include full SQL-style window/CTE syntax, treat these as extension features unless confirmed as ISO-required for target profile.

### 3.1 Window functions

Tasks:

- Extend expression AST with window specification nodes.
- Parse function-call + `OVER (...)` clauses including partition/order/frame variants as agreed scope.
- Add validator rules for window function placement (e.g., disallowed contexts).

Code areas:

- `src/ast/expression.rs`
- `src/parser/expression.rs`
- `src/semantic/validator/context_validation.rs`

### 3.2 WITH clause / CTE support

Tasks:

- Add AST for CTE definitions and references.
- Add parser support at query-entry level before `SELECT`.
- Implement CTE scope/lifetime and shadowing rules.

Code areas:

- `src/ast/query.rs`
- `src/parser/query/mod.rs`
- `src/parser/query/result.rs`
- `src/semantic/validator/scope_analysis.rs`
- `src/semantic/validator/variable_validation.rs`

### 3.3 Complex nested table expressions in FROM

Tasks:

- Expand `SelectFromClause` to support deeply nested derived-table patterns and aliasing.
- Ensure recursive parse of nested query/table expression combinations without ambiguity.
- Strengthen boundary detection logic in query parser to avoid premature clause termination.

Code areas:

- `src/ast/query.rs`
- `src/parser/query/result.rs`
- `src/parser/query/mod.rs`

### 3.4 Tests

Add:

- Matrix tests for each SELECT extension independently and in combination.
- Error-recovery tests for malformed `WITH`, malformed window specs, bad nested FROM expressions.

Suggested files:

- `tests/select_window_tests.rs` (new)
- `tests/select_cte_tests.rs` (new)
- `tests/select_from_nested_tests.rs` (new)

Acceptance criteria:

- Advanced SELECT rows move to implemented or explicitly documented extension scope with tests.

## Workstream 4: Schema-Dependent Features (Not Implemented #4)

Objective: move from label-only schema checks to full schema-aware semantic validation.

### 4.1 Schema model expansion

Tasks:

- Expand schema abstractions for:
  - Property existence per label
  - Property type metadata
  - Constraint metadata
  - Inheritance-aware lookup (if graph type inheritance is enabled)

Code areas:

- `src/semantic/schema.rs`
- `src/semantic/catalog.rs`

### 4.2 Schema-aware scope/type context

Tasks:

- Track label context for bound variables (node/edge) during scope analysis.
- Propagate context into property reference validation and type inference.

Code areas:

- `src/semantic/validator/scope_analysis.rs`
- `src/ir/symbol_table.rs`
- `src/semantic/validator/type_inference.rs`

### 4.3 Property/label/constraint validation

Tasks:

- Validate property existence by context.
- Validate property type compatibility in expressions and assignments.
- Enforce schema constraints where applicable.

Code areas:

- `src/semantic/validator/schema_validation.rs`
- `src/semantic/validator/type_checking.rs`
- `src/semantic/validator/expression_validation.rs`

### 4.4 Tests

Add:

- Schema fixtures for valid/invalid label and property references.
- Type mismatch tests tied to schema-declared property types.
- Constraint violation tests.

Suggested file:

- `tests/schema_semantics_tests.rs` (new)

Acceptance criteria:

- Schema-dependent row set in this document is closed with measurable tests and docs.

## Workstream 5: Advanced Graph Type Features (Partially Closed)

Objective: finish graph-type parser conformance and remove remaining simplifications.

Status (2026-02-20):

- [x] Placeholder typed-spec parsing removed from `src/parser/types/reference.rs`.
- [x] Typed graph/node/edge reference parsing now uses real graph type parsers.
- [x] Edge phrase `CONNECTING (source TO destination)` now preserves endpoints in AST.
- [ ] `ABSTRACT` types not yet implemented.
- [ ] Type inheritance grammar not yet implemented.
- [ ] Full graph-type constraint grammar not yet implemented.
- [ ] `CREATE GRAPH TYPE` still stores coarse `GraphTypeSource::Detailed { span }` instead of full embedded model payload.

Remaining code areas:

- `src/ast/graph_type.rs`
- `src/parser/graph_type.rs`
- `src/ast/catalog.rs`
- `src/parser/program.rs`

Tests:

- Existing coverage extended in `tests/graph_type_tests.rs` and `tests/type_reference_spec_tests.rs`.
- Additional conformance tests still needed for constraints/inheritance/abstract-type forms.

## Workstream 6: Partially Implemented Items

### 6.1 Graph pattern expressions hardening

Tasks:

- Add deep nested quantifier parse tests for parenthesized path patterns.
- Fix parser precedence/association gaps for union/alternation in all contexts.
- Ensure simplified path syntax and full syntax both round-trip into coherent AST.

Code areas:

- `src/parser/patterns/path.rs`
- `src/parser/patterns/mod.rs`
- `src/ast/query.rs`

### 6.2 Built-in function coverage completion

Tasks:

- Build function inventory from grammar and compare against:
  - `FunctionName` enum in `src/ast/expression.rs`
  - `parse_function_name` and classifier in `src/parser/expression.rs`
- Implement missing specialized string/datetime/list variants.
- Add function-arity and argument-type semantic checks.

Code areas:

- `src/ast/expression.rs`
- `src/parser/expression.rs`
- `src/semantic/validator/expression_validation.rs`
- `src/semantic/validator/type_checking.rs`

### 6.3 Statement isolation cleanup

Current issue:

- Composite-query isolation currently uses a brittle statement-id offset strategy in scope analysis.

Tasks:

- Replace offset hack with explicit branch scope model.
- Ensure semicolon-separated and composite branches have formally isolated symbol visibility.
- Expand tests for nested composite forms and mixed query/mutation boundaries.

Code areas:

- `src/semantic/validator/scope_analysis.rs`
- `src/semantic/validator/variable_validation.rs`
- `src/parser/program.rs`

Acceptance criteria:

- All partial rows in this document become implemented with targeted regression tests.

## Workstream 7: Known Limitations

### 7.1 Error recovery improvements

Tasks:

- Standardize synchronization sets by parser context (expression, clause, statement, block).
- Prevent cascading diagnostics from one malformed region.
- Add parser recovery invariants (never panic, always progress, bounded diagnostic storms).

Code areas:

- `src/parser/program.rs`
- `src/parser/query/mod.rs`
- `src/parser/procedure.rs`
- `src/parser/patterns/mod.rs`

### 7.2 Nested quantifier robustness

Tasks:

- Stress/fuzz complex path-quantifier combinations.
- Refactor quantifier parse internals where necessary to avoid ambiguous rollback paths.

Code areas:

- `src/parser/patterns/path.rs`
- `tests/stress_tests.rs`

### 7.3 Type inference quality

Tasks:

- Reduce `Type::Any` fallback usage by integrating symbol and schema context.
- Improve cast/result inference and function return type inference.
- Feed inferred types into downstream checks consistently.

Code areas:

- `src/semantic/validator/type_inference.rs`
- `src/semantic/validator/type_checking.rs`
- `src/ir/type_table.rs`

Acceptance criteria:

- Known-limitation rows are either closed or reduced to explicitly tracked, narrow edge cases.

## Cross-Cutting Test and Quality Strategy

Required per phase:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings` (or project-approved lint baseline)
- `cargo test -q`
- Conformance test subset tied to changed area

Additions:

- Property-based parse tests for critical grammars (path patterns, graph types, nested queries).
- Corpus regression tests using `third_party/opengql-grammar/samples`.
- Performance guardrails via existing benches (`benches/parser_benchmarks.rs`) for large/deep inputs.

## Documentation and Change Management

For each completed sub-workstream:

- Update `GAPS_FIXED.md` status with exact completion date and test references.
- Update `CHANGELOG.md` with user-visible parser/AST/semantic changes.
- Update docs where behavior changed:
  - `docs/USER_GUIDE.md`
  - `docs/AST_GUIDE.md`
  - `docs/SEMANTIC_VALIDATION.md`

If AST-breaking changes occur:

- Add migration notes and explicit versioning guidance in README/changelog.

## Execution Plan (Milestones)

Milestone 1: Re-baseline and tests-first

- Status: In progress
- Deliver Workstream 0
- Freeze acceptance tests for all gap rows

Milestone 2: Procedure + nested expressions

- Status: Partially complete
- Workstream 2 (nested-expression parser core): complete
- Workstream 1 (top-level procedure definitions): open

Milestone 3: Graph type + schema semantics

- Status: Partially complete
- Workstream 5 placeholder-removal sub-slice: complete
- Workstream 5 advanced graph-type grammar + Workstream 4 schema semantics: open

Milestone 4: Advanced SELECT + partial hardening

- Status: Open
- Deliver Workstream 3 + 6
- Finalize extended query functionality and coverage

Milestone 5: Recovery/type-quality polish

- Status: Open
- Deliver Workstream 7
- Lock release-quality diagnostics and inference behavior

## Risk Register and Mitigations

Risk: grammar ambiguity causing parser regressions.

- Mitigation: grammar-mapped tests before and after refactors; token-consumption assertions in parser branches.

Risk: AST churn breaks downstream consumers.

- Mitigation: staged AST changes with compatibility notes and migration documentation.

Risk: semantic passes become tightly coupled and brittle.

- Mitigation: keep pass contracts explicit (input/output metadata), add pass-level unit tests.

Risk: recovery improvements increase false positives.

- Mitigation: diagnostic quality tests for malformed corpora and maximum-diagnostics caps per statement.

Risk: scope/isolation fixes regress existing semantics.

- Mitigation: add targeted scope matrix tests (semicolon, nested queries, UNION/EXCEPT/INTERSECT).

## Definition of Done

This plan is complete only when all conditions hold:

- Every row in this document's gap inventory is marked implemented, deferred with rationale, or explicitly out-of-scope extension.
- Every implemented row has direct tests (unit/integration/conformance) and no placeholder-only implementation path.
- Parser, semantic validator, and docs are aligned (no stale claims).
- Full project test suite passes, including newly added gap-closure tests.
- `GAPS_FIXED.md` is consistent and current.
