# ISO GQL Compliance Gaps

**Overall Compliance: 70-75% grammar coverage, 90-95% practical usability**

Last Updated: 2026-02-19

---

## Not Implemented

### 1. Procedure Definitions (Grammar §9.1-9.2, Lines 138-200)
- [ ] Full `procedureBody` with `bindingVariableDefinitionBlock`
- [ ] `atSchemaClause` for schema context
- [ ] `NEXT` statement for procedure chaining (line 198)
- ✅ Procedure calls (`CALL`, `OPTIONAL CALL`, `YIELD`) work

### 2. Nested Expressions
- [ ] `VALUE <nested_query>` as value expression
- [ ] `PROPERTY GRAPH <expr>` as graph expression
- [ ] `BINDING TABLE <expr>` as binding table expression

### 3. Advanced SELECT Features
- [ ] Window functions
- [ ] `WITH` clause (Common Table Expressions)
- [ ] Complex nested table expressions in FROM

### 4. Schema-Dependent Features (Intentionally Deferred)
- [ ] Type checking against graph schema
- [ ] Property/label existence validation
- [ ] Constraint enforcement
- [ ] Schema-aware semantic validation

### 5. Advanced Graph Type Features (Grammar §33, Lines 2320-2600)
- [ ] Detailed inline graph type bodies
- [ ] `ABSTRACT` types
- [ ] Type inheritance
- [ ] Full constraint specifications

---

## Partially Implemented

### 1. Graph Pattern Expressions
- ⚠️ Complex parenthesized path patterns with deeply nested quantifiers
- ⚠️ Simplified path pattern syntax (AST exists, parser partial)
- ⚠️ Pattern union/alternation in all contexts

### 2. Built-in Functions
- ⚠️ Some specialized string manipulation variants
- ⚠️ Some datetime manipulation functions
- ⚠️ Advanced list/collection operations
- **Coverage: ~90% of standard functions**

### 3. Statement Isolation
- ⚠️ Semicolon-separated statements share scope in some cases
- ⚠️ Parser doesn't create fully isolated Statement objects
- **Documented in tests (2 ignored tests)**

---

## Known Limitations

1. **Error Recovery**: Basic synchronization works; could be more sophisticated
2. **Procedure Bodies**: Focus on invocation; full definition parsing deferred
3. **Nested Quantifiers**: Some deeply nested cases may not parse optimally
4. **Type Inference**: Returns `Type::Any` for complex expressions without schema

---

## Priority for Future Work

### High Priority (95%+ compliance)
1. Full procedure body parsing
2. Schema integration for type checking
3. Advanced nested expressions

### Medium Priority (completeness)
4. Window functions and CTEs
5. Advanced graph type constraints
6. Improved error recovery

### Low Priority (edge cases)
7. Complex pattern composition edge cases
8. Specialized function variants
9. Statement isolation improvements

---

## Reference
- Grammar: `third_party/opengql-grammar/GQL.g4` (3,774 lines)
- ISO Standard: ISO/IEC 39075:2024
