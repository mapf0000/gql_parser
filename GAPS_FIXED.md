# Remaining GQL Milestones (Open Only)

Last Updated: 2026-02-20

This file tracks only milestones that are still open after closing all parser-core milestones marked `DB/Catalog: No`.

- Closed and traceable work is recorded in `docs/conformance_matrix.csv`.
- Generated status output is in `docs/conformance_status.md`.

Notation:

- `DB/Catalog: Yes` means the item needs external schema/catalog metadata.
- `DB/Catalog: Partial` means parser/core parts are local, but full closure needs schema/catalog inputs.

## Milestone 3 (Open): Advanced Graph Type + Schema Semantics

### Workstream 4 (Open): Schema-dependent semantic enforcement

Remaining work:

- `DB/Catalog: Yes` Expand schema model for property existence/type metadata and constraints. Needed from DB/catalog: canonical schema metadata for labels/edge-types, property definitions, declared property value types, declared constraints, and inheritance relations.
- `DB/Catalog: Yes` Add schema-aware context propagation into scope/type inference. Needed from DB/catalog: active graph/schema resolution plus variable-to-label/type metadata derived from catalog schema.
- `DB/Catalog: Yes` Enforce property/type/constraint checks against schema metadata. Needed from DB/catalog: property dictionaries per label/edge-type, type compatibility metadata, and constraint definitions/rules to validate against.
- `DB/Catalog: Yes` Add schema semantics fixtures and regression tests. Needed from DB/catalog: representative schema fixtures (valid/invalid) that reflect real catalog structure and constraints.

Public integration/API plan (generic + mockable):

- Add `pub trait SchemaCatalog` as the engine-facing entrypoint:
  `fn snapshot(&self, request: SchemaSnapshotRequest) -> Result<Arc<dyn SchemaSnapshot>, CatalogError>`.
- Add `pub trait SchemaSnapshot` as an immutable, query-time view:
  `fn node_type(&self, name: &str) -> Option<NodeTypeMeta>`,
  `fn edge_type(&self, name: &str) -> Option<EdgeTypeMeta>`,
  `fn property(&self, owner: TypeRef, property: &str) -> Option<PropertyMeta>`,
  `fn constraints(&self, owner: TypeRef) -> &[ConstraintMeta]`,
  `fn parents(&self, owner: TypeRef) -> &[TypeRef]`.
- Add `pub trait GraphContextResolver`:
  `fn active_graph(&self, session: &SessionContext) -> Result<GraphRef, CatalogError>`,
  `fn active_schema(&self, graph: &GraphRef) -> Result<SchemaRef, CatalogError>`.
- Add `pub trait VariableTypeContextProvider` for scope/type propagation:
  `fn initial_bindings(&self, graph: &GraphRef, ast: &Program) -> Result<VariableTypeContext, CatalogError>`.
- Keep current lightweight `Schema` trait as a compatibility adapter over `SchemaSnapshot` for simple integrations.
- Provide first-party test doubles:
  `InMemorySchemaCatalog`, `MockGraphContextResolver`, `MockVariableTypeContextProvider`.
- Provide fixture loader API:
  `pub trait SchemaFixtureLoader { fn load(&self, fixture: &str) -> Result<Arc<dyn SchemaSnapshot>, FixtureError>; }`
  so schema regression suites are engine-agnostic.

## Milestone 4 (Partially Open): Partial-feature hardening

### Workstream 6.2 (Open subset): Built-in function completeness

Remaining work:

- `DB/Catalog: Partial` Complete function arity/argument semantic validation coverage where still missing. Needed from DB/catalog (only if catalog/UDF validation is in scope): procedure/UDF signatures (name, arity, parameter/return types).

Public integration/API plan (generic + mockable):

- Add `pub trait CallableCatalog`:
  `fn resolve(&self, name: &str, kind: CallableKind, ctx: &CallableLookupContext) -> Result<Vec<CallableSignature>, CatalogError>`.
- Define stable signature payloads:
  `CallableSignature { name, kind, parameters: Vec<ParameterSignature>, return_type, volatility, nullability }`.
- Add arity/type validator interface:
  `pub trait CallableValidator { fn validate_call(&self, call: &FunctionCall, sigs: &[CallableSignature]) -> Vec<Diag>; }`.
- Keep built-in functions in a separate `BuiltinCallableCatalog` so external engines can compose:
  `CompositeCallableCatalog { builtins, external }`.
- Provide test doubles:
  `InMemoryCallableCatalog` with declarative signature registration and deterministic overload ordering.

## Milestone 5 (Open): Recovery and Type-Quality Polish

### Workstream 7.3 (Open): Type inference quality

Remaining work:

- `DB/Catalog: Partial` Reduce `Type::Any` fallback in complex expressions. Needed from DB/catalog for full precision: property type metadata bound to graph labels/edge-types.
- `DB/Catalog: Partial` Improve cast/function-return/result inference quality. Needed from DB/catalog when non-built-in callable resolution is required: function/procedure signatures and return-type metadata.
- `DB/Catalog: Partial` Integrate inference consistently into downstream type checks. Needed from DB/catalog for schema-precise checks: resolved schema type metadata used by type-checking passes.

Public integration/API plan (generic + mockable):

- Add `pub trait TypeMetadataCatalog`:
  `fn property_type(&self, owner: TypeRef, property: &str) -> Option<Type>;`
  `fn callable_return_type(&self, call: &CallableInvocation) -> Option<Type>;`
  `fn cast_rules(&self) -> &dyn CastRuleSet;`.
- Add `pub trait CastRuleSet`:
  `fn can_cast(&self, from: &Type, to: &Type) -> bool;`
  `fn cast_result_type(&self, from: &Type, to: &Type) -> Type;`.
- Add `pub struct InferenceServices` that wires all external dependencies:
  `{ schema_snapshot, callable_catalog, type_metadata_catalog, graph_context }`.
- Add `pub trait TypeCheckContextProvider`:
  `fn type_context(&self, statement_id: StatementId) -> TypeCheckContext;`
  so inferred types are consumed uniformly by downstream checkers.
- Expose deterministic fallback policy:
  `pub struct InferencePolicy { allow_any_fallback: bool, prefer_schema_types: bool, unknown_callable_behavior: UnknownCallableBehavior }`.
- Provide mocks:
  `MockTypeMetadataCatalog`, `MockCastRuleSet`, `MockTypeCheckContextProvider`.

## Cross-milestone API conventions

- All public integration traits must be `Send + Sync` safe to support multi-threaded validator execution.
- All externally supplied metadata is snapshot-based and immutable during one validation run.
- All traits return typed errors (`CatalogError`/`FixtureError`) and never panic.
- All default test implementations must live in-tree and be usable without a running DB engine.
- Validator constructors must accept interfaces via dependency injection, not global singletons, to keep tests isolated.

## Definition of Done (Remaining Scope)

The plan is complete when all remaining rows above are closed with direct tests and catalog-backed fixtures, diagnostics behavior is stable, and docs/changelog are synchronized with final parser + semantic behavior.
