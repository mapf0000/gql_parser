# Semantic Error Catalog

This document catalogs all semantic errors that the GQL parser can detect.

## Error Categories

### 1. Variable Errors

#### Undefined Variable
**Code**: `UNDEFINED_VARIABLE`
**Severity**: Error

**Description**: A variable is referenced but never declared.

**Example**:
```gql
MATCH (n:Person)
RETURN m  -- Error: 'm' is undefined
```

**Solution**: Declare the variable or fix the typo.

#### Variable Shadowing
**Code**: `VARIABLE_SHADOWING`
**Severity**: Warning

**Description**: A variable shadows an outer scope variable.

**Example**:
```gql
MATCH (n:Person)
LET n = n.name  -- Warning: 'n' shadows outer 'n'
RETURN n
```

**Solution**: Use a different variable name.

### 2. Type Errors

#### Type Mismatch
**Code**: `TYPE_MISMATCH`
**Severity**: Error

**Description**: An operation expects one type but receives another.

**Example**:
```gql
LET x = "hello" + 10  -- Error: Cannot add string and integer
```

**Solution**: Ensure operand types match the operation.

### 3. Pattern Errors

#### Disconnected Pattern
**Code**: `DISCONNECTED_PATTERN`
**Severity**: Warning (ISO-conformant behavior)

**Description**: MATCH pattern contains disconnected components. Per ISO GQL standard, this is valid (represents a Cartesian product) but unusual enough to warrant a warning.

**Example**:
```gql
MATCH (a:Person), (b:Company)  -- Warning: Disconnected (Cartesian product)
RETURN a, b
```

**Solution**: If intentional Cartesian product, no action needed. Otherwise, connect pattern elements with edges.

**Note**: Can be disabled via `ValidationConfig::warn_on_disconnected_patterns = false`.

### 4. Schema Errors (Optional)

#### Unknown Label
**Code**: `UNKNOWN_LABEL`
**Severity**: Error

**Description**: Label not found in schema.

**Example**:
```gql
MATCH (n:UnknownLabel)  -- Error if schema doesn't have this label
RETURN n
```

**Solution**: Use a label that exists in the schema.

#### Unknown Property
**Code**: `UNKNOWN_PROPERTY`
**Severity**: Error

**Description**: Property not found in schema for given label.

**Example**:
```gql
MATCH (n:Person)
RETURN n.unknownProp  -- Error if schema doesn't have this property
```

**Solution**: Use a property that exists in the schema.

### 5. Catalog Errors (Optional)

#### Unknown Graph
**Code**: `UNKNOWN_GRAPH`
**Severity**: Error

**Description**: Referenced graph not found in catalog.

**Example**:
```gql
USE GRAPH unknown_graph  -- Error if not in catalog
MATCH (n) RETURN n
```

**Solution**: Use a graph that exists in the catalog.

## Error Recovery

The parser continues after errors to report multiple issues in a single pass.

## See Also

- [Semantic Validation Architecture](SEMANTIC_VALIDATION.md)
- [API Documentation](../README.md)
