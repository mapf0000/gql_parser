# Sprint 5: Values, Literals, and Expression Core

## Sprint Overview

**Sprint Goal**: Implement expression backbone used by nearly all clauses.

**Sprint Duration**: TBD

**Status**: 🔵 **Planned**

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅

## Scope

This sprint implements the complete expression system that forms the computational backbone of GQL. Expressions are used throughout the language in nearly all query clauses, predicates, data modification statements, and result shaping operations. This includes literals, value expressions, predicates, operators with proper precedence, case expressions, cast operations, and built-in functions.

### Feature Coverage from GQL_FEATURES.md

Sprint 5 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 15: Predicates & Conditions** (Lines 1065-1155)
   - Comparison predicates (=, <>, <, >, <=, >=)
   - EXISTS predicate
   - NULL predicate (IS NULL, IS NOT NULL)
   - Value type predicate (IS TYPED)
   - Normalized predicate (IS NORMALIZED)
   - Directed predicate (IS DIRECTED)
   - Labeled predicate (IS LABELED)
   - Source/Destination predicate
   - ALL_DIFFERENT predicate
   - SAME predicate
   - PROPERTY_EXISTS predicate

2. **Section 16: Value Expressions** (Lines 1157-1244)
   - Expression hierarchy and operator precedence
   - Unary operations (+, -)
   - Multiplicative operations (*, /)
   - Additive operations (+, -)
   - String concatenation (||)
   - Comparison operations
   - Boolean operations (AND, OR, NOT, XOR)
   - Truth value tests (IS TRUE, IS FALSE, IS UNKNOWN)
   - Graph expressions (PROPERTY GRAPH)
   - Binding table expressions (BINDING TABLE)
   - Primary expressions (parenthesized, property references, etc.)

3. **Section 17: Built-in Functions** (Lines 1246-1461)
   - Numeric functions (ABS, MOD, FLOOR, CEIL, SQRT, POWER, etc.)
   - Trigonometric functions (SIN, COS, TAN, ASIN, ACOS, ATAN, etc.)
   - Logarithmic and exponential functions (LN, LOG, LOG10, EXP)
   - String functions (UPPER, LOWER, TRIM, BTRIM, LTRIM, RTRIM, LEFT, RIGHT, NORMALIZE)
   - Length and cardinality functions (CHAR_LENGTH, BYTE_LENGTH, PATH_LENGTH, CARDINALITY, SIZE)
   - Case expressions (CASE...WHEN...THEN...ELSE, NULLIF, COALESCE)
   - Cast specification (CAST)
   - Datetime functions (CURRENT_DATE, CURRENT_TIME, CURRENT_TIMESTAMP, DATE, TIME, DATETIME, DURATION_BETWEEN)
   - Duration functions (DURATION, ABS)
   - List functions (TRIM, ELEMENTS)
   - ELEMENT_ID function

4. **Section 18: Literals & Constants** (Lines 1463-1566)
   - Boolean literals (TRUE, FALSE, UNKNOWN)
   - String literals (single-quoted, double-quoted, with escapes)
   - Byte string literals (X'...')
   - Numeric literals:
     - Exact numeric (integers: decimal, hex, octal, binary; decimals)
     - Approximate numeric (scientific notation)
   - Temporal literals (DATE, TIME, DATETIME, TIMESTAMP)
   - Duration literals (DURATION)
   - NULL literal
   - Collection literals:
     - List literals [...]
     - Record literals {...}
   - Constructors:
     - List value constructor
     - Path value constructor (PATH[...])
     - Record constructor (RECORD {...})

5. **Section 19: Variables, Parameters & References** (Lines 1567-1643) - Partial
   - Dynamic parameters ($name)
   - Binding variables (element, path, subpath variables)
   - Property references (expr.property)
   - General value specifications (SESSION_USER)

## Exit Criteria

- [ ] All expression types parse with correct AST forms
- [ ] Operator precedence is implemented correctly per ISO GQL specification
- [ ] All literal types (boolean, numeric, string, temporal, null, collection) are parsed
- [ ] All built-in functions have AST representations
- [ ] All predicate types are implemented
- [ ] Case expressions (simple and searched) parse correctly
- [ ] Cast expressions parse correctly
- [ ] Property references and field access work
- [ ] Expression parser is reusable across query/mutation/procedure contexts
- [ ] Parser produces structured diagnostics for malformed expressions
- [ ] AST nodes have proper span information for all components
- [ ] Recovery mechanisms handle errors at expression boundaries
- [ ] Unit tests cover all expression variants and error cases
- [ ] Parser handles nested expressions and complex operator combinations
- [ ] Expression parsing integrates with existing statement parsing

## Implementation Tasks

### Task 1: AST Node Definitions for Literals

**Description**: Define AST types for all literal value forms.

**Deliverables**:
- `Literal` enum with variants for all literal types:
  - `BooleanLiteral(bool)` - TRUE, FALSE, UNKNOWN
  - `NullLiteral` - NULL
  - `IntegerLiteral(SmolStr)` - decimal, hex, octal, binary integers
  - `FloatLiteral(SmolStr)` - decimal and scientific notation
  - `StringLiteral(SmolStr)` - single/double-quoted strings
  - `ByteStringLiteral(SmolStr)` - X'...' format
  - `DateLiteral(SmolStr)` - DATE '...'
  - `TimeLiteral(SmolStr)` - TIME '...'
  - `DatetimeLiteral(SmolStr)` - DATETIME/TIMESTAMP '...'
  - `DurationLiteral(SmolStr)` - DURATION '...'
  - `ListLiteral(Vec<Expression>)` - [expr1, expr2, ...]
  - `RecordLiteral(Vec<RecordField>)` - {field: value, ...}
- `RecordField` struct with field name and value expression

**Grammar References**:
- `unsignedLiteral` (Line 2913)
- `BOOLEAN_LITERAL` (Line 2919)
- `characterStringLiteral` (Line 2920)
- `BYTE_STRING_LITERAL` (Line 2921)
- `temporalLiteral` (Line 2929)
- `durationLiteral` (Line 2924)
- `nullLiteral` (Line 2924)
- `listLiteral` (Line 2925)
- `recordLiteral` (Line 2926)
- `exactNumericLiteral` (Line 2982)
- `approximateNumericLiteral` (Line 2990)

**Acceptance Criteria**:
- [ ] All literal AST types defined in `src/ast/expression.rs` (new module)
- [ ] Each literal node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each literal type
- [ ] Numeric literals preserve original text for precision

**File Location**: `src/ast/expression.rs` (new file)

---

### Task 2: AST Node Definitions for Value Expressions

**Description**: Define AST types for value expressions with proper operator precedence structure.

**Deliverables**:
- `Expression` enum representing the top-level expression type
- Expression variant types:
  - `UnaryExpression` - unary +, -, NOT
  - `BinaryExpression` - binary operators with kind enum
  - `ComparisonExpression` - comparison operators
  - `LogicalExpression` - AND, OR, XOR
  - `ParenthesizedExpression` - (expr)
  - `PropertyReference` - expr.property_name
  - `VariableReference` - binding variable
  - `ParameterReference` - $name
  - `FunctionCall` - function invocations
  - `CaseExpression` - CASE statements (simple and searched)
  - `CastExpression` - CAST(expr AS type)
  - `LiteralExpression` - wraps Literal
  - `SubqueryExpression` - VALUE <nested_query>
  - `ListConstructor` - [expr1, expr2, ...]
  - `RecordConstructor` - RECORD {field: value, ...}
  - `PathConstructor` - PATH[...]
  - `ExistsExpression` - EXISTS {...}
  - `GraphExpression` - PROPERTY GRAPH expr
  - `BindingTableExpression` - BINDING TABLE expr
- `BinaryOperator` enum:
  - Arithmetic: Add, Subtract, Multiply, Divide, Modulo
  - String: Concatenate (||)
- `ComparisonOperator` enum: Eq, NotEq, Lt, Gt, LtEq, GtEq
- `LogicalOperator` enum: And, Or, Xor, Not
- `UnaryOperator` enum: Plus, Minus, Not

**Grammar References**:
- `valueExpression` (Line 2137)
- `valueExpressionPrimary` (Line 2220)
- `objectExpressionPrimary` (Line 273)

**Acceptance Criteria**:
- [ ] Expression AST hierarchy supports all operator types
- [ ] Expression nodes have proper span tracking
- [ ] AST structure reflects operator precedence naturally
- [ ] Recursive expression structure supports arbitrary nesting
- [ ] All expression variants documented with examples

**File Location**: `src/ast/expression.rs`

---

### Task 3: AST Node Definitions for Predicates

**Description**: Define AST types for all predicate forms used in filtering and conditions.

**Deliverables**:
- `Predicate` enum with variants:
  - `Comparison(ComparisonPredicate)` - expr op expr
  - `IsNull(Expression, bool)` - IS [NOT] NULL
  - `IsTyped(Expression, TypeRef, bool)` - IS [NOT] TYPED type
  - `IsNormalized(Expression, bool)` - IS [NOT] NORMALIZED
  - `IsDirected(Expression, bool)` - IS [NOT] DIRECTED
  - `IsLabeled(Expression, Option<LabelExpression>, bool)` - IS [NOT] LABELED [:label]
  - `IsTruthValue(Expression, TruthValue, bool)` - IS [NOT] TRUE/FALSE/UNKNOWN
  - `IsSource(Expression, Expression, bool)` - IS [NOT] SOURCE OF edge
  - `IsDestination(Expression, Expression, bool)` - IS [NOT] DESTINATION OF edge
  - `AllDifferent(Vec<Expression>)` - ALL_DIFFERENT(...)
  - `Same(Expression, Expression)` - SAME(e1, e2)
  - `PropertyExists(Expression, SmolStr)` - PROPERTY_EXISTS(elem, prop)
  - `Exists(ExistsVariant)` - EXISTS {pattern} | EXISTS (query)
- `TruthValue` enum: True, False, Unknown
- `ExistsVariant` enum to distinguish between graph pattern and query forms

**Grammar References**:
- `searchCondition` (Line 2002)
- `compOp` (Line 2025)
- `existsPredicate` (Line 2036)
- `nullPredicate` (Line 2042)
- `valueTypePredicate` (Line 2052)
- `normalizedPredicatePart2` (Line 2062)
- `directedPredicate` (Line 2068)
- `labeledPredicate` (Line 2078)
- `sourceDestinationPredicate` (Line 2093)
- `all_differentPredicate` (Line 2116)
- `samePredicate` (Line 2122)
- `property_existsPredicate` (Line 2128)

**Acceptance Criteria**:
- [ ] All predicate types have AST representations
- [ ] Predicates properly track negation (IS NOT)
- [ ] Span information captures entire predicate extent
- [ ] Predicate AST integrates with expression AST
- [ ] Documentation explains each predicate's semantics

**File Location**: `src/ast/expression.rs`

---

### Task 4: AST Node Definitions for Built-in Functions

**Description**: Define AST types for built-in function calls.

**Deliverables**:
- `FunctionCall` struct:
  - `name: FunctionName` - function identifier
  - `arguments: Vec<Expression>` - function arguments
  - `span: Span`
- `FunctionName` enum with variants for categorized functions:
  - Numeric: `Abs`, `Mod`, `Floor`, `Ceil`, `Sqrt`, `Power`, `Exp`, `Ln`, `Log`, `Log10`
  - Trigonometric: `Sin`, `Cos`, `Tan`, `Cot`, `Sinh`, `Cosh`, `Tanh`, `Asin`, `Acos`, `Atan`, `Degrees`, `Radians`
  - String: `Upper`, `Lower`, `Trim`, `BTrim`, `LTrim`, `RTrim`, `Left`, `Right`, `Normalize`, `CharLength`, `ByteLength`
  - Datetime: `CurrentDate`, `CurrentTime`, `CurrentTimestamp`, `Date`, `Time`, `Datetime`, `ZonedTime`, `ZonedDatetime`, `LocalTime`, `LocalDatetime`, `Duration`, `DurationBetween`
  - List: `TrimList`, `Elements`
  - Cardinality: `Cardinality`, `Size`, `PathLength`
  - Graph: `ElementId`
  - Conditional: `Coalesce`, `NullIf`
  - User-defined: `Custom(SmolStr)` - for extensibility
- Optional: `TrimSpecification` enum for TRIM functions: Leading, Trailing, Both

**Grammar References**:
- `valueFunction` (Line 2165)
- `numericValueFunction` (Line 2166)
- `trigonometricFunction` (Line 2557)
- `foldCharacterString` (Line 2194)
- `trimSingleCharacterOrByteString` (Line 2190)
- `normalizeCharacterString` (Line 2202)
- `dateFunction` (Line 2749)
- `timeFunction` (Line 2754)
- `datetimeFunction` (Line 2763)
- `durationFunction` (Line 2818)
- `cardinalityExpression` (Line 2554)
- `element_idFunction` (Line 2425)

**Acceptance Criteria**:
- [ ] All ISO GQL built-in functions have enum variants
- [ ] Function call AST captures name, arguments, and spans
- [ ] Function names are case-insensitive (handled in parsing)
- [ ] Special functions (TRIM with specifications) handled
- [ ] User-defined function calls supported for extensibility

**File Location**: `src/ast/expression.rs`

---

### Task 5: AST Node Definitions for Case and Cast Expressions

**Description**: Define AST types for CASE expressions and CAST operations.

**Deliverables**:
- `CaseExpression` enum:
  - `Simple(SimpleCaseExpression)` - CASE operand WHEN ...
  - `Searched(SearchedCaseExpression)` - CASE WHEN condition ...
- `SimpleCaseExpression` struct:
  - `operand: Box<Expression>`
  - `when_clauses: Vec<SimpleWhenClause>`
  - `else_clause: Option<Box<Expression>>`
  - `span: Span`
- `SimpleWhenClause` struct:
  - `when_value: Expression`
  - `then_result: Expression`
  - `span: Span`
- `SearchedCaseExpression` struct:
  - `when_clauses: Vec<SearchedWhenClause>`
  - `else_clause: Option<Box<Expression>>`
  - `span: Span`
- `SearchedWhenClause` struct:
  - `condition: Expression` (predicate)
  - `then_result: Expression`
  - `span: Span`
- `CastExpression` struct:
  - `operand: Box<Expression>`
  - `target_type: TypeReference` (placeholder for Sprint 6)
  - `span: Span`

**Grammar References**:
- `caseExpression` (Line 2298)
- `simpleCase` (Line 2313)
- `searchedCase` (Line 2317)
- `caseAbbreviation` (Line 2303)
- `castSpecification` (Line 2365)

**Acceptance Criteria**:
- [ ] Both simple and searched CASE forms represented
- [ ] WHEN clauses properly structured
- [ ] ELSE clause is optional in AST
- [ ] CAST expression references type system (placeholder for Sprint 6)
- [ ] Span tracking covers entire CASE/CAST extent
- [ ] NULLIF and COALESCE can be represented (as function calls or special variants)

**File Location**: `src/ast/expression.rs`

---

### Task 6: Lexer Extensions for Expression Tokens

**Description**: Ensure lexer supports all tokens needed for expressions.

**Deliverables**:
- Verify existing tokens are sufficient:
  - Operators: +, -, *, /, %, ||, =, <>, <, >, <=, >=
  - Keywords: AND, OR, NOT, XOR, IS, IN, EXISTS, NULL, TRUE, FALSE, UNKNOWN
  - Keywords: CASE, WHEN, THEN, ELSE, END, CAST, AS
  - Function keywords: ABS, FLOOR, CEIL, UPPER, LOWER, TRIM, etc.
  - Temporal keywords: DATE, TIME, TIMESTAMP, DURATION, CURRENT (already exist)
- Add any missing function name keywords (case-insensitive)
- Ensure parameter tokens ($name) are properly lexed
- Verify string literal escaping is correct
- Verify numeric literal formats (hex 0x, octal 0o, binary 0b, scientific notation)

**Lexer Enhancements Needed**:
- Review and add missing built-in function keywords to keyword table
- Ensure byte string literals (X'...') are recognized
- Verify temporal literal tokenization (DATE + string literal form)
- Add keywords: `VALUE`, `PATH`, `RECORD`, `VARIABLE`, `TABLE`, `BINDING`, `PROPERTY` (if missing)
- Add function keywords: `COALESCE`, `NULLIF`, `ELEMENT_ID`, `CARDINALITY`, `SIZE`, `NORMALIZE`, etc.
- Add predicate keywords: `TYPED`, `NORMALIZED`, `DIRECTED`, `LABELED`, `SOURCE`, `DESTINATION`, `ALL_DIFFERENT`, `SAME`, `PROPERTY_EXISTS`

**Grammar References**:
- Token definitions in lexer specification

**Acceptance Criteria**:
- [ ] All operators tokenized correctly
- [ ] All function names recognized as keywords or identifiers
- [ ] Numeric literals in all bases (decimal, hex, octal, binary) tokenize correctly
- [ ] Scientific notation supported (1.23e10)
- [ ] String literals with escape sequences handled
- [ ] Byte string literals (X'...') supported
- [ ] Temporal literals tokenize as keyword + string
- [ ] No new lexer errors introduced

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 7: Expression Parser - Literals

**Description**: Implement parsing for all literal types.

**Deliverables**:
- `parse_literal()` function that dispatches to specific literal parsers
- Parser functions for each literal type:
  - `parse_boolean_literal()` - TRUE, FALSE, UNKNOWN
  - `parse_null_literal()` - NULL
  - `parse_integer_literal()` - decimal, hex, octal, binary
  - `parse_float_literal()` - decimal and scientific notation
  - `parse_string_literal()` - single/double-quoted
  - `parse_byte_string_literal()` - X'...'
  - `parse_temporal_literal()` - DATE/TIME/DATETIME/TIMESTAMP '...'
  - `parse_duration_literal()` - DURATION '...'
  - `parse_list_literal()` - [expr, expr, ...]
  - `parse_record_literal()` - {field: value, ...}
- Literal validation (basic format checking)
- Error diagnostics for malformed literals

**Grammar References**:
- `unsignedLiteral` (Line 2913)

**Acceptance Criteria**:
- [ ] All literal forms parse to correct AST nodes
- [ ] Numeric literals preserve original text
- [ ] String escape sequences handled
- [ ] Temporal literals validate format (basic check)
- [ ] List and record literals recursively parse expressions
- [ ] Error recovery on malformed literals
- [ ] Comprehensive unit tests for each literal type

**File Location**: `src/parser/expression.rs` (new module)

---

### Task 8: Expression Parser - Operators and Precedence

**Description**: Implement expression parser with correct operator precedence using Pratt parsing or precedence climbing.

**Deliverables**:
- Expression parser with precedence levels:
  1. Primary expressions (literals, variables, function calls, parenthesized)
  2. Unary operators (+, -, NOT) - highest precedence
  3. Multiplicative operators (*, /) - right-associative
  4. Additive operators (+, -) - left-associative
  5. String concatenation (||) - left-associative
  6. Comparison operators (=, <>, <, >, <=, >=) - non-associative
  7. Predicates (IS NULL, IS TYPED, EXISTS, etc.)
  8. IS TRUE/FALSE/UNKNOWN tests
  9. NOT operator (boolean negation)
  10. AND operator
  11. XOR operator
  12. OR operator - lowest precedence
- `parse_expression()` - entry point for expression parsing
- `parse_expression_with_precedence()` - Pratt parser implementation
- `parse_primary_expression()` - parse literals, variables, function calls, parenthesized expressions
- Operator binding power tables

**Grammar References**:
- `valueExpression` (Line 2137)
- Operator precedence: Lines 2141-2160

**Acceptance Criteria**:
- [ ] Operator precedence matches ISO GQL specification
- [ ] Associativity rules correctly implemented
- [ ] Complex nested expressions parse correctly
- [ ] Expression parsing is efficient (no excessive backtracking)
- [ ] Error recovery at operator boundaries
- [ ] Tests validate precedence with complex expressions (e.g., `1 + 2 * 3`, `a AND b OR c`)

**File Location**: `src/parser/expression.rs`

---

### Task 9: Expression Parser - Predicates

**Description**: Implement parsing for all predicate types.

**Deliverables**:
- Parser functions for each predicate type:
  - `parse_is_null_predicate()` - IS [NOT] NULL
  - `parse_is_typed_predicate()` - IS [NOT] TYPED type
  - `parse_is_normalized_predicate()` - IS [NOT] NORMALIZED
  - `parse_is_directed_predicate()` - IS [NOT] DIRECTED
  - `parse_is_labeled_predicate()` - IS [NOT] LABELED [:label]
  - `parse_is_truth_value_predicate()` - IS [NOT] TRUE/FALSE/UNKNOWN
  - `parse_is_source_destination_predicate()` - IS [NOT] SOURCE/DESTINATION OF
  - `parse_all_different_predicate()` - ALL_DIFFERENT(...)
  - `parse_same_predicate()` - SAME(e1, e2)
  - `parse_property_exists_predicate()` - PROPERTY_EXISTS(elem, prop)
  - `parse_exists_predicate()` - EXISTS {...} | EXISTS (query)
- Integration with expression parser
- Proper precedence handling for predicates

**Grammar References**:
- Predicate rules (Lines 2036-2130)

**Acceptance Criteria**:
- [ ] All predicate forms parse correctly
- [ ] Negation (NOT) handled properly
- [ ] EXISTS predicate distinguishes graph pattern vs query forms
- [ ] Predicates integrate with boolean expressions (AND/OR/NOT)
- [ ] Error diagnostics for malformed predicates
- [ ] Unit tests for each predicate type

**File Location**: `src/parser/expression.rs`

---

### Task 10: Expression Parser - Built-in Functions

**Description**: Implement parsing for built-in function calls.

**Deliverables**:
- `parse_function_call()` - main function call parser
- Function name resolution (case-insensitive keyword matching)
- Argument list parsing
- Special handling for functions with non-standard syntax:
  - `TRIM([LEADING|TRAILING|BOTH] ... FROM ...)`
  - `CAST(expr AS type)`
  - `EXTRACT(field FROM datetime)`
  - `SUBSTRING(string FROM start [FOR length])`
- Integration with expression parser (function calls are primary expressions)

**Grammar References**:
- `valueFunction` (Line 2165)
- Various function-specific rules (Lines 2165-2825)

**Acceptance Criteria**:
- [ ] All built-in functions parse correctly
- [ ] Function names are case-insensitive
- [ ] Argument lists with variable argument counts supported
- [ ] Special syntax functions (TRIM, CAST) handled
- [ ] Error diagnostics for wrong argument counts
- [ ] User-defined function calls supported (for extensibility)
- [ ] Unit tests for representative functions from each category

**File Location**: `src/parser/expression.rs`

---

### Task 11: Expression Parser - Case and Cast

**Description**: Implement parsing for CASE expressions and CAST operations.

**Deliverables**:
- `parse_case_expression()` - dispatch to simple or searched CASE
- `parse_simple_case()` - CASE operand WHEN value THEN result ...
- `parse_searched_case()` - CASE WHEN condition THEN result ...
- `parse_when_clause()` - parse WHEN...THEN clauses
- `parse_else_clause()` - parse optional ELSE clause
- `parse_cast_expression()` - CAST(expr AS type)
- `parse_nullif()` - NULLIF(e1, e2) - can be represented as function call
- `parse_coalesce()` - COALESCE(e1, e2, ...) - can be represented as function call

**Grammar References**:
- `caseExpression` (Line 2298)
- `simpleCase` (Line 2313)
- `searchedCase` (Line 2317)
- `caseAbbreviation` (Line 2303)
- `castSpecification` (Line 2365)

**Acceptance Criteria**:
- [ ] Both simple and searched CASE forms parse correctly
- [ ] Multiple WHEN clauses supported
- [ ] ELSE clause is optional
- [ ] CASE expressions nest properly in other expressions
- [ ] CAST parses with type placeholder (full type system in Sprint 6)
- [ ] NULLIF and COALESCE parse (as functions or special forms)
- [ ] Error recovery on malformed CASE/CAST
- [ ] Unit tests for both CASE forms and CAST

**File Location**: `src/parser/expression.rs`

---

### Task 12: Expression Parser - Property References and Variables

**Description**: Implement parsing for property access and variable references.

**Deliverables**:
- `parse_property_reference()` - expr.property_name
- `parse_variable_reference()` - binding variables
- `parse_parameter_reference()` - $name dynamic parameters
- `parse_parenthesized_expression()` - (expr)
- Chained property access (e.g., `a.b.c`)
- Field/array indexing if supported (e.g., `list[0]`, `record["field"]`)

**Grammar References**:
- `valueExpressionPrimary` (Line 2220)
- Property reference (Line 2228)
- Binding variable reference (Line 2234)
- `dynamicParameterSpecification` (Line 2280)

**Acceptance Criteria**:
- [ ] Property references parse with proper left-to-right chaining
- [ ] Variable references distinguish element/path/binding variables
- [ ] Parameter references ($name) parse correctly
- [ ] Parenthesized expressions preserve precedence
- [ ] Span tracking captures entire reference chain
- [ ] Error diagnostics for invalid property names
- [ ] Unit tests for chained property access

**File Location**: `src/parser/expression.rs`

---

### Task 13: Expression Parser - Constructors

**Description**: Implement parsing for collection constructors (list, record, path).

**Deliverables**:
- `parse_list_constructor()` - [expr1, expr2, ...]
- `parse_record_constructor()` - RECORD {field: value, ...} or {field: value, ...}
- `parse_path_constructor()` - PATH[node, edge, node, ...]
- Record field parsing with field names and values

**Grammar References**:
- `listValueConstructor` (Line 2496)
- `pathValueConstructor` (Line 2446)
- `recordConstructor` (Line 2514)

**Acceptance Criteria**:
- [ ] List constructors parse with nested expressions
- [ ] Record constructors parse field:value pairs
- [ ] Path constructors parse node/edge sequences
- [ ] Empty constructors supported ([], {}, PATH[])
- [ ] Trailing commas handled appropriately
- [ ] Error recovery on malformed constructors
- [ ] Unit tests for each constructor type

**File Location**: `src/parser/expression.rs`

---

### Task 14: Integration with Statement Parsing

**Description**: Integrate expression parser with existing statement parsers.

**Deliverables**:
- Update session/transaction/catalog parsers to use expression parser:
  - Session parameter values
  - Graph expressions
  - Type expressions (placeholders)
- Prepare integration points for future sprints:
  - MATCH statement (Sprint 7) - will use expressions in WHERE clauses
  - FILTER statement (Sprint 7) - will use expressions as predicates
  - LET statement (Sprint 7) - will use expressions for variable values
  - RETURN statement (Sprint 9) - will use expressions for projections
  - SET statement (Sprint 10) - will use expressions for assignments
- Replace placeholder types in existing AST:
  - `ExpressionPlaceholder` → `Expression`
  - `GraphReferencePlaceholder` → use expression parser where appropriate

**Acceptance Criteria**:
- [ ] Existing statement parsers integrate with expression parser
- [ ] No regressions in existing tests
- [ ] Placeholder types replaced with proper expression AST
- [ ] Expression parser is reusable across all contexts
- [ ] Integration tests validate end-to-end parsing
- [ ] Clear integration points for future sprints documented

**File Location**: `src/parser/program.rs`, `src/parser/session.rs`, `src/parser/catalog.rs`, etc.

---

### Task 15: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for expression parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at operator boundaries
  - Recover at comma separators (in argument lists, constructors)
  - Recover at closing delimiters (parentheses, brackets, braces)
  - Recover at statement boundaries (propagate up from expression parser)
- Diagnostic messages:
  - "Expected expression, found {token}"
  - "Unexpected operator {op}, expected expression"
  - "Unclosed parenthesis/bracket/brace"
  - "Invalid function argument count: expected {n}, found {m}"
  - "Malformed {literal_type} literal"
  - "Unknown function '{name}'"
- Span highlighting for error locations
- Helpful error messages with suggestions

**Acceptance Criteria**:
- [ ] Expression parser recovers from common errors
- [ ] Multiple errors in one expression reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/expression.rs`, `src/diag.rs`

---

### Task 16: Comprehensive Testing

**Description**: Implement comprehensive test suite for expression parsing.

**Deliverables**:

#### Unit Tests (`src/parser/expression.rs`):
- **Literal Tests**:
  - Boolean literals (TRUE, FALSE, UNKNOWN)
  - Null literal
  - Integer literals (decimal, hex, octal, binary)
  - Float literals (decimal, scientific notation)
  - String literals (single/double-quoted, with escapes)
  - Byte string literals
  - Temporal literals (DATE, TIME, DATETIME, TIMESTAMP)
  - Duration literals
  - List literals (empty, single, multiple elements, nested)
  - Record literals (empty, single field, multiple fields, nested)

- **Operator Tests**:
  - Unary operators (+, -, NOT)
  - Arithmetic operators (+, -, *, /, %)
  - Comparison operators (=, <>, <, >, <=, >=)
  - Logical operators (AND, OR, XOR)
  - String concatenation (||)
  - Operator precedence validation (complex expressions)
  - Associativity tests

- **Predicate Tests**:
  - IS NULL / IS NOT NULL
  - IS TYPED type / IS NOT TYPED type
  - IS NORMALIZED / IS NOT NORMALIZED
  - IS DIRECTED / IS NOT DIRECTED
  - IS LABELED / IS NOT LABELED
  - IS TRUE/FALSE/UNKNOWN tests
  - IS SOURCE/DESTINATION OF
  - ALL_DIFFERENT(...)
  - SAME(...)
  - PROPERTY_EXISTS(...)
  - EXISTS {...} (with placeholder pattern)

- **Function Call Tests**:
  - Numeric functions (ABS, MOD, FLOOR, CEIL, SQRT, POWER)
  - Trigonometric functions (SIN, COS, TAN)
  - String functions (UPPER, LOWER, TRIM, NORMALIZE)
  - Datetime functions (CURRENT_DATE, DATE, DURATION_BETWEEN)
  - Cardinality functions (SIZE, CARDINALITY)
  - Special syntax functions (TRIM, CAST)

- **Case Expression Tests**:
  - Simple CASE (with operand)
  - Searched CASE (with conditions)
  - Multiple WHEN clauses
  - With and without ELSE clause
  - Nested CASE expressions

- **Cast Expression Tests**:
  - CAST to different types (with placeholder type references)

- **Property Reference Tests**:
  - Simple property access (a.b)
  - Chained property access (a.b.c)

- **Variable and Parameter Tests**:
  - Binding variables
  - Dynamic parameters ($name)

- **Constructor Tests**:
  - List constructors
  - Record constructors
  - Path constructors

- **Error Recovery Tests**:
  - Missing operands
  - Unclosed parentheses/brackets/braces
  - Invalid operators
  - Malformed function calls
  - Invalid literals

#### Integration Tests (`tests/expression_tests.rs` - new file):
- Complex nested expressions
- Expressions in different statement contexts
- Mixed operator types in single expression
- Edge cases (empty lists, deeply nested expressions)
- Performance tests (deeply nested expressions)

#### Snapshot Tests:
- Capture AST output for representative expressions
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for expression parser
- [ ] All expression variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (empty collections, deeply nested)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/expression.rs`, `tests/expression_tests.rs`

---

### Task 17: Documentation and Examples

**Description**: Document expression parsing system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all AST node types
  - Module-level documentation for `src/ast/expression.rs`
  - Module-level documentation for `src/parser/expression.rs`
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase expression parsing
  - Add `examples/expression_demo.rs` demonstrating:
    - Parsing different literal types
    - Complex nested expressions
    - Predicate examples
    - Function call examples
    - Case expression examples

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for expressions
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all expression error codes
- [ ] Documentation explains operator precedence clearly

**File Location**: `src/ast/expression.rs`, `src/parser/expression.rs`, `examples/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Operator Precedence**: Use Pratt parsing (precedence climbing) for efficient operator precedence handling. This approach:
   - Avoids excessive recursion depth
   - Naturally handles left and right associativity
   - Provides clear precedence levels
   - Supports easy extension with new operators

2. **Expression Context**: Expressions appear in many contexts:
   - WHERE clauses (predicates)
   - RETURN/SELECT projections
   - LET variable bindings
   - SET assignments
   - DEFAULT values
   - Function arguments
   - Constructor elements
   Parser should be context-agnostic and reusable.

3. **Recursive Descent**: Use recursive descent for non-operator expression forms:
   - Function calls
   - Case expressions
   - Constructors
   - Property references

4. **Lookahead**: Minimal lookahead needed:
   - Distinguish CASE simple vs searched (look for operand after CASE)
   - Distinguish predicates from operators (IS keyword)
   - Function vs variable (identifier followed by parenthesis)

5. **Type References**: Sprint 5 includes CAST expressions, which reference types. Use a placeholder `TypeReference` type that will be fully implemented in Sprint 6 (Type System).

### AST Design Considerations

1. **Span Tracking**: Every expression node must track its source span for diagnostic purposes. Use `Spanned<T>` wrapper or embed `span: Span` in every struct.

2. **Box for Recursion**: Use `Box<Expression>` for recursive expression fields to avoid infinite size:
   - Binary expression operands
   - Unary expression operands
   - Case expression results
   - Cast operands

3. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Literal values (preserve original text)
   - Identifiers and function names
   - Property names
   - Parameter names

4. **Operator Enums**: Separate enums for different operator categories:
   - `BinaryOperator`: arithmetic and string operators
   - `ComparisonOperator`: comparison operators
   - `LogicalOperator`: boolean operators
   - `UnaryOperator`: unary plus, minus, not
   This makes pattern matching cleaner and type-safer.

5. **Expression Variants**: Keep `Expression` enum flat enough for easy pattern matching but structured enough to represent semantic differences:
   - Don't create too many fine-grained variants
   - Group related forms (e.g., all literals under `Literal` variant)
   - Use nested enums for complex sub-categories

### Error Recovery Strategy

1. **Synchronization Points**:
   - Commas (in argument lists, array/record elements)
   - Closing delimiters (parentheses, brackets, braces)
   - Operators (can signal end of malformed sub-expression)
   - Statement keywords (propagate up to statement parser)

2. **Panic Mode Recovery**: When error detected:
   - Skip tokens until synchronization point
   - Construct partial AST node
   - Continue parsing
   - Accumulate diagnostics

3. **Operator Recovery**: If operator missing operand:
   - Report error at operator location
   - Insert error token as operand
   - Continue parsing to find more errors

4. **Delimiter Matching**: Track opening delimiters and ensure closing:
   - Use stack for nested delimiters
   - Report unclosed delimiter errors with helpful span

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error"
   - Good: "Expected expression after '+' operator, found ';'"

2. **Helpful Suggestions**:
   - "Did you mean to use '=' instead of '=='?"
   - "Unclosed parenthesis opened at line 42"
   - "Function 'UPPERCASE' not found. Did you mean 'UPPER'?"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing tokens, point to location where token expected
   - For malformed constructs, highlight entire construct
   - For precedence issues, highlight relevant operators

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing function arguments..."
   - "In CASE expression starting at line 42..."

### Performance Considerations

1. **Lexer Efficiency**: Expression tokens are common, so lexer must be fast:
   - Use logos for efficient keyword/operator recognition
   - Minimize allocation for token payloads
   - Use `SmolStr` for inline storage of short strings

2. **Parser Efficiency**: Expression parsing is hot path:
   - Pratt parsing is O(n) with minimal overhead
   - Avoid excessive backtracking
   - Use lookahead sparingly
   - Reuse allocations where possible (e.g., Vec for argument lists)

3. **AST Allocation**: Minimize allocations:
   - Use `Box` only where needed for recursion
   - Use `SmolStr` to avoid heap allocation for short strings
   - Consider arena allocation for AST nodes (future optimization)

4. **Precedence Table**: Operator precedence should be table-driven:
   - Constant-time lookup of operator precedence
   - Separate tables for unary, binary, comparison, logical operators

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (operators, keywords, literals, parameters)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure for integration testing

### Dependencies on Future Sprints

- **Sprint 6**: Type system (CAST expressions reference types - use placeholder)
- **Sprint 7**: Query clauses (WHERE, RETURN, LET will use expressions)
- **Sprint 8**: Graph pattern matching (EXISTS predicate with patterns, label expressions)
- **Sprint 9**: Result shaping (expressions in RETURN/SELECT)
- **Sprint 10**: Data modification (expressions in SET clauses)
- **Sprint 11**: Procedure calls (expressions as arguments)

### Cross-Sprint Integration Points

- Expressions are foundational and will be used throughout all future sprints
- Expression parser must be designed for reusability
- AST expression types should be stable to avoid downstream breakage
- Consider semantic validation in Sprint 14 (type checking, etc.)

## Test Strategy

### Unit Tests

For each expression component:
1. **Happy Path**: Valid expressions parse correctly
2. **Variants**: All operator combinations and expression forms
3. **Error Cases**: Missing operands, invalid operators, malformed syntax
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Expressions in different contexts:
1. **Session Commands**: Expressions in session parameter values
2. **Future Query Clauses**: Prepare test infrastructure for WHERE, RETURN, etc.
3. **Nested Expressions**: Deeply nested expression trees
4. **Mixed Operators**: Complex expressions with multiple operator types

### Snapshot Tests

Capture AST output:
1. Representative expressions from each category
2. Complex nested expressions
3. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid expressions
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries (from future sprints):
1. Identify queries with complex expressions
2. Verify parser handles real-world syntax

### Performance Tests

1. **Deeply Nested Expressions**: Ensure parser handles deep nesting efficiently
2. **Large Argument Lists**: Functions with many arguments
3. **Complex Operator Chains**: Long chains of operators

## Performance Considerations

1. **Lexer Efficiency**: Expression tokens are frequent; lexer must be fast
2. **Parser Efficiency**: Use Pratt parsing for O(n) performance
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Operator Precedence Lookup**: Table-driven for constant-time lookup

## Documentation Requirements

1. **API Documentation**: Rustdoc for all AST nodes and parser functions
2. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
3. **Examples**: Demonstrate expression parsing in examples
4. **Error Catalog**: Document all diagnostic codes and messages
5. **Operator Precedence Table**: Document operator precedence levels clearly

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Operator precedence bugs cause subtle parsing errors | High | Medium | Comprehensive tests with complex expressions; reference ISO GQL spec carefully |
| Expression grammar ambiguity with other clauses | Medium | Low | Clear separation of concerns; expression parser takes token stream |
| Performance degradation with deeply nested expressions | Medium | Low | Use iterative Pratt parsing; performance tests; consider expression depth limits |
| Type system integration complexity (CAST) | Medium | Medium | Use placeholder types in Sprint 5; defer full type system to Sprint 6 |
| Literal parsing edge cases (numeric precision, string escapes) | Low | Medium | Preserve original text in AST; defer semantic validation |
| Error recovery quality degrades with expression complexity | High | Medium | Invest in recovery testing; use synchronization points effectively |
| AST instability causes downstream breakage | High | Low | Design AST carefully; minimize breaking changes; version AST if needed |

## Success Metrics

1. **Coverage**: All expression types parse with correct AST
2. **Correctness**: Operator precedence matches ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for expression parser
6. **Performance**: Parser handles expressions with 100+ operators in <1ms
7. **Reusability**: Expression parser integrates cleanly into future sprints

## Sprint Completion Checklist

- [ ] All tasks completed and reviewed
- [ ] All acceptance criteria met
- [ ] Unit tests pass with >95% coverage
- [ ] Integration tests demonstrate end-to-end functionality
- [ ] Documentation complete (rustdoc, examples, grammar mapping)
- [ ] Performance baseline established
- [ ] Error catalog documented
- [ ] Code review completed
- [ ] CI/CD pipeline passes
- [ ] Expression parser tested in multiple statement contexts
- [ ] AST design reviewed for stability and extensibility
- [ ] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 6: Type System and Reference Forms** will build on the expression foundation to implement complete type grammar and catalog/object reference syntax. With expressions implemented, Sprint 6 can focus on type annotations, type constructors, and how types integrate with expressions (in CAST, IS TYPED, etc.).

---

## Appendix: Operator Precedence Table

Based on ISO GQL specification, operator precedence from highest to lowest:

| Precedence Level | Operators | Associativity | Description |
|------------------|-----------|---------------|-------------|
| 1 | Primary expressions | N/A | Literals, variables, function calls, `(expr)` |
| 2 | Property access `.` | Left | Property references |
| 3 | Unary `+`, `-`, `NOT` | Right | Unary operators |
| 4 | `*`, `/`, `%` | Left | Multiplicative |
| 5 | `+`, `-` | Left | Additive |
| 6 | `||` | Left | String concatenation |
| 7 | `=`, `<>`, `<`, `>`, `<=`, `>=` | Non-associative | Comparison |
| 8 | Predicates | N/A | `IS NULL`, `IS TYPED`, `EXISTS`, etc. |
| 9 | `IS TRUE/FALSE/UNKNOWN` | N/A | Truth value tests |
| 10 | `NOT` | Right | Boolean negation |
| 11 | `AND` | Left | Logical conjunction |
| 12 | `XOR` | Left | Logical exclusive or |
| 13 | `OR` | Left | Logical disjunction |

Notes:
- Non-associative operators cannot be chained without parentheses (e.g., `a < b < c` is invalid)
- Right-associative operators bind from right to left (e.g., `NOT NOT a` = `NOT (NOT a)`)
- Left-associative operators bind from left to right (e.g., `a + b + c` = `(a + b) + c`)

---

## Appendix: Expression Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `unsignedLiteral` | 2913 | `Literal` enum | `parse_literal()` |
| `BOOLEAN_LITERAL` | 2919 | `Literal::Boolean` | `parse_boolean_literal()` |
| `characterStringLiteral` | 2920 | `Literal::String` | `parse_string_literal()` |
| `BYTE_STRING_LITERAL` | 2921 | `Literal::ByteString` | `parse_byte_string_literal()` |
| `temporalLiteral` | 2929 | `Literal::Date/Time/Datetime` | `parse_temporal_literal()` |
| `durationLiteral` | 2924 | `Literal::Duration` | `parse_duration_literal()` |
| `nullLiteral` | 2924 | `Literal::Null` | `parse_null_literal()` |
| `listLiteral` | 2925 | `Literal::List` | `parse_list_literal()` |
| `recordLiteral` | 2926 | `Literal::Record` | `parse_record_literal()` |
| `exactNumericLiteral` | 2982 | `Literal::Integer` | `parse_integer_literal()` |
| `approximateNumericLiteral` | 2990 | `Literal::Float` | `parse_float_literal()` |
| `valueExpression` | 2137 | `Expression` enum | `parse_expression()` |
| `compOp` | 2025 | `ComparisonOperator` | `parse_comparison()` |
| `existsPredicate` | 2036 | `Predicate::Exists` | `parse_exists_predicate()` |
| `nullPredicate` | 2042 | `Predicate::IsNull` | `parse_is_null_predicate()` |
| `valueTypePredicate` | 2052 | `Predicate::IsTyped` | `parse_is_typed_predicate()` |
| `caseExpression` | 2298 | `CaseExpression` | `parse_case_expression()` |
| `simpleCase` | 2313 | `CaseExpression::Simple` | `parse_simple_case()` |
| `searchedCase` | 2317 | `CaseExpression::Searched` | `parse_searched_case()` |
| `castSpecification` | 2365 | `CastExpression` | `parse_cast_expression()` |
| `valueFunction` | 2165 | `FunctionCall` | `parse_function_call()` |
| `dynamicParameterSpecification` | 2280 | `Expression::Parameter` | `parse_parameter_reference()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-17
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4 (completed)
**Next Sprint**: Sprint 6 (Type System and Reference Forms)
