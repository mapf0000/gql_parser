# GQL Parser Sprint Roadmap

## Purpose
High-level sprint plan from project start to feature-complete ISO GQL parser delivery.
This document is intentionally broad and will be refined into detailed sprint plans later.

## Planning Assumptions
- Parser stack: `logos` + `chumsky` + custom `ast` + `miette` diagnostics.
- Source-of-truth feature scope: `GQL_FEATURES.md` and `third_party/opengql-grammar/GQL.g4`.
- Output contract: `parse(&str) -> { ast: Option<Ast>, diags: Vec<miette::Report> }`.
- Parser must never panic on invalid input and should return partial AST where possible.

## Architecture Direction Update (February 17, 2026)
- The project has now committed to `logos` for lexing and `chumsky` for parser control flow/recovery.
- Token payload storage is being standardized around `smol_str` to reduce allocation pressure.
- Top-level semicolon handling no longer materializes empty statements in AST output.
- Temporal values are tokenized structurally (`DATE` + string literal, etc.) and will be assembled in parser/AST layers.
- Public `parse(&str)` diagnostics are emitted as `miette::Report` values at the API boundary.

## Completion Definition
- All feature families from `GQL_FEATURES.md` are parsed with documented coverage status.
- Recovery and diagnostics are stable for malformed input and large queries.
- Conformance test suite and sample corpus pass at agreed quality threshold.
- Public API and AST are documented and versioned for downstream use.

## Sprint Sequence

### Sprint 0: Project Foundations
- Goal: Establish base project architecture and engineering guardrails.
- Scope: crate layout, module boundaries (`lexer`, `parser`, `ast`, `diag`, `lib`), CI, lint/test baseline.
- Exit Criteria: baseline crate builds cleanly, strict clippy/test wiring in place, developer workflow documented.

### Sprint 1: Diagnostics and Span Infrastructure (Completed February 17, 2026)
- Goal: Build reusable diagnostics model before language breadth.
- Scope: `Span`, `Spanned<T>`, internal `Diag`, severity/help/labels/notes, `miette` conversion.
- Exit Criteria: diagnostic pipeline supports multi-label errors and source snippets end-to-end.

### Sprint 2: Lexer Core and Token Model
- Goal: Implement robust lexical layer with error-tolerant scanning.
- Scope: token kinds, keywords, identifiers, literals, operators, comments/whitespace, parameter tokens, lexer errors.
- Exit Criteria: lexer emits `Vec<Token> + Vec<Diag>` with continuation after lexical errors.
- Implementation note: lexer implementation is `logos`-based.

### Sprint 3: Parser Skeleton and Recovery Framework
- Goal: Establish parser control flow, AST entry points, and recovery strategy.
- Scope: parser context/state, clause-boundary recovery, partial AST policy, top-level program framing.
- Exit Criteria: parser handles malformed inputs without panic and returns structured partial results.
- Implementation note: statement skeleton parsing and recovery are `chumsky`-driven.

### Sprint 4: Program, Session, Transaction, Catalog Statements (Completed February 17, 2026)
- Goal: Cover operational statement surface outside query/pattern core.
- Scope: program structure, session set/reset/close, transaction lifecycle, create/drop schema/graph/graph type, catalog calls.
- Exit Criteria: all corresponding statement families parse with correct AST forms and diagnostics.
- Status: ✅ **Completed** - All AST nodes defined, parser builds command-level AST variants for Sprint 4 statement families, and comprehensive tests pass.

### Sprint 5: Values, Literals, and Expression Core
- Goal: Implement expression backbone used by nearly all clauses.
- Scope: value expressions, predicates, case/cast, parameters, literals, constructors, operator precedence.
- Exit Criteria: expression parser is reusable across query/mutation/procedure contexts.

### Sprint 6: Type System and Reference Forms
- Goal: Add complete type grammar and catalog/object reference syntax.
- Scope: predefined/constructed/dynamic union types, graph/node/edge/binding-table types, schema/graph/procedure references.
- Exit Criteria: type annotations and reference forms parse consistently in all legal contexts.

### Sprint 7: Query Pipeline Core
- Goal: Implement linear/composite query composition and clause chaining.
- Scope: composite queries, set operators plus `OTHERWISE`, focused/ambient linear queries, `MATCH`, `FILTER`, `LET`, `FOR`, `SELECT`.
- Exit Criteria: query chains and nested query forms parse correctly with recovery at clause boundaries.

### Sprint 8: Graph Pattern and Path Pattern System
- Goal: Deliver full graph matching syntax breadth.
- Scope: graph pattern binding/yield, match modes, node/edge patterns, path prefixes/search modes, quantifiers, simplified path patterns, label expressions.
- Exit Criteria: pattern AST supports all directional and quantified variants in grammar.

### Sprint 9: Result Shaping and Aggregation
- Goal: Complete result production features.
- Scope: `RETURN`, `FINISH`, grouping, ordering, pagination, aggregate functions, set quantifiers, having/yield interactions.
- Exit Criteria: result-shaping clauses compose correctly with query and procedure constructs.

### Sprint 10: Data Modification Statements
- Goal: Implement graph mutation grammar end-to-end.
- Scope: `INSERT` graph patterns, `SET`, `REMOVE`, `[DETACH|NODETACH] DELETE`, data-modifying calls.
- Exit Criteria: mutation statements parse in focused and ambient forms with valid AST structure.

### Sprint 11: Procedures, Nested Specs, and Execution Flow
- Goal: Complete procedural composition features.
- Scope: inline/named calls, variable scope clause, procedure args, optional call, `YIELD`, nested procedure specs, `NEXT` chaining, `AT`/`USE` context clauses.
- Exit Criteria: procedural flows parse across query, catalog-modifying, and data-modifying contexts.

### Sprint 12: Graph Type Specification Depth
- Goal: Finish advanced schema/type modeling grammar.
- Scope: nested graph type specs, node/edge type patterns and phrases, endpoint pairs, label/property type specs.
- Exit Criteria: graph type AST is stable and ready for future semantic validation.

### Sprint 13: Conformance Hardening and Edge Cases
- Goal: Raise parser reliability and standards alignment.
- Scope: reserved/pre-reserved/non-reserved keyword behavior, ambiguity handling, stress cases, grammar sample corpus integration.
- Exit Criteria: high-confidence parse behavior on official samples and curated edge-case corpus.

### Sprint 14: Semantic Validation Pass (Post-Parse Phase)
- Goal: Add first semantic layer planned in architecture.
- Scope: `Ast -> IR + Vec<Diag>` checks for undefined vars, invalid pattern references, type-shape constraints, context rules.
- Exit Criteria: semantic diagnostics are emitted without destabilizing syntax parser guarantees.

### Sprint 15: Release Readiness and Project Completion
- Goal: Finalize quality gates for feature-complete parser release.
- Scope: API stabilization, coverage report, performance baseline, fuzz/property tests, docs, migration notes.
- Exit Criteria: release candidate approved against completion definition.

## Feature Coverage Mapping
- Sprints 4-12 map directly to feature families in `GQL_FEATURES.md` sections 1-21.
- Sprint 13 validates conformance breadth and recovery quality across all sections.
- Sprint 14 introduces planned semantic validation layer from `AGENTS.md`.
- Sprint 15 closes delivery risks and prepares downstream adoption.

## Risks to Track Across All Sprints
- Grammar permissiveness vs semantic correctness boundaries.
- Error recovery quality regressions as grammar breadth increases.
- AST stability pressure from early consumers.
- Keyword/identifier ambiguities and case-insensitive lexing edge cases.

## Inputs for Detailed Sprint Planning (Next Step)
- Target sprint length and team capacity assumptions.
- Priority policy: strict standards coverage first vs common-query usability first.
- Explicit test strategy split: unit, snapshot, corpus, fuzz, property-based.
- Definition of coverage metrics for parser and diagnostics.
