# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project adheres to Semantic Versioning.

## [0.1.0] - 2026-02-19

### Added
- Public AST visitor framework:
  - `AstVisitor` and `AstVisitorMut`
  - query-focused walk helpers with `ControlFlow` short-circuiting
  - ready-to-use visitors: `CollectingVisitor`, `SpanCollector`, `VariableCollector`
- Public analysis APIs for compiler-facing metadata:
  - `ExpressionInfo::analyze(&Expression)`
  - `PatternInfo::analyze(&GraphPattern)`
  - `QueryInfo::from_ast(&Statement)`
  - `VariableDependencyGraph::build(&Statement)`
- New analysis examples and user-facing guides.
- Release assets:
  - Apache-2.0 licensing (`LICENSE-APACHE`)
  - crate metadata for publishing
  - publish dry-run readiness checks

### Changed
- Improved visitor traversal coverage for simplified path-pattern forms.
- Updated README with parse, traversal, and analysis usage.
- Parser now treats `VALUE` payloads as nested query specifications (`VALUE { ... }`) instead of expression payloads.
- Reference value type parsing now uses full graph-type parsers for typed graph/node/edge specs.

### Fixed
- Resolved visitor/AST field mismatches in simplified path expression walking.
- Addressed strict clippy violations required for release gates.
- Preserved endpoint aliases when parsing edge phrase patterns with `CONNECTING (source TO destination)`.
- Added parser conformance tests for nested expressions and typed reference specifications.
