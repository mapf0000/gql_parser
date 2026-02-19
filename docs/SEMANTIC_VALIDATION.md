# Semantic Validation Architecture

This document describes the semantic validation layer of the GQL parser, which provides validation beyond syntax checking.

## Overview

The semantic validator validates parsed GQL queries for semantic correctness, including:
- Variable scoping and binding
- Type inference and compatibility
- Pattern connectivity
- Context appropriateness
- Aggregation rules
- Optional schema and catalog validation

## Architecture

### Multi-Pass Design

The semantic validator uses a 9-pass architecture that processes the AST in multiple stages:

```
┌─────────────────────────────────────────────────────────────┐
│                        Parse Result (AST)                     │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
            ┌───────────────────────────────────────┐
            │   Pass 1: Scope Analysis              │
            │   - Build symbol table                 │
            │   - Track variable declarations        │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 2: Type Inference              │
            │   - Infer expression types             │
            │   - Build type table                   │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 3: Variable Validation         │
            │   - Check undefined variables          │
            │   - Detect variable shadowing          │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 4: Pattern Validation          │
            │   - Validate pattern connectivity      │
            │   - Check for disconnected patterns    │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 5: Context Validation          │
            │   - Validate clause usage contexts     │
            │   - Check aggregation contexts         │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 6: Type Checking               │
            │   - Check type compatibility           │
            │   - Validate operations                │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 7: Expression Validation       │
            │   - Validate CASE expressions          │
            │   - Check null propagation             │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 8: Reference Validation        │
            │   (Optional - Catalog-dependent)       │
            │   - Validate graph references          │
            │   - Validate procedure references      │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Pass 9: Schema Validation           │
            │   (Optional - Schema-dependent)        │
            │   - Validate label names               │
            │   - Validate property names            │
            └────────────────┬──────────────────────┘
                             │
                             ▼
            ┌───────────────────────────────────────┐
            │   Result: IR or Diagnostics           │
            │   - Success: IR with semantic info     │
            │   - Failure: List of diagnostics       │
            └───────────────────────────────────────┘
```

### Pass Details

#### Pass 1: Scope Analysis

**Purpose**: Build the symbol table by tracking all variable declarations in the query.

**What it does**:
- Tracks node/edge variables from MATCH patterns
- Tracks variables from LET bindings
- Tracks FOR loop variables
- Maintains scoping hierarchy

**Example**:
```gql
MATCH (n:Person)
LET age = n.age
FOR item IN n.items
RETURN n, age, item
```
Variables: `n`, `age`, `item`

#### Pass 2: Type Inference

**Purpose**: Infer types for expressions throughout the query.

**What it does**:
- Infers types for literals (42 → INTEGER, "hello" → STRING)
- Infers types for arithmetic operations
- Infers types for function calls and aggregations
- Builds type table for later type checking

**Example**:
```gql
LET x = 42          // x: INTEGER
LET y = "hello"     // y: STRING
LET z = x + 10      // z: INTEGER
```

#### Pass 3: Variable Validation

**Purpose**: Ensure all variable references are defined.

**What it does**:
- Checks RETURN/SELECT item references
- Validates property access base variables
- Detects undefined variables
- Optionally warns on variable shadowing

**Example**:
```gql
MATCH (n:Person)
RETURN m  // ERROR: Variable 'm' is undefined
```

#### Pass 4: Pattern Validation

**Purpose**: Validate pattern connectivity in MATCH clauses.

**What it does**:
- Builds connectivity graph for pattern elements
- Performs DFS to check all elements are reachable
- Reports disconnected patterns

**Example**:
```gql
// ISO-CONFORMANT (warns): Disconnected pattern (Cartesian product)
MATCH (a:Person), (b:Company)
RETURN a, b

// CONNECTED: Pattern with relationship
MATCH (a:Person)-[:WORKS_AT]->(b:Company)
RETURN a, b
```

**Note**: Disconnected patterns are ISO-conformant (Cartesian product semantics) but generate warnings by default. This can be configured via `warn_on_disconnected_patterns`.

#### Pass 5: Context Validation

**Purpose**: Ensure clauses are used in appropriate contexts.

**What it does**:
- Validates MATCH in query context
- Validates INSERT/DELETE in mutation context
- Checks aggregation function usage
- Validates ORDER BY placement

**Example**:
```gql
// VALID: Aggregation in SELECT
MATCH (n:Person)
SELECT COUNT(*), AVG(n.age)
```

#### Pass 6: Type Checking

**Purpose**: Validate type compatibility in operations.

**What it does**:
- Checks arithmetic requires numeric types
- Checks logical operations require boolean types
- Validates function argument types
- Detects obvious type mismatches

**Example**:
```gql
// ERROR: String in arithmetic
LET x = "hello" + 10  // Type mismatch

// VALID: Numeric arithmetic
LET y = 42 + 10  // OK
```

#### Pass 7: Expression Validation

**Purpose**: Validate complex expression semantics.

**What it does**:
- Validates CASE expression consistency
- Checks null propagation rules
- Validates subquery structures
- Checks list/record operations

**Example**:
```gql
// Validates CASE branches
SELECT CASE
  WHEN n.age < 18 THEN 'minor'
  WHEN n.age < 65 THEN 'adult'
  ELSE 'senior'
END
```

#### Pass 8: Reference Validation (Optional)

**Purpose**: Validate references against a catalog.

**Requires**: A `Catalog` trait implementation

**What it does**:
- Validates graph references (USE GRAPH)
- Validates schema references
- Validates procedure calls
- Validates type references

**Example**:
```gql
// Validates 'social_graph' exists in catalog
USE GRAPH social_graph
MATCH (n:Person) RETURN n
```

#### Pass 9: Schema Validation (Optional)

**Purpose**: Validate labels and properties against a schema.

**Requires**: A `Schema` trait implementation

**What it does**:
- Validates node labels exist in schema
- Validates edge labels exist in schema
- Validates property names
- Checks property types match schema

**Example**:
```gql
// Validates 'Person' label and 'name' property exist
MATCH (n:Person)
RETURN n.name
```

## Configuration

The semantic validator can be configured with various options:

```rust
use gql_parser::semantic::{SemanticValidator, ValidationConfig};

let config = ValidationConfig {
    strict_mode: true,                       // Enable stricter validation
    schema_validation: false,                // Disabled by default
    catalog_validation: false,               // Disabled by default
    warn_on_shadowing: true,                 // Warn on variable shadowing
    warn_on_disconnected_patterns: true,     // Warn on disconnected patterns
};

let validator = SemanticValidator::with_config(config);
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `strict_mode` | `false` | Enables more stringent validation rules |
| `schema_validation` | `false` | Enables schema-dependent validation (requires Schema) |
| `catalog_validation` | `false` | Enables catalog-dependent validation (requires Catalog) |
| `warn_on_shadowing` | `true` | Warns when variables shadow outer scope variables |
| `warn_on_disconnected_patterns` | `true` | Warns when MATCH patterns are disconnected |

## Usage

### Basic Usage

```rust
use gql_parser::{parse, semantic::SemanticValidator};

let source = "MATCH (n:Person) RETURN n.name";
let parse_result = parse(source);

if let Some(ast) = parse_result.ast {
    let validator = SemanticValidator::new();

    let outcome = validator.validate(&ast);

    // Check for errors
    if let Some(ir) = outcome.ir {
        println!("Validation successful!");
        // IR contains AST + semantic information

        // Check for warnings
        for diag in &outcome.diagnostics {
            if diag.severity == Severity::Warning {
                eprintln!("Warning: {}", diag.message);
            }
        }
    } else {
        // Errors present
        for diag in &outcome.diagnostics {
            if diag.severity == Severity::Error {
                eprintln!("Error: {}", diag.message);
            }
        }
    }
}
```

**Note**: The validator returns a `ValidationOutcome` struct containing both the optional IR (present only when no errors occurred) and a list of diagnostics (warnings and/or errors). This allows warning-level diagnostics to be visible even on successful validation.

### With Schema Validation

```rust
use gql_parser::{parse, semantic::{SemanticValidator, schema::MockSchema}};

let source = "MATCH (n:Person) RETURN n";
let parse_result = parse(source);

if let Some(ast) = parse_result.ast {
    // Create a schema
    let schema = MockSchema::example();

    // Create validator with schema
    let validator = SemanticValidator::new()
        .with_schema(&schema);

    let outcome = validator.validate(&ast);

    if let Some(ir) = outcome.ir {
        println!("Valid with schema!");
    } else {
        for diag in &outcome.diagnostics {
            if diag.severity == Severity::Error {
                eprintln!("Schema error: {}", diag.message);
            }
        }
    }
}
```

### With Catalog Validation

```rust
use gql_parser::{parse, semantic::{SemanticValidator, catalog::MockCatalog}};

let source = "USE GRAPH social MATCH (n:Person) RETURN n";
let parse_result = parse(source);

if let Some(ast) = parse_result.ast {
    // Create a catalog
    let catalog = MockCatalog::example();

    // Create validator with catalog
    let validator = SemanticValidator::new()
        .with_catalog(&catalog);

    let outcome = validator.validate(&ast);

    if let Some(ir) = outcome.ir {
        println!("Valid with catalog!");
    } else {
        for diag in &outcome.diagnostics {
            if diag.severity == Severity::Error {
                eprintln!("Catalog error: {}", diag.message);
            }
        }
    }
}
```

## Intermediate Representation (IR)

Upon successful validation (no errors, though warnings may be present), the validator produces an IR that wraps the AST with semantic information:

```rust
pub struct IR {
    /// Original AST
    pub program: Program,

    /// Symbol table with variable declarations
    pub symbol_table: SymbolTable,

    /// Type table with inferred types
    pub type_table: TypeTable,
}
```

The IR is returned via `ValidationOutcome`:
```rust
pub struct ValidationOutcome {
    /// IR is Some when no errors occurred (warnings are OK)
    pub ir: Option<IR>,

    /// All diagnostics (warnings and/or errors)
    pub diagnostics: Vec<Diag>,
}
```

### Symbol Table

The symbol table tracks all variable declarations:

```rust
// Query all variables
for (name, symbol_info) in ir.symbol_table.symbols() {
    println!("Variable: {}, Kind: {:?}", name, symbol_info.kind);
}
```

### Type Table

The type table stores inferred types for expressions:

```rust
// Check if type information is available
// (Currently a placeholder for future type inference integration)
```

## Error Handling

The validator produces structured diagnostics:

```rust
pub struct Diag {
    /// Error or warning message
    pub message: String,

    /// Source span where error occurred
    pub span: Range<usize>,

    /// Severity (error, warning, etc.)
    // ... additional fields
}
```

Diagnostics include:
- Clear error messages
- Source location (line/column)
- Suggestions for fixes (when available)

## Extending Validation

### Implementing Custom Schema

To provide schema validation, implement the `Schema` trait:

```rust
use gql_parser::semantic::schema::{Schema, LabelDefinition, PropertyDefinition, SchemaResult};

struct MySchema {
    // Your schema data
}

impl Schema for MySchema {
    fn get_node_label(&self, name: &str) -> Option<&LabelDefinition> {
        // Look up node label
    }

    fn get_edge_label(&self, name: &str) -> Option<&LabelDefinition> {
        // Look up edge label
    }

    fn get_property(&self, label: Option<&str>, property: &str)
        -> Option<&PropertyDefinition>
    {
        // Look up property
    }
}
```

### Implementing Custom Catalog

To provide catalog validation, implement the `Catalog` trait:

```rust
use gql_parser::semantic::catalog::{Catalog, GraphDefinition, SchemaDefinition, ProcedureDefinition};

struct MyCatalog {
    // Your catalog data
}

impl Catalog for MyCatalog {
    fn get_graph(&self, name: &str) -> Option<&GraphDefinition> {
        // Look up graph
    }

    fn get_schema(&self, name: &str) -> Option<&SchemaDefinition> {
        // Look up schema
    }

    fn get_procedure(&self, name: &str) -> Option<&ProcedureDefinition> {
        // Look up procedure
    }
}
```

## Performance Considerations

The semantic validator is designed to be efficient:

- **Single-pass AST traversal**: Each validation pass traverses the AST once
- **Early termination**: Validation continues after errors to report multiple issues
- **Lazy evaluation**: Optional passes (schema, catalog) only run when configured
- **No copying**: Works on references to the AST

**Typical performance** (on modern hardware):
- Small queries (<100 nodes): <5ms
- Medium queries (100-1000 nodes): <50ms
- Large queries (>1000 nodes): <500ms

## Testing

The semantic validator includes comprehensive tests:

- **Unit tests**: 50+ tests for each validation pass
- **Integration tests**: End-to-end validation scenarios
- **Edge case tests**: Unusual but valid constructs
- **Error tests**: Invalid queries that should fail

Run tests with:
```bash
cargo test --lib semantic
```

## Future Enhancements

Planned improvements:
1. **Enhanced type inference**: More sophisticated type propagation
2. **Property validation**: Validate property access against schema
3. **Cardinality checking**: Validate relationship cardinalities
4. **Performance optimization**: Further optimize hot paths
5. **Better error messages**: More context and suggestions

## Known Limitations

The semantic validator is under active development. Current known limitations include:

### Scope Resolution (P0-2)
- **Issue**: Cross-statement variable leakage - variables from one statement may incorrectly be visible in subsequent statements
- **Status**: Requires architectural refactor to thread scope context through validation passes
- **Impact**: Rare false negatives for undefined variables across statement boundaries
- **Planned**: Sprint 15

### Incomplete Validation Paths (P0-3)
- **Issue**: Some semantic validation paths remain stubbed or partial, including:
  - Mutation support (INSERT/DELETE/SET/REMOVE validation)
  - CALL argument and yield validation
  - Nested EXISTS/subquery reference checks
  - CASE expression type consistency enforcement
  - Comprehensive reference validation (catalog-backed)
  - Optional MATCH schema traversal
- **Impact**: Some invalid constructs may not be detected
- **Planned**: Sprint 14/15

### Type Inference (P1-1)
- **Issue**: Inferred types are computed locally but not persisted to TypeTable
- **Impact**: Type information not available for downstream consumers of IR
- **Planned**: Sprint 15

### Aggregation/Grouping (P1-2)
- **Issue**: Strict-mode aggregation and GROUP BY legality checks are not fully enforced
- **Impact**: Mixed aggregate/non-aggregate expressions may not be properly validated
- **Planned**: Sprint 15

### General
- The validator continues execution after errors to report multiple issues, which may result in cascading false positives
- Performance has not been optimized for very large queries (>10,000 nodes)
- Some error messages could be more actionable with better suggestions

## References

- [GQL Standard](https://www.gqlstandards.org/)
- [Architecture Documentation](ARCHITECTURE.md)
- [Semantic Error Catalog](SEMANTIC_ERROR_CATALOG.md)
