# Metadata Unification Plan

## Overview

**Goal**: Replace scattered validation traits with a single, clean `MetadataProvider` interface.

**Status**: Core implementation complete ✅ | Tests need cleanup 🔧

---

## What Was Accomplished

### 1. Core Architecture ✅

Created `MetadataProvider` trait in [src/semantic/metadata_provider.rs](src/semantic/metadata_provider.rs):

```rust
pub trait MetadataProvider: Send + Sync {
    // Schema operations
    fn get_schema_snapshot(&self, graph: &GraphRef, schema: Option<&SchemaRef>)
        -> Result<Arc<dyn SchemaSnapshot>, CatalogError>;
    fn resolve_active_graph(&self, session: &SessionContext) -> Result<GraphRef, CatalogError>;
    fn resolve_active_schema(&self, graph: &GraphRef) -> Result<SchemaRef, CatalogError>;
    fn validate_graph_exists(&self, name: &str) -> Result<(), CatalogError>;

    // Callable operations
    fn lookup_callable(&self, name: &str) -> Option<CallableSignature>;
    fn validate_callable_invocation(&self, sig: &CallableSignature, args: &[&Expression])
        -> Result<(), String>;

    // Type inference metadata
    fn get_property_metadata(&self, owner: &TypeRef, property: &str) -> Option<ValueType>;
    fn get_callable_return_type_metadata(&self, name: &str) -> Option<ValueType>;
    fn get_variable_type_metadata(&self, graph: &GraphRef, program: &Program)
        -> Result<VariableTypeContext, CatalogError>;
}
```

### 2. Test Implementation ✅

Created `InMemoryMetadataProvider` with:
- Schema snapshot management
- Callable signature storage
- Property type metadata
- Full test coverage

### 3. Validator Integration ✅

Updated `SemanticValidator`:
```rust
pub struct SemanticValidator<'m> {
    config: ValidationConfig,
    metadata_provider: Option<&'m dyn MetadataProvider>,
}

impl<'m> SemanticValidator<'m> {
    pub fn with_metadata_provider<M: MetadataProvider>(mut self, provider: &'m M) -> Self {
        self.metadata_provider = Some(provider);
        self.config.metadata_validation = true;
        self
    }
}
```

### 4. Backward Compatibility ✅

Implemented `MetadataProvider` for:
- `BuiltinCallableCatalog` - for callable validation tests
- `InMemoryCallableCatalog` - for custom callable tests

### 5. Compilation ✅

Main codebase compiles successfully: `cargo build` passes

---

## What Remains: Test Cleanup

### Current Problem

Tests still use deprecated APIs:
```rust
// ❌ OLD - scattered APIs
validator
    .with_schema(&schema)
    .with_catalog(&catalog)
    .with_schema_catalog(&schema_catalog)
    .with_callable_catalog(&callable_catalog)
    .with_type_metadata(&type_metadata)
```

### Target State

Tests should use only:
```rust
// ✅ NEW - unified API
let metadata = InMemoryMetadataProvider::example();
let validator = SemanticValidator::new()
    .with_metadata_provider(&metadata);
```

### Files Requiring Cleanup

The following test files need updates:

1. **tests/semantic/validator.rs**
   - Remove: `use gql_parser::semantic::schema`
   - Remove: `use gql_parser::semantic::catalog`
   - Replace: `with_schema()` → `with_metadata_provider()`
   - Replace: `with_catalog()` → `with_metadata_provider()`

2. **tests/semantic/schema_integration.rs**
   - Replace: `with_schema_catalog()` → `with_metadata_provider()`
   - Use: `InMemoryMetadataProvider` instead of `InMemorySchemaCatalog`

3. **tests/semantic/mutation_validation.rs**
   - Update: `ValidationConfig { metadata_validation: true, ... }`
   - Replace: old method calls with `with_metadata_provider()`

4. **tests/semantic/callable_validation.rs**
   - Tests already passing callables through `BuiltinCallableCatalog`
   - Just ensure using `with_metadata_provider()` (already done)
   - Remove unused `DefaultCallableValidator` instances

5. **tests/semantic/type_inference.rs**
   - Replace: `with_type_metadata()` → `with_metadata_provider()`
   - Use: `InMemoryMetadataProvider` for property type tests

6. **tests/semantic/procedure_validation.rs**
   - Already uses `with_metadata_provider(&catalog)` ✅
   - Just verify tests pass

7. **tests/integration/type_inference.rs**
   - Replace: `with_type_metadata()` → `with_metadata_provider()`

### Cleanup Steps

```bash
# 1. Remove deprecated field references
find tests -name "*.rs" -exec sed -i '' '/schema_validation:/d' {} +
find tests -name "*.rs" -exec sed -i '' '/catalog_validation:/d' {} +
find tests -name "*.rs" -exec sed -i '' '/callable_validation:/d' {} +
find tests -name "*.rs" -exec sed -i '' '/enhanced_type_inference:/d' {} +
find tests -name "*.rs" -exec sed -i '' '/advanced_schema_validation:/d' {} +

# 2. Replace method calls (already done)
# with_schema → with_metadata_provider
# with_catalog → with_metadata_provider
# with_schema_catalog → with_metadata_provider
# with_callable_catalog → with_metadata_provider
# with_type_metadata → with_metadata_provider

# 3. Fix remaining imports and struct usage
```

### Validation Config Cleanup

OLD fields (remove):
- `schema_validation`
- `catalog_validation`
- `callable_validation`
- `advanced_schema_validation`
- `enhanced_type_inference`

NEW field (use):
- `metadata_validation: bool`

Example:
```rust
let config = ValidationConfig {
    strict_mode: false,
    warn_on_shadowing: true,
    warn_on_disconnected_patterns: true,
    metadata_validation: true,  // ✅ Single flag
};
```

---

## Implementation Guidelines

### For Test Writers

**DO:**
```rust
// Create unified metadata provider
let mut metadata = InMemoryMetadataProvider::new();

// Add schema
let snapshot = InMemorySchemaSnapshot::example();
metadata.add_schema_snapshot("default", snapshot);

// Add callables
metadata.add_callable("count", CallableSignature::aggregate("count"));

// Add property types
metadata.add_property_type_metadata(
    TypeRef::NodeType("Person".into()),
    "name",
    ValueType::String
);

// Use it
let validator = SemanticValidator::new()
    .with_metadata_provider(&metadata);
```

**DON'T:**
```rust
// ❌ Don't use separate catalogs
let schema = InMemorySchemaCatalog::new();
let catalog = MockCatalog::new();
validator.with_schema(&schema).with_catalog(&catalog);

// ❌ Don't reference deprecated traits
use gql_parser::semantic::schema::Schema;
use gql_parser::semantic::catalog::Catalog;

// ❌ Don't use deprecated ValidationConfig fields
let config = ValidationConfig {
    schema_validation: true,  // ❌ REMOVED
    ...
};
```

### For Database Implementors

Implement just one trait:

```rust
struct MyDatabaseCatalog {
    connection: DatabaseConnection,
}

impl MetadataProvider for MyDatabaseCatalog {
    fn get_schema_snapshot(&self, graph: &GraphRef, schema: Option<&SchemaRef>)
        -> Result<Arc<dyn SchemaSnapshot>, CatalogError>
    {
        // Query your database catalog tables
        self.connection.query_schema(graph, schema)
    }

    fn lookup_callable(&self, name: &str) -> Option<CallableSignature> {
        // Query your stored procedures/functions
        self.connection.query_callable(name)
    }

    fn get_property_metadata(&self, owner: &TypeRef, property: &str) -> Option<ValueType> {
        // Query your schema metadata
        self.connection.query_property_type(owner, property)
    }

    // Other methods have sensible defaults or can be implemented similarly
}

// Usage is clean:
let db = MyDatabaseCatalog::new(connection);
let validator = SemanticValidator::new()
    .with_metadata_provider(&db);
```

---

## Testing the Cleanup

After cleanup, verify:

```bash
# All tests should pass
cargo test

# No warnings about deprecated APIs
cargo build 2>&1 | grep -i deprecat

# No references to old traits
rg "with_schema\(|with_catalog\(|with_schema_catalog\(|with_callable_catalog\(|with_type_metadata\(" tests/

# ValidationConfig only has new fields
rg "schema_validation:|catalog_validation:|callable_validation:|enhanced_type_inference:|advanced_schema_validation:" tests/
```

Expected result: **All commands should produce no output** (except `cargo test` passing).

---

## Benefits of This Design

1. **Simplicity**: One trait, one injection point
2. **Consistency**: Single metadata snapshot across all validation
3. **Thread-safe**: `Send + Sync` bounds
4. **Testability**: `InMemoryMetadataProvider` for unit tests
5. **Flexibility**: Default implementations for optional features
6. **Performance**: Arc-based sharing of immutable snapshots
7. **Clean API**: No backward compatibility hacks in production code

---

## Architecture Diagram

```
┌─────────────────────────────────────────┐
│         SemanticValidator               │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │   metadata_provider: Option<&M>   │ │
│  │   where M: MetadataProvider       │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
                    │
                    │ with_metadata_provider()
                    ▼
        ┌──────────────────────────┐
        │   MetadataProvider       │
        │   (trait)                │
        ├──────────────────────────┤
        │ • get_schema_snapshot()  │
        │ • resolve_active_graph() │
        │ • lookup_callable()      │
        │ • get_property_metadata()│
        │ • ... (8 methods total)  │
        └──────────────────────────┘
                    △
                    │ implements
        ┌───────────┴───────────┐
        │                       │
┌───────────────────┐   ┌──────────────────┐
│ InMemory          │   │ Database         │
│ MetadataProvider  │   │ Implementation   │
│ (tests)           │   │ (production)     │
└───────────────────┘   └──────────────────┘
```

---

## Next Steps

1. **Fix remaining test compilation errors** (~2 hours)
   - Update imports to remove deprecated modules
   - Replace method calls with `with_metadata_provider()`
   - Update `ValidationConfig` initialization

2. **Verify test coverage** (~30 min)
   - Run `cargo test`
   - Ensure all scenarios still covered
   - No regressions

3. **Clean up documentation** (~30 min)
   - Update examples in doc comments
   - Add migration guide in docs
   - Update README if needed

4. **Delete deprecated code** (future, v2.0)
   - Mark old traits as deprecated
   - Remove in next major version
   - For now, keep for backward compatibility

**Total remaining effort: ~3 hours**

---

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ `cargo test` passes (all tests)
- ✅ No deprecated API usage in tests
- ✅ Only `MetadataProvider` in test code
- ✅ Clean, idiomatic Rust throughout
- ✅ Good documentation and examples
