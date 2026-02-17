# GQL Language Feature Overview

> **Purpose**: This document provides a comprehensive high-level overview of all features in the GQL (Graph Query Language) specification, as defined by the ISO standard (ISO/IEC 39075:2024). This overview will serve as the foundation for planning implementation sprints for a full feature-complete GQL parser.
>
> **Grammar Reference**: All features are based on the ANTLR4 grammar located at `third_party/opengql-grammar/GQL.g4` (3774 lines, 571 parser rules plus lexer/token rules).

## Table of Contents

1. [Program Structure & Execution Model](#1-program-structure--execution-model)
2. [Session Management](#2-session-management)
3. [Transaction Management](#3-transaction-management)
4. [Catalog & Schema Management](#4-catalog--schema-management)
5. [Query Features](#5-query-features)
6. [Graph Pattern Matching](#6-graph-pattern-matching)
7. [Path Patterns & Quantifiers](#7-path-patterns--quantifiers)
8. [Data Modification Operations](#8-data-modification-operations)
9. [Result & Output Features](#9-result--output-features)
10. [Ordering, Pagination & Grouping](#10-ordering-pagination--grouping)
11. [Aggregation Functions](#11-aggregation-functions)
12. [Procedure Calls](#12-procedure-calls)
13. [Label Expressions](#13-label-expressions)
14. [Type System](#14-type-system)
15. [Predicates & Conditions](#15-predicates--conditions)
16. [Value Expressions](#16-value-expressions)
17. [Built-in Functions](#17-built-in-functions)
18. [Literals & Constants](#18-literals--constants)
19. [Variables, Parameters & References](#19-variables-parameters--references)
20. [Graph Type Specification](#20-graph-type-specification)
21. [Reserved & Non-Reserved Keywords](#21-reserved--non-reserved-keywords)

---

## 1. Program Structure & Execution Model

**Grammar Reference**: Lines 7-26, 138-196

### Core Structure
- **GQL Program** (`gqlProgram`, Line 7): Top-level entry point for GQL programs
  - Program activity (session or transaction)
  - Optional session close command
  - EOF termination

### Program Activities
- **Session Activity** (`sessionActivity`, Line 17): Session-level operations
- **Transaction Activity** (`transactionActivity`, Line 22): Transaction-scoped operations with procedures

### Procedure Specifications
- **Procedure Specification** (`procedureSpecification`, Line 145): Complete procedure body structure
- **Nested Procedures** (`nestedProcedureSpecification`, Line 138): Block-scoped procedures
  - Nested data modifying procedures (Line 156)
  - Nested query specifications (Line 164)
- **Procedure Body** (`procedureBody`, Line 174): AT schema clause, variable definitions, statement blocks
- **Statement Block** (`statementBlock`, Line 188): Sequential statement execution
- **Next Statement** (`nextStatement`, Line 198): NEXT with yield clause

---

## 2. Session Management

**Grammar Reference**: Lines 33-101

### Session Set Commands
**Grammar Rule**: `sessionSetCommand` (Line 35)

- **Set Schema** (`sessionSetSchemaClause`, Line 39): `SESSION SET SCHEMA <schema_reference>`
  - Set the active schema for the session

- **Set Graph** (`sessionSetGraphClause`, Line 43): `SESSION SET [PROPERTY] GRAPH <graph_expression>`
  - Set the active property graph for the session

- **Set Time Zone** (`sessionSetTimeZoneClause`, Line 47): `SESSION SET TIME ZONE <timezone_string>`
  - Configure session timezone

- **Set Parameters** (`sessionSetParameterClause`, Line 55):
  - **Graph Parameters** (`sessionSetGraphParameterClause`, Line 61): Set graph variables
  - **Binding Table Parameters** (`sessionSetBindingTableParameterClause`, Line 65): Set binding table variables
  - **Value Parameters** (`sessionSetValueParameterClause`, Line 69): Set value variables

### Session Reset Commands
**Grammar Rule**: `sessionResetCommand` (Line 79)

- **Reset Arguments** (`sessionResetArguments`, Line 83):
  - Reset all session settings
  - Reset parameters
  - Reset characteristics
  - Reset schema
  - Reset graph
  - Reset time zone

### Session Close Commands
**Grammar Rule**: `sessionCloseCommand` (Line 93)

- Close the current session

---

## 3. Transaction Management

**Grammar Reference**: Lines 103-134

### Transaction Control

- **Start Transaction** (`startTransactionCommand`, Line 105):
  - `START TRANSACTION [<transaction_characteristics>]`
  - Begin a new transaction with optional characteristics

- **Transaction Characteristics** (`transactionCharacteristics`, Line 111):
  - **Transaction Mode** (`transactionMode`, Line 115): Mode specification
  - **Transaction Access Mode** (`transactionAccessMode`, Line 119):
    - `READ ONLY`: Read-only transactions
    - `READ WRITE`: Read-write transactions

- **Commit Transaction** (`commitCommand`, Line 132):
  - `COMMIT [WORK]`
  - Commit the current transaction

- **Rollback Transaction** (`rollbackCommand`, Line 126):
  - `ROLLBACK [WORK]`
  - Rollback the current transaction

---

## 4. Catalog & Schema Management

**Grammar Reference**: Lines 279-367

### Schema Operations

- **Create Schema** (`createSchemaStatement`, Line 301):
  - `CREATE [OR REPLACE] SCHEMA [IF NOT EXISTS] <schema_name>`
  - Create new schemas in the catalog

- **Drop Schema** (`dropSchemaStatement`, Line 307):
  - `DROP SCHEMA [IF EXISTS] <schema_name>`
  - Remove schemas from the catalog

### Graph Operations

- **Create Graph** (`createGraphStatement`, Line 313):
  - `CREATE [OR REPLACE] [PROPERTY] GRAPH [IF NOT EXISTS] <graph_name>`
  - **Graph Type Specifications** (`openGraphType`, Line 317): Any graph type
  - **Typed Graphs** (`ofGraphType`, Line 321): `OF <graph_type>`
  - **Like Clause** (`graphTypeLikeGraph`, Line 327): `LIKE <graph_reference>`
  - **Copy Semantics** (`graphSource`, Line 331): `AS COPY OF <graph_reference>`

- **Drop Graph** (`dropGraphStatement`, Line 337):
  - `DROP [PROPERTY] GRAPH [IF EXISTS] <graph_name>`

### Graph Type Operations

- **Create Graph Type** (`createGraphTypeStatement`, Line 343):
  - `CREATE [OR REPLACE] [PROPERTY] GRAPH TYPE [IF NOT EXISTS] <graph_type_name>`
  - **Type Source** (`graphTypeSource`, Line 347): Type specification
  - **Copy Of Type** (`copyOfGraphType`, Line 353): `AS COPY OF <graph_type_reference>`

- **Drop Graph Type** (`dropGraphTypeStatement`, Line 359):
  - `DROP [PROPERTY] GRAPH TYPE [IF EXISTS] <graph_type_name>`

### Catalog Procedure Calls

- **Call Catalog Modifying Procedure** (`callCatalogModifyingProcedureStatement`, Line 365):
  - Invoke catalog management procedures

---

## 5. Query Features

**Grammar Reference**: Lines 496-725

### Composite Query Operations

- **Composite Query Statement** (`compositeQueryStatement`, Line 498):
  - Top-level query structure
  - Supports nested queries and subqueries

- **Set Operators** (`setOperator`, Line 514):
  - **UNION**: Combine results from multiple queries
  - **EXCEPT**: Return elements in first query but not in second
  - **INTERSECT**: Return common elements from queries
  - **Set Quantifiers**: `ALL` or `DISTINCT` (default)

- **Query Conjunction** (`queryConjunction`, Line 509):
  - Combine multiple query expressions with set operators
  - Supports `OTHERWISE` fallback composition in addition to set operators

### Linear Query Statements

- **Linear Query Statement** (`linearQueryStatement`, Line 526):
  - Sequential query processing pipeline

- **Focused Linear Query** (`focusedLinearQueryStatement`, Line 531):
  - With `USE GRAPH` clause
  - Explicitly specify working graph

- **Ambient Linear Query** (`ambientLinearQueryStatement`, Line 554):
  - Without explicit `USE GRAPH` clause
  - Uses session default graph

- **Simple Linear Query** (`simpleLinearQueryStatement`, Line 559):
  - Chain of primitive query operations

### Primitive Query Statements

**Grammar Rule**: `primitiveQueryStatement` (Line 568)

- **MATCH Statement** (Line 569): Graph pattern matching
- **LET Statement** (Line 570): Variable binding
- **FOR Statement** (Line 571): Iteration over collections
- **FILTER Statement** (Line 572): Filtering conditions
- **ORDER BY and PAGINATION** (Line 573): Result ordering and limiting

### Match Statements

**Grammar Reference**: Lines 578-599

- **Simple Match** (`simpleMatchStatement`, Line 583):
  - `MATCH <graph_pattern>`
  - Basic graph pattern matching

- **Optional Match** (`optionalMatchStatement`, Line 587):
  - `OPTIONAL <optional_operand>`
  - Left outer join semantics
  - Returns NULL for unmatched patterns

- **Optional Operand Forms** (`optionalOperand`, Line 591):
  - `MATCH <graph_pattern>`
  - `{ <match_statement>+ }`
  - `( <match_statement>+ )`

- **Match Statement Block** (`matchStatementBlock`, Line 597):
  - Multiple match statements in a block
  - `{ <match_statement>+ }`

### Filter Statements

**Grammar Reference**: Lines 609-611

- **Filter Statement** (`filterStatement`, Line 609):
  - `FILTER [WHERE] <search_condition>`
  - Filter results based on predicates

### Let Statements

**Grammar Reference**: Lines 615-626

- **Let Statement** (`letStatement`, Line 615):
  - `LET <variable_definition_list>`
  - Define variables with computed values

- **Let Variable Definition** (`letVariableDefinition`, Line 623):
  - Bind expressions to variables
  - Support for multiple simultaneous bindings

### For Statements

**Grammar Reference**: Lines 630-648

- **For Statement** (`forStatement`, Line 630):
  - `FOR <for_item>`
  - Iterate over collections/bindings

- **For Item** (`forItem`, Line 634):
  - Mandatory alias binding form: `<binding_variable> IN <value_expression>`

- **For Ordinality/Offset** (`forOrdinalityOrOffset`, Line 646):
  - `WITH ORDINALITY`: Include position in iteration
  - `WITH OFFSET`: Include offset value

### Select Statements

**Grammar Reference**: Lines 689-724

- **Select Statement** (`selectStatement`, Line 689):
  - Full SQL-style SELECT syntax
  - `SELECT [DISTINCT|ALL] <select_items> [FROM ...] [WHERE ...] [GROUP BY ...] [HAVING ...] [ORDER BY ...] [OFFSET ...] [LIMIT ...]`

- **Select Items** (`selectItem`, Line 697):
  - Column/expression selection
  - Computed expressions with aliases

- **Select Graph Match List** (`selectGraphMatchList`, Line 713):
  - FROM clause for graph matching
  - Multiple graph pattern sources

- **Select Query Specification** (`selectQuerySpecification`, Line 721):
  - `FROM <nested_query_specification>`
  - `FROM <graph_expression> <nested_query_specification>`
  - SELECT over nested query bodies with optional graph context

- **Having Clause** (`havingClause`, Line 705):
  - `HAVING <search_condition>`
  - Filter aggregated results

---

## 6. Graph Pattern Matching

**Grammar Reference**: Lines 777-1147

### Graph Pattern Binding

- **Graph Pattern Binding Table** (`graphPatternBindingTable`, Line 779):
  - Bind pattern matches to binding tables

- **Graph Pattern Yield Clause** (`graphPatternYieldClause`, Line 783):
  - `YIELD <yield_items>`
  - Select specific variables from pattern matches

### Graph Patterns

**Grammar Reference**: Lines 803-848

- **Graph Pattern** (`graphPattern`, Line 803):
  - Main pattern matching structure
  - Combines path patterns with where clauses

- **Match Modes** (`matchMode`, Line 807):
  - **Repeatable Elements** (`repeatableElementsMatchMode`, Line 812):
    - `REPEATABLE ELEMENTS`: Allow same element in multiple matches
  - **Different Edges** (`differentEdgesMatchMode`, Line 816):
    - `DIFFERENT EDGES`: Ensure edge uniqueness in matches

- **Path Pattern List** (`pathPatternList`, Line 830):
  - Comma-separated list of path patterns

- **Path Variable Declaration** (`pathVariableDeclaration`, Line 838):
  - Declare variables for matched paths

- **Keep Clause** (`keepClause`, Line 842):
  - `KEEP <path_pattern_prefix>`
  - Retain specific path patterns in results

- **Graph Pattern Where Clause** (`graphPatternWhereClause`, Line 846):
  - `WHERE <search_condition>`
  - Filter pattern matches

---

## 7. Path Patterns & Quantifiers

**Grammar Reference**: Lines 898-1146

### Path Pattern Prefix

**Grammar Reference**: Lines 898-962

- **Path Pattern Prefix** (`pathPatternPrefix`, Line 898):
  - Modify path matching behavior

- **Path Mode Prefix** (`pathModePrefix`, Line 903):
  - **Path Mode** (`pathMode`, Line 907):
    - `WALK`: Any path (default)
    - `TRAIL`: No repeated edges
    - `SIMPLE`: No repeated nodes or edges
    - `ACYCLIC`: No repeated nodes

- **Path Search Prefix** (`pathSearchPrefix`, Line 914):
  - **All Path Search** (`allPathSearch`, Line 920):
    - `ALL [<path_mode>] [PATHS]`
    - Find all matching paths

  - **Any Path Search** (`anyPathSearch`, Line 929):
    - `ANY [<path_mode>]`
    - Find any matching path (non-deterministic)

  - **Shortest Path Search** (`shortestPathSearch`, Line 937):
    - **All Shortest** (`allShortestPathSearch`, Line 944):
      - `ALL SHORTEST [<path_mode>]`
      - Find all shortest paths

    - **Any Shortest** (`anyShortestPathSearch`, Line 948):
      - `ANY SHORTEST [<path_mode>]`
      - Find any shortest path

    - **Counted Shortest** (`countedShortestPathSearch`, Line 952):
      - `SHORTEST <k> [<path_mode>] [PATHS]`
      - Find top k shortest paths

    - **Shortest Groups** (`countedShortestGroupSearch`, Line 956):
      - `SHORTEST <k> [<path_mode>] GROUPS`
      - Find top k shortest path groups

### Path Pattern Expressions

**Grammar Reference**: Lines 966-986

- **Path Pattern Expression** (`pathPatternExpression`, Line 966):
  - Complex path expressions with operators
  - **Alternation**: `|` - Match either pattern
  - **Union**: Path union operations

- **Path Term** (`pathTerm`, Line 972):
  - Individual path components

- **Path Factor** (`pathFactor`, Line 976):
  - Path with quantifiers
  - Supports repetition and optionality

- **Path Primary** (`pathPrimary`, Line 982):
  - Element patterns
  - Parenthesized sub-patterns

### Element Patterns

**Grammar Reference**: Lines 988-1086

- **Element Pattern** (`elementPattern`, Line 988):
  - Node or edge pattern

- **Node Pattern** (`nodePattern`, Line 993):
  - `(<variable> [:label_expression] [{property_specification}] [WHERE predicate])`
  - Match nodes with labels and properties

- **Element Pattern Filler** (`elementPatternFiller`, Line 997):
  - Variable declaration
  - Label expression
  - Property specification
  - WHERE clause

- **Element Variable Declaration** (`elementVariableDeclaration`, Line 1001):
  - Declare element variables in patterns

- **Is Label Expression** (`isLabelExpression`, Line 1005):
  - `:label_name` or `:label_expression`
  - Match elements by label

- **Element Pattern Predicate** (`elementPatternPredicate`, Line 1014):
  - Additional predicates on elements

- **Element Property Specification** (`elementPropertySpecification`, Line 1023):
  - `{prop1: value1, prop2: value2, ...}`
  - Match by property values

### Edge Patterns

**Grammar Reference**: Lines 1035-1082

- **Edge Pattern** (`edgePattern`, Line 1035):
  - Full or abbreviated edge syntax

- **Full Edge Pattern** (`fullEdgePattern`, Line 1040):
  - **7 Direction Types** (Lines 1050-1076):
    1. `<-[edge]-`: Left pointing
    2. `-[edge]->`: Right pointing
    3. `~[edge]~`: Undirected
    4. `<-[edge]->`: Any direction (bidirectional)
    5. `<~[edge]~>`: Any undirected
    6. `-[edge]-`: Any direction
    7. `~[edge]-`: Mixed directed/undirected

- **Abbreviated Edge Pattern** (`abbreviatedEdgePattern`, Line 1078):
  - `<-`: Left arrow
  - `->`: Right arrow
  - `~`: Undirected
  - `-`: Any direction

- **Parenthesized Path Pattern** (`parenthesizedPathPatternExpression`, Line 1088):
  - `(<path_pattern_expression>)`
  - Group patterns for precedence

### Graph Pattern Quantifiers

**Grammar Reference**: Lines 1125-1146

- **Graph Pattern Quantifier** (`graphPatternQuantifier`, Line 1125):
  - **`*`**: Zero or more (Kleene star)
  - **`+`**: One or more (Kleene plus)
  - **`?`**: Zero or one (optional)
  - **`{n}`**: Exactly n occurrences (`fixedQuantifier`, Line 1132)
  - **`{n,m}`**: Between n and m occurrences (`generalQuantifier`, Line 1136)
  - **`{n,}`**: At least n occurrences
  - **`{,m}`**: At most m occurrences

### Simplified Path Patterns

**Grammar Reference**: Lines 1150-1281

Alternative, simplified syntax for path patterns:

- **Simplified Path Pattern Expression** (`simplifiedPathPatternExpression`, Line 1150):
  - All 7 direction types with simplified syntax

- **Simplified Contents** (`simplifiedContents`, Line 1188):
  - Simplified pattern content

- **Simplified Path Union** (`simplifiedPathUnion`, Line 1194):
  - Union of simplified paths

- **Simplified Multiset Alternation** (`simplifiedMultisetAlternation`, Line 1198):
  - `|+|` operator for multiset alternation

- **Simplified Quantified** (`simplifiedQuantified`, Line 1218):
  - Quantifiers in simplified syntax

- **Simplified Questioned** (`simplifiedQuestioned`, Line 1222):
  - Optional patterns in simplified syntax

- **Simplified Direction Override** (`simplifiedDirectionOverride`, Line 1231):
  - Override edge direction

- **Simplified Negation** (`simplifiedNegation`, Line 1274):
  - Negated patterns

---

## 8. Data Modification Operations

**Grammar Reference**: Lines 369-494

### Linear Data Modifying Statements

- **Focused Linear Data Modifying Statement** (`focusedLinearDataModifyingStatement`, Line 376):
  - With `USE GRAPH` clause

- **Ambient Linear Data Modifying Statement** (`ambientLinearDataModifyingStatement`, Line 389):
  - Without explicit graph specification

### Primitive Data Modifying Statements

**Grammar Rule**: `primitiveDataModifyingStatement` (Line 412)

- INSERT statement (Line 413)
- SET statement (Line 414)
- REMOVE statement (Line 415)
- DELETE statement (Line 416)

### Insert Operations

**Grammar Reference**: Lines 421-423, 852-894

- **Insert Statement** (`insertStatement`, Line 421):
  - `INSERT <insert_graph_pattern>`
  - Create new nodes and edges

- **Insert Graph Pattern** (`insertGraphPattern`, Line 852):
  - Specify nodes and edges to insert

- **Insert Path Pattern** (`insertPathPattern`, Line 860):
  - Path-based insertion

- **Insert Node Pattern** (`insertNodePattern`, Line 864):
  - `(<variable> [:label_expression] {properties})`
  - Create nodes with labels and properties

- **Insert Edge Pattern** (`insertEdgePattern`, Line 868):
  - **Pointing Left** (Line 872): `<-[edge]-`
  - **Pointing Right** (Line 874): `-[edge]->`
  - **Undirected** (Line 876): `~[edge]~`

- **Insert Element Pattern Filler** (`insertElementPatternFiller`, Line 886):
  - Variable declaration
  - Label expression
  - Property specification

### Set Operations

**Grammar Reference**: Lines 427-451

- **Set Statement** (`setStatement`, Line 427):
  - `SET <set_item_list>`
  - Update properties and labels

- **Set Item List** (`setItemList`, Line 431):
  - Multiple set operations

- **Set Item Types** (`setItem`, Line 435):
  - **Set Property** (`setPropertyItem`, Line 441):
    - `variable.property = value`
    - Update specific property

  - **Set All Properties** (`setAllPropertiesItem`, Line 445):
    - `variable = {properties}`
    - Replace all properties

  - **Set Label** (`setLabelItem`, Line 449):
    - `variable :label_name`
    - `variable IS label_name`
    - Add label to element

### Remove Operations

**Grammar Reference**: Lines 455-474

- **Remove Statement** (`removeStatement`, Line 455):
  - `REMOVE <remove_item_list>`
  - Remove properties and labels

- **Remove Item Types** (`removeItem`, Line 463):
  - **Remove Property** (`removePropertyItem`, Line 468):
    - `variable.property`
    - Delete specific property

  - **Remove Label** (`removeLabelItem`, Line 472):
    - `variable :label_name`
    - `variable IS label_name`
    - Remove label from element

### Delete Operations

**Grammar Reference**: Lines 478-488

- **Delete Statement** (`deleteStatement`, Line 478):
  - `[DETACH | NODETACH] DELETE <delete_item_list>`
  - Delete nodes and edges

- **Delete Options**:
  - **DETACH**: Delete node and all connected edges
  - **NODETACH** (default): Only delete if no edges remain

- **Delete Item** (`deleteItem`, Line 486):
  - Element variables to delete

### Data Modifying Procedure Calls

**Grammar Reference**: Lines 492-494

- **Call Data Modifying Procedure** (`callDataModifyingProcedureStatement`, Line 492):
  - Invoke procedures that modify data

---

## 9. Result & Output Features

**Grammar Reference**: Lines 658-685

### Primitive Result Statements

**Grammar Rule**: `primitiveResultStatement` (Line 660)

- RETURN statement (Line 661)
- FINISH statement (Line 662)

### Return Statements

**Grammar Reference**: Lines 667-685

- **Return Statement** (`returnStatement`, Line 667):
  - `RETURN [DISTINCT|ALL] (* | <return_item_list>) [GROUP BY ...]`
  - Return results from query

- **Return Statement Body** (`returnStatementBody`, Line 671):
  - Set quantifier (DISTINCT/ALL)
  - `*` or return item list
  - Optional `GROUP BY` clause

- **Return Item List** (`returnItemList`, Line 675):
  - Items to return in result set

- **Return Item** (`returnItem`, Line 679):
  - Expression to return
  - Computed values

- **Return Item Alias** (`returnItemAlias`, Line 683):
  - `AS alias_name`
  - Name result columns

---

## 10. Ordering, Pagination & Grouping

**Grammar Reference**: Lines 650-1377

### Combined Ordering and Pagination

- **Order By and Page Statement** (`orderByAndPageStatement`, Line 652):
  - Combined statement for ordering and pagination

### Order By Clause

**Grammar Reference**: Lines 1332-1360

- **Order By Clause** (`orderByClause`, Line 1332):
  - `ORDER BY <sort_specification_list>`
  - Sort query results

- **Sort Specification** (`sortSpecification`, Line 1342):
  - Individual sort key with direction

- **Sort Key** (`sortKey`, Line 1346):
  - Expression to sort by

- **Ordering Specification** (`orderingSpecification`, Line 1350):
  - `ASC` / `ASCENDING`: Ascending order
  - `DESC` / `DESCENDING`: Descending order

- **Null Ordering** (`nullOrdering`, Line 1357):
  - `NULLS FIRST`: NULL values first
  - `NULLS LAST`: NULL values last

### Limit and Offset

**Grammar Reference**: Lines 1364-1377

- **Limit Clause** (`limitClause`, Line 1364):
  - `LIMIT <n>`
  - Restrict number of results

- **Offset Clause** (`offsetClause`, Line 1370):
  - `OFFSET <n>` or `SKIP <n>`
  - Skip first n results

### Group By Clause

**Grammar Reference**: Lines 1313-1328

- **Group By Clause** (`groupByClause`, Line 1313):
  - `GROUP BY <grouping_element_list>`
  - Aggregate results by grouping keys

- **Grouping Element** (`groupingElement`, Line 1322):
  - Expression to group by

- **Empty Grouping Set** (`emptyGroupingSet`, Line 1326):
  - `()`: Single aggregated result

---

## 11. Aggregation Functions

**Grammar Reference**: Lines 2380-2421

### Aggregate Function Types

- **Aggregate Function** (`aggregateFunction`, Line 2380):
  - `COUNT(*)`
  - General and binary set functions

### General Set Functions

**Grammar Rule**: `generalSetFunction` (Line 2386)

- **General Set Function Type** (`generalSetFunctionType`, Line 2394):
  - **`AVG`** (Line 2395): Average of values
  - **`COUNT`** (Line 2396): Count of values
  - **`MAX`** (Line 2397): Maximum value
  - **`MIN`** (Line 2398): Minimum value
  - **`SUM`** (Line 2399): Sum of values
  - **`COLLECT_LIST`** (Line 2400): Collect into list
  - **`STDDEV_SAMP`** (Line 2401): Sample standard deviation
  - **`STDDEV_POP`** (Line 2402): Population standard deviation

### Binary Set Functions

**Grammar Rule**: `binarySetFunction` (Line 2390)

- **Binary Set Function Type** (`binarySetFunctionType`, Line 2410):
  - **`PERCENTILE_CONT`** (Line 2411): Continuous percentile
  - **`PERCENTILE_DISC`** (Line 2412): Discrete percentile

### Set Quantifiers

**Grammar Rule**: `setQuantifier` (Line 2405)

- **`DISTINCT`**: Remove duplicates before aggregation
- **`ALL`**: Include all values (default)

---

## 12. Procedure Calls

**Grammar Reference**: Lines 727-763

### Call Statements

- **Call Procedure Statement** (`callProcedureStatement`, Line 728):
  - `[OPTIONAL] CALL <procedure_call>`
  - Invoke named or inline procedures

- **Optional Procedures**:
  - `OPTIONAL CALL`: Continue execution even if procedure fails

### Procedure Call Types

- **Procedure Call** (`procedureCall`, Line 732):
  - Inline or named procedure invocation

- **Inline Procedure Call** (`inlineProcedureCall`, Line 739):
  - Execute procedure defined inline
  - Nested procedure specification

- **Variable Scope Clause** (`variableScopeClause`, Line 743):
  - Control variable visibility
  - Bind input/output variables

- **Named Procedure Call** (`namedProcedureCall`, Line 753):
  - `procedure_name([arguments]) [YIELD <yield_item_list>]`
  - Call stored procedures

- **Procedure Argument List** (`procedureArgumentList`, Line 757):
  - Arguments passed to procedure

- **Procedure Argument** (`procedureArgument`, Line 761):
  - Individual argument value

### Procedure and Query Context Clauses

**Grammar Reference**: Lines 767-777

- **AT Schema Clause** (`atSchemaClause`, Line 767):
  - `AT <schema_reference>`
  - Sets schema context for a procedure body

- **Use Graph Clause** (`useGraphClause`, Line 773):
  - `USE <graph_expression>`
  - Sets graph context for focused query/data-modifying statements

---

## 13. Label Expressions

**Grammar Reference**: Lines 1102-1109, 1679-1687

### Label Expression Operations

**Grammar Rule**: `labelExpression` (Line 1102)

Boolean algebra over labels:

- **Negation** (Line 1103):
  - `! label_expression`
  - Match elements NOT having label

- **Conjunction** (Line 1104):
  - `label_expression & label_expression`
  - Match elements having both labels (AND)

- **Disjunction** (Line 1105):
  - `label_expression | label_expression`
  - Match elements having either label (OR)

- **Label Name** (Line 1106):
  - Simple label name

- **Wildcard** (Line 1107):
  - `%`: Match any label

- **Parenthesized** (Line 1108):
  - `(label_expression)`: Grouping for precedence

### Label Set Specifications

**Grammar Reference**: Lines 1679-1687

- **Label Set Phrase** (`labelSetPhrase`, Line 1679):
  - `LABEL` or `LABELS` keyword

- **Label Set Specification** (`labelSetSpecification`, Line 1685):
  - Ampersand-separated labels: `label1 & label2 & label3`

---

## 14. Type System

**Grammar Reference**: Lines 1713-1998

### Value Types

**Grammar Rule**: `valueType` (Line 1719)

Union of all type categories:
- Predefined types
- Path value types
- List value types
- Record types
- Dynamic union types (open/closed)

### Predefined Types

**Grammar Rule**: `predefinedType` (Line 1740)

Core type categories:
- Boolean (Line 1741)
- Character string (Line 1742)
- Byte string (Line 1743)
- Numeric (Line 1744)
- Temporal (Line 1745)
- Reference value (Line 1746)
- Immaterial value (Line 1747)

### Boolean Types

**Grammar Reference**: Lines 1750-1752

- **Boolean Type** (`booleanType`, Line 1750):
  - `BOOL` or `BOOLEAN`
  - TRUE, FALSE, UNKNOWN values

### String Types

**Grammar Reference**: Lines 1754-1776

- **Character String Type** (`characterStringType`, Line 1754):
  - `STRING`: Variable-length character string
  - `CHAR(n)`: Fixed-length character string
  - `VARCHAR(n)`: Variable-length with max length

- **Byte String Type** (`byteStringType`, Line 1760):
  - `BYTES`: Variable-length byte string
  - `BINARY(n)`: Fixed-length byte string
  - `VARBINARY(n)`: Variable-length with max length

### Numeric Types

**Grammar Reference**: Lines 1778-1852

- **Numeric Type** (`numericType`, Line 1778):
  - Exact or approximate numeric

#### Exact Numeric Types

**Grammar Rule**: `exactNumericType` (Line 1783)

- **Signed Binary Exact Numeric** (`signedBinaryExactNumericType`, Line 1793):
  - `INT8`, `INT16`, `INT32`, `INT64`, `INT128`, `INT256`
  - `SMALLINT`, `INT` / `INTEGER`, `BIGINT`
  - `SIGNED` variants

- **Unsigned Binary Exact Numeric** (`unsignedBinaryExactNumericType`, Line 1806):
  - `UINT8`, `UINT16`, `UINT32`, `UINT64`, `UINT128`, `UINT256`
  - `USMALLINT`, `UINT`, `UBIGINT`
  - `UNSIGNED` variants

- **Decimal Exact Numeric** (`decimalExactNumericType`, Line 1831):
  - `DECIMAL(p, s)` / `DEC(p, s)`
  - Precision and scale parameters

#### Approximate Numeric Types

**Grammar Rule**: `approximateNumericType` (Line 1843)

- `FLOAT16`, `FLOAT32`, `FLOAT64`, `FLOAT128`, `FLOAT256`
- `FLOAT(p)`
- `REAL`
- `DOUBLE PRECISION`

### Temporal Types

**Grammar Reference**: Lines 1854-1898

- **Temporal Type** (`temporalType`, Line 1854):
  - Instant or duration types

#### Temporal Instant Types

**Grammar Rule**: `temporalInstantType` (Line 1859)

- **Datetime Type** (`datetimeType`, Line 1867):
  - `ZONED DATETIME`: With timezone
  - `TIMESTAMP WITH TIME ZONE`

- **Local Datetime Type** (`localdatetimeType`, Line 1872):
  - `LOCAL DATETIME`: Without timezone
  - `TIMESTAMP [WITHOUT TIME ZONE]`

- **Date Type** (`dateType`, Line 1877):
  - `DATE`: Calendar date

- **Time Type** (`timeType`, Line 1881):
  - `ZONED TIME`: With timezone
  - `TIME WITH TIME ZONE`

- **Local Time Type** (`localtimeType`, Line 1886):
  - `LOCAL TIME`: Without timezone
  - `TIME [WITHOUT TIME ZONE]`

#### Temporal Duration Types

**Grammar Rule**: `temporalDurationType` (Line 1891)

- **Duration Type** (`durationType`, Line 1891):
  - `DURATION`: General duration
  - `DURATION YEAR TO MONTH` (Line 1896): Year-month interval
  - `DURATION DAY TO SECOND` (Line 1897): Day-time interval

### Reference Value Types

**Grammar Reference**: Lines 1900-1962

- **Reference Value Type** (`referenceValueType`, Line 1900):
  - References to graph elements

- **Graph Reference Value Type** (`graphReferenceValueType`, Line 1921):
  - Open form: `ANY [PROPERTY] GRAPH [NOT NULL]`
  - Closed form: `[PROPERTY] GRAPH <nested_graph_type_specification> [NOT NULL]`

- **Binding Table Reference Value Type** (`bindingTableReferenceValueType`, Line 1934):
  - `[BINDING] TABLE <field_types_specification> [NOT NULL]`

- **Node Reference Value Type** (`nodeReferenceValueType`, Line 1938):
  - Open form: `[ANY] NODE [NOT NULL]` (or `VERTEX`)
  - Closed form: `<node_type_specification> [NOT NULL]`

- **Edge Reference Value Type** (`edgeReferenceValueType`, Line 1951):
  - Open form: `[ANY] EDGE [NOT NULL]` (or `RELATIONSHIP`)
  - Closed form: `<edge_type_specification> [NOT NULL]`

### Immaterial Value Types

**Grammar Reference**: Lines 1907-1919

- **Immaterial Value Type** (`immaterialValueType`, Line 1907):
  - Types representing absence

- **Null Type** (`nullType`, Line 1912):
  - `NULL`: Nullable type

- **Empty Type** (`emptyType`, Line 1916):
  - `NULL NOT NULL`: Never null but can be absent
  - `NOTHING`: Empty type

### Path Value Types

**Grammar Reference**: Lines 1964-1966

- **Path Value Type** (`pathValueType`, Line 1964):
  - `PATH`: Represents a path through the graph

### List Value Types

**Grammar Reference**: Lines 1968-1975

- **List Value Type Name** (`listValueTypeName`, Line 1968):
  - `LIST<value_type>`: Ordered list of values
  - `ARRAY<value_type>`: Array (synonym for LIST)
  - `<value_type> LIST`: Postfix list syntax
  - `<value_type> ARRAY`: Postfix array syntax

### Record Types

**Grammar Reference**: Lines 1977-1988

- **Record Type** (`recordType`, Line 1977):
  - `ANY RECORD`: Generic record type
  - `RECORD`: Record with field specifications

- **Field Types Specification** (`fieldTypesSpecification`, Line 1982):
  - Define record field types

- **Field Type** (`fieldType`, Line 1996):
  - `field_name :: type`
  - Individual field with type

### Type Modifiers

**Grammar Reference**: Lines 1735-1738, 1990-1992

- **Typed** (`typed`, Line 1735):
  - `::` operator or `TYPED` keyword
  - Type annotation

- **Not Null** (`notNull`, Line 1990):
  - `NOT NULL`: Non-nullable constraint
  - Enforces value presence

### Binding Table Types

**Grammar Reference**: Lines 1713-1715

- **Binding Table Type** (`bindingTableType`, Line 1713):
  - `BINDING TABLE <type_specification>`
  - Table of variable bindings

---

## 15. Predicates & Conditions

**Grammar Reference**: Lines 2000-2130

### Search Conditions

**Grammar Rule**: `searchCondition` (Line 2002)

- Boolean value expression used for filtering

### Comparison Predicates

**Grammar Rule**: `compOp` (Line 2025)

- `=`: Equal to
- `<>`: Not equal to
- `<`: Less than
- `>`: Greater than
- `<=`: Less than or equal to
- `>=`: Greater than or equal to

### Special Predicates

#### Exists Predicate

**Grammar Rule**: `existsPredicate` (Line 2036)

- `EXISTS { <graph_pattern> }`: Check if pattern exists
- `EXISTS ( <graph_pattern> )`: Parenthesized graph pattern existence
- `EXISTS { <match_statement_block> }`: Block-form match existence
- `EXISTS ( <match_statement_block> )`: Parenthesized match block existence
- `EXISTS <nested_query_specification>`: Nested query existence test

#### Null Predicate

**Grammar Rule**: `nullPredicate` (Line 2042)

- `IS NULL`: Check if value is null
- `IS NOT NULL`: Check if value is not null

#### Value Type Predicate

**Grammar Rule**: `valueTypePredicate` (Line 2052)

- `IS [NOT] TYPED <value_type>`: Check if value has specific type

#### Normalized Predicate

**Grammar Rule**: `normalizedPredicatePart2` (Line 2062)

- `IS [NOT] NORMALIZED`: Check if string is normalized

#### Directed Predicate

**Grammar Rule**: `directedPredicate` (Line 2068)

- `IS [NOT] DIRECTED`: Check if edge is directed

#### Labeled Predicate

**Grammar Rule**: `labeledPredicate` (Line 2078)

- `IS [NOT] LABELED`: Check if element has any label
- `:label_expression`: Check specific label(s)

#### Source/Destination Predicate

**Grammar Rule**: `sourceDestinationPredicate` (Line 2093)

- `IS [NOT] SOURCE OF <edge>`: Check if node is source of edge
- `IS [NOT] DESTINATION OF <edge>`: Check if node is destination of edge

#### All Different Predicate

**Grammar Rule**: `all_differentPredicate` (Line 2116)

- `ALL_DIFFERENT(element1, element2, ...)`: Check if all elements are distinct

#### Same Predicate

**Grammar Rule**: `samePredicate` (Line 2122)

- `SAME(element1, element2)`: Check if elements refer to same graph element

#### Property Exists Predicate

**Grammar Rule**: `property_existsPredicate` (Line 2128)

- `PROPERTY_EXISTS(element, property_name)`: Check if property exists on element

---

## 16. Value Expressions

**Grammar Reference**: Lines 2137-2235

### Expression Hierarchy

**Grammar Rule**: `valueExpression` (Line 2137)

Comprehensive expression tree supporting:

1. **Unary Operations** (Line 2141):
   - `+` (unary plus)
   - `-` (unary minus, negation)

2. **Multiplicative Operations** (Line 2142):
   - `*` (multiplication)
   - `/` (division)

3. **Additive Operations** (Line 2143):
   - `+` (addition)
   - `-` (subtraction)

4. **String Operations** (Line 2148):
   - `||` (concatenation operator)

5. **Comparison Operations** (Line 2150):
   - All comparison operators (=, <>, <, >, <=, >=)

6. **Predicate Expressions** (Line 2151):
   - All predicate types

7. **Boolean Operations**:
   - `NOT` (Line 2155): Logical negation
   - `AND` (Line 2157): Logical conjunction
   - `OR` / `XOR` (Line 2158): Logical disjunction / exclusive or

8. **Truth Value Tests** (Line 2156):
   - `IS [NOT] TRUE`
   - `IS [NOT] FALSE`
   - `IS [NOT] UNKNOWN`

9. **Graph Expressions** (Line 2159):
   - `PROPERTY GRAPH <expression>`

10. **Binding Table Expressions** (Line 2160):
    - `BINDING TABLE <expression>`

11. **Value Functions** (Line 2161):
    - All built-in functions

12. **Primary Expressions** (Line 2162):
    - Literals, variables, subqueries, etc.

13. **Object Primary Expressions** (`objectExpressionPrimary`, Line 273):
    - `VARIABLE <value_expression_primary>`
    - Parenthesized and non-parenthesized expression-primary forms

### Value Expression Primary

**Grammar Rule**: `valueExpressionPrimary` (Line 2220)

Base expression types:

- **Parenthesized Expression** (Line 2221): `(<expression>)`
- **Aggregate Functions** (Line 2222): COUNT, SUM, AVG, etc.
- **Unsigned Value Specifications** (Line 2223): Literals
- **Path Value Constructor** (Line 2227): PATH construction
- **Property Reference** (Line 2228): `expression.property_name`
- **Value Query Expression** (Line 2229): `VALUE <nested_query>`
- **Case Expression** (Line 2230): CASE statements
- **Cast Specification** (Line 2231): `CAST(expr AS type)`
- **ELEMENT_ID Function** (Line 2232): Get element identifier
- **Let Value Expression** (Line 2233): `LET ... IN ... END`
- **Binding Variable Reference** (Line 2234): Variable references

### Value Specifications and Special Primaries

**Grammar Reference**: Lines 2261-2292, 271-279

- **General Value Specification** (`generalValueSpecification`, Line 2273):
  - Dynamic parameters (`$name`)
  - `SESSION_USER`

- **Object Expression Primary** (`objectExpressionPrimary`, Line 273):
  - `VARIABLE <value_expression_primary>`
  - Parenthesized/non-parenthesized primary expression forms

---

## 17. Built-in Functions

**Grammar Reference**: Lines 2165-2825

### Value Functions Overview

**Grammar Rule**: `valueFunction` (Line 2165)

Categories:
- Numeric value functions (Line 2166)
- Datetime subtraction (Line 2167)
- Datetime value functions (Line 2168)
- Duration value functions (Line 2169)
- Character/byte string functions (Line 2170)
- List value functions (Line 2171)

### Numeric Functions

**Grammar Reference**: Lines 2552-2677

#### Length and Cardinality

- **Length Expression** (`lengthExpression`, Line 2553):
  - `CHAR_LENGTH(string)` / `CHARACTER_LENGTH(string)` (Line 2583)
  - `BYTE_LENGTH(string)` / `OCTET_LENGTH(string)` (Line 2587)
  - `PATH_LENGTH(path)` (Line 2591)

- **Cardinality Expression** (`cardinalityExpression`, Line 2554):
  - `CARDINALITY(collection)` / `SIZE(collection)`

#### Arithmetic Functions

- **Absolute Value** (`absoluteValueExpression`, Line 2555):
  - `ABS(value)`

- **Modulus** (`modulusExpression`, Line 2556):
  - `MOD(dividend, divisor)`

#### Trigonometric Functions

**Grammar Rule**: `trigonometricFunction` (Line 2557)

- `SIN(x)`: Sine
- `COS(x)`: Cosine
- `TAN(x)`: Tangent
- `COT(x)`: Cotangent
- `SINH(x)`: Hyperbolic sine
- `COSH(x)`: Hyperbolic cosine
- `TANH(x)`: Hyperbolic tangent
- `ASIN(x)`: Arcsine
- `ACOS(x)`: Arccosine
- `ATAN(x)`: Arctangent
- `DEGREES(radians)`: Convert radians to degrees
- `RADIANS(degrees)`: Convert degrees to radians

#### Logarithmic and Exponential Functions

- **General Logarithm** (`generalLogarithmFunction`, Line 2558):
  - `LOG(base, value)`

- **Common Logarithm** (`commonLogarithm`, Line 2559):
  - `LOG10(value)`

- **Natural Logarithm** (`naturalLogarithm`, Line 2560):
  - `LN(value)`

- **Exponential Function** (`exponentialFunction`, Line 2561):
  - `EXP(value)`

- **Power Function** (`powerFunction`, Line 2562):
  - `POWER(base, exponent)`

- **Square Root** (`squareRoot`, Line 2563):
  - `SQRT(value)`

#### Rounding Functions

- **Floor Function** (`floorFunction`, Line 2564):
  - `FLOOR(value)`

- **Ceiling Function** (`ceilingFunction`, Line 2565):
  - `CEIL(value)` / `CEILING(value)`

### String Functions

**Grammar Reference**: Lines 2178-2204

#### Sub-character/Byte Functions

**Grammar Rule**: `subCharacterOrByteString` (Line 2186)

- `LEFT(string, n)`: Left n characters
- `RIGHT(string, n)`: Right n characters

#### Trimming Functions

- **Trim Single Character** (`trimSingleCharacterOrByteString`, Line 2190):
  - `TRIM([LEADING | TRAILING | BOTH] [trim_char] FROM string)`

- **Trim Multiple Characters** (`trimMultiCharacterCharacterString`, Line 2198):
  - `BTRIM(string, [trim_chars])`: Trim both ends
  - `LTRIM(string, [trim_chars])`: Trim left
  - `RTRIM(string, [trim_chars])`: Trim right

#### Case Functions

**Grammar Rule**: `foldCharacterString` (Line 2194)

- `UPPER(string)`: Convert to uppercase
- `LOWER(string)`: Convert to lowercase

#### Normalization Functions

**Grammar Rule**: `normalizeCharacterString` (Line 2202)

- `NORMALIZE(string, [form])`: Unicode normalization
  - `NFC`: Canonical composition (default)
  - `NFD`: Canonical decomposition
  - `NFKC`: Compatibility composition
  - `NFKD`: Compatibility decomposition

### Datetime Functions

**Grammar Reference**: Lines 2741-2825

#### Date Functions

**Grammar Rule**: `dateFunction` (Line 2749)

- `CURRENT_DATE`: Current date
- `DATE()`: Construct date
- `DATE(string)`: Parse date

#### Time Functions

**Grammar Rule**: `timeFunction` (Line 2754)

- `CURRENT_TIME`: Current time with timezone
- `ZONED_TIME()`: Construct zoned time
- `LOCAL_TIME()` (`localtimeFunction`, Line 2759): Construct local time

#### Datetime Functions

**Grammar Rule**: `datetimeFunction` (Line 2763)

- `CURRENT_TIMESTAMP`: Current timestamp with timezone
- `ZONED_DATETIME()`: Construct zoned datetime
- `LOCAL_TIMESTAMP` / `LOCAL_DATETIME()` (`localdatetimeFunction`, Line 2768): Construct local datetime

### Duration Functions

**Grammar Reference**: Lines 2813-2825

- **Duration Between** (`datetimeSubtraction`, Line 2795):
  - `DURATION_BETWEEN(datetime1, datetime2)`

- **Duration Function** (`durationFunction`, Line 2818):
  - `DURATION(string)`
  - `DURATION(record)` (record constructor form)

- **Absolute Duration** (Line 2815):
  - `ABS(duration)`

### List Functions

**Grammar Reference**: Lines 2481-2492

- **Trim List Function** (`trimListFunction`, Line 2486):
  - `TRIM(list, n)`: Trim list by numeric amount

- **Elements Function** (`elementsFunction`, Line 2490):
  - `ELEMENTS(path)`: Extract elements from path

### Case Expressions

**Grammar Reference**: Lines 2298-2361

- **Case Abbreviation** (`caseAbbreviation`, Line 2303):
  - `NULLIF(expr1, expr2)` (Line 2304): Return null if equal
  - `COALESCE(expr1, expr2, ...)` (Line 2305): First non-null value

- **Simple Case** (`simpleCase`, Line 2313):
  ```
  CASE operand
    WHEN value1 THEN result1
    WHEN value2 THEN result2
    [ELSE default_result]
  END
  ```

- **Searched Case** (`searchedCase`, Line 2317):
  ```
  CASE
    WHEN condition1 THEN result1
    WHEN condition2 THEN result2
    [ELSE default_result]
  END
  ```

### Cast Specification

**Grammar Reference**: Lines 2365-2376

- **Cast Specification** (`castSpecification`, Line 2365):
  - `CAST(operand AS target_type)`
  - Convert value to specified type

### Element ID Function

**Grammar Reference**: Lines 2425-2427

- **ELEMENT_ID Function** (`element_idFunction`, Line 2425):
  - `ELEMENT_ID(element_variable)`
  - Get unique identifier of graph element

---

## 18. Literals & Constants

**Grammar Reference**: Lines 2913-3031

### Unsigned Literals

**Grammar Rule**: `unsignedLiteral` (Line 2913)

#### Boolean Literals

**Grammar Rule**: `BOOLEAN_LITERAL` (Line 2919)

- `TRUE`
- `FALSE`
- `UNKNOWN`

#### String Literals

- **Character String Literal** (`characterStringLiteral`, Line 2920):
  - Single-quoted strings: `'text'`
  - Double-quoted strings: `"text"`
  - Unicode escape sequences
  - Escaped quoting and Unicode escape support

- **Byte String Literal** (`BYTE_STRING_LITERAL`, Line 2921):
  - Hexadecimal byte strings: `X'4A7E'`

#### Numeric Literals

**Grammar Reference**: Lines 2977-3002

- **Exact Numeric Literal** (`exactNumericLiteral`, Line 2982):
  - **Decimal Integers**: `123`, `456789`
  - **Hexadecimal Integers**: `0x1A2B`
  - **Octal Integers**: `0o755`
  - **Binary Integers**: `0b1010`
  - **Decimal Numbers**: `123.45`, `.67`, `89.`

- **Approximate Numeric Literal** (`approximateNumericLiteral`, Line 2990):
  - Scientific notation: `1.23e10`, `4.56E-7`
  - Approximate suffix forms: `2.718e0F`, `42D`

#### Temporal Literals

**Grammar Rule**: `temporalLiteral` (Line 2929)

- **Date Literals**:
  - `DATE '2024-04-15'`

- **Time Literals**:
  - `TIME '14:30:00'`
  - `TIME '14:30:00+02:00'`

- **Datetime Literals**:
  - `DATETIME '2024-04-15T14:30:00'`
  - `TIMESTAMP '2024-04-15 14:30:00'`

#### Duration Literals

**Grammar Rule**: `durationLiteral` (Line 2924)

- `DURATION 'P1Y2M'`: 1 year, 2 months
- `DURATION 'P3DT4H'`: 3 days, 4 hours
- `DURATION 'PT30M'`: 30 minutes

#### Null Literal

**Grammar Rule**: `nullLiteral` (Line 2924)

- `NULL`: Null value

#### Collection Literals

- **List Literal** (`listLiteral`, Line 2925):
  - `[element1, element2, element3, ...]`

- **Record Literal** (`recordLiteral`, Line 2926):
  - `{field1: value1, field2: value2, ...}`

### Constructors

#### List Value Constructor

**Grammar Reference**: Lines 2496-2510

- **List Value Constructor** (`listValueConstructor`, Line 2496):
  - `[element1, element2, ...]` (`listValueConstructorByEnumeration`, Line 2500)

#### Path Value Constructor

**Grammar Reference**: Lines 2446-2464

- **Path Value Constructor** (`pathValueConstructor`, Line 2446):
  - `PATH[node1, edge1, node2, edge2, ...]` (`pathValueConstructorByEnumeration`, Line 2450)

#### Record Constructor

**Grammar Reference**: Lines 2514-2530

- **Record Constructor** (`recordConstructor`, Line 2514):
  - `RECORD {field1: value1, field2: value2, ...}`

---

## 19. Variables, Parameters & References

**Grammar Reference**: Lines 178-242, 1379-1478, 2280-2282, 2829-2909

### Variable Definition

**Grammar Reference**: Lines 178-242

#### Binding Variable Definition Block

**Grammar Rule**: `bindingVariableDefinitionBlock` (Line 178)

- Define multiple variables in a block

#### Variable Types

- **Graph Variable Definition** (`graphVariableDefinition`, Line 204):
  - `[PROPERTY] GRAPH variable_name [:: type] [= initializer]`

- **Binding Table Variable Definition** (`bindingTableVariableDefinition`, Line 218):
  - `[BINDING] TABLE variable_name [:: type] [= initializer]`

- **Value Variable Definition** (`valueVariableDefinition`, Line 232):
  - `VALUE variable_name [:: type] [= initializer]`

### Variable Names

**Grammar Reference**: Lines 2829-2909

- **Element Variable** (`elementVariable`, Line 2895): Variables for nodes/edges
- **Path Variable** (`pathVariable`, Line 2899): Variables for paths
- **Subpath Variable** (`subpathVariable`, Line 2903): Variables for subpaths
- **Binding Variable** (`bindingVariable`, Line 2907): General binding variables

### Parameter Specifications

**Grammar Reference**: Lines 2280-2282, 1476-1478

- **Dynamic Parameter Specification** (`dynamicParameterSpecification`, Line 2280):
  - `$parameter_name`: Dynamic parameters

- **Reference Parameter Specification** (`referenceParameterSpecification`, Line 1476):
  - `$$parameter_name`: Reference parameters

### Catalog/Object References

**Grammar Reference**: Lines 1379-1478

- **Schema Reference** (`schemaReference`, Line 1381):
  - Absolute paths (`/ ...`)
  - Relative paths (`../ ...`)
  - Predefined references (`HOME_SCHEMA`, `CURRENT_SCHEMA`, `.`)
  - Reference parameters (`$$name`)

- **Graph Reference** (`graphReference`, Line 1421):
  - Catalog-qualified graph name
  - Delimited graph name
  - Home graph (`HOME_GRAPH`, `HOME_PROPERTY_GRAPH`)
  - Reference parameter form

- **Graph Type Reference** (`graphTypeReference`, Line 1439):
  - Catalog-qualified graph type name
  - Reference parameter form

- **Binding Table Reference** (`bindingTableReference`, Line 1450):
  - Catalog-qualified binding table name
  - Delimited binding table name
  - Reference parameter form

- **Procedure Reference** (`procedureReference`, Line 1458):
  - Catalog-qualified procedure name
  - Reference parameter form

- **Catalog Parent Paths** (`catalogObjectParentReference`, Line 1469):
  - Parent-object qualification with directory/schema/object path components

---

## 20. Graph Type Specification

**Grammar Reference**: Lines 1481-1709

### Nested Graph Type Specifications

**Grammar Reference**: Lines 1482-1497

- **Nested Graph Type Specification** (`nestedGraphTypeSpecification`, Line 1482):
  - Define graph types inline

- **Graph Type Specification Body** (`graphTypeSpecificationBody`, Line 1486):
  - List of element type specifications

- **Element Type List** (`elementTypeList`, Line 1490):
  - Multiple element types

- **Element Type Specification** (`elementTypeSpecification`, Line 1494):
  - Node type or edge type

### Node Type Specifications

**Grammar Reference**: Lines 1501-1544

- **Node Type Specification** (`nodeTypeSpecification`, Line 1501):
  - Define node types in graph schema

- **Node Type Pattern** (`nodeTypePattern`, Line 1506):
  - Pattern-based node type definition

- **Node Type Phrase** (`nodeTypePhrase`, Line 1510):
  - `NODE [TYPE] <node_type_phrase_filler> [AS <local_node_type_alias>]`

- **Node Type Filler** (`nodeTypeFiller`, Line 1519):
  - Label sets
  - Property types
  - Key specifications

- **Node Type Implied Content** (`nodeTypeImpliedContent`, Line 1528):
  - Default content for node type

- **Node Type Key Label Set** (`nodeTypeKeyLabelSet`, Line 1534):
  - Key constraint on labels

- **Node Type Label Set** (`nodeTypeLabelSet`, Line 1538):
  - Labels for node type

- **Node Type Property Types** (`nodeTypePropertyTypes`, Line 1542):
  - Property type specifications

### Edge Type Specifications

**Grammar Reference**: Lines 1548-1675

- **Edge Type Specification** (`edgeTypeSpecification`, Line 1548):
  - Define edge types in graph schema

- **Edge Type Pattern** (`edgeTypePattern`, Line 1553):
  - Pattern-based edge type definition
  - Directed or undirected

- **Edge Type Phrase** (`edgeTypePhrase`, Line 1557):
  - `<edge_kind> EDGE [TYPE] <edge_type_phrase_filler> CONNECTING (<endpoint_pair>)`

- **Edge Type Filler** (`edgeTypeFiller`, Line 1566):
  - Label sets
  - Property types
  - Endpoint specifications

- **Edge Type Pattern Directed** (`edgeTypePatternDirected`, Line 1589):
  - **Pointing Right** (`arcTypePointingRight`, Line 1606): `-[edge_type]->` source to destination
  - **Pointing Left** (`arcTypePointingLeft`, Line 1610): `<-[edge_type]-` destination to source

- **Edge Type Pattern Undirected** (`edgeTypePatternUndirected`, Line 1602):
  - **Undirected Arc** (`arcTypeUndirected`, Line 1614): `~[edge_type]~`

- **Edge Kind** (`edgeKind`, Line 1628):
  - `DIRECTED EDGE`: Directed edges only
  - `UNDIRECTED EDGE`: Undirected edges only

- **Endpoint Pair Phrase** (`endpointPairPhrase`, Line 1633):
  - `CONNECTING (<endpoint_pair>)`

- **Endpoint Pair** (`endpointPair`, Line 1637):
  - Source and destination node types
  - Directed or undirected connections

### Property Type Specifications

**Grammar Reference**: Lines 1691-1709

- **Property Types Specification** (`propertyTypesSpecification`, Line 1691):
  - `{ <property_type_list>? }`

- **Property Type List** (`propertyTypeList`, Line 1695):
  - Multiple property definitions

- **Property Type** (`propertyType`, Line 1701):
  - `property_name :: value_type [NOT NULL]`

- **Property Value Type** (`propertyValueType`, Line 1707):
  - Type of property values

---

## 21. Reserved & Non-Reserved Keywords

**Grammar Reference**: Lines 3277-3584

### Reserved Words (Lines 3277-3494)

Core reserved keywords (~200+ keywords) including:

**Query Keywords**: SELECT, MATCH, WHERE, RETURN, WITH, FILTER, ORDER, BY, GROUP, HAVING, LIMIT, OFFSET, SKIP

**Data Modification**: INSERT, DELETE, SET, REMOVE, DETACH, NODETACH

**Graph Pattern**: PATH, PATHS, WALK, TRAIL, SIMPLE, ACYCLIC, ANY, ALL, SHORTEST

**Schema/Catalog**: CREATE, DROP, SCHEMA, GRAPH, TYPE, OF, AS, COPY

**Session/Transaction**: SESSION, TRANSACTION, START, COMMIT, ROLLBACK, RESET, CLOSE

**Procedure**: CALL, PROCEDURE, OPTIONAL, YIELD, RETURN

**Types**: INT, STRING, BOOLEAN, DATE, TIME, TIMESTAMP, DURATION, LIST, RECORD, NULL

**Operators**: AND, OR, NOT, XOR, IS, IN, EXISTS, CASE, WHEN, THEN, ELSE, END, CAST

**Quantifiers**: DISTINCT, ALL, ANY

**Boolean**: TRUE, FALSE, UNKNOWN

**Temporal**: CURRENT_DATE, CURRENT_TIME, CURRENT_TIMESTAMP

**Aggregates**: COUNT, SUM, AVG, MAX, MIN, COLLECT_LIST, STDDEV_SAMP, STDDEV_POP

**Set Operations**: UNION, EXCEPT, INTERSECT

**Sorting**: ASC, DESC, ASCENDING, DESCENDING, NULLS, FIRST, LAST

**Graph Elements**: NODE, EDGE, RELATIONSHIP, LABEL, LABELS, PROPERTY

**Predicates**: DIRECTED, UNDIRECTED, SOURCE, DESTINATION, LABELED

**Path Modes**: REPEATABLE, DIFFERENT, ELEMENTS, EDGES

**Clauses**: USE, AT, KEEP, FOR, LET, VALUE, BINDING, TABLE

### Pre-Reserved Words (Lines 3497-3535)

Future-proofed keywords (~40 keywords) including:

**Core Examples**: ABSTRACT, AGGREGATE, AGGREGATES, ALTER, CATALOG, CLEAR, CLONE, CONSTRAINT, CURRENT_ROLE, CURRENT_USER

**Additional Examples**: DATA, DIRECTORY, DRYRUN, EXACT, EXISTING, FUNCTION, GQLSTATUS, GRANT, INFINITY, INSTANT, NUMBER, NUMERIC

**More Examples**: ON, OPEN, PARTITION, PROCEDURE, PRODUCT, PROJECT, QUERY, RECORDS, REFERENCE, RENAME, REVOKE, SUBSTRING, SYSTEM_USER, TEMPORAL, UNIQUE, UNIT, VALUES

### Non-Reserved Words (Lines 3538-3584)

Context-sensitive keywords that can be used as identifiers (~50 keywords):

**Complete Set Examples**: ACYCLIC, BINDING, BINDINGS, CONNECTING, DESTINATION, DIFFERENT, DIRECTED, EDGE, EDGES, ELEMENT, ELEMENTS, FIRST, GRAPH, GROUPS, KEEP, LABEL, LABELED, LABELS, LAST

**Unicode/Normalization Examples**: NFC, NFD, NFKC, NFKD, NORMALIZED

**Additional Examples**: NO, NODE, ONLY, ORDINALITY, PROPERTY, READ, RELATIONSHIP, RELATIONSHIPS, REPEATABLE, SHORTEST, SIMPLE, SOURCE, TABLE, TO, TRAIL, TRANSACTION, TYPE, UNDIRECTED, VERTEX, WALK, WITHOUT, WRITE, ZONE

### Lexical and Tokenization Features

**Grammar Reference**: Lines 2956-3774

- **Identifier Forms**:
  - Regular identifiers (`REGULAR_IDENTIFIER`)
  - Delimited identifiers via double quotes or accents
  - Non-reserved words usable as regular identifiers

- **Parameter Tokens**:
  - General parameter references: `$<name>`
  - Substituted/reference parameter references: `$$<name>`

- **Operator and Symbol Tokens**:
  - Path/pattern operators such as `|+|`, `<~`, `<->`, `::`, `->`, `<-`, and related bracketed variants
  - Arithmetic, boolean, comparison, concatenation, punctuation, and grouping terminals

- **Comments and Whitespace**:
  - Bracketed comments: `/* ... */`
  - Single-line comments: `// ...` and `-- ...`
  - Extensive Unicode-aware whitespace handling

---

## Summary Statistics

- **Total Grammar Lines**: 3,774 lines
- **Parser Rules (Nonterminals)**: 571 rules
- **Major Feature Categories**: 21 distinct categories
- **Total Keywords**: 250+ keywords
  - Reserved: ~200 keywords
  - Pre-reserved: ~40 keywords
  - Non-reserved: ~50 keywords

---

## Implementation Notes

This feature overview is derived from the **ISO/IEC 39075:2024** standard via the ANTLR4 grammar located at [`third_party/opengql-grammar/GQL.g4`](third_party/opengql-grammar/GQL.g4).

### Grammar Characteristics

1. **Permissiveness**: The grammar is more permissive than a full implementation requires. Type checking and semantic analysis must be performed as post-processing steps after parsing.

2. **Case Insensitivity**: The grammar uses `options { caseInsensitive = true; }`, meaning all keywords are case-insensitive.

3. **Alternative Labels**: The grammar uses alternative labels for complex rules to simplify visitor/listener implementation.

4. **Single File**: Lexer and parser rules are combined in a single file (GQL.g4) rather than separate files, due to JetBrains ANTLR plugin compatibility.

### Next Steps for Implementation

This feature overview should be used to:

1. **Define Implementation Phases**: Break down features into logical implementation sprints
2. **Prioritize Features**: Determine core vs. advanced features
3. **Identify Dependencies**: Understand which features depend on others
4. **Estimate Complexity**: Assess implementation complexity for each feature area
5. **Plan Testing Strategy**: Develop test cases for each feature category

### Reference Documentation

- **Grammar File**: [`third_party/opengql-grammar/GQL.g4`](third_party/opengql-grammar/GQL.g4)
- **Upstream Repository**: See [`third_party/opengql-grammar/UPSTREAM.md`](third_party/opengql-grammar/UPSTREAM.md)
- **Sample Queries**: See [`third_party/opengql-grammar/samples/`](third_party/opengql-grammar/samples/)
- **ISO Standard**: ISO/IEC 39075:2024 (GQL - Property Graph Query Language)

---

**Document Version**: 1.0
**Last Updated**: 2026-02-17
**Maintained By**: GQL Parser Project Team
