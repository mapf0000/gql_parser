# Sprint 8: Graph Pattern and Path Pattern System

## Sprint Overview

**Sprint Goal**: Deliver full graph matching syntax breadth.

**Sprint Duration**: TBD

**Status**: ✅ **Completed** (February 17, 2026)

**Dependencies**:
- Sprint 1 (Diagnostics and Span Infrastructure) ✅
- Sprint 2 (Lexer Core and Token Model) ✅
- Sprint 3 (Parser Skeleton and Recovery Framework) ✅
- Sprint 4 (Program, Session, Transaction, Catalog Statements) ✅
- Sprint 5 (Values, Literals, and Expression Core) ✅
- Sprint 6 (Type System and Reference Forms) ✅
- Sprint 7 (Query Pipeline Core) ✅

## Scope

This sprint implements the complete graph pattern and path pattern system that forms the core of GQL's graph matching capabilities. Graph patterns enable matching nodes, edges, and paths in property graphs using powerful syntax including quantifiers, path modes, search strategies, and label expressions. Sprint 7 created the MATCH statement structure; Sprint 8 completes the pattern matching content that was deferred.

## Implementation Snapshot (February 17, 2026)

- Completed:
  - Pattern AST surface for Sprint 8 grammar families.
  - Top-level query integration (`MATCH`/`OPTIONAL MATCH` paths flow through program parser).
  - Deterministic graph pattern token consumption with diagnostics and clause-boundary recovery.
  - `SELECT ... FROM MATCH ...` integration with query parser.
  - Full path-pattern directional coverage (all 7 edge directions + abbreviated forms).
  - Graph-pattern quantifiers (`*`, `+`, `?`, `{n}`, `{n,m}`, `{n,}`, `{,m}`).
  - Path mode/search coverage (`WALK`, `TRAIL`, `SIMPLE`, `ACYCLIC`, `ALL`/`ANY`/`SHORTEST` variants).
  - Simplified-path parsing coverage (union, multiset alternation, conjunction, concatenation, negation, questioned, quantified).
  - Label expression coverage (`!`, `&`, `|`, `%`, parenthesized forms, `LABEL`/`LABELS` phrases).
  - Graph pattern `YIELD` parsing and binding-table integration.
  - Comprehensive behavior-driven tests for pattern/query integration, semantics, and recovery.

### Feature Coverage from GQL_FEATURES.md

Sprint 8 maps to the following sections in `GQL_FEATURES.md`:

1. **Section 6: Graph Pattern Matching** (Lines 296-336)
   - Graph pattern binding table
   - Graph pattern yield clause
   - Graph patterns with match modes
   - Match modes (REPEATABLE ELEMENTS, DIFFERENT EDGES)
   - Path pattern lists
   - Path variable declarations
   - Keep clause
   - Graph pattern where clause

2. **Section 7: Path Patterns & Quantifiers** (Lines 339-503)
   - Path pattern prefix (path mode, path search)
   - Path modes (WALK, TRAIL, SIMPLE, ACYCLIC)
   - Path search strategies:
     - ALL path search
     - ANY path search
     - Shortest path search (ALL SHORTEST, ANY SHORTEST, SHORTEST k, SHORTEST k GROUPS)
   - Path pattern expressions (alternation, union)
   - Path terms, factors, and primaries
   - Element patterns (node patterns, edge patterns)
   - Element pattern fillers (variables, labels, properties, predicates)
   - Edge patterns (7 direction types: left, right, undirected, bidirectional, etc.)
   - Abbreviated edge patterns
   - Parenthesized path patterns
   - Graph pattern quantifiers (*, +, ?, {n}, {n,m})
   - Simplified path patterns (alternative simplified syntax)

3. **Section 13: Label Expressions** (Lines 809-850)
   - Label expression operations (negation, conjunction, disjunction)
   - Label names
   - Wildcard labels
   - Parenthesized label expressions
   - Label set phrases
   - Label set specifications

## Exit Criteria

- [x] All graph pattern types parse with correct AST forms
- [x] Graph pattern binding and yield clauses work correctly
- [x] Match modes (REPEATABLE ELEMENTS, DIFFERENT EDGES) parse correctly
- [x] Path pattern lists and path variable declarations work
- [x] Keep clause and graph pattern where clause parse correctly
- [x] Path mode prefix (WALK, TRAIL, SIMPLE, ACYCLIC) parses correctly
- [x] All path search strategies parse (ALL, ANY, SHORTEST variants)
- [x] Path pattern expressions with alternation and union work
- [x] Node patterns parse with all features (variables, labels, properties, predicates)
- [x] Edge patterns parse with all 7 direction types
- [x] Abbreviated edge patterns work correctly
- [x] Element pattern fillers (variables, labels, properties, where clauses) parse
- [x] Graph pattern quantifiers (*, +, ?, {n}, {n,m}) work correctly
- [x] Simplified path pattern syntax parses correctly
- [x] Label expressions parse with all operations (!, &, |, %, parentheses)
- [x] Label set specifications parse correctly
- [x] Parser produces structured diagnostics for malformed patterns
- [x] AST nodes have proper span information for all pattern components
- [x] Recovery mechanisms handle errors at pattern boundaries
- [x] Unit tests cover all pattern variants and error cases
- [x] Pattern parsing integrates with expression parsing from Sprint 5
- [x] Pattern parsing integrates with query parsing from Sprint 7
- [x] Match statement placeholders from Sprint 7 replaced with real pattern parsing

## Implementation Tasks

### Task 1: AST Node Definitions for Graph Patterns

**Description**: Define AST types for graph patterns and match modes.

**Deliverables**:
- `GraphPattern` struct (replacing Sprint 7 placeholder):
  - `match_mode: Option<MatchMode>` - optional match mode
  - `paths: PathPatternList` - path patterns to match
  - `keep_clause: Option<KeepClause>` - optional keep clause
  - `where_clause: Option<GraphPatternWhereClause>` - optional where clause
  - `span: Span`
- `MatchMode` enum:
  - `RepeatableElements` - REPEATABLE ELEMENTS mode
  - `DifferentEdges` - DIFFERENT EDGES mode
- `PathPatternList` struct:
  - `patterns: Vec<PathPattern>` - comma-separated path patterns
  - `span: Span`
- `PathPattern` struct:
  - `prefix: Option<PathPatternPrefix>` - path mode/search prefix
  - `expression: PathPatternExpression` - path pattern expression
  - `variable_declaration: Option<PathVariableDeclaration>` - optional path variable
  - `span: Span`
- `PathVariableDeclaration` struct:
  - `variable: PathVariable` - path variable name
  - `span: Span`
- `KeepClause` struct:
  - `prefix: PathPatternPrefix` - path pattern prefix to keep
  - `span: Span`
- `GraphPatternWhereClause` struct:
  - `condition: Expression` - filter condition (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `graphPattern` (Line 803)
- `matchMode` (Line 807)
- `repeatableElementsMatchMode` (Line 812)
- `differentEdgesMatchMode` (Line 816)
- `pathPatternList` (Line 830)
- `pathVariableDeclaration` (Line 838)
- `keepClause` (Line 842)
- `graphPatternWhereClause` (Line 846)

**Acceptance Criteria**:
- [ ] All graph pattern AST types defined in `src/ast/query.rs` (update Sprint 7 placeholder)
- [ ] Each node has `Span` information
- [ ] Nodes implement necessary traits (Debug, Clone, PartialEq)
- [ ] Documentation comments explain each variant
- [ ] Match modes properly distinguished
- [ ] Integration with Sprint 7 MATCH statement structure clear

**File Location**: `src/ast/query.rs` (update existing file)

---

### Task 2: AST Node Definitions for Path Pattern Prefixes

**Description**: Define AST types for path mode and path search prefixes.

**Deliverables**:
- `PathPatternPrefix` enum:
  - `PathMode(PathMode)` - path mode prefix
  - `PathSearch(PathSearch)` - path search prefix
- `PathMode` enum:
  - `Walk` - WALK (any path, default)
  - `Trail` - TRAIL (no repeated edges)
  - `Simple` - SIMPLE (no repeated nodes or edges)
  - `Acyclic` - ACYCLIC (no repeated nodes)
- `PathSearch` enum with variants:
  - `All(AllPathSearch)` - ALL paths
  - `Any(AnyPathSearch)` - ANY path
  - `Shortest(ShortestPathSearch)` - SHORTEST paths
- `AllPathSearch` struct:
  - `mode: Option<PathMode>` - optional path mode
  - `use_paths_keyword: bool` - whether PATHS keyword present
  - `span: Span`
- `AnyPathSearch` struct:
  - `mode: Option<PathMode>` - optional path mode
  - `span: Span`
- `ShortestPathSearch` enum:
  - `AllShortest { mode: Option<PathMode>, span: Span }` - ALL SHORTEST
  - `AnyShortest { mode: Option<PathMode>, span: Span }` - ANY SHORTEST
  - `CountedShortest { count: Expression, mode: Option<PathMode>, use_paths_keyword: bool, span: Span }` - SHORTEST k [PATHS]
  - `CountedShortestGroups { count: Expression, mode: Option<PathMode>, span: Span }` - SHORTEST k GROUPS

**Grammar References**:
- `pathPatternPrefix` (Line 898)
- `pathModePrefix` (Line 903)
- `pathMode` (Line 907)
- `pathSearchPrefix` (Line 914)
- `allPathSearch` (Line 920)
- `anyPathSearch` (Line 929)
- `shortestPathSearch` (Line 937)
- `allShortestPathSearch` (Line 944)
- `anyShortestPathSearch` (Line 948)
- `countedShortestPathSearch` (Line 952)
- `countedShortestGroupSearch` (Line 956)

**Acceptance Criteria**:
- [ ] All path prefix AST types defined in `src/ast/query.rs`
- [ ] Path modes enumerated correctly (WALK, TRAIL, SIMPLE, ACYCLIC)
- [ ] All path search strategies represented (ALL, ANY, SHORTEST variants)
- [ ] Shortest path variants properly distinguished
- [ ] Optional path modes tracked in each search strategy
- [ ] Count expressions use Expression from Sprint 5
- [ ] Span tracking covers entire prefix
- [ ] Documentation explains each prefix variant

**File Location**: `src/ast/query.rs`

---

### Task 3: AST Node Definitions for Path Pattern Expressions

**Description**: Define AST types for path pattern expressions, terms, factors, and primaries.

**Deliverables**:
- `PathPatternExpression` enum:
  - `Union { left: Box<PathPatternExpression>, right: Box<PathPatternExpression>, span: Span }` - path union
  - `Alternation { alternatives: Vec<PathTerm>, span: Span }` - | alternation
  - `Term(PathTerm)` - single path term
- `PathTerm` struct:
  - `factors: Vec<PathFactor>` - sequential path factors
  - `span: Span`
- `PathFactor` struct:
  - `primary: PathPrimary` - base path element
  - `quantifier: Option<GraphPatternQuantifier>` - optional quantifier
  - `span: Span`
- `PathPrimary` enum:
  - `ElementPattern(ElementPattern)` - node or edge pattern
  - `ParenthesizedExpression(Box<PathPatternExpression>)` - grouped subpattern
- `GraphPatternQuantifier` enum:
  - `Star` - * (zero or more)
  - `Plus` - + (one or more)
  - `QuestionMark` - ? (zero or one)
  - `Fixed { count: u32 }` - {n} (exactly n)
  - `General { min: Option<u32>, max: Option<u32> }` - {n,m}, {n,}, {,m}
  - Each variant includes `span: Span`

**Grammar References**:
- `pathPatternExpression` (Line 966)
- `pathTerm` (Line 972)
- `pathFactor` (Line 976)
- `pathPrimary` (Line 982)
- `graphPatternQuantifier` (Line 1125)
- `fixedQuantifier` (Line 1132)
- `generalQuantifier` (Line 1136)

**Acceptance Criteria**:
- [ ] All path expression AST types defined
- [ ] Recursive path expression structure supported (union, alternation)
- [ ] Path term supports sequential concatenation
- [ ] Path factor links primary with optional quantifier
- [ ] All quantifier forms represented (*, +, ?, {n}, {n,m})
- [ ] Parenthesized subpatterns supported for precedence control
- [ ] Span tracking covers entire expression tree
- [ ] Documentation explains expression composition

**File Location**: `src/ast/query.rs`

---

### Task 4: AST Node Definitions for Element Patterns (Nodes)

**Description**: Define AST types for node patterns and element pattern fillers.

**Deliverables**:
- `ElementPattern` enum:
  - `Node(NodePattern)` - node pattern
  - `Edge(EdgePattern)` - edge pattern
- `NodePattern` struct:
  - `variable: Option<ElementVariableDeclaration>` - optional element variable
  - `label_expression: Option<LabelExpression>` - optional label filter
  - `properties: Option<ElementPropertySpecification>` - optional property filters
  - `where_clause: Option<ElementPatternPredicate>` - optional where predicate
  - `span: Span`
- `ElementVariableDeclaration` struct:
  - `variable: ElementVariable` - element variable name
  - `span: Span`
- `ElementPropertySpecification` struct:
  - `properties: Vec<PropertyKeyValuePair>` - property constraints
  - `span: Span`
- `PropertyKeyValuePair` struct:
  - `key: SmolStr` - property name
  - `value: Expression` - property value expression (from Sprint 5)
  - `span: Span`
- `ElementPatternPredicate` struct:
  - `condition: Expression` - where condition (from Sprint 5)
  - `span: Span`

**Grammar References**:
- `elementPattern` (Line 988)
- `nodePattern` (Line 993)
- `elementPatternFiller` (Line 997)
- `elementVariableDeclaration` (Line 1001)
- `isLabelExpression` (Line 1005)
- `elementPatternPredicate` (Line 1014)
- `elementPropertySpecification` (Line 1023)

**Acceptance Criteria**:
- [ ] Node pattern AST defined
- [ ] All element pattern filler components optional (variables, labels, properties, where)
- [ ] Variable declarations tracked
- [ ] Label expressions integrated (Task 9)
- [ ] Property specifications use expressions from Sprint 5
- [ ] Where predicates use expressions from Sprint 5
- [ ] Span tracking for each component
- [ ] Documentation explains node pattern matching semantics

**File Location**: `src/ast/query.rs`

---

### Task 5: AST Node Definitions for Element Patterns (Edges)

**Description**: Define AST types for edge patterns with all direction variants.

**Deliverables**:
- `EdgePattern` enum with 7 direction types:
  - `Full(FullEdgePattern)` - full edge pattern with details
  - `Abbreviated(AbbreviatedEdgePattern)` - abbreviated edge syntax
- `FullEdgePattern` struct:
  - `direction: EdgeDirection` - edge direction
  - `filler: FullEdgePointingFiller` - edge details (variable, labels, properties, where)
  - `span: Span`
- `EdgeDirection` enum:
  - `PointingLeft` - <-[edge]-
  - `PointingRight` - -[edge]->
  - `Undirected` - ~[edge]~
  - `AnyDirected` - <-[edge]->
  - `LeftOrUndirected` - <~[edge]~
  - `AnyDirection` - -[edge]-
  - `RightOrUndirected` - ~[edge]->
- `FullEdgePointingFiller` struct:
  - `variable: Option<ElementVariableDeclaration>` - optional element variable
  - `label_expression: Option<LabelExpression>` - optional label filter
  - `properties: Option<ElementPropertySpecification>` - optional property filters
  - `where_clause: Option<ElementPatternPredicate>` - optional where predicate
  - `span: Span`
- `AbbreviatedEdgePattern` enum:
  - `LeftArrow` - <-
  - `RightArrow` - ->
  - `Undirected` - ~
  - `AnyDirection` - -
  - Each variant includes `span: Span`

**Grammar References**:
- `edgePattern` (Line 1035)
- `fullEdgePattern` (Line 1040)
- `fullEdgePointingLeft` (Line 1050)
- `fullEdgePointingRight` (Line 1054)
- `fullEdgeUndirected` (Line 1058)
- `fullEdgeAnyDirection` (Line 1062)
- `fullEdgeLeftOrUndirected` (Line 1066)
- `fullEdgeUndirectedOrRight` (Line 1070)
- `fullEdgeLeftOrRight` (Line 1074)
- `abbreviatedEdgePattern` (Line 1078)

**Acceptance Criteria**:
- [ ] All 7 full edge direction types enumerated
- [ ] Edge pattern filler structure matches node pattern filler
- [ ] Abbreviated edge patterns support 4 arrow forms
- [ ] Variable, label, property, and where components optional
- [ ] Label expressions integrated (Task 9)
- [ ] Property specifications use expressions from Sprint 5
- [ ] Where predicates use expressions from Sprint 5
- [ ] Span tracking for each edge variant
- [ ] Documentation explains edge direction semantics

**File Location**: `src/ast/query.rs`

---

### Task 6: AST Node Definitions for Simplified Path Patterns

**Description**: Define AST types for simplified path pattern syntax.

**Deliverables**:
- `SimplifiedPathPattern` struct:
  - `expression: SimplifiedPathPatternExpression` - simplified pattern expression
  - `span: Span`
- `SimplifiedPathPatternExpression` enum:
  - Simplified variants for all 7 edge directions
  - Union and alternation operations
  - Quantified patterns
  - Direction overrides
  - Negation
- `SimplifiedContents` struct:
  - Simplified pattern contents (placeholder for detailed specification)
  - `span: Span`
- `SimplifiedPathUnion` struct:
  - Union of simplified paths
  - `span: Span`
- `SimplifiedMultisetAlternation` struct:
  - `|+|` operator for multiset alternation
  - `span: Span`
- `SimplifiedQuantified` struct:
  - Quantifiers in simplified syntax
  - `span: Span`
- `SimplifiedQuestioned` struct:
  - Optional patterns in simplified syntax
  - `span: Span`
- `SimplifiedDirectionOverride` struct:
  - Override edge direction
  - `span: Span`
- `SimplifiedNegation` struct:
  - Negated patterns
  - `span: Span`

**Grammar References**:
- `simplifiedPathPatternExpression` (Line 1150)
- `simplifiedContents` (Line 1188)
- `simplifiedPathUnion` (Line 1194)
- `simplifiedMultisetAlternation` (Line 1198)
- `simplifiedQuantified` (Line 1218)
- `simplifiedQuestioned` (Line 1222)
- `simplifiedDirectionOverride` (Line 1231)
- `simplifiedNegation` (Line 1274)

**Acceptance Criteria**:
- [ ] Simplified path pattern AST types defined
- [ ] All 7 direction types supported in simplified form
- [ ] Simplified union and alternation operations represented
- [ ] Quantified and questioned patterns supported
- [ ] Direction override mechanism defined
- [ ] Negation supported
- [ ] Span tracking for simplified patterns
- [ ] Documentation notes simplified syntax as alternative to standard patterns
- [ ] Clear distinction between simplified and standard path patterns

**File Location**: `src/ast/query.rs`

---

### Task 7: AST Node Definitions for Graph Pattern Binding and Yield

**Description**: Define AST types for graph pattern binding table and yield clause.

**Deliverables**:
- `GraphPatternBindingTable` struct:
  - Bind pattern matches to binding tables (placeholder for detailed specification)
  - `span: Span`
- `GraphPatternYieldClause` struct:
  - `items: Vec<YieldItem>` - yield items
  - `span: Span`
- `YieldItem` struct:
  - `expression: Expression` - expression to yield (from Sprint 5)
  - `alias: Option<SmolStr>` - optional alias
  - `span: Span`

**Grammar References**:
- `graphPatternBindingTable` (Line 779)
- `graphPatternYieldClause` (Line 783)

**Acceptance Criteria**:
- [ ] Graph pattern binding table AST defined
- [ ] Yield clause AST defined with yield items
- [ ] Yield items use expressions from Sprint 5
- [ ] Optional aliases supported for yield items
- [ ] Span tracking for binding and yield structures
- [ ] Documentation explains binding table and yield semantics

**File Location**: `src/ast/query.rs`

---

### Task 8: AST Node Definitions for Parenthesized Path Patterns

**Description**: Define AST types for parenthesized path pattern expressions.

**Deliverables**:
- `ParenthesizedPathPatternExpression` struct:
  - `expression: Box<PathPatternExpression>` - nested path expression
  - `span: Span`

**Grammar References**:
- `parenthesizedPathPatternExpression` (Line 1088)

**Acceptance Criteria**:
- [ ] Parenthesized path pattern AST defined
- [ ] Recursive nesting supported via Box
- [ ] Span tracking includes parentheses
- [ ] Documentation explains grouping for precedence control

**File Location**: `src/ast/query.rs`

---

### Task 9: AST Node Definitions for Label Expressions

**Description**: Define AST types for label expressions and label set specifications.

**Deliverables**:
- `LabelExpression` enum:
  - `Negation { operand: Box<LabelExpression>, span: Span }` - ! negation
  - `Conjunction { left: Box<LabelExpression>, right: Box<LabelExpression>, span: Span }` - & AND
  - `Disjunction { left: Box<LabelExpression>, right: Box<LabelExpression>, span: Span }` - | OR
  - `LabelName { name: SmolStr, span: Span }` - simple label name
  - `Wildcard { span: Span }` - % wildcard
  - `Parenthesized { expression: Box<LabelExpression>, span: Span }` - (expr)
- `IsLabelExpression` struct:
  - `expression: LabelExpression` - label expression
  - `span: Span`
- `LabelSetSpecification` struct:
  - `labels: Vec<SmolStr>` - ampersand-separated labels
  - `span: Span`
- `LabelSetPhrase` enum:
  - `Label` - LABEL keyword
  - `Labels` - LABELS keyword

**Grammar References**:
- `labelExpression` (Line 1102)
- `isLabelExpression` (Line 1005)
- `labelSetPhrase` (Line 1679)
- `labelSetSpecification` (Line 1685)

**Acceptance Criteria**:
- [ ] All label expression operations represented (!, &, |, %, parens)
- [ ] Recursive label expression structure supported
- [ ] Label names use SmolStr for efficiency
- [ ] Wildcard variant for matching any label
- [ ] Label set specifications track ampersand-separated lists
- [ ] LABEL vs LABELS keyword distinction tracked
- [ ] Span tracking for each label expression node
- [ ] Documentation explains label expression boolean algebra

**File Location**: `src/ast/query.rs` (or new `src/ast/labels.rs`)

---

### Task 10: Lexer Extensions for Graph Pattern Tokens

**Description**: Ensure lexer supports all tokens needed for graph pattern and path pattern parsing.

**Deliverables**:
- Verify existing pattern keywords are sufficient:
  - Match modes: REPEATABLE, ELEMENTS, DIFFERENT, EDGES
  - Path modes: WALK, TRAIL, SIMPLE, ACYCLIC
  - Path search: ALL, ANY, SHORTEST, PATHS, GROUPS
  - Pattern keywords: KEEP, WHERE
  - Quantifiers: *, +, ?
  - Label operations: !, &, |, %
- Add any missing keywords to keyword table
- Ensure edge direction operators tokenized correctly:
  - `<-[` (left pointing start)
  - `]-` (edge end neutral)
  - `]->` (right pointing end)
  - `~[` (undirected start)
  - `]~` (undirected end)
  - Abbreviated: `<-`, `->`, `~`, `-`
- Ensure quantifier syntax tokenized:
  - `{`, `}` (braces for quantifiers)
  - Numeric literals in quantifiers
- Ensure label expression operators tokenized:
  - `!` (negation, may conflict with NOT operator)
  - `&` (conjunction, may conflict with bitwise AND)
  - `|` (disjunction, may conflict with bitwise OR or alternation)
  - `%` (wildcard)
- Ensure simplified pattern operator tokenized:
  - `|+|` (multiset alternation operator)

**Lexer Enhancements Needed**:
- Add REPEATABLE, ELEMENTS, DIFFERENT keywords if missing
- Add WALK, TRAIL, SIMPLE, ACYCLIC keywords if missing
- Add SHORTEST, GROUPS keywords if missing
- Ensure edge arrow operators are distinct tokens
- Handle `|+|` as single operator token
- Disambiguate label operators from expression operators (context-dependent)
- Ensure brace quantifiers parse correctly

**Grammar References**:
- Pattern keyword definitions throughout Lines 803-1281
- Edge direction operators (Lines 1050-1076)
- Quantifier syntax (Lines 1125-1146)
- Label expression operators (Lines 1102-1109)
- Simplified pattern operators (Line 1198)

**Acceptance Criteria**:
- [ ] All pattern keywords tokenized correctly
- [ ] Case-insensitive keyword matching works
- [ ] Edge direction operators tokenized as distinct tokens
- [ ] Quantifier braces and syntax work correctly
- [ ] Label expression operators disambiguated from expression operators
- [ ] `|+|` operator tokenized correctly
- [ ] No new lexer errors introduced
- [ ] All pattern-related tokens have proper span information

**File Location**: `src/lexer/keywords.rs`, `src/lexer/mod.rs`, `src/lexer/token.rs`

---

### Task 11: Pattern Parser - Graph Patterns and Match Modes

**Description**: Implement parsing for graph patterns and match modes.

**Deliverables**:
- `parse_graph_pattern()` - parse complete graph pattern (replace Sprint 7 placeholder)
- `parse_match_mode()` - parse REPEATABLE ELEMENTS or DIFFERENT EDGES
- `parse_path_pattern_list()` - parse comma-separated path patterns
- `parse_path_variable_declaration()` - parse path variable declarations
- `parse_keep_clause()` - parse KEEP clause
- `parse_graph_pattern_where_clause()` - parse WHERE clause in graph pattern
- Integration with expression parser from Sprint 5 for where clause conditions

**Grammar References**:
- `graphPattern` (Line 803)
- `matchMode` (Line 807)
- `repeatableElementsMatchMode` (Line 812)
- `differentEdgesMatchMode` (Line 816)
- `pathPatternList` (Line 830)
- `pathVariableDeclaration` (Line 838)
- `keepClause` (Line 842)
- `graphPatternWhereClause` (Line 846)

**Acceptance Criteria**:
- [ ] Graph patterns parse with all optional components
- [ ] Match modes (REPEATABLE ELEMENTS, DIFFERENT EDGES) parse correctly
- [ ] Path pattern lists support multiple comma-separated patterns
- [ ] Path variable declarations work
- [ ] KEEP clause parses with path pattern prefix
- [ ] WHERE clause uses expression parser from Sprint 5
- [ ] Error recovery at pattern boundaries
- [ ] Unit tests for graph pattern variants

**File Location**: `src/parser/query.rs` (update existing file from Sprint 7)

---

### Task 12: Pattern Parser - Path Pattern Prefixes

**Description**: Implement parsing for path mode and path search prefixes.

**Deliverables**:
- `parse_path_pattern_prefix()` - dispatch to mode or search prefix
- `parse_path_mode_prefix()` - parse WALK, TRAIL, SIMPLE, ACYCLIC
- `parse_path_mode()` - parse individual path mode
- `parse_path_search_prefix()` - dispatch to search strategy parsers
- `parse_all_path_search()` - ALL [path_mode] [PATHS]
- `parse_any_path_search()` - ANY [path_mode]
- `parse_shortest_path_search()` - dispatch to shortest variants
- `parse_all_shortest_path_search()` - ALL SHORTEST [path_mode]
- `parse_any_shortest_path_search()` - ANY SHORTEST [path_mode]
- `parse_counted_shortest_path_search()` - SHORTEST k [path_mode] [PATHS]
- `parse_counted_shortest_group_search()` - SHORTEST k [path_mode] GROUPS

**Grammar References**:
- `pathPatternPrefix` (Line 898)
- `pathModePrefix` (Line 903)
- `pathMode` (Line 907)
- `pathSearchPrefix` (Line 914)
- `allPathSearch` (Line 920)
- `anyPathSearch` (Line 929)
- `shortestPathSearch` (Line 937)
- `allShortestPathSearch` (Line 944)
- `anyShortestPathSearch` (Line 948)
- `countedShortestPathSearch` (Line 952)
- `countedShortestGroupSearch` (Line 956)

**Acceptance Criteria**:
- [ ] All path modes parse correctly
- [ ] ALL path search with optional mode and PATHS keyword works
- [ ] ANY path search with optional mode works
- [ ] ALL SHORTEST and ANY SHORTEST variants parse
- [ ] SHORTEST k with count expression parses (uses Sprint 5 expressions)
- [ ] SHORTEST k GROUPS variant works
- [ ] Optional path modes tracked in each search strategy
- [ ] Error recovery on malformed prefixes
- [ ] Unit tests for all prefix variants

**File Location**: `src/parser/query.rs`

---

### Task 13: Pattern Parser - Path Pattern Expressions

**Description**: Implement parsing for path pattern expressions with alternation and union.

**Deliverables**:
- `parse_path_pattern_expression()` - parse path expression with operators
- `parse_path_term()` - parse sequential path factors
- `parse_path_factor()` - parse primary with optional quantifier
- `parse_path_primary()` - dispatch to element pattern or parenthesized expression
- Operator precedence handling:
  - Union (lowest precedence)
  - Alternation (|)
  - Sequential concatenation (highest precedence)
- Quantifier binding (quantifier applies to immediately preceding primary)

**Grammar References**:
- `pathPatternExpression` (Line 966)
- `pathTerm` (Line 972)
- `pathFactor` (Line 976)
- `pathPrimary` (Line 982)

**Acceptance Criteria**:
- [ ] Path pattern expressions parse with alternation (|)
- [ ] Path union operations work
- [ ] Sequential concatenation (path terms) works
- [ ] Quantifiers bind to primaries correctly
- [ ] Parenthesized subexpressions work for precedence control
- [ ] Operator precedence matches grammar specification
- [ ] Error recovery on malformed expressions
- [ ] Unit tests for expression composition

**File Location**: `src/parser/query.rs`

---

### Task 14: Pattern Parser - Graph Pattern Quantifiers

**Description**: Implement parsing for graph pattern quantifiers.

**Deliverables**:
- `parse_graph_pattern_quantifier()` - dispatch to quantifier types
- `parse_fixed_quantifier()` - {n}
- `parse_general_quantifier()` - {n,m}, {n,}, {,m}
- Quantifier syntax:
  - `*` - Kleene star
  - `+` - Kleene plus
  - `?` - optional
  - `{n}` - exactly n
  - `{n,m}` - between n and m
  - `{n,}` - at least n
  - `{,m}` - at most m
- Validate quantifier bounds (n ≤ m)

**Grammar References**:
- `graphPatternQuantifier` (Line 1125)
- `fixedQuantifier` (Line 1132)
- `generalQuantifier` (Line 1136)

**Acceptance Criteria**:
- [ ] All quantifier forms parse correctly
- [ ] `*`, `+`, `?` operators work
- [ ] Fixed quantifier {n} parses
- [ ] General quantifier {n,m} variants parse
- [ ] Quantifier bounds validated (n ≤ m)
- [ ] Error diagnostics for invalid quantifiers
- [ ] Unit tests for all quantifier forms

**File Location**: `src/parser/query.rs`

---

### Task 15: Pattern Parser - Element Patterns (Nodes)

**Description**: Implement parsing for node patterns and element pattern fillers.

**Deliverables**:
- `parse_element_pattern()` - dispatch to node or edge pattern
- `parse_node_pattern()` - (variable :label {props} WHERE pred)
- `parse_element_pattern_filler()` - parse filler components
- `parse_element_variable_declaration()` - parse element variable
- `parse_element_property_specification()` - {prop1: val1, prop2: val2}
- `parse_element_pattern_predicate()` - WHERE clause in pattern
- Integration with label expression parser (Task 17)
- Integration with expression parser from Sprint 5

**Grammar References**:
- `elementPattern` (Line 988)
- `nodePattern` (Line 993)
- `elementPatternFiller` (Line 997)
- `elementVariableDeclaration` (Line 1001)
- `isLabelExpression` (Line 1005)
- `elementPatternPredicate` (Line 1014)
- `elementPropertySpecification` (Line 1023)

**Acceptance Criteria**:
- [ ] Node patterns parse with all optional components
- [ ] Variable declarations work
- [ ] Label expressions integrated (from Task 17)
- [ ] Property specifications parse with key-value pairs
- [ ] Property values use expressions from Sprint 5
- [ ] WHERE predicates use expressions from Sprint 5
- [ ] Error recovery on malformed node patterns
- [ ] Unit tests for node pattern variants

**File Location**: `src/parser/query.rs`

---

### Task 16: Pattern Parser - Element Patterns (Edges)

**Description**: Implement parsing for edge patterns with all direction types.

**Deliverables**:
- `parse_edge_pattern()` - dispatch to full or abbreviated edge
- `parse_full_edge_pattern()` - parse full edge with filler
- Parse all 7 full edge direction types:
  - `parse_full_edge_pointing_left()` - <-[edge]-
  - `parse_full_edge_pointing_right()` - -[edge]->
  - `parse_full_edge_undirected()` - ~[edge]~
  - `parse_full_edge_any_direction()` - <-[edge]->
  - `parse_full_edge_left_or_undirected()` - <~[edge]~
  - `parse_full_edge_undirected_or_right()` - ~[edge]->
  - `parse_full_edge_left_or_right()` - -[edge]-
- `parse_full_edge_pointing_filler()` - parse edge filler (variable, labels, properties, where)
- `parse_abbreviated_edge_pattern()` - parse <-, ->, ~, -
- Integration with label expression parser (Task 17)
- Integration with expression parser from Sprint 5

**Grammar References**:
- `edgePattern` (Line 1035)
- `fullEdgePattern` (Line 1040)
- `fullEdgePointingLeft` (Line 1050)
- `fullEdgePointingRight` (Line 1054)
- `fullEdgeUndirected` (Line 1058)
- `fullEdgeAnyDirection` (Line 1062)
- `fullEdgeLeftOrUndirected` (Line 1066)
- `fullEdgeUndirectedOrRight` (Line 1070)
- `fullEdgeLeftOrRight` (Line 1074)
- `abbreviatedEdgePattern` (Line 1078)

**Acceptance Criteria**:
- [ ] All 7 full edge direction types parse correctly
- [ ] Edge filler components parse (variable, labels, properties, where)
- [ ] Abbreviated edge patterns work
- [ ] Label expressions integrated (from Task 17)
- [ ] Property specifications use expressions from Sprint 5
- [ ] WHERE predicates use expressions from Sprint 5
- [ ] Error recovery on malformed edge patterns
- [ ] Unit tests for all edge direction types

**File Location**: `src/parser/query.rs`

---

### Task 17: Pattern Parser - Label Expressions

**Description**: Implement parsing for label expressions with boolean algebra.

**Deliverables**:
- `parse_label_expression()` - parse label expression with operators
- `parse_is_label_expression()` - :label_expression in patterns
- Operator precedence:
  - `!` (negation, highest precedence)
  - `&` (conjunction, medium precedence)
  - `|` (disjunction, lowest precedence)
  - Parentheses for grouping
- `parse_label_name()` - simple label name
- `parse_label_wildcard()` - % wildcard
- `parse_label_set_specification()` - ampersand-separated labels
- `parse_label_set_phrase()` - LABEL or LABELS keyword

**Grammar References**:
- `labelExpression` (Line 1102)
- `isLabelExpression` (Line 1005)
- `labelSetPhrase` (Line 1679)
- `labelSetSpecification` (Line 1685)

**Acceptance Criteria**:
- [ ] All label expression operations parse (!, &, |, %)
- [ ] Operator precedence matches specification
- [ ] Parenthesized label expressions work
- [ ] Label names parse correctly
- [ ] Wildcard % operator works
- [ ] Label set specifications parse ampersand-separated lists
- [ ] LABEL vs LABELS keyword distinction tracked
- [ ] Error recovery on malformed label expressions
- [ ] Unit tests for label expression combinations

**File Location**: `src/parser/query.rs` (or new `src/parser/labels.rs`)

---

### Task 18: Pattern Parser - Simplified Path Patterns

**Description**: Implement parsing for simplified path pattern syntax.

**Deliverables**:
- `parse_simplified_path_pattern_expression()` - parse simplified patterns
- Parse simplified variants for all 7 edge directions
- `parse_simplified_contents()` - parse simplified pattern contents
- `parse_simplified_path_union()` - parse union of simplified paths
- `parse_simplified_multiset_alternation()` - parse |+| operator
- `parse_simplified_quantified()` - parse quantifiers in simplified syntax
- `parse_simplified_questioned()` - parse optional patterns in simplified syntax
- `parse_simplified_direction_override()` - parse direction override
- `parse_simplified_negation()` - parse negated patterns

**Grammar References**:
- `simplifiedPathPatternExpression` (Line 1150)
- `simplifiedContents` (Line 1188)
- `simplifiedPathUnion` (Line 1194)
- `simplifiedMultisetAlternation` (Line 1198)
- `simplifiedQuantified` (Line 1218)
- `simplifiedQuestioned` (Line 1222)
- `simplifiedDirectionOverride` (Line 1231)
- `simplifiedNegation` (Line 1274)

**Acceptance Criteria**:
- [ ] Simplified path patterns parse correctly
- [ ] All 7 direction types supported in simplified form
- [ ] Simplified union and alternation operations work
- [ ] `|+|` multiset alternation operator works
- [ ] Quantified patterns parse in simplified syntax
- [ ] Optional patterns (questioned) parse
- [ ] Direction override mechanism works
- [ ] Negation parses correctly
- [ ] Error recovery on malformed simplified patterns
- [ ] Unit tests for simplified pattern variants
- [ ] Documentation explains simplified vs standard pattern differences

**File Location**: `src/parser/query.rs`

---

### Task 19: Pattern Parser - Graph Pattern Binding and Yield

**Description**: Implement parsing for graph pattern binding table and yield clause.

**Deliverables**:
- `parse_graph_pattern_binding_table()` - parse binding table
- `parse_graph_pattern_yield_clause()` - YIELD <yield_items>
- `parse_yield_item()` - parse individual yield item with optional alias
- Integration with expression parser from Sprint 5

**Grammar References**:
- `graphPatternBindingTable` (Line 779)
- `graphPatternYieldClause` (Line 783)

**Acceptance Criteria**:
- [ ] Graph pattern binding table parses
- [ ] Yield clause parses with yield items
- [ ] Yield items use expressions from Sprint 5
- [ ] Optional aliases work for yield items
- [ ] Error recovery on malformed yield clauses
- [ ] Unit tests for binding and yield clauses

**File Location**: `src/parser/query.rs`

---

### Task 20: Pattern Parser - Parenthesized Path Patterns

**Description**: Implement parsing for parenthesized path pattern expressions.

**Deliverables**:
- `parse_parenthesized_path_pattern_expression()` - (<path_pattern_expression>)
- Recursive nesting support
- Precedence control via parentheses

**Grammar References**:
- `parenthesizedPathPatternExpression` (Line 1088)

**Acceptance Criteria**:
- [ ] Parenthesized path patterns parse correctly
- [ ] Recursive nesting works
- [ ] Precedence control via parentheses works
- [ ] Error recovery on unclosed parentheses
- [ ] Unit tests for nested parenthesized patterns

**File Location**: `src/parser/query.rs`

---

### Task 21: Integration with Sprint 5 (Expression Parser)

**Description**: Integrate pattern parser with expression parser from Sprint 5.

**Deliverables**:
- Use expression parser for:
  - Element property values in property specifications
  - WHERE clause conditions in element patterns
  - WHERE clause conditions in graph patterns
  - Yield item expressions
  - Quantifier count expressions (SHORTEST k)
- Ensure no parser conflicts between pattern and expression parsing
- Test expressions in all pattern contexts

**Acceptance Criteria**:
- [ ] All pattern parsers use expression parser correctly
- [ ] No parser conflicts between pattern and expression parsing
- [ ] Expressions work in all pattern contexts
- [ ] Integration tests validate end-to-end parsing
- [ ] Expression parsing is context-aware

**File Location**: `src/parser/query.rs`, `src/parser/expression.rs`

---

### Task 22: Integration with Sprint 7 (Query Parser)

**Description**: Integrate pattern parser with query parser from Sprint 7.

**Deliverables**:
- Replace GraphPattern placeholder from Sprint 7 with real implementation
- Update MATCH statement parsing to use real pattern parser
- Test MATCH statements with all pattern variants
- Ensure query pipeline works with complete pattern parsing

**Acceptance Criteria**:
- [ ] GraphPattern placeholder fully replaced
- [ ] MATCH statements parse with real patterns
- [ ] All Sprint 7 query tests work with real pattern parsing
- [ ] No regressions in existing query tests
- [ ] Integration tests validate end-to-end query parsing with patterns

**File Location**: `src/parser/query.rs`

---

### Task 23: Error Recovery and Diagnostics

**Description**: Implement comprehensive error recovery and diagnostic quality for pattern parsing.

**Deliverables**:
- Error recovery strategies:
  - Recover at element pattern boundaries (node, edge)
  - Recover at path pattern boundaries
  - Recover at quantifier boundaries
  - Recover at comma separators (in path pattern lists, property specs)
  - Recover at closing delimiters (], ), })
- Diagnostic messages:
  - "Expected element pattern, found {token}"
  - "Invalid edge direction syntax"
  - "Quantifier requires closing brace }"
  - "Label expression syntax error: expected label name or operator"
  - "SHORTEST requires count expression"
  - "Malformed node pattern: expected variable, label, properties, or WHERE"
  - "Edge pattern missing closing bracket ]"
  - "Property specification requires key: value pairs"
- Span highlighting for error locations
- Helpful error messages with suggestions:
  - "Did you mean '-[edge]->' instead of '-[edge->'?"
  - "Node patterns use parentheses: (variable :label {props})"
  - "Edge patterns use brackets: -[edge]->"
  - "Quantifiers use braces: {n,m}"
  - "Label expressions use : prefix: :Label"

**Grammar References**:
- All pattern parsing rules (Lines 777-1281)

**Acceptance Criteria**:
- [ ] Pattern parser recovers from common errors
- [ ] Multiple errors in one pattern reported
- [ ] Error messages are clear and actionable
- [ ] Span information highlights exact error location
- [ ] Recovery produces sensible partial AST
- [ ] Suggestions provided for common pattern syntax errors
- [ ] Tests validate error recovery behavior

**File Location**: `src/parser/query.rs`, `src/diag.rs`

---

### Task 24: Comprehensive Testing

**Description**: Implement comprehensive test suite for pattern parsing.

**Deliverables**:

#### Unit Tests (`src/parser/query.rs`):
- **Graph Pattern Tests**:
  - Graph patterns with all optional components
  - Match modes (REPEATABLE ELEMENTS, DIFFERENT EDGES)
  - Path pattern lists with multiple patterns
  - Path variable declarations
  - KEEP clause
  - Graph pattern WHERE clause

- **Path Prefix Tests**:
  - Path modes (WALK, TRAIL, SIMPLE, ACYCLIC)
  - ALL path search (with/without PATHS keyword)
  - ANY path search
  - ALL SHORTEST path search
  - ANY SHORTEST path search
  - SHORTEST k path search
  - SHORTEST k GROUPS

- **Path Expression Tests**:
  - Path alternation (|)
  - Path union
  - Sequential concatenation (path terms)
  - Parenthesized subexpressions

- **Quantifier Tests**:
  - `*` (Kleene star)
  - `+` (Kleene plus)
  - `?` (optional)
  - `{n}` (fixed)
  - `{n,m}` (general)
  - `{n,}` (at least n)
  - `{,m}` (at most m)

- **Node Pattern Tests**:
  - Empty node patterns ()
  - Node patterns with variables
  - Node patterns with label expressions
  - Node patterns with property specifications
  - Node patterns with WHERE clauses
  - Node patterns with all components combined

- **Edge Pattern Tests**:
  - All 7 full edge direction types
  - Abbreviated edge patterns (<-, ->, ~, -)
  - Edge patterns with variables
  - Edge patterns with label expressions
  - Edge patterns with property specifications
  - Edge patterns with WHERE clauses

- **Label Expression Tests**:
  - Simple label names
  - Negation (!)
  - Conjunction (&)
  - Disjunction (|)
  - Wildcard (%)
  - Parenthesized expressions
  - Complex combinations

- **Simplified Pattern Tests**:
  - Simplified path patterns for all 7 directions
  - Simplified union and alternation
  - `|+|` multiset alternation
  - Simplified quantifiers
  - Simplified questioned patterns
  - Direction override
  - Negation

- **Binding and Yield Tests**:
  - Graph pattern binding tables
  - Yield clauses with expressions
  - Yield items with aliases

- **Error Recovery Tests**:
  - Missing closing brackets/braces/parentheses
  - Invalid edge direction syntax
  - Malformed quantifiers
  - Invalid label expressions
  - Malformed node/edge patterns

#### Integration Tests (`tests/pattern_tests.rs` - new file):
- Complete MATCH statements with complex patterns
- Patterns with expressions from Sprint 5
- Patterns in query pipeline from Sprint 7
- Nested patterns with multiple levels
- Patterns with all quantifier variants
- Edge cases (deeply nested, complex label expressions)

#### Snapshot Tests:
- Capture AST output for representative patterns
- Ensure AST changes are intentional

**Acceptance Criteria**:
- [ ] >95% code coverage for pattern parser
- [ ] All pattern variants have positive tests
- [ ] Error cases produce clear diagnostics
- [ ] Edge cases covered (deeply nested, complex patterns)
- [ ] Integration tests validate end-to-end parsing
- [ ] Snapshot tests capture AST structure
- [ ] Performance tests ensure reasonable parse times

**File Location**: `src/parser/query.rs`, `tests/pattern_tests.rs`

---

### Task 25: Documentation and Examples

**Description**: Document pattern parsing system and provide examples.

**Deliverables**:
- **API Documentation**:
  - Rustdoc comments for all pattern AST node types
  - Module-level documentation for pattern parsing
  - Examples in documentation comments

- **Examples**:
  - Update `examples/parser_demo.rs` to showcase pattern parsing
  - Add `examples/pattern_demo.rs` demonstrating:
    - Simple node and edge patterns
    - Complex path patterns with quantifiers
    - Label expressions
    - Path search strategies (ALL, ANY, SHORTEST)
    - Graph patterns with all components
    - MATCH statements with various patterns

- **Pattern Matching Overview Documentation**:
  - Document pattern matching semantics
  - Document path modes and search strategies
  - Document quantifier semantics
  - Document label expression boolean algebra
  - Document edge direction types
  - Cross-reference with ISO GQL specification sections

- **Grammar Mapping Documentation**:
  - Document mapping from ISO GQL grammar rules to Rust parser functions
  - Cross-reference with GQL.g4 line numbers

- **Error Catalog**:
  - Document all diagnostic codes and messages for patterns
  - Provide examples of each error case

**Acceptance Criteria**:
- [ ] All public API documented with rustdoc
- [ ] Examples compile and run successfully
- [ ] Pattern matching overview document complete
- [ ] Grammar mapping document complete
- [ ] Error catalog includes all pattern error codes
- [ ] Documentation explains pattern matching semantics clearly
- [ ] Cross-references to ISO GQL spec provided

**File Location**: `src/ast/query.rs`, `src/parser/query.rs`, `examples/`, `docs/`

---

## Implementation Notes

### Parser Architecture Considerations

1. **Pattern Precedence**: Pattern expressions have precedence rules:
   - Parentheses (highest precedence)
   - Quantifiers (bind to immediately preceding primary)
   - Sequential concatenation (path terms)
   - Alternation (|)
   - Union (lowest precedence)
   - Parser should use precedence climbing or recursive descent with explicit precedence handling

2. **Edge Direction Complexity**: 7 edge direction types require careful parsing:
   - Full edge patterns: `-[edge]->`, `<-[edge]-`, `~[edge]~`, etc.
   - Abbreviated patterns: `->`, `<-`, `~`, `-`
   - Parser must lookahead to distinguish full vs abbreviated
   - Token sequences like `<-[` must be recognized as single unit

3. **Quantifier Binding**: Quantifiers apply to immediately preceding primary:
   - `(a)-[e]->(b)+` means node b is quantified, not the entire path
   - `((a)-[e]->(b))+` means the entire parenthesized pattern is quantified
   - Parser must track quantifier scope carefully

4. **Label Expression Operators**: Label operators may conflict with other operators:
   - `!` negation vs NOT operator
   - `&` conjunction vs bitwise AND
   - `|` disjunction vs alternation or bitwise OR
   - Parser must use context to disambiguate (inside :label context vs expression context)

5. **Simplified vs Standard Patterns**: Two pattern syntaxes:
   - Standard: full syntax with all features
   - Simplified: alternative abbreviated syntax
   - Parser must recognize and distinguish between these forms
   - Consider separate parser entry points for each

6. **Pattern Recursion**: Patterns can nest deeply:
   - Path expressions can contain parenthesized subexpressions
   - Label expressions can nest with parentheses
   - Use Box<T> to avoid infinite-size types
   - Consider recursion depth limits for safety

### AST Design Considerations

1. **Span Tracking**: Every pattern node must track its source span for diagnostic purposes.

2. **Pattern Hierarchy**: Use enum hierarchy for pattern types:
   - `ElementPattern`: Node, Edge
   - `EdgePattern`: Full, Abbreviated
   - `PathPatternExpression`: Union, Alternation, Term
   - This makes pattern matching cleaner and type-safer

3. **Optional Fields**: Many pattern components are optional:
   - Variables in element patterns
   - Label expressions
   - Property specifications
   - WHERE clauses
   - Path mode in search strategies
   - Use `Option<T>` appropriately

4. **SmolStr for Efficiency**: Use `SmolStr` for:
   - Variable names
   - Label names
   - Property names
   - Short identifiers

5. **Box for Recursion**: Use `Box<T>` for recursive pattern fields:
   - Nested label expressions
   - Nested path expressions
   - Quantified patterns
   - This avoids infinite size types

6. **Vec for Collections**: Use `Vec<T>` for:
   - Path pattern lists
   - Property key-value pairs
   - Label set specifications
   - Path term factors

### Error Recovery Strategy

1. **Synchronization Points**:
   - Element pattern boundaries (after ], ), })
   - Path pattern separators (commas in pattern lists)
   - Quantifier boundaries (after })
   - WHERE keyword (recover to where clause)

2. **Pattern Boundary Recovery**: If pattern malformed:
   - Report error at pattern location
   - Skip to next synchronization point
   - Continue parsing rest of pattern list
   - Construct partial AST

3. **Quantifier Recovery**: If quantifier malformed:
   - Report error at quantifier location
   - Assume default (no quantifier)
   - Continue with next pattern component

4. **Delimiter Matching**: Track opening delimiters ([, (, {) and ensure closing:
   - Use stack for nested delimiters
   - Report unclosed delimiter errors with helpful span

### Diagnostic Quality Guidelines

1. **Clear Messages**: Error messages should be specific:
   - Bad: "Parse error in pattern"
   - Good: "Expected closing bracket ] after edge pattern, found ->"

2. **Helpful Suggestions**:
   - "Node patterns use parentheses: (variable :label {props})"
   - "Edge patterns use brackets: -[edge]->"
   - "Did you mean '-[edge]->' instead of '-[edge->'?"
   - "Quantifiers require braces: {n,m}, not (n,m)"
   - "Label expressions use : prefix: :Label"

3. **Span Highlighting**: Highlight the exact token or range causing the error:
   - For missing delimiters, point to where delimiter expected
   - For malformed patterns, highlight entire pattern
   - For invalid operators, highlight operator token

4. **Context Information**: Provide context about what parser was doing:
   - "While parsing node pattern..."
   - "In edge pattern starting at line 42..."
   - "While parsing label expression..."

### Performance Considerations

1. **Pattern Parsing Efficiency**: Pattern parsing is hot path:
   - Use efficient lookahead (1-2 tokens typically sufficient)
   - Minimize backtracking
   - Use direct dispatch to pattern type parsers

2. **Label Expression Parsing**: Use precedence climbing for efficiency:
   - Single-pass parsing with explicit precedence levels
   - Avoids backtracking
   - More efficient than recursive descent with backtracking

3. **Edge Direction Recognition**: Edge arrows require careful tokenization:
   - Tokenize full arrow sequences as single tokens where possible
   - Use efficient lookahead to distinguish full vs abbreviated patterns
   - Consider token combining in lexer for efficiency

4. **AST Allocation**: Minimize allocations:
   - Use `Box` only where needed for recursion
   - Use `SmolStr` for inline storage of short strings
   - Consider arena allocation for AST nodes (future optimization)

## Dependencies on Other Sprints

### Dependencies on Completed Sprints

- **Sprint 1**: Diagnostic infrastructure (`Span`, `Diag`, `miette` conversion)
- **Sprint 2**: Lexer emits all necessary tokens (pattern keywords, edge arrows, quantifiers)
- **Sprint 3**: Parser skeleton and recovery framework
- **Sprint 4**: Statement structure; integration testing infrastructure
- **Sprint 5**: Expression parsing for property values, where clauses, yield items, quantifier counts
- **Sprint 6**: Type system (for future use in pattern type constraints)
- **Sprint 7**: Query pipeline and MATCH statement structure (GraphPattern placeholder to replace)

### Dependencies on Future Sprints

- **Sprint 9**: Result shaping will use patterns in SELECT FROM graph matches
- **Sprint 10**: Data modification will use patterns in INSERT statements
- **Sprint 11**: Procedures may use patterns in procedure bodies
- **Sprint 12**: Graph type specifications will use pattern syntax for type definitions
- **Sprint 14**: Semantic validation (variable scoping, label validation, pattern consistency)

### Cross-Sprint Integration Points

- Patterns are foundational for graph querying in GQL
- Pattern parser must be designed for reusability across contexts
- AST pattern types should be stable to avoid downstream breakage
- Expression integration (Sprint 5) is critical throughout
- Query integration (Sprint 7) completes MATCH statement implementation
- Consider semantic validation in Sprint 14 (scoping, type checking, etc.)

## Test Strategy

### Unit Tests

For each pattern component:
1. **Happy Path**: Valid patterns parse correctly
2. **Variants**: All syntax variants and optional components
3. **Error Cases**: Missing delimiters, invalid syntax, malformed patterns
4. **Recovery**: Parser recovers and continues after errors

### Integration Tests

Patterns in different contexts:
1. **MATCH Statements**: All pattern types in MATCH clauses
2. **Nested Patterns**: Deeply nested path expressions
3. **Complex Patterns**: Patterns with all components combined
4. **Expression Integration**: Patterns with complex expressions from Sprint 5
5. **Query Integration**: Patterns in complete query pipeline from Sprint 7

### Snapshot Tests

Capture AST output:
1. Representative patterns from each category
2. Complex nested patterns
3. Patterns with quantifiers
4. Ensure AST changes are intentional

### Property-Based Tests (Optional, Advanced)

Use `proptest` or `quickcheck`:
1. Generate random valid patterns
2. Verify parser never panics
3. Verify parsed AST can be pretty-printed and re-parsed

### Corpus Tests

Test against real GQL queries:
1. Official GQL sample queries with patterns
2. Real-world graph pattern queries
3. Verify parser handles production syntax

### Performance Tests

1. **Deeply Nested Patterns**: Ensure parser handles deep nesting efficiently
2. **Long Pattern Lists**: Many comma-separated patterns
3. **Complex Quantifiers**: Nested quantified patterns

## Performance Considerations

1. **Lexer Efficiency**: Pattern tokens are frequent; lexer must be fast
2. **Parser Efficiency**: Use direct dispatch and minimal lookahead
3. **AST Allocation**: Minimize allocations with `SmolStr` and arena allocation (future)
4. **Pattern Matching**: Use efficient algorithms for precedence and associativity

## Documentation Requirements

1. **API Documentation**: Rustdoc for all pattern AST nodes and parser functions
2. **Pattern Matching Overview**: Document pattern semantics and composition
3. **Grammar Mapping**: Document ISO GQL grammar to Rust parser mapping
4. **Examples**: Demonstrate pattern parsing in examples
5. **Error Catalog**: Document all diagnostic codes and messages

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Pattern grammar complexity causes parser confusion | High | High | Careful grammar analysis; extensive testing; clear AST design; implement in phases |
| Edge direction syntax ambiguity | High | Medium | Clear tokenization rules; comprehensive lookahead; test all 7 direction types |
| Label expression operator conflicts | Medium | Medium | Use context to disambiguate; clear precedence rules; extensive testing |
| Quantifier scoping complexity | Medium | Medium | Clear binding rules; parenthesization for clarity; good error messages |
| Pattern recursion depth issues | Low | Low | Implement recursion depth limits; test deeply nested patterns |
| Simplified pattern syntax confusion | Medium | Low | Clear distinction between standard and simplified; separate parsers if needed |
| Performance on complex patterns | Medium | Medium | Optimize hot paths; use efficient algorithms; profile and optimize |

## Success Metrics

1. **Coverage**: All pattern types parse with correct AST
2. **Correctness**: Pattern semantics match ISO GQL specification (validated by tests)
3. **Diagnostics**: At least 90% of error cases produce helpful diagnostics
4. **Recovery**: Parser never panics on invalid input
5. **Test Coverage**: >95% code coverage for pattern parser
6. **Performance**: Parser handles patterns with 100+ elements in <1ms
7. **Integration**: Pattern parser integrates cleanly with Sprint 5 (expressions) and Sprint 7 (queries)
8. **Reusability**: Pattern parser used in multiple contexts (MATCH, INSERT, SELECT FROM)

## Sprint Completion Checklist

- [x] All tasks completed and reviewed
- [x] All acceptance criteria met
- [x] Unit tests pass with >95% coverage
- [x] Integration tests demonstrate end-to-end functionality
- [x] Documentation complete (rustdoc, examples, grammar mapping, pattern overview)
- [x] Performance baseline established
- [x] Error catalog documented
- [x] Code review completed
- [x] CI/CD pipeline passes
- [x] Pattern parser tested in multiple contexts (MATCH, queries)
- [x] AST design reviewed for stability and extensibility
- [x] Sprint 5 integration complete (expressions in patterns)
- [x] Sprint 7 integration complete (patterns in MATCH statements)
- [x] GraphPattern placeholder from Sprint 7 fully replaced
- [x] Sprint demo prepared for stakeholders

## Next Sprint Preview

**Sprint 9: Result Shaping and Aggregation** will build on the pattern foundation to implement result production features including RETURN, FINISH, grouping, ordering, pagination, aggregate functions, set quantifiers, and having/yield interactions. With patterns implemented, Sprint 9 can focus on how query results are shaped and aggregated.

---

## Appendix: Pattern Hierarchy

```
GraphPattern
├── MatchMode (optional)
│   ├── RepeatableElements
│   └── DifferentEdges
├── PathPatternList
│   └── Vec<PathPattern>
│       ├── PathPatternPrefix (optional)
│       │   ├── PathMode
│       │   │   ├── Walk
│       │   │   ├── Trail
│       │   │   ├── Simple
│       │   │   └── Acyclic
│       │   └── PathSearch
│       │       ├── All { mode, use_paths_keyword }
│       │       ├── Any { mode }
│       │       └── Shortest
│       │           ├── AllShortest { mode }
│       │           ├── AnyShortest { mode }
│       │           ├── CountedShortest { count, mode, use_paths_keyword }
│       │           └── CountedShortestGroups { count, mode }
│       ├── PathPatternExpression
│       │   ├── Union { left, right }
│       │   ├── Alternation { alternatives: Vec<PathTerm> }
│       │   └── Term(PathTerm)
│       │       └── PathTerm { factors: Vec<PathFactor> }
│       │           └── PathFactor { primary, quantifier }
│       │               ├── PathPrimary
│       │               │   ├── ElementPattern
│       │               │   │   ├── Node(NodePattern)
│       │               │   │   │   ├── variable (optional)
│       │               │   │   │   ├── label_expression (optional)
│       │               │   │   │   ├── properties (optional)
│       │               │   │   │   └── where_clause (optional)
│       │               │   │   └── Edge(EdgePattern)
│       │               │   │       ├── Full { direction, filler }
│       │               │   │       │   ├── EdgeDirection
│       │               │   │       │   │   ├── PointingLeft
│       │               │   │       │   │   ├── PointingRight
│       │               │   │       │   │   ├── Undirected
│       │               │   │       │   │   ├── AnyDirected
│       │               │   │       │   │   ├── LeftOrUndirected
│       │               │   │       │   │   ├── AnyDirection
│       │               │   │       │   │   └── RightOrUndirected
│       │               │   │       │   └── FullEdgePointingFiller
│       │               │   │       │       ├── variable (optional)
│       │               │   │       │       ├── label_expression (optional)
│       │               │   │       │       ├── properties (optional)
│       │               │   │       │       └── where_clause (optional)
│       │               │   │       └── Abbreviated
│       │               │   │           ├── LeftArrow
│       │               │   │           ├── RightArrow
│       │               │   │           ├── Undirected
│       │               │   │           └── AnyDirection
│       │               │   └── ParenthesizedExpression
│       │               └── GraphPatternQuantifier (optional)
│       │                   ├── Star
│       │                   ├── Plus
│       │                   ├── QuestionMark
│       │                   ├── Fixed { count }
│       │                   └── General { min, max }
│       └── PathVariableDeclaration (optional)
├── KeepClause (optional)
└── GraphPatternWhereClause (optional)

LabelExpression
├── Negation { operand }
├── Conjunction { left, right }
├── Disjunction { left, right }
├── LabelName { name }
├── Wildcard
└── Parenthesized { expression }
```

---

## Appendix: Pattern Grammar Coverage Map

| Grammar Rule | Line Number | AST Node | Parser Function |
|--------------|-------------|----------|-----------------|
| `graphPattern` | 803 | `GraphPattern` struct | `parse_graph_pattern()` |
| `matchMode` | 807 | `MatchMode` enum | `parse_match_mode()` |
| `repeatableElementsMatchMode` | 812 | `MatchMode::RepeatableElements` | `parse_match_mode()` |
| `differentEdgesMatchMode` | 816 | `MatchMode::DifferentEdges` | `parse_match_mode()` |
| `pathPatternList` | 830 | `PathPatternList` struct | `parse_path_pattern_list()` |
| `pathVariableDeclaration` | 838 | `PathVariableDeclaration` struct | `parse_path_variable_declaration()` |
| `keepClause` | 842 | `KeepClause` struct | `parse_keep_clause()` |
| `graphPatternWhereClause` | 846 | `GraphPatternWhereClause` struct | `parse_graph_pattern_where_clause()` |
| `pathPatternPrefix` | 898 | `PathPatternPrefix` enum | `parse_path_pattern_prefix()` |
| `pathModePrefix` | 903 | `PathMode` enum | `parse_path_mode_prefix()` |
| `pathMode` | 907 | `PathMode` enum | `parse_path_mode()` |
| `pathSearchPrefix` | 914 | `PathSearch` enum | `parse_path_search_prefix()` |
| `allPathSearch` | 920 | `AllPathSearch` struct | `parse_all_path_search()` |
| `anyPathSearch` | 929 | `AnyPathSearch` struct | `parse_any_path_search()` |
| `shortestPathSearch` | 937 | `ShortestPathSearch` enum | `parse_shortest_path_search()` |
| `allShortestPathSearch` | 944 | `ShortestPathSearch::AllShortest` | `parse_all_shortest_path_search()` |
| `anyShortestPathSearch` | 948 | `ShortestPathSearch::AnyShortest` | `parse_any_shortest_path_search()` |
| `countedShortestPathSearch` | 952 | `ShortestPathSearch::CountedShortest` | `parse_counted_shortest_path_search()` |
| `countedShortestGroupSearch` | 956 | `ShortestPathSearch::CountedShortestGroups` | `parse_counted_shortest_group_search()` |
| `pathPatternExpression` | 966 | `PathPatternExpression` enum | `parse_path_pattern_expression()` |
| `pathTerm` | 972 | `PathTerm` struct | `parse_path_term()` |
| `pathFactor` | 976 | `PathFactor` struct | `parse_path_factor()` |
| `pathPrimary` | 982 | `PathPrimary` enum | `parse_path_primary()` |
| `elementPattern` | 988 | `ElementPattern` enum | `parse_element_pattern()` |
| `nodePattern` | 993 | `NodePattern` struct | `parse_node_pattern()` |
| `elementPatternFiller` | 997 | Node/Edge filler fields | `parse_element_pattern_filler()` |
| `elementVariableDeclaration` | 1001 | `ElementVariableDeclaration` struct | `parse_element_variable_declaration()` |
| `isLabelExpression` | 1005 | `IsLabelExpression` struct | `parse_is_label_expression()` |
| `elementPatternPredicate` | 1014 | `ElementPatternPredicate` struct | `parse_element_pattern_predicate()` |
| `elementPropertySpecification` | 1023 | `ElementPropertySpecification` struct | `parse_element_property_specification()` |
| `edgePattern` | 1035 | `EdgePattern` enum | `parse_edge_pattern()` |
| `fullEdgePattern` | 1040 | `FullEdgePattern` struct | `parse_full_edge_pattern()` |
| `fullEdgePointingLeft` | 1050 | `EdgeDirection::PointingLeft` | `parse_full_edge_pointing_left()` |
| `fullEdgePointingRight` | 1054 | `EdgeDirection::PointingRight` | `parse_full_edge_pointing_right()` |
| `fullEdgeUndirected` | 1058 | `EdgeDirection::Undirected` | `parse_full_edge_undirected()` |
| `fullEdgeAnyDirection` | 1062 | `EdgeDirection::AnyDirected` | `parse_full_edge_any_direction()` |
| `fullEdgeLeftOrUndirected` | 1066 | `EdgeDirection::LeftOrUndirected` | `parse_full_edge_left_or_undirected()` |
| `fullEdgeUndirectedOrRight` | 1070 | `EdgeDirection::RightOrUndirected` | `parse_full_edge_undirected_or_right()` |
| `fullEdgeLeftOrRight` | 1074 | `EdgeDirection::AnyDirection` | `parse_full_edge_left_or_right()` |
| `abbreviatedEdgePattern` | 1078 | `AbbreviatedEdgePattern` enum | `parse_abbreviated_edge_pattern()` |
| `parenthesizedPathPatternExpression` | 1088 | `ParenthesizedPathPatternExpression` struct | `parse_parenthesized_path_pattern_expression()` |
| `labelExpression` | 1102 | `LabelExpression` enum | `parse_label_expression()` |
| `graphPatternQuantifier` | 1125 | `GraphPatternQuantifier` enum | `parse_graph_pattern_quantifier()` |
| `fixedQuantifier` | 1132 | `GraphPatternQuantifier::Fixed` | `parse_fixed_quantifier()` |
| `generalQuantifier` | 1136 | `GraphPatternQuantifier::General` | `parse_general_quantifier()` |
| `simplifiedPathPatternExpression` | 1150 | `SimplifiedPathPattern` struct | `parse_simplified_path_pattern_expression()` |
| `simplifiedContents` | 1188 | `SimplifiedContents` struct | `parse_simplified_contents()` |
| `simplifiedPathUnion` | 1194 | `SimplifiedPathUnion` struct | `parse_simplified_path_union()` |
| `simplifiedMultisetAlternation` | 1198 | `SimplifiedMultisetAlternation` struct | `parse_simplified_multiset_alternation()` |
| `simplifiedQuantified` | 1218 | `SimplifiedQuantified` struct | `parse_simplified_quantified()` |
| `simplifiedQuestioned` | 1222 | `SimplifiedQuestioned` struct | `parse_simplified_questioned()` |
| `simplifiedDirectionOverride` | 1231 | `SimplifiedDirectionOverride` struct | `parse_simplified_direction_override()` |
| `simplifiedNegation` | 1274 | `SimplifiedNegation` struct | `parse_simplified_negation()` |
| `labelSetPhrase` | 1679 | `LabelSetPhrase` enum | `parse_label_set_phrase()` |
| `labelSetSpecification` | 1685 | `LabelSetSpecification` struct | `parse_label_set_specification()` |
| `graphPatternBindingTable` | 779 | `GraphPatternBindingTable` struct | `parse_graph_pattern_binding_table()` |
| `graphPatternYieldClause` | 783 | `GraphPatternYieldClause` struct | `parse_graph_pattern_yield_clause()` |

---

**Document Version**: 1.0
**Date Created**: 2026-02-17
**Status**: Planned
**Dependencies**: Sprints 1, 2, 3, 4, 5, 6, 7 (completed or required)
**Next Sprint**: Sprint 9 (Result Shaping and Aggregation)
