# Sprint 9: Result Shaping and Aggregation

## Sprint Overview

**Sprint Goal**: Complete result production features.

**Sprint Duration**: TBD

**Status**: 🟢 **Implemented** (standard-compliance update: February 18, 2026)

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅
- Sprint 5 (Values, Literals, and Expression Core) ✅
- Sprint 6 (Type System and Reference Forms) ✅
- Sprint 7 (Query Pipeline Core) ✅
- Sprint 8 (Graph Pattern and Path Pattern System) ✅

## Scope

This sprint implements the complete result shaping and aggregation system that controls how query results are produced, grouped, aggregated, ordered, and paginated. Result shaping is the final stage of query processing, transforming matched graph patterns and intermediate results into structured output. Sprint 7 established query composition, Sprint 8 completed pattern matching, and Sprint 9 delivers the full result production pipeline.

### Standards Compliance Clarifications (OpenGQL grammar snapshot)

- Binary percentile functions follow the `binarySetFunction` production in `third_party/opengql-grammar/GQL.g4`:
  - `PERCENTILE_CONT(<dependent_expr>, <independent_expr>)`
  - `PERCENTILE_DISC(<dependent_expr>, <independent_expr>)`
  - `dependent_expr` allows optional set quantifier (`DISTINCT`/`ALL`).
- `WITHIN GROUP (ORDER BY ...)` is not in the vendored OpenGQL grammar snapshot and is treated as non-standard syntax.
- Property references after `.` use `propertyName -> identifier` semantics:
  - unquoted regular identifiers
  - non-reserved keywords
  - delimited identifiers for reserved keywords

### Feature Coverage from GQL_FEATURES.md

Sprint 9 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 9: Result & Output Features** (Lines 622-656)
   - Primitive result statements
   - RETURN statement
   - FINISH statement
   - Return statement body
   - Return item lists
   - Return items with expressions
   - Return item aliases

2. **Section 10: Ordering, Pagination & Grouping** (Lines 659-716)
   - Combined ordering and pagination
   - ORDER BY clause
   - Sort specifications
   - Sort keys
   - Ordering specifications (ASC, DESC, ASCENDING, DESCENDING)
   - Null ordering (NULLS FIRST, NULLS LAST)
   - LIMIT clause
   - OFFSET/SKIP clause
   - GROUP BY clause
   - Grouping elements
   - Empty grouping sets

3. **Section 11: Aggregation Functions** (Lines 718-756)
   - Aggregate functions
   - General set functions (AVG, COUNT, MAX, MIN, SUM, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP)
   - Binary set functions (PERCENTILE_CONT, PERCENTILE_DISC)
   - Set quantifiers (DISTINCT, ALL)
   - COUNT(*)

4. **Additional Features**:
   - HAVING clause integration
   - Set quantifiers in RETURN statements (DISTINCT, ALL)
   - Integration with SELECT statements from Sprint 7

## Exit Criteria

- [ ] RETURN statements parse with all options (DISTINCT, ALL, *, item lists)
- [ ] FINISH statements parse correctly
- [ ] Return item lists with expressions and aliases parse correctly
- [ ] GROUP BY clause parses with grouping elements and empty grouping sets
- [ ] HAVING clause parses with aggregate predicates
- [ ] ORDER BY clause parses with sort specifications
- [ ] Sort specifications support ASC/DESC and NULLS FIRST/LAST
- [ ] LIMIT clause parses correctly
- [ ] OFFSET/SKIP clause parses correctly
- [ ] Combined ORDER BY and pagination (LIMIT/OFFSET) works correctly
- [ ] All aggregate functions parse (AVG, COUNT, MAX, MIN, SUM, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP)
- [ ] Binary set functions parse (PERCENTILE_CONT, PERCENTILE_DISC)
- [ ] COUNT(*) parses correctly
- [ ] Set quantifiers (DISTINCT, ALL) work in aggregate functions
- [ ] Set quantifiers work in RETURN statements
- [ ] Result shaping clauses integrate with query pipeline from Sprint 7
- [ ] Result shaping clauses integrate with expression parsing from Sprint 5
- [ ] Parser produces structured diagnostics for malformed result clauses
- [ ] AST nodes have proper span information for all components
- [ ] Recovery mechanisms handle errors at clause boundaries
- [ ] Unit tests cover all result shaping variants and error cases
- [ ] Integration tests validate end-to-end query with result shaping

## Implementation Tasks

### Task 1: AST Node Definitions for RETURN and FINISH Statements

**Description**: Define AST types for result production statements.

**Deliverables**:
- `PrimitiveResultStatement` enum:
  - `Return(ReturnStatement)` - RETURN statement
  - `Finish(FinishStatement)` - FINISH statement
- `ReturnStatement` struct:
  - `body: ReturnStatementBody` - return statement body
  - `order_by_and_page: Option<OrderByAndPageStatement>` - optional ordering and pagination
  - `span: Span`
- `ReturnStatementBody` struct:
  - `quantifier: Option<SetQuantifier>` - optional DISTINCT/ALL
  - `items: ReturnItemListOrAsterisk` - what to return
  - `group_by: Option<GroupByClause>` - optional GROUP BY clause
  - `span: Span`
- `ReturnItemListOrAsterisk` enum:
  - `Asterisk` - return all columns (*)
  - `ItemList(ReturnItemList)` - specific items to return
- `ReturnItemList` struct:
  - `items: Vec<ReturnItem>` - comma-separated return items
  - `span: Span`
- `ReturnItem` struct:
  - `expression: Expression` - expression to return (from Sprint 5)
  - `alias: Option<ReturnItemAlias>` - optional alias
  - `span: Span`
- `ReturnItemAlias` struct:
  - `name: SmolStr` - alias name
  - `span: Span`
- `FinishStatement` struct (placeholder for grammar completeness):
  - `span: Span`
- `SetQuantifier` enum (may already exist from Sprint 7):
  - `All` - ALL (include duplicates)
  - `Distinct` - DISTINCT (remove duplicates)

**Grammar References**:
- `primitiveResultStatement` (Line 660)
- `returnStatement` (Line 667)
- `returnStatementBody` (Line 671)
- `returnItemList` (Line 675)
- `returnItem` (Line 679)
- `returnItemAlias` (Line 683)
- `setQuantifier` (Line 2405)

**Acceptance Criteria**:
- [ ] All return statement AST types defined in `src/ast/query.rs`
- [ ] Each node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each variant
- [ ] Set quantifiers properly captured
- [ ] Asterisk vs item list distinguished
- [ ] Integration with expression parsing from Sprint 5
- [ ] Optional order by and pagination supported

**File Location**: `src/ast/query.rs`

---

### Task 2: AST Node Definitions for GROUP BY Clause

**Description**: Define AST types for grouping operations.

**Deliverables**:
- `GroupByClause` struct:
  - `elements: Vec<GroupingElement>` - grouping elements
  - `span: Span`
- `GroupingElement` enum:
  - `Expression(Expression)` - group by expression (from Sprint 5)
  - `EmptyGroupingSet(EmptyGroupingSet)` - () for single aggregated result
- `EmptyGroupingSet` struct:
  - `span: Span`

**Grammar References**:
- `groupByClause` (Line 1313)
- `groupingElement` (Line 1322)
- `emptyGroupingSet` (Line 1326)

**Acceptance Criteria**:
- [ ] GROUP BY clause AST defined
- [ ] Grouping elements use expressions from Sprint 5
- [ ] Empty grouping set () supported
- [ ] Multiple grouping elements supported (comma-separated)
- [ ] Span tracking for each grouping element
- [ ] Documentation explains grouping semantics

**File Location**: `src/ast/query.rs`

---

### Task 3: AST Node Definitions for HAVING Clause

**Description**: Define AST types for HAVING clause (aggregate filtering).

**Deliverables**:
- `HavingClause` struct:
  - `condition: Expression` - having predicate (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `havingClause` (Line 705)

**Acceptance Criteria**:
- [ ] HAVING clause AST defined
- [ ] Condition uses expression parser from Sprint 5
- [ ] Span tracking for HAVING clause
- [ ] Documentation explains HAVING vs WHERE distinction

**File Location**: `src/ast/query.rs`

---

### Task 4: AST Node Definitions for ORDER BY Clause

**Description**: Define AST types for result ordering.

**Deliverables**:
- `OrderByClause` struct:
  - `specifications: Vec<SortSpecification>` - sort keys
  - `span: Span`
- `SortSpecification` struct:
  - `key: SortKey` - expression to sort by
  - `ordering: Option<OrderingSpecification>` - ASC/DESC
  - `null_ordering: Option<NullOrdering>` - NULLS FIRST/LAST
  - `span: Span`
- `SortKey` struct:
  - `expression: Expression` - sort key expression (from Sprint 5)
  - `span: Span`
- `OrderingSpecification` enum:
  - `Ascending` - ASC or ASCENDING
  - `Descending` - DESC or DESCENDING
- `NullOrdering` enum:
  - `NullsFirst` - NULLS FIRST
  - `NullsLast` - NULLS LAST

**Grammar References**:
- `orderByClause` (Line 1332)
- `sortSpecification` (Line 1342)
- `sortKey` (Line 1346)
- `orderingSpecification` (Line 1350)
- `nullOrdering` (Line 1357)

**Acceptance Criteria**:
- [ ] ORDER BY clause AST defined
- [ ] Multiple sort specifications supported
- [ ] Sort keys use expressions from Sprint 5
- [ ] ASC/ASCENDING and DESC/DESCENDING distinguished (or normalized)
- [ ] NULLS FIRST/LAST options supported
- [ ] Default ordering (ASC if not specified) documented
- [ ] Span tracking for each component
- [ ] Documentation explains sort order semantics

**File Location**: `src/ast/query.rs`

---

### Task 5: AST Node Definitions for LIMIT and OFFSET Clauses

**Description**: Define AST types for result pagination.

**Deliverables**:
- `LimitClause` struct:
  - `count: Expression` - limit count (from Sprint 5)
  - `span: Span`
- `OffsetClause` struct:
  - `count: Expression` - offset count (from Sprint 5)
  - `span: Span`
- `OrderByAndPageStatement` struct:
  - Captures combined ORDER BY + OFFSET + LIMIT
  - `order_by: Option<OrderByClause>` - optional ORDER BY
  - `offset: Option<OffsetClause>` - optional OFFSET/SKIP
  - `limit: Option<LimitClause>` - optional LIMIT
  - `span: Span`

**Grammar References**:
- `limitClause` (Line 1364)
- `offsetClause` (Line 1370)
- `orderByAndPageStatement` (Line 652)

**Acceptance Criteria**:
- [ ] LIMIT clause AST defined
- [ ] OFFSET clause AST defined (also SKIP as alias)
- [ ] Limit/offset counts use expressions from Sprint 5
- [ ] OrderByAndPageStatement combines clauses correctly
- [ ] All three orderings supported:
  - ORDER BY + OFFSET + LIMIT
  - OFFSET + LIMIT
  - LIMIT only
- [ ] Span tracking for each clause
- [ ] Documentation explains pagination semantics

**File Location**: `src/ast/query.rs`

---

### Task 6: AST Node Definitions for Aggregate Functions

**Description**: Define AST types for aggregate functions.

**Deliverables**:
- `AggregateFunction` enum:
  - `CountStar { span: Span }` - COUNT(*)
  - `GeneralSetFunction(GeneralSetFunction)` - general aggregate functions
  - `BinarySetFunction(BinarySetFunction)` - binary set functions
- `GeneralSetFunction` struct:
  - `function_type: GeneralSetFunctionType` - function type
  - `quantifier: Option<SetQuantifier>` - optional DISTINCT/ALL
  - `expression: Expression` - expression to aggregate (from Sprint 5)
  - `span: Span`
- `GeneralSetFunctionType` enum:
  - `Avg` - AVG
  - `Count` - COUNT
  - `Max` - MAX
  - `Min` - MIN
  - `Sum` - SUM
  - `CollectList` - COLLECT_LIST
  - `StddevSamp` - STDDEV_SAMP
  - `StddevPop` - STDDEV_POP
- `BinarySetFunction` struct:
  - `function_type: BinarySetFunctionType` - function type
  - `quantifier: Option<SetQuantifier>` - optional DISTINCT/ALL for dependent value expression
  - `inverse_distribution_argument: Expression` - percentile value (from Sprint 5)
  - `expression: Expression` - expression to aggregate (from Sprint 5)
  - `span: Span`
- `BinarySetFunctionType` enum:
  - `PercentileCont` - PERCENTILE_CONT
  - `PercentileDisc` - PERCENTILE_DISC

**Grammar References**:
- `aggregateFunction` (Line 2380)
- `generalSetFunction` (Line 2386)
- `generalSetFunctionType` (Line 2394)
- `binarySetFunction` (Line 2390)
- `binarySetFunctionType` (Line 2410)
- `setQuantifier` (Line 2405)

**Acceptance Criteria**:
- [ ] All aggregate function types enumerated
- [ ] COUNT(*) distinguished from COUNT(expression)
- [ ] General set functions support DISTINCT/ALL quantifiers
- [ ] Binary set functions support two arguments
- [ ] Arguments use expressions from Sprint 5
- [ ] Span tracking for each function
- [ ] Documentation explains each aggregate function's semantics

**File Location**: `src/ast/expression.rs` (aggregate functions are expressions)

---

### Task 7: Lexer Extensions for Result Shaping Tokens

**Description**: Ensure lexer supports all tokens needed for result shaping and aggregation.

**Deliverables**:
- Verify existing result keywords are sufficient:
  - Result statements: RETURN, FINISH
  - Ordering: ORDER, BY, ASC, ASCENDING, DESC, DESCENDING, NULLS, FIRST, LAST
  - Pagination: LIMIT, OFFSET, SKIP
  - Grouping: GROUP, HAVING
  - Set quantifiers: DISTINCT, ALL
  - Aggregate functions: AVG, COUNT, MAX, MIN, SUM, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP, PERCENTILE_CONT, PERCENTILE_DISC
- Add any missing keywords to keyword table
- Ensure operators work:
  - `*` (asterisk for RETURN * and COUNT(*))
  - `,` (comma for item lists)

**Lexer Enhancements Needed** (if any):
- Add RETURN, FINISH keywords if missing
- Add COLLECT_LIST, STDDEV_SAMP, STDDEV_POP if missing
- Add PERCENTILE_CONT, PERCENTILE_DISC if missing
- Add NULLS keyword if missing
- Add SKIP keyword if missing (alias for OFFSET)
- Ensure all keywords are case-insensitive

**Grammar References**:
- Result keyword definitions throughout Lines 650-2421

**Acceptance Criteria**:
- [ ] All result shaping keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] Aggregate function names tokenized as keywords
- [ ] No new lexer errors introduced
- [ ] All result-related tokens have proper span information

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 8: Result Statement Parser - RETURN and FINISH

**Description**: Implement parsing for RETURN and FINISH statements.

**Deliverables**:
- `parse_primitive_result_statement()` - dispatch to RETURN or FINISH
- `parse_return_statement()` - RETURN [DISTINCT|ALL] (* | items) [GROUP BY ...] [ORDER BY ...] [LIMIT ...]
- `parse_return_statement_body()` - parse return body
- `parse_return_item_list()` - parse comma-separated return items
- `parse_return_item()` - parse individual return item with optional alias
- `parse_return_item_alias()` - AS alias_name
- `parse_finish_statement()` - FINISH (placeholder)
- Integration with expression parser from Sprint 5 for return items
- Integration with GROUP BY, ORDER BY, LIMIT/OFFSET parsers

**Grammar References**:
- `primitiveResultStatement` (Line 660)
- `returnStatement` (Line 667)
- `returnStatementBody` (Line 671)
- `returnItemList` (Line 675)
- `returnItem` (Line 679)
- `returnItemAlias` (Line 683)

**Acceptance Criteria**:
- [ ] RETURN statements parse with all options
- [ ] RETURN * works
- [ ] RETURN with item list works
- [ ] DISTINCT and ALL quantifiers work
- [ ] Return items use expression parser from Sprint 5
- [ ] Aliases work with AS keyword
- [ ] GROUP BY integration works (Task 9)
- [ ] ORDER BY and pagination integration works (Task 10-11)
- [ ] FINISH statement parses (placeholder for now)
- [ ] Error recovery at item boundaries
- [ ] Unit tests for all return variants

**File Location**: `src/parser/query.rs`

---

### Task 9: Result Statement Parser - GROUP BY and HAVING

**Description**: Implement parsing for GROUP BY and HAVING clauses.

**Deliverables**:
- `parse_group_by_clause()` - GROUP BY grouping_element_list
- `parse_grouping_element()` - expression or empty grouping set
- `parse_empty_grouping_set()` - ()
- `parse_having_clause()` - HAVING condition
- Integration with expression parser from Sprint 5

**Grammar References**:
- `groupByClause` (Line 1313)
- `groupingElement` (Line 1322)
- `emptyGroupingSet` (Line 1326)
- `havingClause` (Line 705)

**Acceptance Criteria**:
- [ ] GROUP BY clause parses correctly
- [ ] Multiple grouping elements supported (comma-separated)
- [ ] Grouping elements use expressions from Sprint 5
- [ ] Empty grouping set () works
- [ ] HAVING clause parses with expression predicate
- [ ] HAVING uses expression parser from Sprint 5
- [ ] Error recovery on malformed grouping
- [ ] Unit tests for GROUP BY and HAVING

**File Location**: `src/parser/query.rs`

---

### Task 10: Result Statement Parser - ORDER BY

**Description**: Implement parsing for ORDER BY clause with sort specifications.

**Deliverables**:
- `parse_order_by_clause()` - ORDER BY sort_specification_list
- `parse_sort_specification()` - expression [ASC|DESC] [NULLS FIRST|LAST]
- `parse_sort_key()` - expression to sort by
- `parse_ordering_specification()` - ASC/ASCENDING/DESC/DESCENDING
- `parse_null_ordering()` - NULLS FIRST/LAST
- Integration with expression parser from Sprint 5 for sort keys

**Grammar References**:
- `orderByClause` (Line 1332)
- `sortSpecification` (Line 1342)
- `sortKey` (Line 1346)
- `orderingSpecification` (Line 1350)
- `nullOrdering` (Line 1357)

**Acceptance Criteria**:
- [ ] ORDER BY clause parses correctly
- [ ] Multiple sort specifications supported (comma-separated)
- [ ] Sort keys use expressions from Sprint 5
- [ ] ASC/ASCENDING works
- [ ] DESC/DESCENDING works
- [ ] Default ordering (ASC) handled
- [ ] NULLS FIRST works
- [ ] NULLS LAST works
- [ ] Error recovery on malformed sort specifications
- [ ] Unit tests for all ordering combinations

**File Location**: `src/parser/query.rs`

---

### Task 11: Result Statement Parser - LIMIT and OFFSET

**Description**: Implement parsing for LIMIT and OFFSET/SKIP clauses.

**Deliverables**:
- `parse_limit_clause()` - LIMIT expression
- `parse_offset_clause()` - OFFSET expression or SKIP expression
- `parse_order_by_and_page_statement()` - combine ORDER BY + OFFSET + LIMIT
- Handle three valid orderings:
  - ORDER BY + OFFSET + LIMIT
  - OFFSET + LIMIT
  - LIMIT only
- Integration with expression parser from Sprint 5 for counts

**Grammar References**:
- `limitClause` (Line 1364)
- `offsetClause` (Line 1370)
- `orderByAndPageStatement` (Line 652)

**Acceptance Criteria**:
- [ ] LIMIT clause parses correctly
- [ ] OFFSET clause parses correctly
- [ ] SKIP clause works (alias for OFFSET)
- [ ] Limit/offset expressions use expression parser from Sprint 5
- [ ] Combined ORDER BY + OFFSET + LIMIT works
- [ ] OFFSET + LIMIT without ORDER BY works
- [ ] LIMIT without ORDER BY or OFFSET works
- [ ] Error recovery on malformed pagination
- [ ] Unit tests for all pagination combinations

**File Location**: `src/parser/query.rs`

---

### Task 12: Expression Parser Extension - Aggregate Functions

**Description**: Extend expression parser to support aggregate functions.

**Deliverables**:
- `parse_aggregate_function()` - dispatch to aggregate function types
- `parse_count_star()` - COUNT(*)
- `parse_general_set_function()` - AVG, COUNT, MAX, MIN, SUM, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP
- `parse_binary_set_function()` - `PERCENTILE_CONT(dependent_expr, independent_expr)`, `PERCENTILE_DISC(dependent_expr, independent_expr)`
- Integration into expression primary parsing
- Support DISTINCT/ALL quantifiers in aggregate functions (including binary set dependent argument)

**Grammar References**:
- `aggregateFunction` (Line 2380)
- `generalSetFunction` (Line 2386)
- `generalSetFunctionType` (Line 2394)
- `binarySetFunction` (Line 2390)
- `binarySetFunctionType` (Line 2410)

**Acceptance Criteria**:
- [ ] COUNT(*) parses correctly
- [ ] All general set functions parse (AVG, COUNT, MAX, MIN, SUM, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP)
- [ ] DISTINCT and ALL quantifiers work in aggregate functions
- [ ] Binary set functions parse with two arguments (PERCENTILE_CONT, PERCENTILE_DISC)
- [ ] Aggregate functions integrate into expression parsing
- [ ] Nested expressions work in aggregate arguments
- [ ] Error recovery on malformed aggregate calls
- [ ] Unit tests for all aggregate function types

**File Location**: `src/parser/expression.rs`

---

### Task 13: Integration with Query Pipeline (Sprint 7)

**Description**: Integrate result shaping with query pipeline from Sprint 7.

**Deliverables**:
- Update query AST to include result statements
- Ensure RETURN can follow query clauses (MATCH, FILTER, LET, etc.)
- Ensure FINISH can follow query clauses
- Test result shaping in linear queries
- Test result shaping in composite queries (with set operators)
- Test result shaping in SELECT statements

**Acceptance Criteria**:
- [ ] RETURN integrates with linear queries
- [ ] RETURN integrates with composite queries
- [ ] GROUP BY, HAVING, ORDER BY, LIMIT work in query pipeline
- [ ] SELECT statements (from Sprint 7) support result shaping
- [ ] No regressions in existing query tests
- [ ] Integration tests validate end-to-end query with result shaping

**File Location**: `src/parser/query.rs`, `src/ast/query.rs`

---

### Task 14: Integration with SELECT Statements

**Description**: Ensure SELECT statements support result shaping features.

**Deliverables**:
- Update SELECT statement parsing (from Sprint 7) to support:
  - Set quantifiers (DISTINCT, ALL)
  - GROUP BY clause
  - HAVING clause
  - ORDER BY clause
  - LIMIT and OFFSET clauses
- Test SELECT with all result shaping features

**Grammar References**:
- `selectStatement` (Line 690)
- `selectStatementBody` mentions whereClause, groupByClause, havingClause, orderByClause, offsetClause, limitClause

**Acceptance Criteria**:
- [ ] SELECT statements support DISTINCT and ALL
- [ ] SELECT statements support GROUP BY
- [ ] SELECT statements support HAVING
- [ ] SELECT statements support ORDER BY
- [ ] SELECT statements support LIMIT and OFFSET
- [ ] All result shaping features work in SELECT
- [ ] Integration tests validate SELECT with result shaping

**File Location**: `src/parser/query.rs`

---

### Task 15: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for result shaping.

**Deliverables**:
- Error recovery strategies:
  - Recover at clause boundaries (RETURN, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET)
  - Recover at comma separators (in return item lists, grouping elements, sort specifications)
  - Recover at keyword boundaries
  - Partial AST construction on errors
- Diagnostic messages:
  - "Expected return item after comma"
  - "Expected expression after GROUP BY"
  - "HAVING clause requires GROUP BY"
  - "Expected sort key after ORDER BY"
  - "Expected ASC, DESC, ASCENDING, or DESCENDING"
  - "NULLS FIRST/LAST requires ORDER BY"
  - "Expected numeric expression after LIMIT"
  - "Expected numeric expression after OFFSET"
  - "Invalid aggregate function syntax"
  - "COUNT(*) does not accept DISTINCT quantifier"
  - "Binary set functions require two arguments"
- Span highlighting for error locations
- Helpful error messages with suggestions:
  - "Did you mean RETURN * instead of RETURN?"
  - "Aggregate functions require GROUP BY or single-row result"
  - "HAVING filters aggregated results; use WHERE for row filtering"
  - "ORDER BY expression must match RETURN item or grouping key"

**Grammar References**:
- All result shaping rules (Lines 650-2421)

**Acceptance Criteria**:
- [ ] Result parser recovers from common errors
- [ ] Multiple errors in one query reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Suggestions provided for common result shaping errors
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/query.rs`, `src/parser/expression.rs`, `src/diag.rs`

---

### Task 16: Comprehensive Testing

**Description**: Implement comprehensive test suite for result shaping and aggregation.

**Deliverables**:

#### Unit Tests (`src/parser/query.rs`, `src/parser/expression.rs`):
- **RETURN Statement Tests**:
  - RETURN *
  - RETURN with item list
  - RETURN with DISTINCT quantifier
  - RETURN with ALL quantifier
  - RETURN with aliases
  - RETURN with GROUP BY
  - RETURN with ORDER BY
  - RETURN with LIMIT/OFFSET
  - RETURN with all clauses combined

- **FINISH Statement Tests**:
  - Basic FINISH statement

- **GROUP BY Tests**:
  - GROUP BY single expression
  - GROUP BY multiple expressions
  - GROUP BY with empty grouping set ()
  - GROUP BY with expressions from Sprint 5

- **HAVING Tests**:
  - HAVING with simple predicate
  - HAVING with aggregate functions
  - HAVING with complex expressions

- **ORDER BY Tests**:
  - ORDER BY single expression
  - ORDER BY multiple expressions
  - ORDER BY with ASC
  - ORDER BY with DESC
  - ORDER BY with ASCENDING
  - ORDER BY with DESCENDING
  - ORDER BY with NULLS FIRST
  - ORDER BY with NULLS LAST
  - ORDER BY with all options combined

- **LIMIT/OFFSET Tests**:
  - LIMIT alone
  - OFFSET alone
  - SKIP alone (alias for OFFSET)
  - LIMIT with OFFSET
  - LIMIT with expressions
  - ORDER BY + OFFSET + LIMIT

- **Aggregate Function Tests**:
  - COUNT(*)
  - COUNT(expression)
  - COUNT(DISTINCT expression)
  - AVG, MAX, MIN, SUM
  - COLLECT_LIST
  - STDDEV_SAMP, STDDEV_POP
  - PERCENTILE_CONT, PERCENTILE_DISC
  - Nested expressions in aggregates
  - Aggregates in HAVING clause

- **Integration Tests**:
  - RETURN in linear queries
  - RETURN in composite queries
  - SELECT with result shaping
  - Complex queries with all result shaping features

- **Error Recovery Tests**:
  - Missing expressions after keywords
  - Invalid aggregate function syntax
  - HAVING without GROUP BY
  - Malformed ORDER BY
  - Invalid LIMIT/OFFSET values

#### Integration Tests (`tests/result_shaping_tests.rs` - new file):
- Complete queries with MATCH + RETURN
- Queries with GROUP BY and aggregates
- Queries with ORDER BY and pagination
- SELECT statements with all result features
- Nested queries with result shaping
- Edge cases (complex expressions, multiple clauses)

#### Snapshot Tests:
- Capture AST output for representative result clauses
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for result shaping parser
- [ ] All result shaping variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (complex expressions, all clauses combined)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/query.rs`, `src/parser/expression.rs`, `tests/result_shaping_tests.rs`

---

### Task 17: Documentation and Examples

**Description**: Document result shaping system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all result shaping AST node types
  - Module-level documentation for result shaping
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase result shaping
  - Add `examples/result_shaping_demo.rs` demonstrating:
    - Simple RETURN statements
    - RETURN with GROUP BY and aggregates
    - RETURN with ORDER BY and pagination
    - Complex result shaping with all features
    - SELECT statements with result shaping

- **Result Shaping Overview Documentation**:
  - Document result production semantics
  - Document grouping and aggregation
  - Document ordering and pagination
  - Document aggregate function semantics
  - Document HAVING vs WHERE distinction
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for result shaping
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Result shaping overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all result shaping error codes
- [ ] Documentation explains result shaping semantics clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/query.rs`, `src/ast/expression.rs`, `src/parser/query.rs`, `src/parser/expression.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Clause Ordering**: Result shaping clauses have specific ordering requirements:
   - RETURN/FINISH body comes first
   - GROUP BY follows RETURN items
   - HAVING follows GROUP BY
   - ORDER BY follows HAVING (or GROUP BY, or RETURN)
   - OFFSET/LIMIT follow ORDER BY (or can appear without ORDER BY)
   - Parser must enforce or document ordering rules

2. **Optional Clause Composition**: Most result shaping clauses are optional:
   - RETURN * vs RETURN items
   - GROUP BY optional
   - HAVING optional (but requires GROUP BY)
   - ORDER BY optional
   - LIMIT/OFFSET optional
   - Parser should handle all valid combinations

3. **Expression Integration**: Result shaping heavily uses expressions:
   - Return items are expressions
   - Grouping elements are expressions
   - Sort keys are expressions
   - Aggregate function arguments are expressions
   - HAVING predicates are expressions
   - LIMIT/OFFSET counts are expressions
   - Must use expression parser from Sprint 5 consistently

4. **Aggregate Function Context**: Aggregates have special rules:
   - COUNT(*) is special syntax (not an expression with *)
   - Aggregates typically require GROUP BY or single-row context
   - Aggregates can appear in RETURN items and HAVING clause
   - Parser should track aggregate context for semantic validation

5. **Set Quantifiers**: DISTINCT/ALL appear in multiple contexts:
   - RETURN DISTINCT/ALL (affects result deduplication)
   - Aggregate functions with DISTINCT/ALL (affects aggregation input)
   - SELECT DISTINCT/ALL (same as RETURN)
   - Parser must distinguish these contexts

6. **Error Recovery**: Result clauses have clear boundaries:
   - Recover at clause keywords (GROUP, HAVING, ORDER, LIMIT, OFFSET)
   - Recover at comma separators in lists
   - Continue parsing after errors to report multiple issues

### AST Design Considerations

1. **Span Tracking**: Every result clause node must track its source span for diagnostic purposes.

2. **Optional Fields**: Many result components are optional:
   - Set quantifiers (DISTINCT/ALL)
   - Aliases in return items
   - ORDER BY clause
   - LIMIT/OFFSET clauses
   - GROUP BY clause
   - HAVING clause
   - Ordering specification (ASC/DESC)
   - Null ordering (NULLS FIRST/LAST)
   - Use `Option<T>` appropriately

3. **Expression Reuse**: Use expression AST from Sprint 5:
   - Return items are expressions
   - Sort keys are expressions
   - Grouping elements are expressions
   - Aggregate arguments are expressions
   - HAVING predicates are expressions
   - Don't duplicate expression types

4. **Aggregate Function Placement**: Aggregates are expressions:
   - Define aggregate function types in `src/ast/expression.rs`
   - Integrate into expression enum hierarchy
   - Aggregate functions can nest with other expressions

5. **List Types**: Use `Vec<T>` for:
   - Return item lists
   - Grouping element lists
   - Sort specification lists
   - Clear comma-separated list parsing

6. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Return item aliases
   - Short identifiers
   - Keyword-like names

### Error Recovery Strategy

1. **Synchronization Points**:
   - Clause keywords (GROUP, HAVING, ORDER, LIMIT, OFFSET)
   - Comma separators in lists
   - End of statement (semicolon or next major clause)
   - Closing delimiters

2. **Clause Boundary Recovery**: If clause malformed:
   - Report error at clause location
   - Skip to next clause keyword
   - Continue parsing remaining clauses
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
   - Bad: "Parse error in result clause"
   - Good: "Expected return item after comma, found ORDER"

2. **Helpful Suggestions**:
   - "Did you mean RETURN * instead of RETURN?"
   - "Aggregate functions require GROUP BY or single-row result"
   - "HAVING filters aggregated results; use WHERE for row filtering"
   - "ORDER BY expression must match RETURN item or grouping key"
   - "COUNT(*) does not accept DISTINCT quantifier"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing clauses, point to where clause expected
   - For malformed items, highlight entire item
   - For invalid keywords, highlight keyword token

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing RETURN statement..."
   - "In GROUP BY clause starting at line 42..."
   - "While parsing aggregate function..."

### Performance Considerations

1. **Result Clause Parsing Efficiency**: Result clauses are frequent:
   - Use efficient lookahead (1-2 tokens typically sufficient)
   - Minimize backtracking
   - Use direct dispatch to clause parsers

2. **List Parsing**: Use efficient comma-separated list parsing:
   - Single-pass parsing
   - Clear termination conditions
   - Avoid unnecessary allocations

3. **Expression Reuse**: Reuse expression parser from Sprint 5:
   - Don't duplicate expression parsing logic
   - Leverage existing expression performance

4. **AST Allocation**: Minimize allocations:
   - Use `SmolStr` for inline storage of short strings
   - Use `Vec` with appropriate capacity hints where possible
   - Consider arena allocation for AST nodes (future optimization)

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (result keywords, operators)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure; integration testing infrastructure
- **Sprint 5**: Expression parsing for return items, sort keys, grouping elements, HAVING predicates, aggregate arguments, LIMIT/OFFSET counts
- **Sprint 6**: Type system (for future semantic validation of aggregates)
- **Sprint 7**: Query pipeline structure; result statements integrate with queries
- **Sprint 8**: Pattern matching; complete MATCH...RETURN queries

### Dependencies on Future Sprints

- **Sprint 10**: Data modification will return modified elements (integration with RETURN)
- **Sprint 11**: Procedures may return results (integration with RETURN/YIELD)
- **Sprint 12**: Graph type specifications (not directly related)
- **Sprint 13**: Conformance hardening (stress testing result shaping)
- **Sprint 14**: Semantic validation (aggregate context, grouping consistency, expression scoping)

### Cross-Sprint Integration Points

- Result shaping is the final stage of query processing
- RETURN integrates with MATCH (Sprint 8) for graph queries
- GROUP BY and aggregates work with expression system (Sprint 5)
- Result clauses integrate with query pipeline (Sprint 7)
- SELECT statements (Sprint 7) use same result shaping features
- Aggregate functions extend expression system (Sprint 5)
- Semantic validation (Sprint 14) will check:
  - Aggregate context (GROUP BY requirements)
  - HAVING vs WHERE appropriateness
  - Return item scoping
  - Expression validity in different clauses

## Test Strategy

### Unit Tests

For each result component:
1. **Happy Path**: Valid clauses parse correctly
2. **Variants**: All syntax variants and optional components
3. **Error Cases**: Missing components, invalid syntax, malformed clauses
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Result shaping in different contexts:
1. **Query Integration**: RETURN in MATCH queries, SELECT statements
2. **Clause Combinations**: All valid clause orderings and combinations
3. **Complex Expressions**: Result clauses with complex expressions from Sprint 5
4. **Aggregate Integration**: GROUP BY with HAVING and aggregates
5. **Complete Queries**: End-to-end queries with all result features

### Snapshot Tests

Capture AST output:
1. Representative result clauses from each category
2. Complex queries with all result shaping features
3. Aggregate functions in various contexts
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid result clauses
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries:
1. Official GQL sample queries with result shaping
2. Real-world graph queries with aggregates
3. Verify parser handles production syntax

### Performance Tests

1. **Long Item Lists**: Many return items, grouping elements, sort keys
2. **Complex Expressions**: Nested expressions in result clauses
3. **Aggregate Heavy**: Queries with many aggregate functions

## Performance Considerations

1. **Lexer Efficiency**: Result keywords are frequent; lexer must be fast
2. **Parser Efficiency**: Use direct dispatch and minimal lookahead
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Expression Reuse**: Leverage Sprint 5 expression parser performance

## Documentation Requirements

1. **API Documentation**: Rustdoc for all result shaping AST nodes and parser functions
2. **Result Shaping Overview**: Document result production, grouping, aggregation, ordering semantics
3. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
4. **Examples**: Demonstrate result shaping in examples
5. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Aggregate function complexity | Medium | Medium | Clear AST design; thorough testing; separate aggregate parsing; leverage Sprint 5 expression parser |
| Clause ordering ambiguity | Medium | Low | Follow ISO GQL grammar ordering strictly; document ordering rules; test all valid combinations |
| Integration with Sprint 7 queries | Medium | Low | Careful integration testing; preserve Sprint 7 query tests; add result shaping incrementally |
| HAVING vs WHERE confusion | Low | Medium | Clear error messages; documentation explains distinction; semantic validation in Sprint 14 |
| Set quantifier context confusion | Medium | Low | Clear AST design distinguishing RETURN DISTINCT from aggregate DISTINCT; thorough testing |
| Expression parser integration issues | Low | Low | Sprint 5 expression parser is stable; use consistently throughout; test expression contexts |
| Performance on complex result clauses | Low | Low | Optimize hot paths; use efficient list parsing; profile and optimize if needed |

## Success Metrics

1. **Coverage**: All result shaping features parse with correct AST
2. **Correctness**: Result semantics match ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for result shaping parser
6. **Performance**: Parser handles queries with 50+ result items/sort keys in <1ms
7. **Integration**: Result shaping integrates cleanly with Sprint 5 (expressions) and Sprint 7 (queries)
8. **Completeness**: All aggregate functions work; all result clauses compose correctly

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping, result shaping overview)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Result shaping tested in multiple contexts (RETURN, SELECT, with various query types)
- [ ] AST design reviewed for stability and extensibility
- [ ] Sprint 5 integration complete (expressions in result clauses)
- [ ] Sprint 7 integration complete (result shaping in queries)
- [ ] Sprint 8 integration complete (MATCH...RETURN queries)
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 10: Data Modification Statements** will build on the query foundation to implement graph mutation features including INSERT graph patterns, SET, REMOVE, and [DETACH|NODETACH] DELETE operations. With querying and result shaping complete, Sprint 10 adds the ability to modify graph data.

---

## Appendix: Result Shaping Clause Hierarchy

```
ReturnStatement
├── ReturnStatementBody
│   ├── SetQuantifier (optional)
│   │   ├── All
│   │   └── Distinct
│   ├── ReturnItemListOrAsterisk
│   │   ├── Asterisk (*)
│   │   └── ItemList(ReturnItemList)
│   │       └── Vec<ReturnItem>
│   │           ├── expression: Expression (from Sprint 5)
│   │           └── alias: Option<ReturnItemAlias>
│   └── GroupByClause (optional)
│       └── Vec<GroupingElement>
│           ├── Expression(Expression)
│           └── EmptyGroupingSet (())
└── OrderByAndPageStatement (optional)
    ├── OrderByClause (optional)
    │   └── Vec<SortSpecification>
    │       ├── key: SortKey (Expression from Sprint 5)
    │       ├── ordering: Option<OrderingSpecification>
    │       │   ├── Ascending (ASC/ASCENDING)
    │       │   └── Descending (DESC/DESCENDING)
    │       └── null_ordering: Option<NullOrdering>
    │           ├── NullsFirst
    │           └── NullsLast
    ├── OffsetClause (optional)
    │   └── count: Expression (from Sprint 5)
    └── LimitClause (optional)
        └── count: Expression (from Sprint 5)

HavingClause (in SELECT)
└── condition: Expression (from Sprint 5)

AggregateFunction (extends Expression from Sprint 5)
├── CountStar (COUNT(*))
├── GeneralSetFunction
│   ├── function_type: GeneralSetFunctionType
│   │   ├── Avg (AVG)
│   │   ├── Count (COUNT)
│   │   ├── Max (MAX)
│   │   ├── Min (MIN)
│   │   ├── Sum (SUM)
│   │   ├── CollectList (COLLECT_LIST)
│   │   ├── StddevSamp (STDDEV_SAMP)
│   │   └── StddevPop (STDDEV_POP)
│   ├── quantifier: Option<SetQuantifier> (DISTINCT/ALL)
│   └── expression: Expression
└── BinarySetFunction
    ├── function_type: BinarySetFunctionType
    │   ├── PercentileCont (PERCENTILE_CONT)
    │   └── PercentileDisc (PERCENTILE_DISC)
    ├── inverse_distribution_argument: Expression
    └── expression: Expression
```

---

## Appendix: Result Shaping Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `primitiveResultStatement` | 660 | `PrimitiveResultStatement` enum | `parse_primitive_result_statement()` |
| `returnStatement` | 667 | `ReturnStatement` struct | `parse_return_statement()` |
| `returnStatementBody` | 671 | `ReturnStatementBody` struct | `parse_return_statement_body()` |
| `returnItemList` | 675 | `ReturnItemList` struct | `parse_return_item_list()` |
| `returnItem` | 679 | `ReturnItem` struct | `parse_return_item()` |
| `returnItemAlias` | 683 | `ReturnItemAlias` struct | `parse_return_item_alias()` |
| `orderByAndPageStatement` | 652 | `OrderByAndPageStatement` struct | `parse_order_by_and_page_statement()` |
| `orderByClause` | 1332 | `OrderByClause` struct | `parse_order_by_clause()` |
| `sortSpecification` | 1342 | `SortSpecification` struct | `parse_sort_specification()` |
| `sortKey` | 1346 | `SortKey` struct | `parse_sort_key()` |
| `orderingSpecification` | 1350 | `OrderingSpecification` enum | `parse_ordering_specification()` |
| `nullOrdering` | 1357 | `NullOrdering` enum | `parse_null_ordering()` |
| `limitClause` | 1364 | `LimitClause` struct | `parse_limit_clause()` |
| `offsetClause` | 1370 | `OffsetClause` struct | `parse_offset_clause()` |
| `groupByClause` | 1313 | `GroupByClause` struct | `parse_group_by_clause()` |
| `groupingElement` | 1322 | `GroupingElement` enum | `parse_grouping_element()` |
| `emptyGroupingSet` | 1326 | `EmptyGroupingSet` struct | `parse_empty_grouping_set()` |
| `havingClause` | 705 | `HavingClause` struct | `parse_having_clause()` |
| `aggregateFunction` | 2380 | `AggregateFunction` enum | `parse_aggregate_function()` |
| `generalSetFunction` | 2386 | `GeneralSetFunction` struct | `parse_general_set_function()` |
| `generalSetFunctionType` | 2394 | `GeneralSetFunctionType` enum | (dispatch in `parse_general_set_function()`) |
| `binarySetFunction` | 2390 | `BinarySetFunction` struct | `parse_binary_set_function()` |
| `binarySetFunctionType` | 2410 | `BinarySetFunctionType` enum | (dispatch in `parse_binary_set_function()`) |
| `setQuantifier` | 2405 | `SetQuantifier` enum | `parse_set_quantifier()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-18
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4, 5, 6, 7, 8 (completed or required)
**Next Sprint**: Sprint 10 (Data Modification Statements)
