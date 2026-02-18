# Sprint 10: Data Modification Statements

## Sprint Overview

**Sprint Goal**: Implement graph mutation grammar end-to-end.

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

## Scope

This sprint implements the complete data modification system for GQL, enabling mutation operations on property graphs. Data modification includes inserting new nodes and edges, updating properties and labels, removing properties and labels, and deleting nodes and edges. Sprint 8 established pattern matching, and Sprint 10 adds the ability to modify graph data using similar pattern syntax in mutation contexts.

### Feature Coverage from GQL_FEATURES.md

Sprint 10 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 8: Data Modification Operations** (Lines 506-619)
   - Linear data modifying statements
   - Focused linear data modifying statements (with USE GRAPH)
   - Ambient linear data modifying statements (without USE GRAPH)
   - Primitive data modifying statements
   - INSERT statement
   - SET statement
   - REMOVE statement
   - DELETE statement (DETACH/NODETACH variants)
   - Data modifying procedure calls

2. **INSERT Operations** (Lines 527-554):
   - Insert graph patterns
   - Insert path patterns
   - Insert node patterns with labels and properties
   - Insert edge patterns (3 direction types: left, right, undirected)
   - Insert element pattern fillers (variables, labels, properties)

3. **SET Operations** (Lines 556-578):
   - Set property items (update single property)
   - Set all properties items (replace all properties)
   - Set label items (add label to element)

4. **REMOVE Operations** (Lines 580-596):
   - Remove property items (delete specific property)
   - Remove label items (remove label from element)

5. **DELETE Operations** (Lines 598-612):
   - Delete statements with DETACH/NODETACH options
   - Delete items (element variables to delete)

## Exit Criteria

- [ ] Linear data modifying statements parse correctly
- [ ] Focused data modifying statements (with USE GRAPH) parse correctly
- [ ] Ambient data modifying statements (without USE GRAPH) parse correctly
- [ ] INSERT statements parse with graph patterns
- [ ] Insert node patterns with labels and properties work
- [ ] Insert edge patterns parse with all 3 direction types (left, right, undirected)
- [ ] SET statements parse with all set item types
- [ ] Set property items work (single property updates)
- [ ] Set all properties items work (replace all properties)
- [ ] Set label items work (add labels to elements)
- [ ] REMOVE statements parse with all remove item types
- [ ] Remove property items work (delete specific properties)
- [ ] Remove label items work (remove labels from elements)
- [ ] DELETE statements parse with DETACH/NODETACH options
- [ ] Delete items work (delete element variables)
- [ ] Data modifying procedure calls parse correctly
- [ ] Integration with expression parsing from Sprint 5 for property values
- [ ] Integration with pattern parsing from Sprint 8 for INSERT patterns
- [ ] Parser produces structured diagnostics for malformed data modification statements
- [ ] AST nodes have proper span information for all components
- [ ] Recovery mechanisms handle errors at statement boundaries
- [ ] Unit tests cover all data modification variants and error cases
- [ ] Integration tests validate end-to-end data modification statements

## Implementation Tasks

### Task 1: AST Node Definitions for Linear Data Modifying Statements

**Description**: Define AST types for linear data modifying statement structure.

**Deliverables**:
- `LinearDataModifyingStatement` enum:
  - `Focused(FocusedLinearDataModifyingStatement)` - with USE GRAPH
  - `Ambient(AmbientLinearDataModifyingStatement)` - without USE GRAPH
- `FocusedLinearDataModifyingStatement` struct:
  - `use_graph_clause: UseGraphClause` - USE GRAPH clause (from Sprint 7)
  - `primitives: Vec<PrimitiveDataModifyingStatement>` - modification operations
  - `primitive_result_statement: Option<PrimitiveResultStatement>` - optional RETURN (from Sprint 9)
  - `span: Span`
- `AmbientLinearDataModifyingStatement` struct:
  - `primitives: Vec<SimplePrimitiveDataModifyingStatement>` - modification operations
  - `primitive_result_statement: Option<PrimitiveResultStatement>` - optional RETURN (from Sprint 9)
  - `span: Span`
- `SimplePrimitiveDataModifyingStatement` struct:
  - `statement: PrimitiveDataModifyingStatement` - individual modification statement
  - `span: Span`

**Grammar References**:
- `focusedLinearDataModifyingStatement` (Line 376)
- `ambientLinearDataModifyingStatement` (Line 389)
- `simpleLinearDataModifyingStatement` (Line 394)
- `simpleDataModifyingStatement` (Line 401)
- `simplePrimitiveDataModifyingStatement` (Line 408)

**Acceptance Criteria**:
- [ ] All linear data modifying statement AST types defined in `src/ast/mutation.rs` (new file)
- [ ] Each node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each variant
- [ ] Focused vs ambient distinction clear
- [ ] Integration with USE GRAPH clause from Sprint 7
- [ ] Integration with RETURN statement from Sprint 9

**File Location**: `src/ast/mutation.rs` (new file)

---

### Task 2: AST Node Definitions for Primitive Data Modifying Statements

**Description**: Define AST types for primitive data modification operations.

**Deliverables**:
- `PrimitiveDataModifyingStatement` enum:
  - `Insert(InsertStatement)` - INSERT statement
  - `Set(SetStatement)` - SET statement
  - `Remove(RemoveStatement)` - REMOVE statement
  - `Delete(DeleteStatement)` - DELETE statement
- Each variant includes `span: Span` for diagnostics

**Grammar References**:
- `primitiveDataModifyingStatement` (Line 412)

**Acceptance Criteria**:
- [ ] Primitive data modifying statement enum defined
- [ ] All four statement types represented (INSERT, SET, REMOVE, DELETE)
- [ ] Span tracking for each statement type
- [ ] Documentation explains each statement purpose

**File Location**: `src/ast/mutation.rs`

---

### Task 3: AST Node Definitions for INSERT Statements

**Description**: Define AST types for INSERT statement and insert patterns.

**Deliverables**:
- `InsertStatement` struct:
  - `pattern: InsertGraphPattern` - pattern to insert
  - `span: Span`
- `InsertGraphPattern` struct:
  - `paths: Vec<InsertPathPattern>` - comma-separated insert path patterns
  - `span: Span`
- `InsertPathPattern` struct:
  - `elements: Vec<InsertElementPattern>` - sequential insert elements
  - `span: Span`
- `InsertElementPattern` enum:
  - `Node(InsertNodePattern)` - insert node pattern
  - `Edge(InsertEdgePattern)` - insert edge pattern
- `InsertNodePattern` struct:
  - `filler: InsertElementPatternFiller` - node details
  - `span: Span`
- `InsertEdgePattern` enum:
  - `PointingLeft(InsertEdgePointingLeft)` - `<-[edge]-`
  - `PointingRight(InsertEdgePointingRight)` - `-[edge]->`
  - `Undirected(InsertEdgeUndirected)` - `~[edge]~`
- `InsertEdgePointingLeft` struct:
  - `filler: InsertElementPatternFiller` - edge details
  - `span: Span`
- `InsertEdgePointingRight` struct:
  - `filler: InsertElementPatternFiller` - edge details
  - `span: Span`
- `InsertEdgeUndirected` struct:
  - `filler: InsertElementPatternFiller` - edge details
  - `span: Span`
- `InsertElementPatternFiller` struct:
  - `variable: Option<ElementVariableDeclaration>` - optional element variable (from Sprint 8)
  - `label_expression: Option<LabelExpression>` - optional label (from Sprint 8)
  - `properties: Option<ElementPropertySpecification>` - optional properties (from Sprint 8)
  - `span: Span`

**Grammar References**:
- `insertStatement` (Line 421)
- `insertGraphPattern` (Line 852)
- `insertPathPattern` (Line 860)
- `insertNodePattern` (Line 864)
- `insertEdgePattern` (Line 868)
- `insertEdgePointingLeft` (Line 872)
- `insertEdgePointingRight` (Line 874)
- `insertEdgeUndirected` (Line 876)
- `insertElementPatternFiller` (Line 886)

**Acceptance Criteria**:
- [ ] INSERT statement AST defined
- [ ] Insert graph pattern structure supports multiple path patterns
- [ ] Insert path patterns support sequential elements
- [ ] Insert node patterns support variables, labels, and properties
- [ ] Insert edge patterns support all 3 direction types
- [ ] Element pattern filler reuses components from Sprint 8
- [ ] Span tracking for each component
- [ ] Documentation explains INSERT semantics

**File Location**: `src/ast/mutation.rs`

---

### Task 4: AST Node Definitions for SET Statements

**Description**: Define AST types for SET statement and set item types.

**Deliverables**:
- `SetStatement` struct:
  - `items: SetItemList` - set operations to perform
  - `span: Span`
- `SetItemList` struct:
  - `items: Vec<SetItem>` - comma-separated set items
  - `span: Span`
- `SetItem` enum:
  - `Property(SetPropertyItem)` - update single property
  - `AllProperties(SetAllPropertiesItem)` - replace all properties
  - `Label(SetLabelItem)` - add label
- `SetPropertyItem` struct:
  - `element: BindingVariableReference` - element variable (from Sprint 5)
  - `property: PropertyName` - property name
  - `value: Expression` - new value (from Sprint 5)
  - `span: Span`
- `SetAllPropertiesItem` struct:
  - `element: BindingVariableReference` - element variable (from Sprint 5)
  - `properties: Expression` - record expression with all properties (from Sprint 5)
  - `span: Span`
- `SetLabelItem` struct:
  - `element: BindingVariableReference` - element variable (from Sprint 5)
  - `label: LabelExpression` - label to add (from Sprint 8)
  - `use_is_keyword: bool` - whether IS keyword used (vs colon syntax)
  - `span: Span`

**Grammar References**:
- `setStatement` (Line 427)
- `setItemList` (Line 431)
- `setItem` (Line 435)
- `setPropertyItem` (Line 441)
- `setAllPropertiesItem` (Line 445)
- `setLabelItem` (Line 449)

**Acceptance Criteria**:
- [ ] SET statement AST defined
- [ ] Set item list supports multiple comma-separated items
- [ ] Set property item supports single property updates
- [ ] Set all properties item supports record expression
- [ ] Set label item supports both colon and IS keyword syntax
- [ ] Integration with expressions from Sprint 5
- [ ] Integration with label expressions from Sprint 8
- [ ] Span tracking for each component
- [ ] Documentation explains SET semantics

**File Location**: `src/ast/mutation.rs`

---

### Task 5: AST Node Definitions for REMOVE Statements

**Description**: Define AST types for REMOVE statement and remove item types.

**Deliverables**:
- `RemoveStatement` struct:
  - `items: RemoveItemList` - remove operations to perform
  - `span: Span`
- `RemoveItemList` struct:
  - `items: Vec<RemoveItem>` - comma-separated remove items
  - `span: Span`
- `RemoveItem` enum:
  - `Property(RemovePropertyItem)` - remove single property
  - `Label(RemoveLabelItem)` - remove label
- `RemovePropertyItem` struct:
  - `element: BindingVariableReference` - element variable (from Sprint 5)
  - `property: PropertyName` - property name
  - `span: Span`
- `RemoveLabelItem` struct:
  - `element: BindingVariableReference` - element variable (from Sprint 5)
  - `label: LabelExpression` - label to remove (from Sprint 8)
  - `use_is_keyword: bool` - whether IS keyword used (vs colon syntax)
  - `span: Span`

**Grammar References**:
- `removeStatement` (Line 455)
- `removeItemList` (Line 459)
- `removeItem` (Line 463)
- `removePropertyItem` (Line 468)
- `removeLabelItem` (Line 472)

**Acceptance Criteria**:
- [ ] REMOVE statement AST defined
- [ ] Remove item list supports multiple comma-separated items
- [ ] Remove property item supports single property deletion
- [ ] Remove label item supports both colon and IS keyword syntax
- [ ] Integration with label expressions from Sprint 8
- [ ] Span tracking for each component
- [ ] Documentation explains REMOVE semantics

**File Location**: `src/ast/mutation.rs`

---

### Task 6: AST Node Definitions for DELETE Statements

**Description**: Define AST types for DELETE statement with DETACH/NODETACH options.

**Deliverables**:
- `DeleteStatement` struct:
  - `detach_option: DetachOption` - DETACH/NODETACH/default
  - `items: DeleteItemList` - elements to delete
  - `span: Span`
- `DetachOption` enum:
  - `Detach` - DETACH DELETE (delete node and all connected edges)
  - `NoDetach` - NODETACH DELETE (only delete if no edges remain)
  - `Default` - no keyword specified (defaults to NODETACH behavior)
- `DeleteItemList` struct:
  - `items: Vec<DeleteItem>` - comma-separated delete items
  - `span: Span`
- `DeleteItem` struct:
  - `element: BindingVariableReference` - element variable to delete (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `deleteStatement` (Line 478)
- `deleteItemList` (Line 482)
- `deleteItem` (Line 486)

**Acceptance Criteria**:
- [ ] DELETE statement AST defined
- [ ] DETACH option properly distinguished (DETACH/NODETACH/default)
- [ ] Delete item list supports multiple comma-separated items
- [ ] Integration with binding variable references from Sprint 5
- [ ] Span tracking for each component
- [ ] Documentation explains DELETE semantics and DETACH behavior

**File Location**: `src/ast/mutation.rs`

---

### Task 7: AST Node Definitions for Data Modifying Procedure Calls

**Description**: Define AST types for data modifying procedure calls.

**Deliverables**:
- `CallDataModifyingProcedureStatement` struct:
  - `procedure_call: ProcedureCall` - procedure to call (from Sprint 11 or placeholder)
  - `span: Span`
- Note: Full procedure call support will be implemented in Sprint 11; this is a placeholder to maintain grammar coverage.

**Grammar References**:
- `callDataModifyingProcedureStatement` (Line 492)

**Acceptance Criteria**:
- [ ] Data modifying procedure call AST defined
- [ ] Placeholder integration with procedure calls
- [ ] Span tracking
- [ ] Documentation notes Sprint 11 dependency

**File Location**: `src/ast/mutation.rs`

---

### Task 8: Lexer Extensions for Data Modification Tokens

**Description**: Ensure lexer supports all tokens needed for data modification parsing.

**Deliverables**:
- Verify existing keywords are sufficient:
  - Data modification: INSERT, SET, REMOVE, DELETE, DETACH, NODETACH
  - Edge directions in INSERT: already covered by Sprint 8
  - Label syntax: IS keyword (for label operations)
- Ensure operators work:
  - `.` (property reference)
  - `=` (assignment in SET)
  - `:` (label syntax)
- No new lexer tokens should be needed (all covered by previous sprints)

**Lexer Enhancements Needed** (if any):
- Verify INSERT, SET, REMOVE, DELETE, DETACH, NODETACH keywords exist
- Ensure IS keyword exists (for SET/REMOVE label syntax)
- All other tokens already covered by Sprints 2, 5, and 8

**Grammar References**:
- Data modification keyword definitions throughout Lines 421-494, 852-894

**Acceptance Criteria**:
- [ ] All data modification keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] No new lexer errors introduced
- [ ] All data modification tokens have proper span information

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 9: Mutation Parser - INSERT Statements

**Description**: Implement parsing for INSERT statements.

**Deliverables**:
- `parse_insert_statement()` - INSERT <insert_graph_pattern>
- `parse_insert_graph_pattern()` - parse insert graph pattern
- `parse_insert_path_pattern()` - parse insert path pattern
- `parse_insert_element_pattern()` - parse insert node or edge
- `parse_insert_node_pattern()` - parse insert node pattern
- `parse_insert_edge_pattern()` - parse insert edge pattern
- `parse_insert_edge_pointing_left()` - `<-[edge]-`
- `parse_insert_edge_pointing_right()` - `-[edge]->`
- `parse_insert_edge_undirected()` - `~[edge]~`
- `parse_insert_element_pattern_filler()` - parse variables, labels, properties
- Integration with label expressions from Sprint 8
- Integration with property specifications from Sprint 8
- Integration with expression parser from Sprint 5 for property values

**Grammar References**:
- `insertStatement` (Line 421)
- `insertGraphPattern` (Line 852)
- `insertPathPattern` (Line 860)
- `insertNodePattern` (Line 864)
- `insertEdgePattern` (Line 868)
- `insertEdgePointingLeft` (Line 872)
- `insertEdgePointingRight` (Line 874)
- `insertEdgeUndirected` (Line 876)
- `insertElementPatternFiller` (Line 886)

**Acceptance Criteria**:
- [ ] INSERT statements parse correctly
- [ ] Insert graph patterns support multiple path patterns
- [ ] Insert path patterns support sequential elements (nodes and edges)
- [ ] Insert node patterns work with variables, labels, and properties
- [ ] Insert edge patterns work with all 3 direction types
- [ ] Label expressions integrated from Sprint 8
- [ ] Property specifications use expressions from Sprint 5
- [ ] Error recovery at element boundaries
- [ ] Unit tests for all INSERT variants

**File Location**: `src/parser/mutation.rs` (new file)

---

### Task 10: Mutation Parser - SET Statements

**Description**: Implement parsing for SET statements.

**Deliverables**:
- `parse_set_statement()` - SET <set_item_list>
- `parse_set_item_list()` - parse comma-separated set items
- `parse_set_item()` - dispatch to set item types
- `parse_set_property_item()` - element.property = value
- `parse_set_all_properties_item()` - element = {properties}
- `parse_set_label_item()` - element :label or element IS label
- Integration with expression parser from Sprint 5 for property values
- Integration with label expressions from Sprint 8

**Grammar References**:
- `setStatement` (Line 427)
- `setItemList` (Line 431)
- `setItem` (Line 435)
- `setPropertyItem` (Line 441)
- `setAllPropertiesItem` (Line 445)
- `setLabelItem` (Line 449)

**Acceptance Criteria**:
- [ ] SET statements parse correctly
- [ ] Set item lists support multiple comma-separated items
- [ ] Set property items parse (element.property = value)
- [ ] Set all properties items parse (element = {properties})
- [ ] Set label items parse with both colon and IS keyword syntax
- [ ] Property values use expression parser from Sprint 5
- [ ] Label expressions integrated from Sprint 8
- [ ] Error recovery at item boundaries
- [ ] Unit tests for all SET variants

**File Location**: `src/parser/mutation.rs`

---

### Task 11: Mutation Parser - REMOVE Statements

**Description**: Implement parsing for REMOVE statements.

**Deliverables**:
- `parse_remove_statement()` - REMOVE <remove_item_list>
- `parse_remove_item_list()` - parse comma-separated remove items
- `parse_remove_item()` - dispatch to remove item types
- `parse_remove_property_item()` - element.property
- `parse_remove_label_item()` - element :label or element IS label
- Integration with label expressions from Sprint 8

**Grammar References**:
- `removeStatement` (Line 455)
- `removeItemList` (Line 459)
- `removeItem` (Line 463)
- `removePropertyItem` (Line 468)
- `removeLabelItem` (Line 472)

**Acceptance Criteria**:
- [ ] REMOVE statements parse correctly
- [ ] Remove item lists support multiple comma-separated items
- [ ] Remove property items parse (element.property)
- [ ] Remove label items parse with both colon and IS keyword syntax
- [ ] Label expressions integrated from Sprint 8
- [ ] Error recovery at item boundaries
- [ ] Unit tests for all REMOVE variants

**File Location**: `src/parser/mutation.rs`

---

### Task 12: Mutation Parser - DELETE Statements

**Description**: Implement parsing for DELETE statements with DETACH/NODETACH options.

**Deliverables**:
- `parse_delete_statement()` - [DETACH | NODETACH] DELETE <delete_item_list>
- `parse_detach_option()` - parse optional DETACH/NODETACH keyword
- `parse_delete_item_list()` - parse comma-separated delete items
- `parse_delete_item()` - parse individual delete item

**Grammar References**:
- `deleteStatement` (Line 478)
- `deleteItemList` (Line 482)
- `deleteItem` (Line 486)

**Acceptance Criteria**:
- [ ] DELETE statements parse correctly
- [ ] DETACH option parses (DETACH DELETE)
- [ ] NODETACH option parses (NODETACH DELETE)
- [ ] Default behavior (no keyword) handled correctly
- [ ] Delete item lists support multiple comma-separated items
- [ ] Error recovery at item boundaries
- [ ] Unit tests for all DELETE variants

**File Location**: `src/parser/mutation.rs`

---

### Task 13: Mutation Parser - Linear Data Modifying Statements

**Description**: Implement parsing for linear data modifying statement structure.

**Deliverables**:
- `parse_linear_data_modifying_statement()` - dispatch to focused or ambient
- `parse_focused_linear_data_modifying_statement()` - USE GRAPH ... <statements>
- `parse_ambient_linear_data_modifying_statement()` - <statements> (no USE GRAPH)
- `parse_simple_linear_data_modifying_statement()` - sequential statements
- `parse_simple_data_modifying_statement()` - individual statement wrapper
- `parse_simple_primitive_data_modifying_statement()` - primitive statement wrapper
- Integration with USE GRAPH clause from Sprint 7
- Integration with RETURN statement from Sprint 9

**Grammar References**:
- `focusedLinearDataModifyingStatement` (Line 376)
- `ambientLinearDataModifyingStatement` (Line 389)
- `simpleLinearDataModifyingStatement` (Line 394)
- `simpleDataModifyingStatement` (Line 401)
- `simplePrimitiveDataModifyingStatement` (Line 408)

**Acceptance Criteria**:
- [ ] Linear data modifying statements parse correctly
- [ ] Focused statements (with USE GRAPH) work
- [ ] Ambient statements (without USE GRAPH) work
- [ ] Sequential chaining of modification statements works
- [ ] Optional RETURN clause at end works (from Sprint 9)
- [ ] Error recovery at statement boundaries
- [ ] Unit tests for linear statement variants

**File Location**: `src/parser/mutation.rs`

---

### Task 14: Integration with Sprint 5 (Expression Parser)

**Description**: Integrate mutation parser with expression parser from Sprint 5.

**Deliverables**:
- Use expression parser for:
  - Property values in SET statements
  - Property values in INSERT patterns
  - Record expressions in SET all properties
- Ensure no parser conflicts between mutation and expression parsing
- Test expressions in all mutation contexts

**Acceptance Criteria**:
- [ ] All mutation parsers use expression parser correctly
- [ ] No parser conflicts between mutation and expression parsing
- [ ] Expressions work in all mutation contexts
- [ ] Integration tests validate end-to-end parsing
- [ ] Expression parsing is context-aware

**File Location**: `src/parser/mutation.rs`, `src/parser/expression.rs`

---

### Task 15: Integration with Sprint 8 (Pattern Parser)

**Description**: Integrate mutation parser with pattern parser from Sprint 8.

**Deliverables**:
- Reuse pattern components in INSERT:
  - Element variable declarations
  - Label expressions
  - Property specifications
- Ensure consistent syntax between query patterns and insert patterns
- Test INSERT patterns with all pattern features

**Acceptance Criteria**:
- [ ] All INSERT parsers use pattern components from Sprint 8
- [ ] No parser conflicts between mutation and pattern parsing
- [ ] INSERT patterns work with label expressions from Sprint 8
- [ ] INSERT patterns work with property specifications from Sprint 8
- [ ] Integration tests validate end-to-end parsing
- [ ] Pattern parsing is context-aware

**File Location**: `src/parser/mutation.rs`, `src/parser/query.rs`

---

### Task 16: Integration with Sprint 7 (Query Parser)

**Description**: Integrate mutation parser with query parser from Sprint 7.

**Deliverables**:
- Update program parser to include data modification statements
- Support USE GRAPH clause in focused data modification
- Test data modification in transaction contexts
- Ensure query and mutation statements can be mixed (MATCH ... INSERT ...)

**Acceptance Criteria**:
- [ ] Data modification statements integrated into program parser
- [ ] USE GRAPH clause works in focused data modification
- [ ] Data modification statements work in transaction contexts
- [ ] Query and mutation statements can be chained
- [ ] No regressions in existing query tests
- [ ] Integration tests validate end-to-end data modification

**File Location**: `src/parser/mutation.rs`, `src/parser/program.rs`

---

### Task 17: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for data modification parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at statement boundaries (INSERT, SET, REMOVE, DELETE)
  - Recover at comma separators (in item lists, path patterns)
  - Recover at closing delimiters (], }, ))
  - Partial AST construction on errors
- Diagnostic messages:
  - "Expected element pattern after comma in INSERT"
  - "Expected property name after dot in SET"
  - "Expected = after property name in SET"
  - "Invalid edge direction in INSERT pattern"
  - "DETACH and NODETACH are mutually exclusive"
  - "Expected variable reference in DELETE"
  - "SET requires property value expression"
  - "REMOVE requires property or label to remove"
- Span highlighting for error locations
- Helpful error messages with suggestions:
  - "Did you mean SET element.property = value?"
  - "INSERT patterns require variables for newly created elements"
  - "DETACH DELETE removes node and all connected edges"
  - "Use IS keyword for label operations: element IS Label"

**Grammar References**:
- All data modification rules (Lines 369-494, 852-894)

**Acceptance Criteria**:
- [ ] Mutation parser recovers from common errors
- [ ] Multiple errors in one statement reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Suggestions provided for common mutation syntax errors
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/mutation.rs`, `src/diag.rs`

---

### Task 18: Comprehensive Testing

**Description**: Implement comprehensive test suite for data modification parsing.

**Deliverables**:

#### Unit Tests (`src/parser/mutation.rs`):
- **INSERT Statement Tests**:
  - Insert single node
  - Insert node with labels
  - Insert node with properties
  - Insert node with labels and properties
  - Insert edge pointing left
  - Insert edge pointing right
  - Insert undirected edge
  - Insert path pattern (multiple elements)
  - Insert multiple paths (comma-separated)
  - Insert with variable declarations

- **SET Statement Tests**:
  - Set single property
  - Set multiple properties (comma-separated)
  - Set all properties with record expression
  - Set label with colon syntax
  - Set label with IS keyword
  - Set multiple items mixed (properties and labels)

- **REMOVE Statement Tests**:
  - Remove single property
  - Remove multiple properties (comma-separated)
  - Remove label with colon syntax
  - Remove label with IS keyword
  - Remove multiple items mixed (properties and labels)

- **DELETE Statement Tests**:
  - DELETE without DETACH/NODETACH (default)
  - DETACH DELETE
  - NODETACH DELETE
  - Delete single element
  - Delete multiple elements (comma-separated)

- **Linear Statement Tests**:
  - Focused data modification (with USE GRAPH)
  - Ambient data modification (without USE GRAPH)
  - Sequential statements (MATCH ... INSERT ...)
  - Data modification with RETURN

- **Error Recovery Tests**:
  - Missing property value in SET
  - Missing element variable in DELETE
  - Invalid edge direction in INSERT
  - Malformed property specification
  - Invalid label syntax

#### Integration Tests (`tests/mutation_tests.rs` - new file):
- Complete MATCH ... INSERT ... RETURN queries
- Complex INSERT patterns with expressions from Sprint 5
- SET statements with complex expressions
- REMOVE statements with label expressions from Sprint 8
- DELETE statements in transaction contexts
- Edge cases (deeply nested, complex patterns)

#### Snapshot Tests:
- Capture AST output for representative mutation statements
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for mutation parser
- [ ] All mutation variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (complex patterns, all clauses combined)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/mutation.rs`, `tests/mutation_tests.rs`

---

### Task 19: Documentation and Examples

**Description**: Document data modification system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all mutation AST node types
  - Module-level documentation for data modification
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase data modification
  - Add `examples/mutation_demo.rs` demonstrating:
    - Simple INSERT statements
    - Complex INSERT patterns with labels and properties
    - SET statements for property updates
    - SET statements for label additions
    - REMOVE statements for property deletion
    - REMOVE statements for label removal
    - DELETE statements with DETACH/NODETACH
    - Complete MATCH ... INSERT ... RETURN queries

- **Data Modification Overview Documentation**:
  - Document INSERT semantics
  - Document SET semantics (property vs all properties vs label)
  - Document REMOVE semantics
  - Document DELETE semantics (DETACH vs NODETACH)
  - Document pattern syntax in INSERT
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for data modification
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Data modification overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all mutation error codes
- [ ] Documentation explains data modification semantics clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/mutation.rs`, `src/parser/mutation.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Statement Ordering**: Data modification statements have specific ordering:
   - MATCH clauses typically come before INSERT/SET/REMOVE/DELETE
   - DELETE typically comes after other modifications
   - RETURN clause comes last (if present)
   - Parser should enforce or document ordering rules

2. **Pattern Reuse**: INSERT patterns reuse concepts from Sprint 8:
   - Element variable declarations
   - Label expressions
   - Property specifications
   - But INSERT patterns have restrictions (only 3 edge directions, no quantifiers)

3. **Expression Integration**: Data modification heavily uses expressions:
   - Property values in SET are expressions (from Sprint 5)
   - Property values in INSERT are expressions (from Sprint 5)
   - Record expressions for SET all properties (from Sprint 5)
   - Must use expression parser consistently throughout

4. **Variable Scoping**: Data modification introduces variables:
   - INSERT creates new element variables
   - These variables can be used in subsequent statements
   - SET/REMOVE/DELETE reference existing variables from MATCH or INSERT
   - Parser should track variable context for semantic validation (Sprint 14)

5. **DETACH Semantics**: DELETE has special behavior:
   - DETACH DELETE removes node and all connected edges
   - NODETACH DELETE only removes node if no edges remain
   - Default (no keyword) behaves like NODETACH
   - Parser must distinguish these cases clearly

6. **Error Recovery**: Data modification clauses have clear boundaries:
   - Recover at statement keywords (INSERT, SET, REMOVE, DELETE)
   - Recover at comma separators in lists
   - Continue parsing after errors to report multiple issues

### AST Design Considerations

1. **Span Tracking**: Every mutation node must track its source span for diagnostic purposes.

2. **Optional Fields**: Many mutation components are optional:
   - Variables in INSERT patterns
   - Labels in INSERT patterns
   - Properties in INSERT patterns
   - DETACH option in DELETE
   - Use `Option<T>` appropriately

3. **Expression Reuse**: Use expression AST from Sprint 5:
   - Property values are expressions
   - Record expressions are expressions
   - Don't duplicate expression types

4. **Pattern Component Reuse**: Reuse pattern components from Sprint 8:
   - Element variable declarations
   - Label expressions
   - Property specifications
   - Don't duplicate pattern types

5. **List Types**: Use `Vec<T>` for:
   - Insert path pattern lists
   - Set item lists
   - Remove item lists
   - Delete item lists
   - Clear comma-separated list parsing

6. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Property names
   - Short identifiers

### Error Recovery Strategy

1. **Synchronization Points**:
   - Statement keywords (INSERT, SET, REMOVE, DELETE)
   - Comma separators in lists
   - Closing delimiters (], }, ))
   - End of statement (semicolon or next major clause)

2. **Statement Boundary Recovery**: If statement malformed:
   - Report error at statement location
   - Skip to next statement keyword
   - Continue parsing remaining statements
   - Construct partial AST

3. **List Recovery**: If item in list malformed:
   - Report error at item location
   - Skip to next comma or end of list
   - Continue with next item
   - Include valid items in AST

4. **Expression Recovery**: If expression malformed:
   - Use expression parser's error recovery from Sprint 5
   - Return error placeholder expression
   - Continue parsing

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error in mutation statement"
   - Good: "Expected property value after = in SET statement, found DELETE"

2. **Helpful Suggestions**:
   - "Did you mean SET element.property = value?"
   - "INSERT patterns require variables for newly created elements"
   - "DETACH DELETE removes node and all connected edges"
   - "Use IS keyword for label operations: element IS Label"
   - "Property name must be a valid identifier"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing clauses, point to where clause expected
   - For malformed items, highlight entire item
   - For invalid keywords, highlight keyword token

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing INSERT statement..."
   - "In SET statement starting at line 42..."
   - "While parsing delete item list..."

### Performance Considerations

1. **Mutation Statement Parsing Efficiency**: Mutation statements are common:
   - Use efficient lookahead (1-2 tokens typically sufficient)
   - Minimize backtracking
   - Use direct dispatch to statement parsers

2. **List Parsing**: Use efficient comma-separated list parsing:
   - Single-pass parsing
   - Clear termination conditions
   - Avoid unnecessary allocations

3. **Expression Reuse**: Reuse expression parser from Sprint 5:
   - Don't duplicate expression parsing logic
   - Leverage existing expression performance

4. **Pattern Component Reuse**: Reuse pattern parser from Sprint 8:
   - Don't duplicate pattern parsing logic
   - Leverage existing pattern performance

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (mutation keywords, operators)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure; integration testing infrastructure
- **Sprint 5**: Expression parsing for property values, record expressions
- **Sprint 6**: Type system (for future semantic validation)
- **Sprint 7**: Query pipeline structure; USE GRAPH clause
- **Sprint 8**: Pattern parsing for INSERT patterns; label expressions; property specifications
- **Sprint 9**: RETURN statement for returning modified elements

### Dependencies on Future Sprints

- **Sprint 11**: Data modifying procedure calls will be fully implemented
- **Sprint 12**: Graph type specifications (not directly related)
- **Sprint 13**: Conformance hardening (stress testing data modification)
- **Sprint 14**: Semantic validation (variable scoping, type checking, mutation consistency)

### Cross-Sprint Integration Points

- Data modification is a core GQL feature for graph mutations
- INSERT patterns reuse concepts from Sprint 8 (pattern matching)
- SET/REMOVE use expressions from Sprint 5 for property values
- Data modification can be chained with queries (MATCH ... INSERT ...)
- RETURN statements (Sprint 9) can return modified elements
- Semantic validation (Sprint 14) will check:
  - Variable scoping (INSERT creates, SET/REMOVE/DELETE reference)
  - Type checking (property values, label expressions)
  - Mutation consistency (DELETE with DETACH requirements)
  - Expression validity in different contexts

## Test Strategy

### Unit Tests

For each mutation component:
1. **Happy Path**: Valid statements parse correctly
2. **Variants**: All syntax variants and optional components
3. **Error Cases**: Missing components, invalid syntax, malformed statements
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Data modification in different contexts:
1. **Query Integration**: MATCH ... INSERT ... RETURN queries
2. **Statement Chaining**: Sequential modification statements
3. **Complex Patterns**: INSERT with complex patterns from Sprint 8
4. **Expression Integration**: Mutation statements with complex expressions from Sprint 5
5. **Complete Transactions**: End-to-end transaction with data modification

### Snapshot Tests

Capture AST output:
1. Representative mutation statements from each category
2. Complex patterns with all modification types
3. Chained queries with data modification
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid mutation statements
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries:
1. Official GQL sample queries with data modification
2. Real-world graph mutation queries
3. Verify parser handles production syntax

### Performance Tests

1. **Long Item Lists**: Many set/remove/delete items
2. **Complex INSERT Patterns**: Large path patterns
3. **Chained Mutations**: Many sequential modification statements

## Performance Considerations

1. **Lexer Efficiency**: Mutation keywords are frequent; lexer must be fast
2. **Parser Efficiency**: Use direct dispatch and minimal lookahead
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Expression Reuse**: Leverage Sprint 5 expression parser performance
5. **Pattern Reuse**: Leverage Sprint 8 pattern parser performance

## Documentation Requirements

1. **API Documentation**: Rustdoc for all mutation AST nodes and parser functions
2. **Data Modification Overview**: Document mutation semantics and execution order
3. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
4. **Examples**: Demonstrate data modification in examples
5. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| INSERT pattern complexity causes parser confusion | Medium | Medium | Clear AST design; reuse Sprint 8 components; extensive testing |
| SET/REMOVE/DELETE variable scoping issues | Medium | Low | Track variable context; defer full validation to Sprint 14 |
| DETACH semantics ambiguity | Low | Low | Clear AST design; good error messages; documentation |
| Expression parser integration issues | Low | Low | Sprint 5 expression parser is stable; use consistently throughout |
| Pattern parser integration issues | Low | Low | Sprint 8 pattern parser is stable; reuse components carefully |
| Performance on complex mutations | Low | Low | Optimize hot paths; use efficient algorithms; profile and optimize if needed |

## Success Metrics

1. **Coverage**: All data modification statements parse with correct AST
2. **Correctness**: Mutation semantics match ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for mutation parser
6. **Performance**: Parser handles statements with 50+ items in <1ms
7. **Integration**: Data modification integrates cleanly with Sprint 5 (expressions), Sprint 7 (queries), Sprint 8 (patterns), and Sprint 9 (return)
8. **Completeness**: All mutation statement types work; statements can be chained

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping, mutation overview)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Data modification tested in multiple contexts (INSERT, SET, REMOVE, DELETE)
- [ ] AST design reviewed for stability and extensibility
- [ ] Sprint 5 integration complete (expressions in mutations)
- [ ] Sprint 7 integration complete (USE GRAPH in focused mutations)
- [ ] Sprint 8 integration complete (patterns in INSERT)
- [ ] Sprint 9 integration complete (RETURN in mutations)
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 11: Procedures, Nested Specs, and Execution Flow** will build on the query and mutation foundations to implement procedural composition features including inline/named calls, variable scope clause, procedure args, optional call, YIELD, nested procedure specs, NEXT chaining, and AT/USE context clauses. With querying, result shaping, and data modification complete, Sprint 11 adds powerful procedural composition capabilities.

---

## Appendix: Data Modification Statement Hierarchy

```
LinearDataModifyingStatement
├── Focused (with USE GRAPH)
│   └── FocusedLinearDataModifyingStatement
│       ├── use_graph_clause: UseGraphClause (from Sprint 7)
│       ├── primitives: Vec<PrimitiveDataModifyingStatement>
│       └── primitive_result_statement: Option<PrimitiveResultStatement> (from Sprint 9)
└── Ambient (without USE GRAPH)
    └── AmbientLinearDataModifyingStatement
        ├── primitives: Vec<SimplePrimitiveDataModifyingStatement>
        └── primitive_result_statement: Option<PrimitiveResultStatement> (from Sprint 9)

PrimitiveDataModifyingStatement
├── Insert(InsertStatement)
│   └── InsertStatement
│       └── pattern: InsertGraphPattern
│           └── paths: Vec<InsertPathPattern>
│               └── elements: Vec<InsertElementPattern>
│                   ├── Node(InsertNodePattern)
│                   │   └── filler: InsertElementPatternFiller
│                   │       ├── variable: Option<ElementVariableDeclaration> (from Sprint 8)
│                   │       ├── label_expression: Option<LabelExpression> (from Sprint 8)
│                   │       └── properties: Option<ElementPropertySpecification> (from Sprint 8)
│                   └── Edge(InsertEdgePattern)
│                       ├── PointingLeft
│                       ├── PointingRight
│                       └── Undirected
├── Set(SetStatement)
│   └── SetStatement
│       └── items: SetItemList
│           └── items: Vec<SetItem>
│               ├── Property(SetPropertyItem)
│               │   ├── element: BindingVariableReference (from Sprint 5)
│               │   ├── property: PropertyName
│               │   └── value: Expression (from Sprint 5)
│               ├── AllProperties(SetAllPropertiesItem)
│               │   ├── element: BindingVariableReference (from Sprint 5)
│               │   └── properties: Expression (record) (from Sprint 5)
│               └── Label(SetLabelItem)
│                   ├── element: BindingVariableReference (from Sprint 5)
│                   ├── label: LabelExpression (from Sprint 8)
│                   └── use_is_keyword: bool
├── Remove(RemoveStatement)
│   └── RemoveStatement
│       └── items: RemoveItemList
│           └── items: Vec<RemoveItem>
│               ├── Property(RemovePropertyItem)
│               │   ├── element: BindingVariableReference (from Sprint 5)
│               │   └── property: PropertyName
│               └── Label(RemoveLabelItem)
│                   ├── element: BindingVariableReference (from Sprint 5)
│                   ├── label: LabelExpression (from Sprint 8)
│                   └── use_is_keyword: bool
└── Delete(DeleteStatement)
    └── DeleteStatement
        ├── detach_option: DetachOption
        │   ├── Detach (DETACH DELETE)
        │   ├── NoDetach (NODETACH DELETE)
        │   └── Default (no keyword)
        └── items: DeleteItemList
            └── items: Vec<DeleteItem>
                └── element: BindingVariableReference (from Sprint 5)
```

---

## Appendix: Data Modification Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `focusedLinearDataModifyingStatement` | 376 | `FocusedLinearDataModifyingStatement` struct | `parse_focused_linear_data_modifying_statement()` |
| `ambientLinearDataModifyingStatement` | 389 | `AmbientLinearDataModifyingStatement` struct | `parse_ambient_linear_data_modifying_statement()` |
| `simpleLinearDataModifyingStatement` | 394 | -- | `parse_simple_linear_data_modifying_statement()` |
| `simpleDataModifyingStatement` | 401 | -- | `parse_simple_data_modifying_statement()` |
| `simplePrimitiveDataModifyingStatement` | 408 | `SimplePrimitiveDataModifyingStatement` struct | `parse_simple_primitive_data_modifying_statement()` |
| `primitiveDataModifyingStatement` | 412 | `PrimitiveDataModifyingStatement` enum | `parse_primitive_data_modifying_statement()` |
| `insertStatement` | 421 | `InsertStatement` struct | `parse_insert_statement()` |
| `setStatement` | 427 | `SetStatement` struct | `parse_set_statement()` |
| `setItemList` | 431 | `SetItemList` struct | `parse_set_item_list()` |
| `setItem` | 435 | `SetItem` enum | `parse_set_item()` |
| `setPropertyItem` | 441 | `SetPropertyItem` struct | `parse_set_property_item()` |
| `setAllPropertiesItem` | 445 | `SetAllPropertiesItem` struct | `parse_set_all_properties_item()` |
| `setLabelItem` | 449 | `SetLabelItem` struct | `parse_set_label_item()` |
| `removeStatement` | 455 | `RemoveStatement` struct | `parse_remove_statement()` |
| `removeItemList` | 459 | `RemoveItemList` struct | `parse_remove_item_list()` |
| `removeItem` | 463 | `RemoveItem` enum | `parse_remove_item()` |
| `removePropertyItem` | 468 | `RemovePropertyItem` struct | `parse_remove_property_item()` |
| `removeLabelItem` | 472 | `RemoveLabelItem` struct | `parse_remove_label_item()` |
| `deleteStatement` | 478 | `DeleteStatement` struct | `parse_delete_statement()` |
| `deleteItemList` | 482 | `DeleteItemList` struct | `parse_delete_item_list()` |
| `deleteItem` | 486 | `DeleteItem` struct | `parse_delete_item()` |
| `callDataModifyingProcedureStatement` | 492 | `CallDataModifyingProcedureStatement` struct | `parse_call_data_modifying_procedure_statement()` |
| `insertGraphPattern` | 852 | `InsertGraphPattern` struct | `parse_insert_graph_pattern()` |
| `insertPathPattern` | 860 | `InsertPathPattern` struct | `parse_insert_path_pattern()` |
| `insertNodePattern` | 864 | `InsertNodePattern` struct | `parse_insert_node_pattern()` |
| `insertEdgePattern` | 868 | `InsertEdgePattern` enum | `parse_insert_edge_pattern()` |
| `insertEdgePointingLeft` | 872 | `InsertEdgePointingLeft` struct | `parse_insert_edge_pointing_left()` |
| `insertEdgePointingRight` | 874 | `InsertEdgePointingRight` struct | `parse_insert_edge_pointing_right()` |
| `insertEdgeUndirected` | 876 | `InsertEdgeUndirected` struct | `parse_insert_edge_undirected()` |
| `insertElementPatternFiller` | 886 | `InsertElementPatternFiller` struct | `parse_insert_element_pattern_filler()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-18
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4, 5, 6, 7, 8, 9 (completed or required)
**Next Sprint**: Sprint 11 (Procedures, Nested Specs, and Execution Flow)
