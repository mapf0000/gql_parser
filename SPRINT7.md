# Sprint 7: Query Pipeline Core

## Sprint Overview

**Sprint Goal**: Implement linear/composite query composition and clause chaining.

**Sprint Duration**: TBD

**Status**: 🔵 **Planned**

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅
- Sprint 5 (Values, Literals, and Expression Core) ✅
- Sprint 6 (Type System and Reference Forms) ✅

## Scope

This sprint implements the query pipeline infrastructure that forms the backbone of GQL's compositional query model. The query pipeline enables building complex queries through composition of primitive operations (MATCH, FILTER, LET, FOR, SELECT) using set operators (UNION, EXCEPT, INTERSECT, OTHERWISE) and sequential chaining. This sprint establishes the foundation for data retrieval in GQL, with graph pattern matching details deferred to Sprint 8.

### Feature Coverage from GQL_FEATURES.md

Sprint 7 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 5: Query Features** (Lines 167-293)
   - Composite query operations
   - Set operators (UNION, EXCEPT, INTERSECT, OTHERWISE)
   - Linear query statements (focused and ambient)
   - Simple linear query statements
   - Primitive query statements
   - Match statements (structure only; pattern details in Sprint 8)
   - Filter statements
   - Let statements
   - For statements
   - Select statements

## Exit Criteria

- [ ] All query statement types parse with correct AST forms
- [ ] Composite query operations (set operators, OTHERWISE) work correctly
- [ ] Linear query pipeline (sequential clause chaining) parses correctly
- [ ] Focused queries (with USE GRAPH clause) parse correctly
- [ ] Ambient queries (without USE GRAPH clause) parse correctly
- [ ] MATCH statement structure parses (pattern parsing deferred to Sprint 8)
- [ ] FILTER statements parse with expression predicates from Sprint 5
- [ ] LET statements parse with variable definitions and expressions
- [ ] FOR statements parse with iteration constructs
- [ ] SELECT statements parse with SQL-style syntax
- [ ] Query nesting and subqueries work correctly
- [ ] USE GRAPH clause integrates with queries
- [ ] Parser produces structured diagnostics for malformed queries
- [ ] AST nodes have proper span information for all components
- [ ] Recovery mechanisms handle errors at clause boundaries
- [ ] Unit tests cover all query statement variants and error cases
- [ ] Query parsing integrates with expression parsing from Sprint 5
- [ ] Query parsing integrates with type system from Sprint 6

## Implementation Tasks

### Task 1: AST Node Definitions for Composite Queries

**Description**: Define AST types for composite query operations and set operators.

**Deliverables**:
- `CompositeQuery` struct:
  - `left: Box<Query>` - first query
  - `operator: SetOperator` - set operation
  - `right: Box<Query>` - second query
  - `span: Span`
- `Query` enum with variants:
  - `Linear(LinearQuery)` - linear query statement
  - `Composite(CompositeQuery)` - composite query with set operators
  - `Parenthesized(Box<Query>)` - parenthesized query
- `SetOperator` enum:
  - `Union { quantifier: SetQuantifier }` - UNION [ALL | DISTINCT]
  - `Except { quantifier: SetQuantifier }` - EXCEPT [ALL | DISTINCT]
  - `Intersect { quantifier: SetQuantifier }` - INTERSECT [ALL | DISTINCT]
  - `Otherwise` - OTHERWISE operator
- `SetQuantifier` enum:
  - `All` - ALL (include duplicates)
  - `Distinct` - DISTINCT (remove duplicates, default)

**Grammar References**:
- `compositeQueryStatement` (Line 498)
- `queryConjunction` (Line 509)
- `setOperator` (Line 514)

**Acceptance Criteria**:
- [ ] All composite query AST types defined in `src/ast/query.rs` (new module)
- [ ] Each node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each variant
- [ ] Set operators properly capture quantifiers
- [ ] OTHERWISE operator distinguished from set operators
- [ ] Query nesting supported through recursive structure

**File Location**: `src/ast/query.rs` (new file)

---

### Task 2: AST Node Definitions for Linear Queries

**Description**: Define AST types for linear query statements and primitive query operations.

**Deliverables**:
- `LinearQuery` struct:
  - `primitive_statements: Vec<PrimitiveQueryStatement>` - sequential operations
  - `result_statement: Option<Box<PrimitiveResultStatement>>` - optional RETURN/FINISH
  - `span: Span`
- `FocusedLinearQuery` struct:
  - `use_graph: UseGraphClause` - USE GRAPH clause
  - `primitive_statements: Vec<PrimitiveQueryStatement>` - query operations
  - `result_statement: Option<Box<PrimitiveResultStatement>>` - optional RETURN/FINISH
  - `span: Span`
- `AmbientLinearQuery` struct:
  - `primitive_statements: Vec<PrimitiveQueryStatement>` - query operations
  - `result_statement: Option<Box<PrimitiveResultStatement>>` - optional RETURN/FINISH
  - `span: Span`
- `PrimitiveQueryStatement` enum with variants:
  - `Match(MatchStatement)` - MATCH statement
  - `Filter(FilterStatement)` - FILTER statement
  - `Let(LetStatement)` - LET statement
  - `For(ForStatement)` - FOR statement
  - `OrderByAndPage(OrderByAndPageStatement)` - ORDER BY and pagination
  - `Select(SelectStatement)` - SELECT statement
- `PrimitiveResultStatement` enum:
  - `Return(ReturnStatement)` - RETURN statement
  - `Finish(FinishStatement)` - FINISH statement (placeholder)

**Grammar References**:
- `linearQueryStatement` (Line 526)
- `focusedLinearQueryStatement` (Line 531)
- `ambientLinearQueryStatement` (Line 554)
- `simpleLinearQueryStatement` (Line 559)
- `primitiveQueryStatement` (Line 568)
- `primitiveResultStatement` (Line 660)

**Acceptance Criteria**:
- [ ] All linear query AST types defined
- [ ] Sequential statement chaining supported through Vec
- [ ] Focused vs ambient queries distinguished
- [ ] Primitive statement types enumerated
- [ ] Result statement optional (queries can have no explicit RETURN)
- [ ] Span tracking covers entire query extent
- [ ] Documentation explains query pipeline semantics

**File Location**: `src/ast/query.rs`

---

### Task 3: AST Node Definitions for Match Statements

**Description**: Define AST types for MATCH statement structure (pattern details in Sprint 8).

**Deliverables**:
- `MatchStatement` enum:
  - `Simple(SimpleMatchStatement)` - MATCH <pattern>
  - `Optional(OptionalMatchStatement)` - OPTIONAL MATCH <pattern>
- `SimpleMatchStatement` struct:
  - `pattern: GraphPattern` - graph pattern (placeholder for Sprint 8)
  - `span: Span`
- `OptionalMatchStatement` struct:
  - `operand: OptionalOperand` - what to optionally match
  - `span: Span`
- `OptionalOperand` enum:
  - `Match { pattern: GraphPattern }` - OPTIONAL MATCH <pattern>
  - `Block { statements: Vec<MatchStatement> }` - OPTIONAL { <match_block> }
  - `ParenthesizedBlock { statements: Vec<MatchStatement> }` - OPTIONAL ( <match_block> )
- `GraphPattern` struct (placeholder):
  - Detailed structure deferred to Sprint 8
  - Basic fields for integration:
    - `span: Span`
    - Future fields: match mode, path patterns, where clause, etc.

**Grammar References**:
- `simpleMatchStatement` (Line 583)
- `optionalMatchStatement` (Line 587)
- `optionalOperand` (Line 591)
- `matchStatementBlock` (Line 597)

**Acceptance Criteria**:
- [ ] Match statement structure defined
- [ ] Simple vs optional match distinguished
- [ ] Optional operand variants (match, block, parenthesized) supported
- [ ] GraphPattern placeholder defined for Sprint 8 integration
- [ ] Span information captures entire match statement
- [ ] Documentation notes Sprint 8 will complete pattern parsing
- [ ] Integration points for graph patterns clearly defined

**File Location**: `src/ast/query.rs`

---

### Task 4: AST Node Definitions for Filter Statements

**Description**: Define AST types for FILTER statements.

**Deliverables**:
- `FilterStatement` struct:
  - `where_optional: bool` - whether WHERE keyword present
  - `condition: Expression` - search condition (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `filterStatement` (Line 609)

**Acceptance Criteria**:
- [ ] Filter statement AST defined
- [ ] WHERE keyword optional (tracked in AST)
- [ ] Condition uses Expression from Sprint 5
- [ ] Span tracking covers entire filter statement
- [ ] Documentation explains filter semantics
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 5: AST Node Definitions for Let Statements

**Description**: Define AST types for LET statements and variable bindings.

**Deliverables**:
- `LetStatement` struct:
  - `bindings: Vec<LetVariableDefinition>` - variable definitions
  - `span: Span`
- `LetVariableDefinition` struct:
  - `variable: BindingVariable` - variable name
  - `type_annotation: Option<ValueType>` - optional type (from Sprint 6)
  - `value: Expression` - computed value (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `letStatement` (Line 615)
- `letVariableDefinition` (Line 623)

**Acceptance Criteria**:
- [ ] Let statement AST defined
- [ ] Multiple variable bindings supported
- [ ] Type annotations optional (from Sprint 6)
- [ ] Value expressions from Sprint 5
- [ ] Span tracking for each binding
- [ ] Documentation explains let binding semantics
- [ ] Integration with type system from Sprint 6
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 6: AST Node Definitions for For Statements

**Description**: Define AST types for FOR statements and iteration.

**Deliverables**:
- `ForStatement` struct:
  - `item: ForItem` - iteration specification
  - `ordinality_or_offset: Option<ForOrdinalityOrOffset>` - optional WITH ORDINALITY/OFFSET
  - `span: Span`
- `ForItem` struct:
  - `binding_variable: BindingVariable` - loop variable
  - `collection: Expression` - collection to iterate (from Sprint 5)
  - `span: Span`
- `ForOrdinalityOrOffset` enum:
  - `Ordinality { variable: BindingVariable }` - WITH ORDINALITY
  - `Offset { variable: BindingVariable }` - WITH OFFSET

**Grammar References**:
- `forStatement` (Line 630)
- `forItem` (Line 634)
- `forOrdinalityOrOffset` (Line 646)

**Acceptance Criteria**:
- [ ] For statement AST defined
- [ ] Mandatory alias binding form (variable IN collection)
- [ ] Ordinality and offset variants supported
- [ ] Collection expression from Sprint 5
- [ ] Span tracking covers entire for statement
- [ ] Documentation explains for iteration semantics
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 7: AST Node Definitions for Select Statements

**Description**: Define AST types for SELECT statements with SQL-style syntax.

**Deliverables**:
- `SelectStatement` struct:
  - `quantifier: Option<SetQuantifier>` - DISTINCT or ALL
  - `select_items: SelectItemList` - what to select
  - `from_clause: Option<SelectFromClause>` - FROM clause
  - `where_clause: Option<WhereClause>` - WHERE clause
  - `group_by: Option<GroupByClause>` - GROUP BY clause
  - `having: Option<HavingClause>` - HAVING clause
  - `order_by: Option<OrderByClause>` - ORDER BY clause
  - `offset: Option<OffsetClause>` - OFFSET clause
  - `limit: Option<LimitClause>` - LIMIT clause
  - `span: Span`
- `SelectItemList` enum:
  - `Star` - SELECT *
  - `Items { items: Vec<SelectItem> }` - SELECT item1, item2, ...
- `SelectItem` struct:
  - `expression: Expression` - expression to select (from Sprint 5)
  - `alias: Option<SmolStr>` - optional AS alias
  - `span: Span`
- `SelectFromClause` enum:
  - `GraphMatchList { matches: Vec<GraphPattern> }` - FROM graph matches
  - `QuerySpecification { query: Box<Query> }` - FROM nested query
  - `GraphAndQuerySpecification { graph: Expression, query: Box<Query> }` - FROM graph query
- `WhereClause` struct:
  - `condition: Expression` - filter condition (from Sprint 5)
  - `span: Span`
- `HavingClause` struct:
  - `condition: Expression` - filter condition on aggregates (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `selectStatement` (Line 689)
- `selectItem` (Line 697)
- `havingClause` (Line 705)
- `selectGraphMatchList` (Line 713)
- `selectQuerySpecification` (Line 721)

**Acceptance Criteria**:
- [ ] Select statement AST defined with all clauses
- [ ] SELECT * vs explicit select items distinguished
- [ ] All optional clauses properly represented
- [ ] FROM clause variants (graph matches, nested query, graph + query) supported
- [ ] Expressions from Sprint 5
- [ ] Span tracking covers entire select statement
- [ ] Documentation explains SQL-style select semantics
- [ ] Integration with expression parser from Sprint 5
- [ ] Integration with ORDER BY, GROUP BY, HAVING (shared with other statements)

**File Location**: `src/ast/query.rs`

---

### Task 8: AST Node Definitions for Return Statements

**Description**: Define AST types for RETURN statements.

**Deliverables**:
- `ReturnStatement` struct:
  - `quantifier: Option<SetQuantifier>` - DISTINCT or ALL
  - `items: ReturnItemList` - what to return
  - `group_by: Option<GroupByClause>` - optional GROUP BY
  - `span: Span`
- `ReturnItemList` enum:
  - `Star` - RETURN *
  - `Items { items: Vec<ReturnItem> }` - RETURN item1, item2, ...
- `ReturnItem` struct:
  - `expression: Expression` - expression to return (from Sprint 5)
  - `alias: Option<SmolStr>` - optional AS alias
  - `span: Span`

**Grammar References**:
- `returnStatement` (Line 667)
- `returnStatementBody` (Line 671)
- `returnItemList` (Line 675)
- `returnItem` (Line 679)
- `returnItemAlias` (Line 683)

**Acceptance Criteria**:
- [ ] Return statement AST defined
- [ ] RETURN * vs explicit return items distinguished
- [ ] Set quantifier (DISTINCT/ALL) optional
- [ ] Group by optional
- [ ] Expressions from Sprint 5
- [ ] Span tracking covers entire return statement
- [ ] Documentation explains return semantics
- [ ] Integration with expression parser from Sprint 5
- [ ] Integration with GROUP BY clause

**File Location**: `src/ast/query.rs`

---

### Task 9: AST Node Definitions for Ordering and Pagination

**Description**: Define AST types for ORDER BY, LIMIT, and OFFSET clauses.

**Deliverables**:
- `OrderByAndPageStatement` struct:
  - `order_by: Option<OrderByClause>` - ORDER BY clause
  - `offset: Option<OffsetClause>` - OFFSET clause
  - `limit: Option<LimitClause>` - LIMIT clause
  - `span: Span`
- `OrderByClause` struct:
  - `sort_specifications: Vec<SortSpecification>` - sort keys
  - `span: Span`
- `SortSpecification` struct:
  - `key: Expression` - expression to sort by (from Sprint 5)
  - `ordering: Option<OrderingSpecification>` - ASC or DESC
  - `null_ordering: Option<NullOrdering>` - NULLS FIRST or NULLS LAST
  - `span: Span`
- `OrderingSpecification` enum:
  - `Ascending` - ASC or ASCENDING
  - `Descending` - DESC or DESCENDING
- `NullOrdering` enum:
  - `NullsFirst` - NULLS FIRST
  - `NullsLast` - NULLS LAST
- `LimitClause` struct:
  - `count: Expression` - number of rows to limit (from Sprint 5)
  - `span: Span`
- `OffsetClause` struct:
  - `count: Expression` - number of rows to skip (from Sprint 5)
  - `use_skip_keyword: bool` - whether SKIP keyword used instead of OFFSET
  - `span: Span`

**Grammar References**:
- `orderByAndPageStatement` (Line 652)
- `orderByClause` (Line 1332)
- `sortSpecification` (Line 1342)
- `sortKey` (Line 1346)
- `orderingSpecification` (Line 1350)
- `nullOrdering` (Line 1357)
- `limitClause` (Line 1364)
- `offsetClause` (Line 1370)

**Acceptance Criteria**:
- [ ] All ordering and pagination AST types defined
- [ ] Multiple sort keys supported
- [ ] Ordering direction optional (default ascending)
- [ ] Null ordering optional
- [ ] LIMIT and OFFSET use expressions from Sprint 5
- [ ] SKIP keyword synonym for OFFSET tracked
- [ ] Span tracking for each clause
- [ ] Documentation explains ordering and pagination semantics
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 10: AST Node Definitions for Grouping

**Description**: Define AST types for GROUP BY clauses.

**Deliverables**:
- `GroupByClause` struct:
  - `elements: Vec<GroupingElement>` - grouping keys
  - `span: Span`
- `GroupingElement` enum:
  - `Expression(Expression)` - group by expression (from Sprint 5)
  - `EmptyGroupingSet` - () empty grouping set for single aggregated result

**Grammar References**:
- `groupByClause` (Line 1313)
- `groupingElement` (Line 1322)
- `emptyGroupingSet` (Line 1326)

**Acceptance Criteria**:
- [ ] Group by clause AST defined
- [ ] Multiple grouping elements supported
- [ ] Empty grouping set for full aggregation supported
- [ ] Grouping expressions from Sprint 5
- [ ] Span tracking covers entire group by clause
- [ ] Documentation explains grouping semantics
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 11: AST Node Definitions for USE GRAPH Clause

**Description**: Define AST types for USE GRAPH clause and graph context.

**Deliverables**:
- `UseGraphClause` struct:
  - `graph: Expression` - graph expression (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `useGraphClause` (Line 773)

**Acceptance Criteria**:
- [ ] Use graph clause AST defined
- [ ] Graph expression from Sprint 5
- [ ] Span tracking covers entire clause
- [ ] Documentation explains graph context semantics
- [ ] Integration with expression parser from Sprint 5

**File Location**: `src/ast/query.rs`

---

### Task 12: Lexer Extensions for Query Keywords

**Description**: Ensure lexer supports all tokens needed for query statements.

**Deliverables**:
- Verify existing query keywords are sufficient:
  - Query structure: MATCH, OPTIONAL, FILTER, WHERE, LET, FOR, SELECT, RETURN, FINISH
  - Set operators: UNION, EXCEPT, INTERSECT, OTHERWISE
  - Quantifiers: ALL, DISTINCT
  - Ordering: ORDER, BY, ASC, ASCENDING, DESC, DESCENDING, NULLS, FIRST, LAST
  - Pagination: LIMIT, OFFSET, SKIP
  - Grouping: GROUP, HAVING
  - Context: USE, GRAPH
  - Select: FROM, SELECT
  - For: IN, WITH, ORDINALITY
- Add any missing keywords to keyword table
- Ensure * (star) operator for SELECT/RETURN * is tokenized

**Lexer Enhancements Needed**:
- Add OTHERWISE keyword if missing
- Add FINISH keyword if missing
- Add ORDINALITY keyword if missing
- Verify all ordering keywords (ASCENDING, DESCENDING) exist
- Ensure NULLS keyword exists

**Grammar References**:
- Query keyword definitions throughout Lines 496-725, 1313-1377

**Acceptance Criteria**:
- [ ] All query keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] * operator tokenized for SELECT/RETURN *
- [ ] No new lexer errors introduced
- [ ] All query-related tokens have proper span information

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 13: Query Parser - Composite Queries and Set Operators

**Description**: Implement parsing for composite queries with set operators.

**Deliverables**:
- `parse_composite_query()` - parse composite query with set operators
- `parse_query_conjunction()` - parse query conjunction (left op right)
- `parse_set_operator()` - parse UNION, EXCEPT, INTERSECT, OTHERWISE
- `parse_set_quantifier()` - parse ALL or DISTINCT
- Set operator precedence handling (all set operators have same precedence, left-associative)
- Parenthesized query support

**Grammar References**:
- `compositeQueryStatement` (Line 498)
- `queryConjunction` (Line 509)
- `setOperator` (Line 514)

**Acceptance Criteria**:
- [ ] All set operators parse correctly
- [ ] Set quantifiers (ALL, DISTINCT) parsed
- [ ] DEFAULT DISTINCT behavior (when quantifier omitted)
- [ ] Left-associative set operator chaining works
- [ ] Parenthesized queries preserve precedence
- [ ] OTHERWISE operator distinguished from set operators
- [ ] Error recovery at set operator boundaries
- [ ] Unit tests for each set operator and combinations

**File Location**: `src/parser/query.rs` (new module)

---

### Task 14: Query Parser - Linear Queries

**Description**: Implement parsing for linear query statements.

**Deliverables**:
- `parse_linear_query()` - parse linear query statement
- `parse_focused_linear_query()` - parse with USE GRAPH clause
- `parse_ambient_linear_query()` - parse without USE GRAPH clause
- `parse_simple_linear_query()` - parse chain of primitive operations
- `parse_primitive_query_statement()` - dispatch to specific statement parsers
- Sequential statement chaining (Vec of statements)

**Grammar References**:
- `linearQueryStatement` (Line 526)
- `focusedLinearQueryStatement` (Line 531)
- `ambientLinearQueryStatement` (Line 554)
- `simpleLinearQueryStatement` (Line 559)
- `primitiveQueryStatement` (Line 568)

**Acceptance Criteria**:
- [ ] Linear query statements parse correctly
- [ ] Focused vs ambient queries distinguished
- [ ] Sequential primitive statements chain correctly
- [ ] USE GRAPH clause parsed in focused queries
- [ ] Result statements (RETURN/FINISH) optional
- [ ] Error recovery at statement boundaries
- [ ] Unit tests for linear query chaining

**File Location**: `src/parser/query.rs`

---

### Task 15: Query Parser - Match Statements

**Description**: Implement parsing for MATCH statement structure (pattern details in Sprint 8).

**Deliverables**:
- `parse_match_statement()` - dispatch to simple or optional match
- `parse_simple_match_statement()` - MATCH <pattern>
- `parse_optional_match_statement()` - OPTIONAL MATCH/block
- `parse_optional_operand()` - parse optional operand variants
- `parse_match_statement_block()` - parse block of match statements
- `parse_graph_pattern()` - placeholder for Sprint 8 (basic structure only)

**Grammar References**:
- `simpleMatchStatement` (Line 583)
- `optionalMatchStatement` (Line 587)
- `optionalOperand` (Line 591)
- `matchStatementBlock` (Line 597)

**Acceptance Criteria**:
- [ ] Match statement structure parses
- [ ] Simple match vs optional match distinguished
- [ ] Optional operand variants (match, block, parenthesized) parse
- [ ] Match statement blocks parse correctly
- [ ] GraphPattern placeholder parser defined
- [ ] Error recovery at match statement boundaries
- [ ] Unit tests for match statement structure
- [ ] Documentation notes Sprint 8 will complete pattern parsing

**File Location**: `src/parser/query.rs`, with placeholder in `src/parser/graph_pattern.rs` (new module for Sprint 8)

---

### Task 16: Query Parser - Filter Statements

**Description**: Implement parsing for FILTER statements.

**Deliverables**:
- `parse_filter_statement()` - FILTER [WHERE] <condition>
- WHERE keyword optional
- Condition parsing uses expression parser from Sprint 5

**Grammar References**:
- `filterStatement` (Line 609)

**Acceptance Criteria**:
- [ ] Filter statements parse correctly
- [ ] WHERE keyword optional (tracked in AST)
- [ ] Condition expressions from Sprint 5 parser
- [ ] Error recovery on malformed filter conditions
- [ ] Unit tests for filter statements with various conditions

**File Location**: `src/parser/query.rs`

---

### Task 17: Query Parser - Let Statements

**Description**: Implement parsing for LET statements and variable bindings.

**Deliverables**:
- `parse_let_statement()` - LET <bindings>
- `parse_let_variable_definition()` - parse individual variable binding
- Type annotation parsing (from Sprint 6)
- Value expression parsing (from Sprint 5)
- Multiple simultaneous bindings supported

**Grammar References**:
- `letStatement` (Line 615)
- `letVariableDefinition` (Line 623)

**Acceptance Criteria**:
- [ ] Let statements parse correctly
- [ ] Multiple variable bindings supported
- [ ] Type annotations optional (from Sprint 6)
- [ ] Value expressions from Sprint 5
- [ ] Error recovery on malformed bindings
- [ ] Unit tests for let statements with various bindings

**File Location**: `src/parser/query.rs`

---

### Task 18: Query Parser - For Statements

**Description**: Implement parsing for FOR statements and iteration.

**Deliverables**:
- `parse_for_statement()` - FOR <item> [WITH ORDINALITY/OFFSET]
- `parse_for_item()` - parse binding_variable IN collection
- `parse_for_ordinality_or_offset()` - parse WITH ORDINALITY or WITH OFFSET
- Collection expression parsing (from Sprint 5)

**Grammar References**:
- `forStatement` (Line 630)
- `forItem` (Line 634)
- `forOrdinalityOrOffset` (Line 646)

**Acceptance Criteria**:
- [ ] For statements parse correctly
- [ ] Mandatory alias binding form (variable IN collection)
- [ ] WITH ORDINALITY and WITH OFFSET variants parse
- [ ] Collection expressions from Sprint 5
- [ ] Error recovery on malformed for statements
- [ ] Unit tests for for statements with ordinality/offset

**File Location**: `src/parser/query.rs`

---

### Task 19: Query Parser - Select Statements

**Description**: Implement parsing for SELECT statements with SQL-style syntax.

**Deliverables**:
- `parse_select_statement()` - parse full SELECT syntax
- `parse_select_items()` - parse SELECT * or select item list
- `parse_select_item()` - parse individual select item with optional alias
- `parse_select_from_clause()` - parse FROM clause variants
- `parse_where_clause()` - parse WHERE clause
- `parse_having_clause()` - parse HAVING clause
- Integrate ORDER BY, GROUP BY, LIMIT, OFFSET parsers

**Grammar References**:
- `selectStatement` (Line 689)
- `selectItem` (Line 697)
- `havingClause` (Line 705)
- `selectGraphMatchList` (Line 713)
- `selectQuerySpecification` (Line 721)

**Acceptance Criteria**:
- [ ] Select statements parse with all clauses
- [ ] SELECT * vs explicit select items parse
- [ ] All optional clauses work correctly
- [ ] FROM clause variants (graph matches, nested query, graph + query) parse
- [ ] WHERE and HAVING clauses use expression parser from Sprint 5
- [ ] ORDER BY, GROUP BY, LIMIT, OFFSET integrate correctly
- [ ] Error recovery on malformed select statements
- [ ] Unit tests for select statements with various clause combinations

**File Location**: `src/parser/query.rs`

---

### Task 20: Query Parser - Return Statements

**Description**: Implement parsing for RETURN statements.

**Deliverables**:
- `parse_return_statement()` - RETURN [DISTINCT|ALL] (* | <items>) [GROUP BY]
- `parse_return_items()` - parse RETURN * or return item list
- `parse_return_item()` - parse individual return item with optional alias
- Set quantifier parsing
- Group by clause integration

**Grammar References**:
- `returnStatement` (Line 667)
- `returnStatementBody` (Line 671)
- `returnItemList` (Line 675)
- `returnItem` (Line 679)
- `returnItemAlias` (Line 683)

**Acceptance Criteria**:
- [ ] Return statements parse correctly
- [ ] RETURN * vs explicit return items parse
- [ ] Set quantifier (DISTINCT/ALL) optional
- [ ] Group by optional
- [ ] Expressions from Sprint 5
- [ ] Error recovery on malformed return statements
- [ ] Unit tests for return statements with various configurations

**File Location**: `src/parser/query.rs`

---

### Task 21: Query Parser - Ordering and Pagination

**Description**: Implement parsing for ORDER BY, LIMIT, and OFFSET clauses.

**Deliverables**:
- `parse_order_by_and_page_statement()` - combined statement
- `parse_order_by_clause()` - ORDER BY <sort_specs>
- `parse_sort_specification()` - parse sort key with ordering
- `parse_ordering_specification()` - ASC/DESC
- `parse_null_ordering()` - NULLS FIRST/LAST
- `parse_limit_clause()` - LIMIT <n>
- `parse_offset_clause()` - OFFSET/SKIP <n>

**Grammar References**:
- `orderByAndPageStatement` (Line 652)
- `orderByClause` (Line 1332)
- `sortSpecification` (Line 1342)
- `orderingSpecification` (Line 1350)
- `nullOrdering` (Line 1357)
- `limitClause` (Line 1364)
- `offsetClause` (Line 1370)

**Acceptance Criteria**:
- [ ] All ordering and pagination clauses parse
- [ ] Multiple sort keys supported
- [ ] Ordering direction optional (default ascending)
- [ ] Null ordering optional
- [ ] LIMIT and OFFSET expressions from Sprint 5
- [ ] SKIP keyword synonym for OFFSET works
- [ ] Error recovery on malformed clauses
- [ ] Unit tests for ordering and pagination combinations

**File Location**: `src/parser/query.rs`

---

### Task 22: Query Parser - Grouping

**Description**: Implement parsing for GROUP BY clauses.

**Deliverables**:
- `parse_group_by_clause()` - GROUP BY <elements>
- `parse_grouping_element()` - parse grouping key or empty set
- `parse_empty_grouping_set()` - parse () for full aggregation

**Grammar References**:
- `groupByClause` (Line 1313)
- `groupingElement` (Line 1322)
- `emptyGroupingSet` (Line 1326)

**Acceptance Criteria**:
- [ ] Group by clauses parse correctly
- [ ] Multiple grouping elements supported
- [ ] Empty grouping set () parses
- [ ] Grouping expressions from Sprint 5
- [ ] Error recovery on malformed group by
- [ ] Unit tests for grouping combinations

**File Location**: `src/parser/query.rs`

---

### Task 23: Query Parser - USE GRAPH Clause

**Description**: Implement parsing for USE GRAPH clause.

**Deliverables**:
- `parse_use_graph_clause()` - USE <graph_expression>
- Graph expression parsing (from Sprint 5)

**Grammar References**:
- `useGraphClause` (Line 773)

**Acceptance Criteria**:
- [ ] Use graph clause parses correctly
- [ ] Graph expressions from Sprint 5
- [ ] Error recovery on malformed use graph
- [ ] Unit tests for use graph clause

**File Location**: `src/parser/query.rs`

---

### Task 24: Integration with Expression Parser (Sprint 5)

**Description**: Integrate query parser with expression parser from Sprint 5.

**Deliverables**:
- Use expression parser for:
  - Filter conditions
  - Let variable values
  - For collection expressions
  - Select items
  - Return items
  - Where clause conditions
  - Having clause conditions
  - Order by sort keys
  - Limit/offset counts
  - Group by expressions
- Ensure no parser conflicts between query and expression parsing
- Test expressions in all query contexts

**Acceptance Criteria**:
- [ ] All query statement parsers use expression parser correctly
- [ ] No parser conflicts between query and expression parsing
- [ ] Expressions work in all query contexts
- [ ] Integration tests validate end-to-end parsing
- [ ] Expression parsing is context-aware

**File Location**: `src/parser/query.rs`, `src/parser/expression.rs`

---

### Task 25: Integration with Type System (Sprint 6)

**Description**: Integrate query parser with type system from Sprint 6.

**Deliverables**:
- Use type parser for:
  - Let variable type annotations
  - Variable definitions in queries
- Ensure type annotations work in query contexts
- Test types in query variable bindings

**Acceptance Criteria**:
- [ ] Type annotations parse correctly in LET statements
- [ ] Type system from Sprint 6 integrates cleanly
- [ ] Integration tests validate end-to-end parsing
- [ ] Type parsing is context-aware

**File Location**: `src/parser/query.rs`, `src/parser/types.rs`

---

### Task 26: Query Nesting and Subqueries

**Description**: Implement support for nested queries and subqueries.

**Deliverables**:
- Nested query specifications
- Subquery expressions in SELECT FROM
- Value query expressions (from Sprint 5)
- Recursive query structure support

**Grammar References**:
- Various subquery contexts throughout query grammar

**Acceptance Criteria**:
- [ ] Nested queries parse correctly
- [ ] Subqueries in SELECT FROM clause work
- [ ] Recursive query structure supported
- [ ] Deep nesting handled efficiently
- [ ] Error recovery in nested queries
- [ ] Unit tests for nested query scenarios

**File Location**: `src/parser/query.rs`

---

### Task 27: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for query parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at statement boundaries
  - Recover at clause boundaries (FROM, WHERE, GROUP BY, etc.)
  - Recover at set operator boundaries
  - Recover at comma separators (in select items, return items)
- Diagnostic messages:
  - "Expected query statement, found {token}"
  - "FILTER requires WHERE clause or search condition"
  - "LET variable definition requires value expression"
  - "FOR statement requires IN keyword"
  - "SELECT requires at least one item or *"
  - "Invalid set operator: expected UNION, EXCEPT, INTERSECT, or OTHERWISE"
  - "Malformed ORDER BY specification"
- Span highlighting for error locations
- Helpful error messages with suggestions

**Acceptance Criteria**:
- [ ] Query parser recovers from common errors
- [ ] Multiple errors in one query reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Tests validate error recovery behavior
- [ ] Suggestions provided for common errors

**File Location**: `src/parser/query.rs`, `src/diag.rs`

---

### Task 28: Comprehensive Testing

**Description**: Implement comprehensive test suite for query parsing.

**Deliverables**:

#### Unit Tests (`src/parser/query.rs`):
- **Composite Query Tests**:
  - UNION queries (with ALL and DISTINCT)
  - EXCEPT queries (with ALL and DISTINCT)
  - INTERSECT queries (with ALL and DISTINCT)
  - OTHERWISE queries
  - Chained set operators (left-associative)
  - Parenthesized queries

- **Linear Query Tests**:
  - Focused linear queries (with USE GRAPH)
  - Ambient linear queries (without USE GRAPH)
  - Sequential statement chaining
  - Queries with and without RETURN

- **Match Statement Tests**:
  - Simple MATCH statements
  - OPTIONAL MATCH statements
  - Optional operand variants (match, block, parenthesized)
  - Match statement blocks

- **Filter Statement Tests**:
  - FILTER with WHERE keyword
  - FILTER without WHERE keyword
  - Various filter conditions

- **Let Statement Tests**:
  - Single variable binding
  - Multiple variable bindings
  - Bindings with type annotations
  - Bindings without type annotations

- **For Statement Tests**:
  - Basic FOR loops
  - FOR with ORDINALITY
  - FOR with OFFSET
  - Various collection expressions

- **Select Statement Tests**:
  - SELECT *
  - SELECT with explicit items
  - SELECT with FROM clause variants
  - SELECT with WHERE clause
  - SELECT with GROUP BY
  - SELECT with HAVING
  - SELECT with ORDER BY
  - SELECT with LIMIT/OFFSET
  - SELECT with all clauses combined

- **Return Statement Tests**:
  - RETURN *
  - RETURN with explicit items
  - RETURN with DISTINCT/ALL
  - RETURN with GROUP BY

- **Ordering and Pagination Tests**:
  - ORDER BY with single key
  - ORDER BY with multiple keys
  - ORDER BY with ASC/DESC
  - ORDER BY with NULLS FIRST/LAST
  - LIMIT clause
  - OFFSET clause
  - SKIP clause (synonym for OFFSET)

- **Grouping Tests**:
  - GROUP BY with expressions
  - GROUP BY with empty grouping set

- **Error Recovery Tests**:
  - Missing clauses
  - Invalid set operators
  - Malformed expressions in queries
  - Unclosed parentheses/blocks

#### Integration Tests (`tests/query_tests.rs` - new file):
- Complex composite queries with multiple set operators
- Linear queries with all primitive statement types
- Nested queries and subqueries
- Queries with expressions from Sprint 5
- Queries with type annotations from Sprint 6
- Edge cases (deeply nested, empty queries)

#### Snapshot Tests:
- Capture AST output for representative queries
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for query parser
- [ ] All query statement variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (empty, deeply nested)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/query.rs`, `tests/query_tests.rs`

---

### Task 29: Documentation and Examples

**Description**: Document query parsing system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all AST node types
  - Module-level documentation for `src/ast/query.rs`
  - Module-level documentation for `src/parser/query.rs`
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase query parsing
  - Add `examples/query_demo.rs` demonstrating:
    - Simple linear queries
    - Composite queries with set operators
    - Queries with MATCH, FILTER, LET, FOR, SELECT
    - Queries with ORDER BY, GROUP BY, LIMIT, OFFSET
    - Nested queries and subqueries
    - Focused queries with USE GRAPH

- **Query Pipeline Overview Documentation**:
  - Document query pipeline composition model
  - Document set operator semantics
  - Document sequential statement chaining
  - Document focused vs ambient queries
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for queries
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Query pipeline overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all query error codes
- [ ] Documentation explains query composition model clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/query.rs`, `src/parser/query.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Query Pipeline Model**: GQL uses a compositional query model where:
   - Composite queries combine queries with set operators
   - Linear queries chain primitive operations sequentially
   - Each primitive operation transforms the working table
   - Result statements (RETURN/FINISH) are optional
   - Parser should reflect this pipeline structure in AST

2. **Set Operator Precedence**: All set operators (UNION, EXCEPT, INTERSECT, OTHERWISE) have the same precedence and are left-associative:
   - `q1 UNION q2 EXCEPT q3` = `(q1 UNION q2) EXCEPT q3`
   - Use precedence climbing or iterative parsing
   - Parentheses override default associativity

3. **Statement Chaining**: Linear queries chain statements sequentially:
   - `MATCH ... FILTER ... LET ... RETURN ...`
   - Each statement operates on result of previous statement
   - Parser should accumulate statements in Vec
   - Order matters for semantics

4. **Focused vs Ambient Queries**: Two query contexts:
   - Focused: explicit USE GRAPH clause
   - Ambient: uses session default graph
   - Parser must distinguish these forms
   - AST should make context explicit

5. **Optional Clauses**: Many query clauses are optional:
   - WHERE in FILTER
   - Type annotations in LET
   - WITH ORDINALITY/OFFSET in FOR
   - All SELECT clauses except select items
   - GROUP BY in RETURN
   - Use `Option<T>` appropriately

6. **Graph Pattern Placeholder**: Sprint 7 parses MATCH statement structure, but graph pattern details are deferred to Sprint 8:
   - Create placeholder `GraphPattern` type
   - Parse MATCH keyword and basic structure
   - Defer pattern expression, label expressions, quantifiers to Sprint 8
   - Clear integration points for Sprint 8

### AST Design Considerations

1. **Span Tracking**: Every query node must track its source span for diagnostic purposes.

2. **Query Hierarchy**: Use enum hierarchy for query types:
   - `Query` (top level): Linear, Composite, Parenthesized
   - `LinearQuery`: Focused, Ambient, Simple
   - `PrimitiveQueryStatement`: Match, Filter, Let, For, OrderByAndPage, Select
   - This makes pattern matching cleaner and type-safer

3. **Optional Fields**: Many query components are optional:
   - Result statements in linear queries
   - WHERE keyword in FILTER
   - Type annotations in LET
   - ORDINALITY/OFFSET in FOR
   - All SELECT clauses except items
   - GROUP BY in RETURN
   - Use `Option<T>` to represent optional components

4. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Variable names
   - Aliases
   - Keywords (when needed)
   - Short identifiers

5. **Box for Recursion**: Use `Box<Query>` for recursive query fields:
   - Composite query operands
   - Nested queries in SELECT FROM
   - Parenthesized queries
   - This avoids infinite size types

6. **Vec for Collections**: Use `Vec<T>` for:
   - Sequential statements in linear queries
   - Select items
   - Return items
   - Sort specifications
   - Grouping elements

### Error Recovery Strategy

1. **Synchronization Points**:
   - Statement keywords (MATCH, FILTER, LET, FOR, SELECT, RETURN)
   - Clause keywords (FROM, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET)
   - Set operators (UNION, EXCEPT, INTERSECT, OTHERWISE)
   - Semicolons (statement terminators)

2. **Clause Boundary Recovery**: If clause malformed:
   - Report error at clause location
   - Skip to next clause keyword
   - Continue parsing rest of query
   - Construct partial AST

3. **Statement Boundary Recovery**: If statement malformed:
   - Report error at statement location
   - Skip to next statement keyword
   - Continue parsing rest of query
   - Accumulate diagnostics

4. **Set Operator Recovery**: If set operator or operand malformed:
   - Report error at operator location
   - Attempt to parse right operand
   - Continue if possible
   - Construct partial composite query

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error in query"
   - Good: "Expected expression after SELECT keyword, found FROM"

2. **Helpful Suggestions**:
   - "Did you forget a comma between select items?"
   - "FILTER requires WHERE keyword or search condition"
   - "FOR statement requires IN keyword: FOR variable IN collection"
   - "Use UNION ALL to include duplicates, or omit ALL for DISTINCT (default)"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing clauses, point to where clause expected
   - For malformed statements, highlight entire statement
   - For invalid operators, highlight operator token

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing SELECT statement..."
   - "In composite query starting at line 42..."
   - "While parsing ORDER BY clause..."

### Performance Considerations

1. **Query Parsing Efficiency**: Query parsing is hot path:
   - Use efficient statement dispatch (match on token)
   - Minimize lookahead (most statements identifiable by first keyword)
   - Avoid excessive backtracking
   - Reuse expression parser for embedded expressions

2. **Statement Chaining**: Accumulate statements in Vec:
   - Pre-allocate Vec with reasonable capacity
   - Avoid excessive reallocations
   - Consider statement count estimates

3. **Set Operator Parsing**: Iterative parsing preferred over recursive:
   - Avoids stack depth issues
   - More efficient for long chains
   - Use loop with token lookahead

4. **AST Allocation**: Minimize allocations:
   - Use `Box` only where needed for recursion
   - Use `SmolStr` for inline storage of short strings
   - Consider arena allocation for AST nodes (future optimization)

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (query keywords, operators)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure; integration testing infrastructure
- **Sprint 5**: Expression parsing for conditions, values, predicates
- **Sprint 6**: Type system for type annotations in variable declarations

### Dependencies on Future Sprints

- **Sprint 8**: Graph pattern matching (MATCH statement content, pattern expressions, quantifiers)
- **Sprint 9**: Aggregation functions (for use in SELECT/RETURN with GROUP BY/HAVING)
- **Sprint 10**: Data modification statements (INSERT, SET, REMOVE, DELETE) - different pipeline
- **Sprint 11**: Procedure calls (CALL statements, nested procedures)
- **Sprint 14**: Semantic validation (variable scoping, type checking, graph context validation)

### Cross-Sprint Integration Points

- Queries are foundational for all data retrieval in GQL
- Query parser must be designed for reusability
- AST query types should be stable to avoid downstream breakage
- Expression integration (Sprint 5) is critical throughout
- Type integration (Sprint 6) important for variable declarations
- Graph pattern integration (Sprint 8) completes MATCH statements
- Consider semantic validation in Sprint 14 (scoping, typing, etc.)

## Test Strategy

### Unit Tests

For each query component:
1. **Happy Path**: Valid queries parse correctly
2. **Variants**: All clause combinations and statement types
3. **Error Cases**: Missing clauses, invalid syntax, malformed queries
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Queries in different contexts:
1. **Composite Queries**: Multiple set operators and nesting
2. **Linear Queries**: All primitive statement types chained
3. **Focused Queries**: With USE GRAPH clause
4. **Ambient Queries**: Using session default graph
5. **Nested Queries**: Deeply nested subqueries
6. **Expression Integration**: Queries with complex expressions from Sprint 5
7. **Type Integration**: Queries with type annotations from Sprint 6

### Snapshot Tests

Capture AST output:
1. Representative queries from each category
2. Complex composite queries
3. Complex linear queries with all statement types
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid queries
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries:
1. Official GQL sample queries
2. Real-world query patterns
3. Verify parser handles production syntax

### Performance Tests

1. **Deeply Nested Queries**: Ensure parser handles deep nesting efficiently
2. **Long Statement Chains**: Many sequential statements
3. **Complex Set Operator Chains**: Long chains of UNION/EXCEPT/INTERSECT

## Performance Considerations

1. **Lexer Efficiency**: Query keywords are frequent; lexer must be fast
2. **Parser Efficiency**: Use efficient statement dispatch and minimal lookahead
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Statement Chaining**: Pre-allocate Vec for statements when possible

## Documentation Requirements

1. **API Documentation**: Rustdoc for all query AST nodes and parser functions
2. **Query Pipeline Overview**: Document composition model and semantics
3. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
4. **Examples**: Demonstrate query parsing in examples
5. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Query grammar complexity causes parser confusion | High | Medium | Careful grammar analysis; extensive testing; clear AST design |
| Set operator precedence bugs | Medium | Low | Comprehensive tests with operator chains; follow ISO GQL spec |
| Statement chaining order dependencies unclear | Medium | Medium | Document statement pipeline semantics; test all orderings |
| Graph pattern placeholder limits testing | High | High | Create clear integration points; document Sprint 8 requirements; test structure without content |
| Expression integration complexity | High | Medium | Reuse Sprint 5 parser cleanly; test expressions in all query contexts |
| Query nesting depth causes stack issues | Low | Low | Use iterative parsing where possible; test deep nesting |
| Optional clause combinations cause ambiguity | Medium | Medium | Document clause precedence; test all combinations |

## Success Metrics

1. **Coverage**: All query statement types parse with correct AST
2. **Correctness**: Query semantics match ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for query parser
6. **Performance**: Parser handles queries with 100+ statements in <1ms
7. **Integration**: Expression parser (Sprint 5) integrates cleanly
8. **Integration**: Type system (Sprint 6) integrates cleanly
9. **Reusability**: Query parser integrates cleanly into future sprints

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping, query pipeline overview)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Query parser tested with expressions from Sprint 5
- [ ] Query parser tested with type annotations from Sprint 6
- [ ] AST design reviewed for stability and extensibility
- [ ] Graph pattern placeholder clearly documented for Sprint 8
- [ ] Sprint 5 integration complete (expressions in queries)
- [ ] Sprint 6 integration complete (type annotations in queries)
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 8: Graph Pattern and Path Pattern System** will build on the query foundation to implement the complete graph matching syntax including graph pattern binding/yield, match modes, node/edge patterns, path prefixes/search modes, quantifiers, simplified path patterns, and label expressions. With query pipeline implemented, Sprint 8 can complete the MATCH statement content that was deferred in Sprint 7.

---

## Appendix: Query Statement Hierarchy

```
Query
├── LinearQuery
│   ├── FocusedLinearQuery
│   │   ├── UseGraphClause
│   │   ├── PrimitiveQueryStatements (Vec)
│   │   └── OptionalResultStatement
│   ├── AmbientLinearQuery
│   │   ├── PrimitiveQueryStatements (Vec)
│   │   └── OptionalResultStatement
│   └── SimpleLinearQuery
│       ├── PrimitiveQueryStatements (Vec)
│       └── OptionalResultStatement
├── CompositeQuery
│   ├── Left Query
│   ├── SetOperator (UNION | EXCEPT | INTERSECT | OTHERWISE)
│   │   └── Optional SetQuantifier (ALL | DISTINCT)
│   └── Right Query
└── ParenthesizedQuery
    └── Query

PrimitiveQueryStatement
├── MatchStatement
│   ├── SimpleMatch { GraphPattern }
│   └── OptionalMatch { OptionalOperand }
├── FilterStatement { [WHERE] Expression }
├── LetStatement { Vec<LetVariableDefinition> }
├── ForStatement { ForItem, Optional<ForOrdinalityOrOffset> }
├── OrderByAndPageStatement { OrderBy, Limit, Offset }
└── SelectStatement
    ├── SetQuantifier (optional)
    ├── SelectItemList (* or items)
    ├── FromClause (optional)
    ├── WhereClause (optional)
    ├── GroupByClause (optional)
    ├── HavingClause (optional)
    ├── OrderByClause (optional)
    ├── OffsetClause (optional)
    └── LimitClause (optional)

PrimitiveResultStatement
├── ReturnStatement
│   ├── SetQuantifier (optional)
│   ├── ReturnItemList (* or items)
│   └── GroupByClause (optional)
└── FinishStatement (placeholder)
```

---

## Appendix: Query Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `compositeQueryStatement` | 498 | `CompositeQuery` | `parse_composite_query()` |
| `queryConjunction` | 509 | `CompositeQuery` | `parse_query_conjunction()` |
| `setOperator` | 514 | `SetOperator` enum | `parse_set_operator()` |
| `linearQueryStatement` | 526 | `LinearQuery` | `parse_linear_query()` |
| `focusedLinearQueryStatement` | 531 | `FocusedLinearQuery` | `parse_focused_linear_query()` |
| `ambientLinearQueryStatement` | 554 | `AmbientLinearQuery` | `parse_ambient_linear_query()` |
| `simpleLinearQueryStatement` | 559 | `LinearQuery` | `parse_simple_linear_query()` |
| `primitiveQueryStatement` | 568 | `PrimitiveQueryStatement` enum | `parse_primitive_query_statement()` |
| `simpleMatchStatement` | 583 | `MatchStatement::Simple` | `parse_simple_match_statement()` |
| `optionalMatchStatement` | 587 | `MatchStatement::Optional` | `parse_optional_match_statement()` |
| `optionalOperand` | 591 | `OptionalOperand` enum | `parse_optional_operand()` |
| `matchStatementBlock` | 597 | `Vec<MatchStatement>` | `parse_match_statement_block()` |
| `filterStatement` | 609 | `FilterStatement` | `parse_filter_statement()` |
| `letStatement` | 615 | `LetStatement` | `parse_let_statement()` |
| `letVariableDefinition` | 623 | `LetVariableDefinition` | `parse_let_variable_definition()` |
| `forStatement` | 630 | `ForStatement` | `parse_for_statement()` |
| `forItem` | 634 | `ForItem` | `parse_for_item()` |
| `forOrdinalityOrOffset` | 646 | `ForOrdinalityOrOffset` enum | `parse_for_ordinality_or_offset()` |
| `orderByAndPageStatement` | 652 | `OrderByAndPageStatement` | `parse_order_by_and_page_statement()` |
| `primitiveResultStatement` | 660 | `PrimitiveResultStatement` enum | `parse_primitive_result_statement()` |
| `returnStatement` | 667 | `ReturnStatement` | `parse_return_statement()` |
| `returnStatementBody` | 671 | `ReturnStatement` | `parse_return_statement_body()` |
| `returnItemList` | 675 | `ReturnItemList` | `parse_return_items()` |
| `returnItem` | 679 | `ReturnItem` | `parse_return_item()` |
| `returnItemAlias` | 683 | `Option<SmolStr>` | `parse_return_item_alias()` |
| `selectStatement` | 689 | `SelectStatement` | `parse_select_statement()` |
| `selectItem` | 697 | `SelectItem` | `parse_select_item()` |
| `havingClause` | 705 | `HavingClause` | `parse_having_clause()` |
| `selectGraphMatchList` | 713 | `SelectFromClause::GraphMatchList` | `parse_select_graph_match_list()` |
| `selectQuerySpecification` | 721 | `SelectFromClause::QuerySpecification` | `parse_select_query_specification()` |
| `useGraphClause` | 773 | `UseGraphClause` | `parse_use_graph_clause()` |
| `groupByClause` | 1313 | `GroupByClause` | `parse_group_by_clause()` |
| `groupingElement` | 1322 | `GroupingElement` | `parse_grouping_element()` |
| `emptyGroupingSet` | 1326 | `GroupingElement::EmptyGroupingSet` | `parse_empty_grouping_set()` |
| `orderByClause` | 1332 | `OrderByClause` | `parse_order_by_clause()` |
| `sortSpecification` | 1342 | `SortSpecification` | `parse_sort_specification()` |
| `sortKey` | 1346 | `Expression` | `parse_sort_key()` |
| `orderingSpecification` | 1350 | `OrderingSpecification` enum | `parse_ordering_specification()` |
| `nullOrdering` | 1357 | `NullOrdering` enum | `parse_null_ordering()` |
| `limitClause` | 1364 | `LimitClause` | `parse_limit_clause()` |
| `offsetClause` | 1370 | `OffsetClause` | `parse_offset_clause()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-17
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4, 5, 6 (completed or required)
**Next Sprint**: Sprint 8 (Graph Pattern and Path Pattern System)
