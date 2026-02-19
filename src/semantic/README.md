# Semantic Validation Module

This directory contains the semantic validation layer for the GQL parser.

## Overview

The semantic validator performs post-parse validation to check semantic correctness beyond syntax:
- Variable scoping and binding
- Type inference and compatibility
- Pattern connectivity
- Context-appropriate clause usage
- Aggregation and grouping rules
- Optional schema/catalog validation

## Architecture

```
Program (AST)
    ↓
SemanticValidator (9 passes)
    ├── Pass 1: Scope Analysis
    ├── Pass 2: Type Inference
    ├── Pass 3: Variable Validation
    ├── Pass 4: Pattern Validation
    ├── Pass 5: Context Validation
    ├── Pass 6: Type Checking
    ├── Pass 7: Expression Validation
    ├── Pass 8: Reference Validation (optional)
    └── Pass 9: Schema Validation (optional)
    ↓
IR (Intermediate Representation) + Diagnostics
```

## Module Structure

```
src/semantic/
├── mod.rs          - Main module with documentation
├── diag.rs         - Semantic diagnostic types ✅ COMPLETE
└── validator.rs    - Main validator ✅ COMPLETE (see limitations)

src/ir/
├── mod.rs          - IR structure ✅ COMPLETE
├── symbol_table.rs - Symbol table ✅ COMPLETE
└── type_table.rs   - Type table ✅ COMPLETE
```

## Status

**Overall**: ✅ Core validation complete - 88 tests passing, ISO GQL compliant

- ✅ **Complete (Sprint 14)**:
  - Diagnostic system with warning visibility (F1)
  - Reference-site-aware variable lookups (F2)
  - Scope analysis, variable validation, pattern validation
  - Type inference and type checking
  - Context validation, expression validation
  - **Enhanced aggregation validation (F5)**:
    - RETURN statement aggregation rules
    - Nested aggregation detection
    - WHERE clause aggregation prohibition
    - HAVING clause validation with GROUP BY
    - Enhanced expression equivalence checking
  - Schema and catalog validation (optional)
  - Symbol table with reference-site-aware lookups
  - Type table with persistence infrastructure

- 🔶 **Limitations (Documented)**:
  - **F2: Statement isolation** - Parser limitation (semicolon-separated queries treated as one statement)
  - **F2: Composite query scope isolation** - Requires scope popping/cloning (future work)
  - **F4: Type persistence consumption** - Infrastructure exists, full consumption pending (major refactoring)
  - **F3: Advanced expression validation** - CASE type consistency, null propagation (optional enhancements)

- ⏸️ **Future Work**:
  - Full type-based error messages (F4)
  - CASE type consistency validation (F3)
  - Null propagation warnings (F3)
  - Complete statement isolation (requires parser changes)

**Test Suite**: 88 tests passing, 2 ignored (known parser limitations)

See [SPRINT14_REMAINING.md](../../SPRINT14_REMAINING.md) for detailed implementation plan and [SPRINT14.md](../../SPRINT14.md) for sprint status.

## Known Limitations

### F2: Scope Resolution
1. **Statement Isolation**: The parser doesn't create separate Statement objects for semicolon-separated queries (e.g., `"MATCH (n); RETURN n"` is one Statement). Variables leak across semicolons. Requires parser changes to fix.

2. **Composite Query Isolation**: UNION/EXCEPT sides share accumulated scopes. Variables from left side visible on right side. Fix requires scope popping/cloning (future work).

### F4: Type Persistence
- Types are inferred and stored in TypeTable but not fully consumed by all downstream passes
- Error messages use ad-hoc type detection instead of persisted types
- Infrastructure complete, full consumption requires major refactoring

### F3: Expression Validation
- CASE type consistency checking not implemented (optional enhancement)
- Null propagation warnings not implemented (optional enhancement)
- Subquery result type validation not implemented (optional enhancement)

See [SPRINT14_REMAINING.md](../../SPRINT14_REMAINING.md) for detailed implementation plans.

## Quick Start

### Using the Validator

```rust
use gql_parser::{parse, SemanticValidator};

let source = "MATCH (n:Person) RETURN n.name";
let parse_result = parse(source);

if let Some(program) = parse_result.ast {
    let validator = SemanticValidator::new();

    let outcome = validator.validate(&program);

    if let Some(ir) = outcome.ir {
        // Validation successful (no errors, though warnings may exist)
        // Access symbol table: ir.symbol_table()
        // Access type table: ir.type_table()

        // Check for warnings
        for diag in &outcome.diagnostics {
            eprintln!("Warning: {:?}", diag);
        }
    } else {
        // Semantic errors found
        for diag in &outcome.diagnostics {
            eprintln!("Error: {:?}", diag);
        }
    }
}
```

**Note**: The validator returns a `ValidationOutcome` with `ir: Option<IR>` (present when no errors) and `diagnostics: Vec<Diag>` (warnings and/or errors).

### Configuration

```rust
use gql_parser::SemanticValidator;

let validator = SemanticValidator::new()
    .with_strict_mode(true)
    .with_schema_validation(true)
    .with_catalog_validation(true);
```

## Implementation Guide

### Next Steps (Priority Order)

1. **Implement Scope Analysis (Task 2)**
   - File: `validator/scope_analysis.rs`, function: `run_scope_analysis()`
   - Walk Program AST and extract variable declarations
   - Use `SymbolTable::define()` to track variables
   - See implementation notes in [SPRINT14.md](../../SPRINT14.md)

2. **Implement Variable Validation (Task 3)**
   - File: `validator/variable_validation.rs`, function: `run_variable_validation()`
   - Check all variable references against symbol table
   - Generate `SemanticDiagBuilder::undefined_variable()` diagnostics

3. **Implement Type Inference (Task 5)**
   - File: `validator/type_inference.rs`, function: `run_type_inference()`
   - Assign types to expressions
   - Use `TypeTable::set_type()` to track types

4. **Implement Type Checking (Task 6)**
   - File: `validator/type_checking.rs`, function: `run_type_checking()`
   - Validate type compatibility
   - Use `Type::is_compatible_with()` for checking

### Key APIs

#### SymbolTable (src/ir/symbol_table.rs)

```rust
// Creating and managing scopes
let mut table = SymbolTable::new();
let scope_id = table.push_scope(ScopeKind::Query);
table.pop_scope();

// Defining variables
table.define("n".to_string(), SymbolKind::BindingVariable, span);

// Looking up variables
if let Some(symbol) = table.lookup("n") {
    // Variable found, symbol contains: name, kind, declared_at, scope
}
```

#### TypeTable (src/ir/type_table.rs)

```rust
// Creating type table and allocating IDs
let mut table = TypeTable::new();
let expr_id = table.alloc_expr_id();

// Setting and getting types
table.set_type(expr_id, Type::Int);
if let Some(ty) = table.get_type(expr_id) {
    // Use type
}

// Checking type compatibility
if Type::Int.is_compatible_with(&Type::Float) {
    // Types are compatible
}

// Adding constraints
table.add_constraint(expr_id, TypeConstraint::Numeric);
if table.satisfies_constraints(expr_id) {
    // Constraints satisfied
}
```

#### SemanticDiagBuilder (src/semantic/diag.rs)

```rust
// Creating diagnostics
let diag = SemanticDiagBuilder::undefined_variable("x", span)
    .with_note("Did you mean 'y'?")
    .build();

let diag = SemanticDiagBuilder::type_mismatch("Int", "String", span)
    .with_note("Cannot add Int and String")
    .build();

// Adding to diagnostics list
diagnostics.push(diag);
```

### Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_validation_pass() {
        let source = "MATCH (n:Person) RETURN n";
        let program = parse(source).ast.unwrap();
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        assert!(outcome.ir.is_some());
    }

    #[test]
    fn test_undefined_variable_error() {
        let source = "MATCH (n:Person) RETURN m";
        let program = parse(source).ast.unwrap();
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        assert!(outcome.ir.is_none());
        assert!(!outcome.diagnostics.is_empty());
        // Check diagnostic message/kind
    }

    #[test]
    fn test_warning_with_successful_ir() {
        let source = "MATCH (n:Person), (m:Company) RETURN n, m"; // Disconnected
        let program = parse(source).ast.unwrap();
        let validator = SemanticValidator::new();
        let outcome = validator.validate(&program);

        assert!(outcome.ir.is_some()); // IR still produced
        assert!(!outcome.diagnostics.is_empty()); // But warnings exist
    }
}
```

## Design Principles

1. **Never Panic**: All semantic errors return diagnostics, never panic
2. **Continue After Errors**: Report multiple issues, don't stop at first error
3. **Best-Effort Validation**: Continue even with incomplete information
4. **Actionable Diagnostics**: Provide clear messages and suggestions
5. **Graceful Degradation**: Support validation without schema/catalog

## Error Categories

The semantic validator checks for these error categories:

- **UndefinedVariable**: Variable used without definition
- **TypeMismatch**: Incompatible types in operations
- **DisconnectedPattern**: Unconnected graph patterns
- **ContextViolation**: Clause used in wrong context
- **AggregationError**: Invalid aggregation usage
- **UnknownReference**: Schema/graph/procedure not found
- **ScopeViolation**: Variable not visible in scope
- **VariableShadowing**: Variable shadows previous declaration
- **InvalidPropertyAccess**: Invalid property access
- **InvalidNullHandling**: Incorrect null value handling
- **CaseTypeInconsistency**: CASE expression type mismatch
- **SubqueryTypeError**: Subquery result type error
- **ListOperationError**: Invalid list operation
- **PatternValidationError**: Pattern structure error
- **GroupingAggregationMixing**: Mixed aggregated/non-aggregated expressions
- **InvalidFunctionCall**: Invalid function usage

## Contributing

When implementing a validation pass:

1. Study the AST structure in `src/ast/`
2. Implement the validation logic
3. Add comprehensive unit tests
4. Update this README with examples
5. Document any new diagnostic types

## Resources

- **Sprint Status Document**: [SPRINT14.md](../../SPRINT14.md) - Current status and roadmap
- **AST Documentation**: [src/ast/](../ast/) - AST node structure
- **GQL Features**: [GQL_FEATURES.md](../../GQL_FEATURES.md) - Language features
- **Semantic Validation Architecture**: [docs/SEMANTIC_VALIDATION.md](../../docs/SEMANTIC_VALIDATION.md) - Detailed architecture
- **Error Catalog**: [docs/SEMANTIC_ERROR_CATALOG.md](../../docs/SEMANTIC_ERROR_CATALOG.md) - Error types and examples

## Questions?

For questions or clarifications:
1. Check [SPRINT14.md](../../SPRINT14.md) for current implementation status
2. Review existing implementations (symbol_table.rs, type_table.rs)
3. Look at AST structure in src/ast/
4. Check existing parser tests for AST examples
5. See [docs/SEMANTIC_VALIDATION.md](../../docs/SEMANTIC_VALIDATION.md) for architecture details
