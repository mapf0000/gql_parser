# Sprint 11: Procedures, Nested Specs, and Execution Flow

## Sprint Overview

**Sprint Goal**: Complete procedural composition features.

**Sprint Duration**: TBD

**Status**: 🔵 **Planned**

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅
- Sprint 5 (Values, Literals, and Expression Core) ✅
- Sprint 6 (Type System and Reference Forms) ✅
- Sprint 7 (Query Pipeline Core) ✅
- Sprint 8 (Graph Pattern and Path Pattern System) ✅
- Sprint 9 (Result Shaping and Aggregation) ✅
- Sprint 10 (Data Modification Statements) ✅

## Scope

This sprint implements the complete procedural composition system for GQL, enabling powerful procedural abstractions, modular query organization, and execution flow control. Procedures are reusable query/mutation units that can be invoked inline or by name, with variable scoping, parameter passing, result yielding, and sequential chaining via NEXT. Sprint 11 builds on the query (Sprint 7-9) and mutation (Sprint 10) foundations to deliver the final layer of GQL's procedural execution model.

### Feature Coverage from GQL_FEATURES.md

Sprint 11 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 12: Procedure Calls** (Lines 762-810)
   - Call procedure statements
   - OPTIONAL CALL support
   - Inline procedure calls
   - Named procedure calls
   - Variable scope clause
   - Procedure argument lists
   - Procedure arguments
   - YIELD clauses
   - AT schema clause
   - USE graph clause

2. **Section 1: Program Structure & Execution Model** (Lines 33-55)
   - Nested procedure specifications
   - Nested data modifying procedure specifications
   - Nested query specifications
   - Procedure body structure
   - Statement blocks
   - NEXT statement for sequential chaining

3. **Section 19: Variables, Parameters & References** (Lines 1571-1647)
   - Binding variable definition blocks
   - Graph variable definitions
   - Binding table variable definitions
   - Value variable definitions
   - Variable initializers

## Exit Criteria

- [ ] CALL procedure statements parse correctly
- [ ] OPTIONAL CALL works (continues execution on procedure failure)
- [ ] Inline procedure calls parse with nested procedure specifications
- [ ] Named procedure calls parse with procedure references
- [ ] Procedure argument lists parse correctly
- [ ] Variable scope clause parses with binding variable references
- [ ] YIELD clause parses with yield item lists
- [ ] AT schema clause parses for procedure context
- [ ] USE graph clause parses for focused query/mutation context (already from Sprint 7)
- [ ] Nested procedure specifications parse (braced procedure bodies)
- [ ] Nested data modifying procedure specifications work
- [ ] Nested query specifications work
- [ ] Procedure body parses with AT clause, variable definitions, and statement blocks
- [ ] Binding variable definition blocks parse correctly
- [ ] Graph variable definitions parse with types and initializers
- [ ] Binding table variable definitions parse with types and initializers
- [ ] Value variable definitions parse with types and initializers
- [ ] Statement blocks parse with sequential statements
- [ ] NEXT statement parses for statement chaining
- [ ] NEXT with YIELD clause works
- [ ] Integration with query statements from Sprint 7-9
- [ ] Integration with data modification from Sprint 10
- [ ] Integration with catalog operations from Sprint 4
- [ ] Parser produces structured diagnostics for malformed procedure syntax
- [ ] AST nodes have proper span information for all components
- [ ] Recovery mechanisms handle errors at statement/clause boundaries
- [ ] Unit tests cover all procedure call variants and error cases
- [ ] Integration tests validate end-to-end procedure invocations

## Implementation Tasks

### Task 1: AST Node Definitions for Procedure Call Statements

**Description**: Define AST types for procedure call statements and procedure call dispatch.

**Deliverables**:
- `CallProcedureStatement` struct:
  - `optional: bool` - whether OPTIONAL keyword present
  - `call: ProcedureCall` - procedure to invoke
  - `span: Span`
- `ProcedureCall` enum:
  - `Inline(InlineProcedureCall)` - inline procedure call
  - `Named(NamedProcedureCall)` - named procedure call

**Grammar References**:
- `callProcedureStatement` (Line 728)
- `procedureCall` (Line 732)

**Acceptance Criteria**:
- [ ] CallProcedureStatement AST defined in `src/ast/query.rs` or new `src/ast/procedure.rs`
- [ ] OPTIONAL flag captured in AST
- [ ] ProcedureCall enum distinguishes inline vs named calls
- [ ] Span tracking for each component
- [ ] Documentation explains OPTIONAL semantics (continue on failure)
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)

**File Location**: `src/ast/procedure.rs` (new file)

---

### Task 2: AST Node Definitions for Inline Procedure Calls

**Description**: Define AST types for inline procedure calls with variable scope and nested specs.

**Deliverables**:
- `InlineProcedureCall` struct:
  - `variable_scope: Option<VariableScopeClause>` - optional variable scope
  - `specification: NestedProcedureSpecification` - nested procedure body
  - `span: Span`
- `VariableScopeClause` struct:
  - `variables: Vec<BindingVariableReference>` - input/output variables (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `inlineProcedureCall` (Line 739)
- `variableScopeClause` (Line 743)
- `bindingVariableReferenceList` (Line 747)

**Acceptance Criteria**:
- [ ] InlineProcedureCall AST defined
- [ ] Variable scope clause supports binding variable references
- [ ] Empty variable scope `()` supported
- [ ] Multiple variables in scope clause supported (comma-separated)
- [ ] Integration with BindingVariableReference from Sprint 5
- [ ] Span tracking for each component
- [ ] Documentation explains variable scope semantics

**File Location**: `src/ast/procedure.rs`

---

### Task 3: AST Node Definitions for Named Procedure Calls

**Description**: Define AST types for named procedure calls with arguments and yield.

**Deliverables**:
- `NamedProcedureCall` struct:
  - `procedure: ProcedureReference` - procedure name (from Sprint 6)
  - `arguments: Option<ProcedureArgumentList>` - procedure arguments
  - `yield_clause: Option<YieldClause>` - yield items to return
  - `span: Span`
- `ProcedureArgumentList` struct:
  - `arguments: Vec<ProcedureArgument>` - comma-separated arguments
  - `span: Span`
- `ProcedureArgument` struct:
  - `expression: Expression` - argument value (from Sprint 5)
  - `span: Span`
- `YieldClause` struct:
  - `items: YieldItemList` - items to yield
  - `span: Span`
- `YieldItemList` struct:
  - `items: Vec<YieldItem>` - comma-separated yield items
  - `span: Span`
- `YieldItem` struct:
  - `expression: Expression` - expression to yield (from Sprint 5)
  - `alias: Option<YieldItemAlias>` - optional alias
  - `span: Span`
- `YieldItemAlias` struct:
  - `name: SmolStr` - alias name
  - `span: Span`

**Grammar References**:
- `namedProcedureCall` (Line 753)
- `procedureArgumentList` (Line 757)
- `procedureArgument` (Line 761)
- `yieldClause` (implied, similar to returnStatement body)
- `procedureReference` (from Sprint 6, Line 1458)

**Acceptance Criteria**:
- [ ] NamedProcedureCall AST defined
- [ ] Procedure arguments use expressions from Sprint 5
- [ ] Empty argument list `()` supported
- [ ] Multiple arguments supported (comma-separated)
- [ ] YIELD clause with yield items supported
- [ ] Yield items use expressions from Sprint 5
- [ ] Yield item aliases work
- [ ] Integration with ProcedureReference from Sprint 6
- [ ] Span tracking for each component
- [ ] Documentation explains procedure call semantics

**File Location**: `src/ast/procedure.rs`

---

### Task 4: AST Node Definitions for Nested Procedure Specifications

**Description**: Define AST types for nested procedure specifications and procedure bodies.

**Deliverables**:
- `NestedProcedureSpecification` struct:
  - `body: ProcedureBody` - procedure body content
  - `span: Span`
- `NestedDataModifyingProcedureSpecification` struct:
  - `body: ProcedureBody` - data modifying procedure body
  - `span: Span`
- `NestedQuerySpecification` struct:
  - `body: ProcedureBody` - query procedure body
  - `span: Span`
- `ProcedureBody` struct:
  - `at_schema: Option<AtSchemaClause>` - optional AT schema clause
  - `variable_definitions: Option<BindingVariableDefinitionBlock>` - variable definitions
  - `statements: StatementBlock` - statement execution block
  - `span: Span`
- `StatementBlock` struct:
  - `statements: Vec<Statement>` - sequential statements
  - `next_statements: Vec<NextStatement>` - NEXT chaining
  - `span: Span`

**Grammar References**:
- `nestedProcedureSpecification` (Line 138)
- `nestedDataModifyingProcedureSpecification` (Line 156)
- `nestedQuerySpecification` (Line 164)
- `procedureBody` (Line 174)
- `statementBlock` (Line 188)

**Acceptance Criteria**:
- [ ] All nested procedure specification types defined
- [ ] Nested specs use braces `{ }` for procedure bodies
- [ ] ProcedureBody includes AT schema, variables, and statements
- [ ] StatementBlock supports sequential statement execution
- [ ] NEXT statements for chaining supported
- [ ] Span tracking for each component
- [ ] Documentation explains nested procedure semantics
- [ ] Distinction between procedure/data-modifying/query specs documented

**File Location**: `src/ast/procedure.rs`

---

### Task 5: AST Node Definitions for Variable Definition Blocks

**Description**: Define AST types for binding variable definition blocks and variable types.

**Deliverables**:
- `BindingVariableDefinitionBlock` struct:
  - `definitions: Vec<BindingVariableDefinition>` - variable definitions
  - `span: Span`
- `BindingVariableDefinition` enum:
  - `Graph(GraphVariableDefinition)` - graph variable
  - `BindingTable(BindingTableVariableDefinition)` - binding table variable
  - `Value(ValueVariableDefinition)` - value variable
- `GraphVariableDefinition` struct:
  - `is_property: bool` - whether PROPERTY keyword present
  - `variable: BindingVariable` - variable name (from Sprint 5)
  - `type_annotation: Option<GraphReferenceValueType>` - optional type (from Sprint 6)
  - `initializer: Option<GraphInitializer>` - optional initializer
  - `span: Span`
- `GraphInitializer` struct:
  - `expression: GraphExpression` - graph expression (from Sprint 5)
  - `span: Span`
- `BindingTableVariableDefinition` struct:
  - `is_binding: bool` - whether BINDING keyword present
  - `variable: BindingVariable` - variable name (from Sprint 5)
  - `type_annotation: Option<BindingTableReferenceValueType>` - optional type (from Sprint 6)
  - `initializer: Option<BindingTableInitializer>` - optional initializer
  - `span: Span`
- `BindingTableInitializer` struct:
  - `expression: BindingTableExpression` - binding table expression (from Sprint 5)
  - `span: Span`
- `ValueVariableDefinition` struct:
  - `variable: BindingVariable` - variable name (from Sprint 5)
  - `type_annotation: Option<ValueType>` - optional type (from Sprint 6)
  - `initializer: Option<ValueInitializer>` - optional initializer
  - `span: Span`
- `ValueInitializer` struct:
  - `expression: Expression` - value expression (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `bindingVariableDefinitionBlock` (Line 178)
- `bindingVariableDefinition` (Line 182)
- `graphVariableDefinition` (Line 204)
- `optTypedGraphInitializer` (Line 208)
- `graphInitializer` (Line 212)
- `bindingTableVariableDefinition` (Line 218)
- `valueVariableDefinition` (Line 232)

**Acceptance Criteria**:
- [ ] All variable definition types defined
- [ ] Graph variables support PROPERTY keyword
- [ ] Binding table variables support BINDING keyword
- [ ] Value variables work
- [ ] Optional type annotations supported (from Sprint 6)
- [ ] Optional initializers supported (expressions from Sprint 5)
- [ ] Typed initializers (`::` operator) supported
- [ ] Multiple variable definitions in block supported
- [ ] Span tracking for each component
- [ ] Documentation explains variable scoping and lifecycle

**File Location**: `src/ast/procedure.rs`

---

### Task 6: AST Node Definitions for Statement and NEXT Statement

**Description**: Define AST types for statement blocks and NEXT chaining.

**Deliverables**:
- `Statement` enum:
  - `CompositeQuery(CompositeQueryStatement)` - query statement (from Sprint 7)
  - `LinearCatalogModifying(LinearCatalogModifyingStatement)` - catalog statement (from Sprint 4)
  - `LinearDataModifying(LinearDataModifyingStatement)` - data modification (from Sprint 10)
- `NextStatement` struct:
  - `yield_clause: Option<YieldClause>` - optional yield
  - `statement: Box<Statement>` - next statement to execute
  - `span: Span`

**Grammar References**:
- `statement` (Line 192)
- `nextStatement` (Line 198)

**Acceptance Criteria**:
- [ ] Statement enum encompasses all statement types
- [ ] NextStatement supports NEXT keyword with yield
- [ ] Sequential statement chaining via NEXT works
- [ ] Integration with query statements (Sprint 7)
- [ ] Integration with catalog statements (Sprint 4)
- [ ] Integration with data modification (Sprint 10)
- [ ] Span tracking for each component
- [ ] Documentation explains NEXT chaining semantics

**File Location**: `src/ast/procedure.rs`, integration with `src/ast/query.rs`, `src/ast/catalog.rs`, `src/ast/mutation.rs`

---

### Task 7: AST Node Definitions for AT Schema and USE Graph Clauses

**Description**: Define AST types for procedure and query context clauses.

**Deliverables**:
- `AtSchemaClause` struct:
  - `schema: SchemaReference` - schema reference (from Sprint 6)
  - `span: Span`
- **USE Graph Clause**: Already defined in Sprint 7 (`UseGraphClause`)
  - Verify integration with procedure contexts

**Grammar References**:
- `atSchemaClause` (Line 767)
- `useGraphClause` (Line 773, already from Sprint 7)

**Acceptance Criteria**:
- [ ] AtSchemaClause AST defined
- [ ] Integration with SchemaReference from Sprint 6
- [ ] UseGraphClause from Sprint 7 reused in procedure contexts
- [ ] Span tracking for each component
- [ ] Documentation explains context clause semantics

**File Location**: `src/ast/procedure.rs`, integration with `src/ast/query.rs`

---

### Task 8: Lexer Extensions for Procedure Tokens

**Description**: Ensure lexer supports all tokens needed for procedure parsing.

**Deliverables**:
- Verify existing procedure keywords are sufficient:
  - Procedure calls: CALL, OPTIONAL, YIELD
  - Variable scope: empty already handled by parentheses
  - Variable definitions: GRAPH, BINDING, TABLE, VALUE, PROPERTY (already from Sprint 4-6)
  - Context: AT, USE (AT new, USE from Sprint 7)
  - Chaining: NEXT
  - Procedure bodies: braces `{ }` already handled
- Add any missing keywords to keyword table:
  - **AT**: Schema context clause
  - **NEXT**: Statement chaining
  - **YIELD**: Result yielding (may overlap with graph pattern yield)
  - **OPTIONAL**: Optional procedure execution

**Lexer Enhancements Needed** (if any):
- Add AT keyword if missing
- Add NEXT keyword if missing
- Add YIELD keyword if missing (check if already exists from Sprint 8)
- Add OPTIONAL keyword if missing
- Ensure all keywords are case-insensitive

**Grammar References**:
- Procedure keyword definitions throughout Lines 138-200, 727-775

**Acceptance Criteria**:
- [ ] All procedure keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] No new lexer errors introduced
- [ ] All procedure-related tokens have proper span information
- [ ] Keywords distinguished from identifiers correctly

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 9: Procedure Parser - CALL Statements

**Description**: Implement parsing for CALL procedure statements.

**Deliverables**:
- `parse_call_procedure_statement()` - [OPTIONAL] CALL procedure_call
- Dispatch to inline or named procedure call parsers
- Handle OPTIONAL keyword

**Grammar References**:
- `callProcedureStatement` (Line 728)

**Acceptance Criteria**:
- [ ] CALL statements parse correctly
- [ ] OPTIONAL CALL works
- [ ] Dispatch to inline or named call parsers
- [ ] Error recovery on malformed CALL
- [ ] Unit tests for CALL variants

**File Location**: `src/parser/procedure.rs` (new file)

---

### Task 10: Procedure Parser - Inline Procedure Calls

**Description**: Implement parsing for inline procedure calls with variable scope.

**Deliverables**:
- `parse_inline_procedure_call()` - variable_scope? nested_procedure_spec
- `parse_variable_scope_clause()` - (variable_list?)
- `parse_binding_variable_reference_list()` - comma-separated variable references
- Integration with variable references from Sprint 5

**Grammar References**:
- `inlineProcedureCall` (Line 739)
- `variableScopeClause` (Line 743)
- `bindingVariableReferenceList` (Line 747)

**Acceptance Criteria**:
- [ ] Inline procedure calls parse correctly
- [ ] Variable scope clause with empty list `()` works
- [ ] Variable scope with multiple variables works
- [ ] Integration with binding variable references from Sprint 5
- [ ] Error recovery on malformed scope clause
- [ ] Unit tests for inline call variants

**File Location**: `src/parser/procedure.rs`

---

### Task 11: Procedure Parser - Named Procedure Calls

**Description**: Implement parsing for named procedure calls with arguments and yield.

**Deliverables**:
- `parse_named_procedure_call()` - procedure_ref(args?) yield?
- `parse_procedure_argument_list()` - comma-separated arguments
- `parse_procedure_argument()` - value expression
- `parse_yield_clause()` - YIELD yield_item_list
- `parse_yield_item_list()` - comma-separated yield items
- `parse_yield_item()` - expression [AS alias]
- `parse_yield_item_alias()` - AS alias_name
- Integration with procedure references from Sprint 6
- Integration with expressions from Sprint 5

**Grammar References**:
- `namedProcedureCall` (Line 753)
- `procedureArgumentList` (Line 757)
- `procedureArgument` (Line 761)

**Acceptance Criteria**:
- [ ] Named procedure calls parse correctly
- [ ] Procedure references from Sprint 6 work
- [ ] Empty argument list `()` works
- [ ] Multiple arguments work (comma-separated)
- [ ] Arguments use expressions from Sprint 5
- [ ] YIELD clause with yield items works
- [ ] Yield item aliases work
- [ ] Error recovery on malformed arguments or yield
- [ ] Unit tests for named call variants

**File Location**: `src/parser/procedure.rs`

---

### Task 12: Procedure Parser - Nested Procedure Specifications

**Description**: Implement parsing for nested procedure specifications and procedure bodies.

**Deliverables**:
- `parse_nested_procedure_specification()` - { procedure_body }
- `parse_nested_data_modifying_procedure_specification()` - { procedure_body }
- `parse_nested_query_specification()` - { procedure_body }
- `parse_procedure_body()` - at_schema? variables? statement_block
- Integration with statement block parsing

**Grammar References**:
- `nestedProcedureSpecification` (Line 138)
- `nestedDataModifyingProcedureSpecification` (Line 156)
- `nestedQuerySpecification` (Line 164)
- `procedureBody` (Line 174)

**Acceptance Criteria**:
- [ ] Nested procedure specs parse with braces `{ }`
- [ ] Nested data modifying specs work
- [ ] Nested query specs work
- [ ] Procedure body parses with AT clause, variables, and statements
- [ ] Empty procedure bodies work
- [ ] Error recovery on malformed nested specs
- [ ] Unit tests for nested spec variants

**File Location**: `src/parser/procedure.rs`

---

### Task 13: Procedure Parser - Variable Definition Blocks

**Description**: Implement parsing for binding variable definition blocks.

**Deliverables**:
- `parse_binding_variable_definition_block()` - multiple variable definitions
- `parse_binding_variable_definition()` - dispatch to variable types
- `parse_graph_variable_definition()` - [PROPERTY] GRAPH variable [:: type] [= initializer]
- `parse_binding_table_variable_definition()` - [BINDING] TABLE variable [:: type] [= initializer]
- `parse_value_variable_definition()` - VALUE variable [:: type] [= initializer]
- `parse_graph_initializer()` - = graph_expression
- `parse_binding_table_initializer()` - = binding_table_expression
- `parse_value_initializer()` - = value_expression
- Integration with type annotations from Sprint 6
- Integration with expressions from Sprint 5

**Grammar References**:
- `bindingVariableDefinitionBlock` (Line 178)
- `bindingVariableDefinition` (Line 182)
- `graphVariableDefinition` (Line 204)
- `optTypedGraphInitializer` (Line 208)
- `graphInitializer` (Line 212)
- `bindingTableVariableDefinition` (Line 218)
- `valueVariableDefinition` (Line 232)

**Acceptance Criteria**:
- [ ] Variable definition blocks parse correctly
- [ ] Multiple variable definitions work
- [ ] Graph variable definitions parse
- [ ] Binding table variable definitions parse
- [ ] Value variable definitions parse
- [ ] Optional PROPERTY and BINDING keywords work
- [ ] Type annotations from Sprint 6 work
- [ ] Initializers use expressions from Sprint 5
- [ ] Typed initializers (`::` operator) work
- [ ] Error recovery on malformed variable definitions
- [ ] Unit tests for all variable definition types

**File Location**: `src/parser/procedure.rs`

---

### Task 14: Procedure Parser - Statement Blocks and NEXT

**Description**: Implement parsing for statement blocks and NEXT chaining.

**Deliverables**:
- `parse_statement_block()` - statement next_statement*
- `parse_statement()` - dispatch to query/catalog/data-modifying
- `parse_next_statement()` - NEXT [YIELD] statement
- Integration with query statements from Sprint 7
- Integration with catalog statements from Sprint 4
- Integration with data modification from Sprint 10

**Grammar References**:
- `statementBlock` (Line 188)
- `statement` (Line 192)
- `nextStatement` (Line 198)

**Acceptance Criteria**:
- [ ] Statement blocks parse correctly
- [ ] Sequential statements work
- [ ] NEXT statement chaining works
- [ ] NEXT with YIELD clause works
- [ ] Integration with query statements (Sprint 7)
- [ ] Integration with catalog statements (Sprint 4)
- [ ] Integration with data modification (Sprint 10)
- [ ] Error recovery on malformed statement blocks
- [ ] Unit tests for statement block variants

**File Location**: `src/parser/procedure.rs`, integration with `src/parser/query.rs`, `src/parser/program.rs`, `src/parser/mutation.rs`

---

### Task 15: Procedure Parser - AT Schema and USE Graph Clauses

**Description**: Implement parsing for procedure and query context clauses.

**Deliverables**:
- `parse_at_schema_clause()` - AT schema_reference
- Verify `parse_use_graph_clause()` from Sprint 7 works in procedure contexts
- Integration with schema references from Sprint 6

**Grammar References**:
- `atSchemaClause` (Line 767)
- `useGraphClause` (Line 773, from Sprint 7)

**Acceptance Criteria**:
- [ ] AT schema clause parses correctly
- [ ] Integration with schema references from Sprint 6
- [ ] USE graph clause from Sprint 7 works in procedure contexts
- [ ] Error recovery on malformed context clauses
- [ ] Unit tests for context clauses

**File Location**: `src/parser/procedure.rs`, integration with `src/parser/query.rs`

---

### Task 16: Integration with Query Pipeline (Sprint 7-9)

**Description**: Integrate procedure calls with query pipeline from Sprint 7-9.

**Deliverables**:
- Update query AST to include CALL statements
- Ensure CALL can appear in query contexts
- Test procedure calls in linear queries
- Test procedure calls in composite queries
- Test procedure calls with result shaping (Sprint 9)

**Acceptance Criteria**:
- [ ] CALL integrates with linear queries
- [ ] CALL integrates with composite queries
- [ ] CALL with YIELD works in query pipeline
- [ ] OPTIONAL CALL continues execution flow correctly
- [ ] No regressions in existing query tests
- [ ] Integration tests validate end-to-end queries with procedures

**File Location**: `src/parser/query.rs`, `src/ast/query.rs`, `src/parser/procedure.rs`

---

### Task 17: Integration with Data Modification (Sprint 10)

**Description**: Integrate procedure calls with data modification from Sprint 10.

**Deliverables**:
- Update data modification AST to include data-modifying procedure calls
- Ensure CALL can appear in data modification contexts
- Test data-modifying procedure calls
- Verify nested data-modifying procedure specifications work

**Acceptance Criteria**:
- [ ] Data-modifying procedure calls parse correctly
- [ ] Integration with mutation statements (Sprint 10)
- [ ] Nested data-modifying specs work
- [ ] No regressions in existing mutation tests
- [ ] Integration tests validate end-to-end data modification with procedures

**File Location**: `src/parser/mutation.rs`, `src/ast/mutation.rs`, `src/parser/procedure.rs`

---

### Task 18: Integration with Catalog Operations (Sprint 4)

**Description**: Integrate procedure calls with catalog operations from Sprint 4.

**Deliverables**:
- Update catalog AST to include catalog-modifying procedure calls
- Ensure CALL can appear in catalog contexts
- Test catalog-modifying procedure calls

**Acceptance Criteria**:
- [ ] Catalog-modifying procedure calls parse correctly
- [ ] Integration with catalog statements (Sprint 4)
- [ ] No regressions in existing catalog tests
- [ ] Integration tests validate end-to-end catalog operations with procedures

**File Location**: `src/parser/program.rs`, `src/ast/catalog.rs`, `src/parser/procedure.rs`

---

### Task 19: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for procedure parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at statement boundaries (CALL, NEXT)
  - Recover at clause boundaries (AT, USE, YIELD)
  - Recover at brace delimiters (nested specs)
  - Recover at comma separators (arguments, yield items, variable definitions)
  - Partial AST construction on errors
- Diagnostic messages:
  - "Expected procedure call after CALL"
  - "Expected nested procedure specification after variable scope"
  - "Expected procedure reference in named procedure call"
  - "Expected argument expression after comma"
  - "Invalid variable scope clause"
  - "Expected yield item after YIELD"
  - "Expected statement after NEXT"
  - "AT clause requires schema reference"
  - "Variable definition requires variable name"
  - "Invalid variable initializer"
- Span highlighting for error locations
- Helpful error messages with suggestions:
  - "Did you mean CALL procedure_name()?"
  - "OPTIONAL CALL continues execution even if procedure fails"
  - "Variable scope clause requires parentheses: (variable_list)"
  - "YIELD items must be expressions with optional aliases"
  - "NEXT chains statements sequentially"
  - "AT clause sets schema context for procedure body"

**Grammar References**:
- All procedure rules (Lines 138-200, 727-775)

**Acceptance Criteria**:
- [ ] Procedure parser recovers from common errors
- [ ] Multiple errors in one procedure call reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Suggestions provided for common procedure syntax errors
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/procedure.rs`, `src/diag.rs`

---

### Task 20: Comprehensive Testing

**Description**: Implement comprehensive test suite for procedure parsing.

**Deliverables**:

#### Unit Tests (`src/parser/procedure.rs`):
- **CALL Statement Tests**:
  - Simple CALL
  - OPTIONAL CALL
  - CALL with inline procedure
  - CALL with named procedure

- **Inline Procedure Call Tests**:
  - Inline call with empty variable scope `()`
  - Inline call with variable scope `(var1, var2)`
  - Inline call without variable scope
  - Inline call with nested procedure spec

- **Named Procedure Call Tests**:
  - Named call with no arguments `procedure()`
  - Named call with single argument
  - Named call with multiple arguments
  - Named call with YIELD clause
  - Named call with yield item aliases

- **Nested Procedure Specification Tests**:
  - Empty nested spec `{ }`
  - Nested spec with AT schema clause
  - Nested spec with variable definitions
  - Nested spec with statement block
  - Nested data modifying spec
  - Nested query spec

- **Variable Definition Tests**:
  - Graph variable definition
  - Graph variable with PROPERTY keyword
  - Graph variable with type annotation
  - Graph variable with initializer
  - Binding table variable definition
  - Binding table variable with BINDING keyword
  - Value variable definition
  - Multiple variable definitions in block

- **Statement Block Tests**:
  - Single statement
  - Multiple sequential statements
  - Statement with NEXT chaining
  - NEXT with YIELD clause

- **Context Clause Tests**:
  - AT schema clause
  - USE graph clause in procedure context

- **Error Recovery Tests**:
  - Missing procedure reference
  - Malformed argument list
  - Invalid variable scope
  - Malformed nested spec
  - Invalid NEXT statement

#### Integration Tests (`tests/procedure_tests.rs` - new file):
- Complete query with CALL ... YIELD
- Data modification with CALL
- Catalog operation with CALL
- Complex nested procedure specs
- OPTIONAL CALL with error handling
- NEXT chaining across multiple statements
- Edge cases (deeply nested, complex flows)

#### Snapshot Tests:
- Capture AST output for representative procedure calls
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for procedure parser
- [ ] All procedure variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (complex nesting, all clause combinations)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/procedure.rs`, `tests/procedure_tests.rs`

---

### Task 21: Documentation and Examples

**Description**: Document procedure system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all procedure AST node types
  - Module-level documentation for procedures
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase procedures
  - Add `examples/procedure_demo.rs` demonstrating:
    - Simple CALL statements
    - OPTIONAL CALL with error handling
    - Inline procedure calls with variable scope
    - Named procedure calls with arguments and YIELD
    - Nested procedure specifications
    - Variable definition blocks
    - NEXT statement chaining
    - Complete procedural flows with queries and mutations

- **Procedure Overview Documentation**:
  - Document procedure semantics and execution model
  - Document variable scoping rules
  - Document NEXT chaining semantics
  - Document OPTIONAL behavior
  - Document YIELD semantics
  - Document AT and USE context clauses
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for procedures
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Procedure overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all procedure error codes
- [ ] Documentation explains procedure semantics clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/procedure.rs`, `src/parser/procedure.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Procedure Call Context**: Procedure calls can appear in multiple contexts:
   - As standalone statements in query pipelines
   - As data-modifying operations in mutation flows
   - As catalog operations in schema management
   - Parser should handle all contexts uniformly

2. **Variable Scoping**: Procedures introduce complex scoping:
   - Variable scope clause controls input/output variables
   - Procedure body can define local variables
   - AT schema clause sets schema context
   - USE graph clause sets graph context
   - Parser should track variable contexts for semantic validation (Sprint 14)

3. **Nested Procedure Specifications**: Procedures can be nested:
   - Inline procedures use nested specs
   - Nested specs can contain full statement blocks
   - Braces `{ }` delimit nested specs
   - Parser must handle arbitrary nesting depth

4. **NEXT Chaining**: NEXT provides sequential composition:
   - NEXT chains statements together
   - NEXT can include YIELD clause
   - Parser should handle multiple NEXT statements in sequence
   - Error recovery should handle malformed chains

5. **OPTIONAL Semantics**: OPTIONAL changes execution flow:
   - OPTIONAL CALL continues execution on failure
   - Non-optional CALL aborts on failure
   - Parser should clearly distinguish these cases

6. **YIELD vs RETURN**: YIELD and RETURN are similar but distinct:
   - YIELD appears in procedure calls
   - RETURN appears in result statements
   - Both have item lists with aliases
   - Parser should reuse similar parsing logic where applicable

7. **Error Recovery**: Procedure clauses have clear boundaries:
   - Recover at statement keywords (CALL, NEXT)
   - Recover at clause keywords (AT, USE, YIELD)
   - Recover at brace delimiters for nested specs
   - Recover at comma separators in lists
   - Continue parsing after errors to report multiple issues

### AST Design Considerations

1. **Span Tracking**: Every procedure node must track its source span for diagnostic purposes.

2. **Optional Fields**: Many procedure components are optional:
   - OPTIONAL keyword in CALL
   - Variable scope in inline calls
   - Arguments in named calls
   - YIELD clause in named calls
   - AT schema clause in procedure body
   - Variable definitions in procedure body
   - Use `Option<T>` appropriately

3. **Expression Reuse**: Use expression AST from Sprint 5:
   - Procedure arguments are expressions
   - Yield items are expressions
   - Variable initializers are expressions
   - Don't duplicate expression types

4. **Reference Reuse**: Use reference types from Sprint 6:
   - Procedure references
   - Schema references
   - Graph references (for USE clause)
   - Binding variable references
   - Don't duplicate reference types

5. **List Types**: Use `Vec<T>` for:
   - Procedure argument lists
   - Yield item lists
   - Variable definition lists
   - Variable reference lists
   - Statement sequences
   - Clear comma-separated list parsing

6. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Yield item aliases
   - Variable names
   - Short identifiers

### Error Recovery Strategy

1. **Synchronization Points**:
   - Statement keywords (CALL, NEXT)
   - Clause keywords (AT, USE, YIELD)
   - Brace delimiters `{ }` for nested specs
   - Comma separators in lists
   - End of statement (semicolon or next major clause)

2. **Statement Boundary Recovery**: If statement malformed:
   - Report error at statement location
   - Skip to next statement keyword
   - Continue parsing remaining statements
   - Construct partial AST

3. **Clause Boundary Recovery**: If clause malformed:
   - Report error at clause location
   - Skip to next clause keyword or brace
   - Continue parsing remaining clauses
   - Construct partial AST

4. **List Recovery**: If item in list malformed:
   - Report error at item location
   - Skip to next comma or end of list
   - Continue with next item
   - Include valid items in AST

5. **Expression Recovery**: If expression malformed:
   - Use expression parser's error recovery from Sprint 5
   - Return error placeholder expression
   - Continue parsing

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error in procedure"
   - Good: "Expected procedure reference after CALL keyword, found INTEGER"

2. **Helpful Suggestions**:
   - "Did you mean CALL procedure_name()?"
   - "OPTIONAL CALL continues execution even if procedure fails"
   - "Variable scope clause requires parentheses: (variable_list)"
   - "YIELD items must be expressions with optional aliases"
   - "NEXT chains statements sequentially"
   - "AT clause sets schema context for procedure body"
   - "USE clause sets graph context for queries and mutations"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing clauses, point to where clause expected
   - For malformed items, highlight entire item
   - For invalid keywords, highlight keyword token

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing procedure call..."
   - "In nested procedure specification starting at line 42..."
   - "While parsing variable definition block..."

### Performance Considerations

1. **Procedure Parsing Efficiency**: Procedures are common:
   - Use efficient lookahead (1-2 tokens typically sufficient)
   - Minimize backtracking
   - Use direct dispatch to procedure call parsers

2. **List Parsing**: Use efficient comma-separated list parsing:
   - Single-pass parsing
   - Clear termination conditions
   - Avoid unnecessary allocations

3. **Expression Reuse**: Reuse expression parser from Sprint 5:
   - Don't duplicate expression parsing logic
   - Leverage existing expression performance

4. **Reference Reuse**: Reuse reference parser from Sprint 6:
   - Don't duplicate reference parsing logic
   - Leverage existing reference performance

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (procedure keywords, operators)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Catalog statements; integration testing infrastructure
- **Sprint 5**: Expression parsing for arguments, yield items, variable initializers
- **Sprint 6**: Type system for variable type annotations; reference forms for procedure/schema references
- **Sprint 7**: Query pipeline structure; USE GRAPH clause
- **Sprint 8**: Pattern matching (for use in procedure contexts)
- **Sprint 9**: Result shaping (YIELD similar to RETURN)
- **Sprint 10**: Data modification statements (used in data-modifying procedures)

### Dependencies on Future Sprints

- **Sprint 12**: Graph type specifications (not directly related)
- **Sprint 13**: Conformance hardening (stress testing procedures)
- **Sprint 14**: Semantic validation (variable scoping, procedure signature checking, context validation)

### Cross-Sprint Integration Points

- Procedures are a fundamental abstraction layer in GQL
- CALL integrates with query pipeline (Sprint 7)
- CALL integrates with data modification (Sprint 10)
- CALL integrates with catalog operations (Sprint 4)
- YIELD is similar to RETURN (Sprint 9)
- Variable definitions use types (Sprint 6) and expressions (Sprint 5)
- AT and USE clauses set execution context
- Semantic validation (Sprint 14) will check:
  - Variable scoping (procedure variables vs outer scope)
  - Procedure signature matching (arguments vs parameters)
  - Context validation (AT schema, USE graph)
  - YIELD compatibility with procedure return types

## Test Strategy

### Unit Tests

For each procedure component:
1. **Happy Path**: Valid procedure calls parse correctly
2. **Variants**: All syntax variants and optional components
3. **Error Cases**: Missing components, invalid syntax, malformed calls
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Procedures in different contexts:
1. **Query Integration**: CALL in MATCH queries, SELECT statements
2. **Mutation Integration**: CALL in INSERT/SET/REMOVE/DELETE flows
3. **Catalog Integration**: CALL in schema/graph management
4. **Nested Procedures**: Complex nested procedure specifications
5. **NEXT Chaining**: Sequential statement composition
6. **Complete Flows**: End-to-end procedural execution with queries, mutations, and catalog ops

### Snapshot Tests

Capture AST output:
1. Representative procedure calls from each category
2. Complex nested procedure specifications
3. NEXT chaining with multiple statements
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid procedure calls
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries:
1. Official GQL sample queries with procedures
2. Real-world procedural graph queries
3. Verify parser handles production syntax

### Performance Tests

1. **Deeply Nested Procedures**: Many levels of nesting
2. **Long Argument Lists**: Many procedure arguments
3. **Long Statement Chains**: Many NEXT statements
4. **Complex Variable Definitions**: Many variable definitions with initializers

## Performance Considerations

1. **Lexer Efficiency**: Procedure keywords are frequent; lexer must be fast
2. **Parser Efficiency**: Use direct dispatch and minimal lookahead
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Expression Reuse**: Leverage Sprint 5 expression parser performance
5. **Reference Reuse**: Leverage Sprint 6 reference parser performance

## Documentation Requirements

1. **API Documentation**: Rustdoc for all procedure AST nodes and parser functions
2. **Procedure Overview**: Document procedure semantics, scoping, and execution model
3. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
4. **Examples**: Demonstrate procedures in examples
5. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Variable scoping complexity | High | Medium | Clear AST design; thorough testing; defer full scoping validation to Sprint 14 |
| Nested procedure spec parsing complexity | Medium | Medium | Recursive descent parsing; clear brace matching; extensive testing |
| Integration with multiple statement types | Medium | Low | Uniform statement interface; careful integration testing; preserve existing tests |
| NEXT chaining ambiguity | Low | Low | Clear AST design; good error messages; documentation |
| OPTIONAL semantics confusion | Low | Low | Clear documentation; helpful error messages; semantic validation in Sprint 14 |
| YIELD vs RETURN confusion | Low | Medium | Clear AST distinction; helpful diagnostics; documentation explains difference |
| Performance on deeply nested procedures | Low | Low | Optimize hot paths; use efficient parsing; profile and optimize if needed |

## Success Metrics

1. **Coverage**: All procedure features parse with correct AST
2. **Correctness**: Procedure semantics match ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for procedure parser
6. **Performance**: Parser handles procedures with 50+ arguments/statements in <1ms
7. **Integration**: Procedures integrate cleanly with Sprint 4 (catalog), Sprint 5 (expressions), Sprint 6 (references), Sprint 7 (queries), Sprint 9 (yield), and Sprint 10 (mutations)
8. **Completeness**: All procedure call types work; nested specs work; variable definitions work; NEXT chaining works

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping, procedure overview)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Procedures tested in multiple contexts (query, mutation, catalog)
- [ ] AST design reviewed for stability and extensibility
- [ ] Sprint 4 integration complete (catalog operations)
- [ ] Sprint 5 integration complete (expressions in arguments, yield, initializers)
- [ ] Sprint 6 integration complete (references and types)
- [ ] Sprint 7 integration complete (USE GRAPH clause)
- [ ] Sprint 9 integration complete (YIELD similar to RETURN)
- [ ] Sprint 10 integration complete (data-modifying procedures)
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 12: Graph Type Specification Depth** will complete the schema/type modeling grammar, implementing nested graph type specifications, node/edge type patterns and phrases, endpoint pairs, label/property type specs, and other advanced graph type definition features. With the procedural layer complete, Sprint 12 provides the final piece of the type system for comprehensive schema definition.

---

## Appendix: Procedure Call Hierarchy

```
CallProcedureStatement
├── optional: bool (OPTIONAL keyword)
└── call: ProcedureCall
    ├── Inline(InlineProcedureCall)
    │   ├── variable_scope: Option<VariableScopeClause>
    │   │   └── variables: Vec<BindingVariableReference> (from Sprint 5)
    │   └── specification: NestedProcedureSpecification
    │       └── body: ProcedureBody
    │           ├── at_schema: Option<AtSchemaClause>
    │           │   └── schema: SchemaReference (from Sprint 6)
    │           ├── variable_definitions: Option<BindingVariableDefinitionBlock>
    │           │   └── definitions: Vec<BindingVariableDefinition>
    │           │       ├── Graph(GraphVariableDefinition)
    │           │       │   ├── is_property: bool
    │           │       │   ├── variable: BindingVariable (from Sprint 5)
    │           │       │   ├── type_annotation: Option<GraphReferenceValueType> (from Sprint 6)
    │           │       │   └── initializer: Option<GraphInitializer>
    │           │       │       └── expression: GraphExpression (from Sprint 5)
    │           │       ├── BindingTable(BindingTableVariableDefinition)
    │           │       │   ├── is_binding: bool
    │           │       │   ├── variable: BindingVariable (from Sprint 5)
    │           │       │   ├── type_annotation: Option<BindingTableReferenceValueType> (from Sprint 6)
    │           │       │   └── initializer: Option<BindingTableInitializer>
    │           │       │       └── expression: BindingTableExpression (from Sprint 5)
    │           │       └── Value(ValueVariableDefinition)
    │           │           ├── variable: BindingVariable (from Sprint 5)
    │           │           ├── type_annotation: Option<ValueType> (from Sprint 6)
    │           │           └── initializer: Option<ValueInitializer>
    │           │               └── expression: Expression (from Sprint 5)
    │           └── statements: StatementBlock
    │               ├── statements: Vec<Statement>
    │               │   ├── CompositeQuery(CompositeQueryStatement) (from Sprint 7)
    │               │   ├── LinearCatalogModifying(LinearCatalogModifyingStatement) (from Sprint 4)
    │               │   └── LinearDataModifying(LinearDataModifyingStatement) (from Sprint 10)
    │               └── next_statements: Vec<NextStatement>
    │                   ├── yield_clause: Option<YieldClause>
    │                   │   └── items: YieldItemList
    │                   │       └── items: Vec<YieldItem>
    │                   │           ├── expression: Expression (from Sprint 5)
    │                   │           └── alias: Option<YieldItemAlias>
    │                   └── statement: Box<Statement>
    └── Named(NamedProcedureCall)
        ├── procedure: ProcedureReference (from Sprint 6)
        ├── arguments: Option<ProcedureArgumentList>
        │   └── arguments: Vec<ProcedureArgument>
        │       └── expression: Expression (from Sprint 5)
        └── yield_clause: Option<YieldClause>
            └── items: YieldItemList
                └── items: Vec<YieldItem>
                    ├── expression: Expression (from Sprint 5)
                    └── alias: Option<YieldItemAlias>
```

---

## Appendix: Procedure Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `callProcedureStatement` | 728 | `CallProcedureStatement` struct | `parse_call_procedure_statement()` |
| `procedureCall` | 732 | `ProcedureCall` enum | (dispatch in `parse_call_procedure_statement()`) |
| `inlineProcedureCall` | 739 | `InlineProcedureCall` struct | `parse_inline_procedure_call()` |
| `variableScopeClause` | 743 | `VariableScopeClause` struct | `parse_variable_scope_clause()` |
| `bindingVariableReferenceList` | 747 | `Vec<BindingVariableReference>` | `parse_binding_variable_reference_list()` |
| `namedProcedureCall` | 753 | `NamedProcedureCall` struct | `parse_named_procedure_call()` |
| `procedureArgumentList` | 757 | `ProcedureArgumentList` struct | `parse_procedure_argument_list()` |
| `procedureArgument` | 761 | `ProcedureArgument` struct | `parse_procedure_argument()` |
| `atSchemaClause` | 767 | `AtSchemaClause` struct | `parse_at_schema_clause()` |
| `useGraphClause` | 773 | `UseGraphClause` struct (from Sprint 7) | `parse_use_graph_clause()` (from Sprint 7) |
| `nestedProcedureSpecification` | 138 | `NestedProcedureSpecification` struct | `parse_nested_procedure_specification()` |
| `nestedDataModifyingProcedureSpecification` | 156 | `NestedDataModifyingProcedureSpecification` struct | `parse_nested_data_modifying_procedure_specification()` |
| `nestedQuerySpecification` | 164 | `NestedQuerySpecification` struct | `parse_nested_query_specification()` |
| `procedureBody` | 174 | `ProcedureBody` struct | `parse_procedure_body()` |
| `bindingVariableDefinitionBlock` | 178 | `BindingVariableDefinitionBlock` struct | `parse_binding_variable_definition_block()` |
| `bindingVariableDefinition` | 182 | `BindingVariableDefinition` enum | `parse_binding_variable_definition()` |
| `graphVariableDefinition` | 204 | `GraphVariableDefinition` struct | `parse_graph_variable_definition()` |
| `optTypedGraphInitializer` | 208 | -- | (part of `parse_graph_variable_definition()`) |
| `graphInitializer` | 212 | `GraphInitializer` struct | `parse_graph_initializer()` |
| `bindingTableVariableDefinition` | 218 | `BindingTableVariableDefinition` struct | `parse_binding_table_variable_definition()` |
| `valueVariableDefinition` | 232 | `ValueVariableDefinition` struct | `parse_value_variable_definition()` |
| `statementBlock` | 188 | `StatementBlock` struct | `parse_statement_block()` |
| `statement` | 192 | `Statement` enum | `parse_statement()` |
| `nextStatement` | 198 | `NextStatement` struct | `parse_next_statement()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-18
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 (completed or required)
**Next Sprint**: Sprint 12 (Graph Type Specification Depth)
