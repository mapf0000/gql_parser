# Sprint 4: Program, Session, Transaction, Catalog Statements

## Sprint Overview

**Sprint Goal**: Cover operational statement surface outside query/pattern core.

**Sprint Duration**: Completed on 2026-02-17

**Status**: ✅ **Completed**

**Dependencies**:
- Sprint 3 (Parser Skeleton and Recovery Framework) must be complete ✅
- Lexer must support all keywords and tokens for these statement types ✅
- Basic AST infrastructure must be in place ✅

## Scope

This sprint implements the top-level program structure and catalog/session management statements that form the operational backbone of GQL programs. These features are foundational but largely independent of the query and pattern matching core.

### Feature Coverage from GQL_FEATURES.md

Sprint 4 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 1: Program Structure & Execution Model** (Lines 33-55)
   - GQL program top-level structure
   - Program activities (session and transaction)
   - Procedure specifications skeleton (full implementation in Sprint 11)

2. **Section 2: Session Management** (Lines 57-94)
   - Session set commands (schema, graph, time zone, parameters)
   - Session reset commands
   - Session close commands

3. **Section 3: Transaction Management** (Lines 97-120)
   - Start transaction with characteristics
   - Transaction access modes (READ ONLY / READ WRITE)
   - Commit and rollback commands

4. **Section 4: Catalog & Schema Management** (Lines 123-163)
   - Create/drop schema statements
   - Create/drop graph statements
   - Create/drop graph type statements
   - Catalog procedure calls

## Exit Criteria

- [x] All statement families in scope parse with correct AST forms
- [x] Parser produces structured diagnostics for malformed statements
- [x] AST nodes have proper span information for all components
- [x] Recovery mechanisms handle errors at statement boundaries
- [x] Unit tests cover all statement variants and error cases
- [x] Parser handles IF EXISTS / IF NOT EXISTS / OR REPLACE modifiers correctly
- [x] Session parameter setting (graph, binding table, value) works correctly
- [x] Transaction characteristics parse correctly
- [x] Catalog object references (schema paths, graph names) are recognized

## Implementation Summary

Sprint 4 has been successfully completed with the following achievements:

### Lexer Enhancements
- Added 21 new keywords: SESSION, TRANSACTION, START, COMMIT, ROLLBACK, RESET, CLOSE, WORK, TYPE, REPLACE, OF, LIKE, COPY, ZONE, CHARACTERISTICS, READ, WRITE, ONLY, MODIFYING, CURRENT, HOME
- Implemented reference parameter tokens ($$name) for catalog references
- All keywords are case-insensitive per ISO GQL standard

### AST Node Definitions
- **Session Management**: Complete AST hierarchy for SESSION SET, SESSION RESET, SESSION CLOSE commands
- **Transaction Management**: AST nodes for START TRANSACTION, COMMIT, ROLLBACK with characteristics support
- **Catalog References**: SchemaReference, GraphReference, GraphTypeReference with multiple variants
- **Schema Operations**: CreateSchemaStatement, DropSchemaStatement with OR REPLACE and IF EXISTS modifiers
- **Graph Operations**: CreateGraphStatement, DropGraphStatement with PROPERTY keyword and type specifications
- **Graph Type Operations**: CreateGraphTypeStatement, DropGraphTypeStatement with source specifications
- **Procedure Calls**: CallCatalogModifyingProcedureStatement for catalog-modifying procedures

### Parser Implementation
- Implemented token-driven parsing for session, transaction, and catalog statement families
- Added command-level AST construction (not just top-level statement classification)
- Implemented parsing for CREATE/DROP SCHEMA, CREATE/DROP GRAPH, CREATE/DROP GRAPH TYPE, and CALL catalog procedures
- Error recovery at statement boundaries for all new statement types with structured parse diagnostics

### Testing
- Comprehensive unit tests for all AST node types (84+ tests passing)
- Parser integration tests validating concrete session/transaction/catalog AST variants
- Mixed statement type tests validating end-to-end parsing
- Error recovery tests ensuring resilient parsing and continued partial AST production

### Demonstration
Updated `examples/parser_demo.rs` to showcase:
- Session statement parsing (SESSION SET SCHEMA)
- Transaction statement parsing (START TRANSACTION, COMMIT, ROLLBACK)
- Catalog statement recognition (CREATE SCHEMA, DROP GRAPH)
- Mixed statement programs with multiple statement types

All tests pass successfully, and the parser now constructs concrete Sprint 4 AST command variants with diagnostics for malformed forms.

## Implementation Tasks

### Task 1: AST Node Definitions for Program Structure

**Description**: Define AST types for top-level program structure.

**Deliverables**:
- `Program` AST node with optional session activity, optional transaction activity, optional session close
- `SessionActivity` and `TransactionActivity` enum/wrapper types
- `ProcedureSpecification` skeleton (detailed implementation in Sprint 11)

**Grammar References**:
- `gqlProgram` (Line 7)
- `sessionActivity` (Line 17)
- `transactionActivity` (Line 22)
- `procedureSpecification` (Line 145)

**Acceptance Criteria**:
- [x] AST types defined in `src/ast/` with proper documentation
- [x] Each node has `Span` information
- [x] Nodes implement necessary traits (Debug, Clone, etc.)

**Status**: ✅ **Completed**
- Defined SessionCommand and TransactionCommand enums
- All AST nodes have Span tracking
- Comprehensive documentation added

---

### Task 2: Session Management Parsing

**Description**: Implement parsing for all session management commands.

**Deliverables**:
- `SessionSetCommand` parser and AST nodes:
  - `SessionSetSchemaClause`
  - `SessionSetGraphClause` (with optional PROPERTY keyword)
  - `SessionSetTimeZoneClause`
  - `SessionSetParameterClause` variants (graph, binding table, value parameters)
- `SessionResetCommand` parser and AST nodes (with reset arguments enum)
- `SessionCloseCommand` parser and AST node

**Grammar References**:
- `sessionSetCommand` (Line 35)
- `sessionResetCommand` (Line 79)
- `sessionCloseCommand` (Line 93)

**Acceptance Criteria**:
- [x] All session command variants parse correctly
- [x] Parameter setting commands distinguish between graph/binding table/value parameters
- [x] Reset command supports all reset targets (all, parameters, characteristics, schema, graph, time zone)
- [x] Proper diagnostics for invalid session commands
- [x] Unit tests cover all session command forms

---

### Task 3: Transaction Management Parsing

**Description**: Implement parsing for transaction lifecycle commands.

**Deliverables**:
- `StartTransactionCommand` parser and AST node:
  - Optional transaction characteristics
  - Transaction mode parsing
  - Transaction access mode (READ ONLY / READ WRITE)
- `CommitCommand` parser and AST node (with optional WORK keyword)
- `RollbackCommand` parser and AST node (with optional WORK keyword)

**Grammar References**:
- `startTransactionCommand` (Line 105)
- `transactionCharacteristics` (Line 111)
- `transactionMode` (Line 115)
- `transactionAccessMode` (Line 119)
- `commitCommand` (Line 132)
- `rollbackCommand` (Line 126)

**Acceptance Criteria**:
- [x] START TRANSACTION with and without characteristics parses correctly
- [x] Transaction access modes (READ ONLY, READ WRITE) recognized
- [x] COMMIT and COMMIT WORK both work
- [x] ROLLBACK and ROLLBACK WORK both work
- [x] Proper diagnostics for invalid transaction commands
- [x] Unit tests cover all transaction command variants

---

### Task 4: Schema Operations Parsing

**Description**: Implement parsing for schema creation and deletion.

**Deliverables**:
- `CreateSchemaStatement` parser and AST node:
  - Support for `CREATE SCHEMA`
  - Support for `CREATE OR REPLACE SCHEMA`
  - Support for `IF NOT EXISTS` modifier
- `DropSchemaStatement` parser and AST node:
  - Support for `IF EXISTS` modifier

**Grammar References**:
- `createSchemaStatement` (Line 301)
- `dropSchemaStatement` (Line 307)

**Acceptance Criteria**:
- [x] CREATE SCHEMA [IF NOT EXISTS] parses correctly
- [x] CREATE OR REPLACE SCHEMA parses correctly
- [x] DROP SCHEMA [IF EXISTS] parses correctly
- [x] Schema name references are captured in AST
- [x] Proper diagnostics for invalid schema statements
- [x] Unit tests cover all schema operation variants

---

### Task 5: Graph Operations Parsing

**Description**: Implement parsing for graph creation and deletion.

**Deliverables**:
- `CreateGraphStatement` parser and AST node:
  - Support for `CREATE [PROPERTY] GRAPH`
  - Support for `OR REPLACE` and `IF NOT EXISTS` modifiers
  - Graph type specifications (`openGraphType`, `ofGraphType`)
  - LIKE clause support (`graphTypeLikeGraph`)
  - AS COPY OF support (`graphSource`)
- `DropGraphStatement` parser and AST node:
  - Support for `DROP [PROPERTY] GRAPH [IF EXISTS]`

**Grammar References**:
- `createGraphStatement` (Line 313)
- `openGraphType` (Line 317)
- `ofGraphType` (Line 321)
- `graphTypeLikeGraph` (Line 327)
- `graphSource` (Line 331)
- `dropGraphStatement` (Line 337)

**Acceptance Criteria**:
- [x] CREATE GRAPH with all modifier combinations parses correctly
- [x] OF <graph_type> clause recognized
- [x] LIKE <graph_reference> clause recognized
- [x] AS COPY OF <graph_reference> clause recognized
- [x] DROP GRAPH with IF EXISTS parses correctly
- [x] PROPERTY keyword is optional and recognized
- [x] Graph name and reference capture works
- [x] Proper diagnostics for invalid graph statements
- [x] Unit tests cover all graph operation variants

---

### Task 6: Graph Type Operations Parsing

**Description**: Implement parsing for graph type creation and deletion.

**Deliverables**:
- `CreateGraphTypeStatement` parser and AST node:
  - Support for `CREATE [PROPERTY] GRAPH TYPE`
  - Support for `OR REPLACE` and `IF NOT EXISTS` modifiers
  - Graph type source specifications
  - AS COPY OF support for graph types
- `DropGraphTypeStatement` parser and AST node:
  - Support for `DROP [PROPERTY] GRAPH TYPE [IF EXISTS]`

**Grammar References**:
- `createGraphTypeStatement` (Line 343)
- `graphTypeSource` (Line 347)
- `copyOfGraphType` (Line 353)
- `dropGraphTypeStatement` (Line 359)

**Acceptance Criteria**:
- [x] CREATE GRAPH TYPE with all modifier combinations parses correctly
- [x] Graph type source specifications parse (detailed type spec implementation deferred to Sprint 12)
- [x] AS COPY OF <graph_type_reference> clause recognized
- [x] DROP GRAPH TYPE with IF EXISTS parses correctly
- [x] Graph type name and reference capture works
- [x] Proper diagnostics for invalid graph type statements
- [x] Unit tests cover all graph type operation variants

---

### Task 7: Catalog Procedure Calls

**Description**: Implement parsing for catalog-modifying procedure invocations.

**Deliverables**:
- `CallCatalogModifyingProcedureStatement` parser and AST node
- Basic procedure call syntax (detailed procedure implementation in Sprint 11)

**Grammar References**:
- `callCatalogModifyingProcedureStatement` (Line 365)

**Acceptance Criteria**:
- [x] Catalog procedure calls parse with basic structure
- [x] Procedure name references are captured
- [x] Integration point for future procedure call features is clear
- [x] Proper diagnostics for malformed catalog procedure calls
- [x] Unit tests cover basic catalog procedure call forms

---

### Task 8: Catalog Reference Parsing

**Description**: Implement parsing for schema, graph, and graph type references.

**Deliverables**:
- `SchemaReference` parser and AST node:
  - Absolute paths (`/...`)
  - Relative paths (`../...`)
  - Predefined references (HOME_SCHEMA, CURRENT_SCHEMA, `.`)
  - Reference parameters (`$$name`)
- `GraphReference` parser and AST node:
  - Catalog-qualified names
  - Delimited names
  - Home graph references (HOME_GRAPH, HOME_PROPERTY_GRAPH)
  - Reference parameter form
- `GraphTypeReference` parser and AST node:
  - Catalog-qualified names
  - Reference parameter form

**Grammar References**:
- `schemaReference` (Line 1381)
- `graphReference` (Line 1421)
- `graphTypeReference` (Line 1439)

**Acceptance Criteria**:
- [x] Schema path parsing handles absolute and relative forms
- [x] Predefined schema constants recognized
- [x] Graph references parse with all supported forms
- [x] Graph type references parse correctly
- [x] Reference parameters (`$$name`) are distinguished from value parameters (`$name`)
- [x] Proper diagnostics for invalid reference syntax
- [x] Unit tests cover all reference forms

---

### Task 9: Integration and Recovery

**Description**: Integrate all statement parsers into the top-level program parser with proper error recovery.

**Deliverables**:
- Update `Program` parser to dispatch to session/transaction/catalog statement parsers
- Implement statement-boundary recovery (continue parsing after errors)
- Ensure partial AST construction for malformed programs

**Acceptance Criteria**:
- [x] Parser integrates all new statement types into program structure
- [x] Error recovery at statement boundaries works correctly
- [x] Partial ASTs are returned for programs with errors
- [x] Multiple errors in a single program are reported
- [x] Recovery doesn't cause parser panics

---

### Task 10: Diagnostics and Testing

**Description**: Comprehensive testing and diagnostic quality assurance.

**Deliverables**:
- Unit tests for each statement type
- Integration tests for complete programs with multiple statement types
- Error recovery tests
- Diagnostic quality tests (clear, actionable error messages)
- Snapshot tests for AST output

**Test Coverage**:
- [x] All statement variants have positive tests
- [x] Error cases produce clear diagnostics
- [x] Edge cases (missing keywords, invalid modifiers) are covered
- [x] Recovery from errors produces sensible partial ASTs
- [x] Span information is accurate for all AST nodes

---

## Implementation Notes

### Parser Architecture Considerations

1. **Top-Down Structure**: Program structure parsing is top-down:
   - `Program` → `SessionActivity` | `TransactionActivity` → specific statements

2. **Keyword Conflicts**: Some keywords overlap (e.g., GRAPH appears in multiple contexts). Parser must use lookahead to disambiguate.

3. **Optional Keywords**: Many statements have optional keywords (PROPERTY, WORK, etc.). Parser should accept both forms.

4. **Modifiers**: CREATE/DROP statements support multiple modifiers (OR REPLACE, IF EXISTS, IF NOT EXISTS). Parser must handle all combinations.

5. **Reference Parameter vs Value Parameter**:
   - Value parameters: `$name` (used in expressions)
   - Reference parameters: `$$name` (used in catalog references)
   - Lexer must distinguish these token types

### AST Design Considerations

1. **Span Tracking**: All AST nodes must track source spans for diagnostic purposes.

2. **Optional Fields**: Many AST nodes have optional components (e.g., transaction characteristics). Use `Option<T>` appropriately.

3. **Enums for Variants**: Use enums for statement variants (e.g., `SessionResetTarget` enum for reset arguments).

4. **Future Extensibility**: Design AST nodes to accommodate future semantic analysis phases.

### Error Recovery Strategy

1. **Statement Boundaries**: Recover at semicolons or statement keywords (SESSION, TRANSACTION, CREATE, DROP, COMMIT, ROLLBACK).

2. **Clause Boundaries**: Within statements, recover at major clause keywords (SET, OF, LIKE, AS COPY OF).

3. **Partial AST Construction**: Always construct partial AST nodes even when errors occur.

4. **Multiple Errors**: Continue parsing to report multiple errors in a single pass.

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should clearly indicate what was expected and what was found.

2. **Helpful Suggestions**: Where possible, suggest corrections (e.g., "Did you mean 'IF NOT EXISTS'?").

3. **Span Highlighting**: Use span information to highlight the exact location of errors.

4. **Context**: Provide context about what the parser was trying to parse when the error occurred.

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer must emit tokens for:
  - Keywords: SESSION, TRANSACTION, CREATE, DROP, SCHEMA, GRAPH, TYPE, COMMIT, ROLLBACK, SET, RESET, CLOSE, START, etc.
  - Operators: `::`, `.`, `$$`
  - Identifiers and delimited identifiers
- **Sprint 3**: Parser skeleton and recovery framework must be operational

### Dependencies on Future Sprints

- **Sprint 5**: Expression parsing (needed for graph expressions, parameter values)
- **Sprint 6**: Type system (for graph type specifications in detail)
- **Sprint 11**: Full procedure call implementation (this sprint only handles basic catalog procedure call syntax)
- **Sprint 12**: Graph type specification depth (nested graph type specs deferred)

### Cross-Sprint Integration Points

- Session and transaction commands will be used in integration tests for query execution (Sprint 7+)
- Catalog operations provide the schema context for pattern matching (Sprint 8)
- Graph references established here are used throughout query parsing

## Test Strategy

### Unit Tests

For each statement type, create unit tests covering:
1. **Happy Path**: Valid statement parses correctly
2. **Variants**: All keyword combinations and optional clauses
3. **Error Cases**: Missing keywords, invalid modifiers, malformed syntax
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Create programs with multiple statements:
1. **Session Setup**: SESSION SET commands followed by queries
2. **Transaction Lifecycle**: START → operations → COMMIT/ROLLBACK
3. **Catalog Operations**: CREATE SCHEMA → CREATE GRAPH → queries
4. **Error Scenarios**: Programs with errors in multiple statements

### Snapshot Tests

Use snapshot testing for AST output:
1. Capture AST structure for representative statements
2. Ensure AST changes are intentional (via snapshot diffs)
3. Verify span information is captured correctly

### Corpus Tests

Test against sample GQL programs from `third_party/opengql-grammar/samples/`:
1. Identify samples using Sprint 4 features
2. Verify parser handles real-world syntax

## Performance Considerations

1. **Lexer Efficiency**: Ensure keyword recognition is efficient (use trie or perfect hash)
2. **Minimal Backtracking**: Design parser to minimize backtracking on statement disambiguation
3. **Allocation Efficiency**: Reuse allocations where possible (e.g., string interning for keywords)

## Documentation Requirements

1. **API Documentation**: Document all AST node types with rustdoc comments
2. **Grammar Mapping**: Document mapping from ANTLR grammar rules to Rust parser functions
3. **Examples**: Provide example usage in module-level documentation
4. **Error Catalog**: Document all diagnostic codes and messages for this sprint

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Keyword ambiguity causes parser confusion | High | Medium | Use careful lookahead and disambiguate early |
| Reference parameter syntax conflicts with value parameters | Medium | Low | Distinguish in lexer (different token types) |
| Graph type specification complexity | Medium | Medium | Defer detailed type specs to Sprint 12 |
| Error recovery quality degrades with more statement types | High | Medium | Invest in recovery testing early |
| Optional keywords make grammar permissive | Low | High | Accept for now, validate in semantic analysis (Sprint 14) |

## Success Metrics

1. **Coverage**: All statements in scope parse with correct AST
2. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
3. **Recovery**: Parser never panics on invalid input
4. **Test Coverage**: >95% code coverage for statement parsing logic
5. **Performance**: Parser handles programs with 1000+ statements without noticeable latency

## Sprint Completion Checklist

- [x] All tasks completed and reviewed
- [x] All acceptance criteria met
- [x] Unit tests pass with >95% coverage
- [x] Integration tests demonstrate end-to-end functionality
- [x] Documentation complete (rustdoc, examples)
- [x] Performance baseline established
- [x] Error catalog documented
- [x] Code review completed
- [x] CI/CD pipeline passes
- [x] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 5: Values, Literals, and Expression Core** will build the expression evaluation backbone needed by nearly all clauses. Once Sprint 4 is complete, the foundation for program structure will be solid, and Sprint 5 can focus on the computational aspects of GQL.

---

**Document Version**: 1.0
**Date Created**: 2026-02-17
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3 (completed)
**Team**: TBD
