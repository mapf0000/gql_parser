# Parser Enhancements for Compiler Integration

**Status**: Planning
**Target**: Sprint 15
**Purpose**: Add ergonomic helper methods and validation utilities to AST nodes to simplify compiler implementation

---

## Table of Contents

1. [Overview](#overview)
2. [Enhancement Categories](#enhancement-categories)
3. [Detailed Implementation Plan](#detailed-implementation-plan)
4. [Testing Strategy](#testing-strategy)
5. [Timeline and Milestones](#timeline-and-milestones)
6. [Success Criteria](#success-criteria)

---

## Overview

### Motivation

The current parser produces a complete and correct AST for GQL queries. However, the compiler (query planner) that will consume this AST needs to perform several common operations repeatedly:

- **Pattern analysis**: Extracting chains of nodes/edges, validating MVP constraints
- **Demand analysis**: Determining which properties are actually needed per variable
- **Type extraction**: Getting simple label names vs. complex expressions
- **Variable tracking**: Collecting all bound variables for scope checking

Rather than implement these operations in the compiler (where they would be scattered across multiple modules), we can add these as **zero-cost helper methods** directly on the AST types. This keeps the parser self-contained and makes the compiler implementation significantly cleaner.

### Design Principles

1. **Zero breaking changes**: All additions are new methods on existing types
2. **Pay-for-what-you-use**: Helpers have minimal overhead (mostly just pattern matching)
3. **Type-driven**: Leverage Rust's type system to make invalid patterns unrepresentable
4. **Test-covered**: Every helper has comprehensive unit tests
5. **Documentation**: All public methods have rustdoc with examples

---

## Enhancement Categories

### Category 1: Convenience Accessors
Simple getter-style methods that extract common information from complex nested structures.

**Examples:**
- `NodePattern::variable_name()` → `Option<&SmolStr>`
- `EdgePattern::is_outgoing()` → `bool`
- `LabelExpression::as_simple_label()` → `Option<&SmolStr>`

### Category 2: Analysis Helpers
Methods that traverse AST subtrees to collect information.

**Examples:**
- `GraphPattern::bound_variables()` → `HashSet<&SmolStr>`
- `Expression::collect_property_accesses()` → `Vec<(SmolStr, SmolStr)>`
- `PathTerm::as_simple_chain()` → `Option<Vec<&ElementPattern>>`

### Category 3: Validation Utilities
Methods that check MVP compatibility constraints.

**Examples:**
- `GraphPattern::validate_mvp()` → `MvpCompatibility`
- `EdgePattern::validate_mvp()` → `MvpCompatibility`
- `GraphPatternQuantifier::max_hops()` → `Option<u32>`

### Category 4: Demand Analysis
High-level analysis functions that determine which properties are needed.

**Examples:**
- `analyze_property_demands()` → `PropertyDemands`

---

## Detailed Implementation Plan

### Phase 1: Convenience Accessors (Low-Hanging Fruit)

#### 1.1 NodePattern Helpers

**File**: `src/ast/query.rs`

```rust
impl NodePattern {
    /// Get the variable name if present (convenience accessor).
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::NodePattern;
    /// let pattern = /* (a:Person) */;
    /// assert_eq!(pattern.variable_name(), Some("a"));
    /// ```
    pub fn variable_name(&self) -> Option<&SmolStr> {
        self.variable.as_ref().map(|v| &v.variable)
    }

    /// Check if this node has a simple single-label filter (fast path for compiler).
    ///
    /// Returns the label name if the label expression is a simple `LabelName`,
    /// or `None` if it's a complex expression (disjunction, conjunction, wildcard, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::NodePattern;
    /// let pattern = /* (a:Person) */;
    /// assert_eq!(pattern.simple_label(), Some("Person"));
    ///
    /// let complex = /* (a:Person|Company) */;
    /// assert_eq!(complex.simple_label(), None);
    /// ```
    pub fn simple_label(&self) -> Option<&SmolStr> {
        self.label_expression.as_ref()?.as_simple_label()
    }

    /// Check if this node has any filters (label, properties, or WHERE).
    ///
    /// Useful for determining if a node scan can be optimized.
    pub fn has_filters(&self) -> bool {
        self.label_expression.is_some()
            || self.properties.is_some()
            || self.where_clause.is_some()
    }
}
```

**Tests**: `tests/ast_helpers_test.rs`

```rust
#[cfg(test)]
mod node_pattern_tests {
    use super::*;

    #[test]
    fn variable_name_present() {
        let pattern = parse_node("(a:Person)");
        assert_eq!(pattern.variable_name(), Some(&SmolStr::new("a")));
    }

    #[test]
    fn variable_name_absent() {
        let pattern = parse_node("(:Person)");
        assert_eq!(pattern.variable_name(), None);
    }

    #[test]
    fn simple_label_basic() {
        let pattern = parse_node("(a:Person)");
        assert_eq!(pattern.simple_label(), Some(&SmolStr::new("Person")));
    }

    #[test]
    fn simple_label_disjunction_returns_none() {
        let pattern = parse_node("(a:Person|Company)");
        assert_eq!(pattern.simple_label(), None);
    }

    #[test]
    fn simple_label_wildcard_returns_none() {
        let pattern = parse_node("(a:%)");
        assert_eq!(pattern.simple_label(), None);
    }
}
```

#### 1.2 EdgePattern Helpers

**File**: `src/ast/query.rs`

```rust
impl EdgePattern {
    /// Get variable name if this edge is bound to a variable.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::EdgePattern;
    /// let pattern = /* -[e:KNOWS]-> */;
    /// assert_eq!(pattern.variable_name(), Some("e"));
    ///
    /// let anon = /* -[:KNOWS]-> */;
    /// assert_eq!(anon.variable_name(), None);
    /// ```
    pub fn variable_name(&self) -> Option<&SmolStr> {
        match self {
            EdgePattern::Full(full) => {
                full.filler.variable.as_ref().map(|v| &v.variable)
            }
            EdgePattern::Abbreviated(_) => None,
        }
    }

    /// Check if this edge points right (outgoing direction).
    ///
    /// This is the MVP-supported direction for CSR-backed expansion.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::EdgePattern;
    /// let right = /* -[:KNOWS]-> */;
    /// assert!(right.is_outgoing());
    ///
    /// let left = /* <-[:KNOWS]- */;
    /// assert!(!left.is_outgoing());
    /// ```
    pub fn is_outgoing(&self) -> bool {
        match self {
            EdgePattern::Full(full) => {
                matches!(full.direction, EdgeDirection::PointingRight)
            }
            EdgePattern::Abbreviated(abbrev) => {
                matches!(abbrev, AbbreviatedEdgePattern::RightArrow { .. })
            }
        }
    }

    /// Check if this edge points left (incoming direction).
    pub fn is_incoming(&self) -> bool {
        match self {
            EdgePattern::Full(full) => {
                matches!(full.direction, EdgeDirection::PointingLeft)
            }
            EdgePattern::Abbreviated(abbrev) => {
                matches!(abbrev, AbbreviatedEdgePattern::LeftArrow { .. })
            }
        }
    }

    /// Check if this edge is undirected or bidirectional.
    pub fn is_undirected(&self) -> bool {
        match self {
            EdgePattern::Full(full) => {
                matches!(
                    full.direction,
                    EdgeDirection::Undirected
                        | EdgeDirection::AnyDirection
                        | EdgeDirection::LeftOrUndirected
                        | EdgeDirection::RightOrUndirected
                )
            }
            EdgePattern::Abbreviated(abbrev) => {
                matches!(abbrev,
                    AbbreviatedEdgePattern::AnyDirection { .. }
                    | AbbreviatedEdgePattern::Undirected { .. }
                )
            }
        }
    }

    /// Get the simple label for this edge (fast path type filter).
    ///
    /// Returns the label name if the label expression is a simple `LabelName`,
    /// or `None` for complex expressions or wildcards.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::EdgePattern;
    /// let pattern = /* -[:KNOWS]-> */;
    /// assert_eq!(pattern.simple_type_label(), Some("KNOWS"));
    ///
    /// let complex = /* -[:KNOWS|FRIEND]-> */;
    /// assert_eq!(complex.simple_type_label(), None);
    /// ```
    pub fn simple_type_label(&self) -> Option<&SmolStr> {
        match self {
            EdgePattern::Full(full) => {
                full.filler.label_expression.as_ref()?.as_simple_label()
            }
            EdgePattern::Abbreviated(_) => None,
        }
    }

    /// Get the direction of this edge pattern.
    pub fn direction(&self) -> EdgeDirection {
        match self {
            EdgePattern::Full(full) => full.direction,
            EdgePattern::Abbreviated(abbrev) => {
                match abbrev {
                    AbbreviatedEdgePattern::LeftArrow { .. } => EdgeDirection::PointingLeft,
                    AbbreviatedEdgePattern::RightArrow { .. } => EdgeDirection::PointingRight,
                    AbbreviatedEdgePattern::AnyDirection { .. } => EdgeDirection::AnyDirection,
                    AbbreviatedEdgePattern::Undirected { .. } => EdgeDirection::Undirected,
                }
            }
        }
    }
}
```

**Tests**: `tests/ast_helpers_test.rs`

```rust
#[cfg(test)]
mod edge_pattern_tests {
    use super::*;

    #[test]
    fn is_outgoing_abbreviated() {
        let pattern = parse_edge("->");
        assert!(pattern.is_outgoing());
    }

    #[test]
    fn is_outgoing_full() {
        let pattern = parse_edge("-[:KNOWS]->");
        assert!(pattern.is_outgoing());
    }

    #[test]
    fn is_incoming_abbreviated() {
        let pattern = parse_edge("<-");
        assert!(pattern.is_incoming());
        assert!(!pattern.is_outgoing());
    }

    #[test]
    fn is_undirected() {
        let pattern = parse_edge("-");
        assert!(pattern.is_undirected());
    }

    #[test]
    fn simple_type_label() {
        let pattern = parse_edge("-[:KNOWS]->");
        assert_eq!(pattern.simple_type_label(), Some(&SmolStr::new("KNOWS")));
    }

    #[test]
    fn simple_type_label_disjunction() {
        let pattern = parse_edge("-[:KNOWS|FRIEND]->");
        assert_eq!(pattern.simple_type_label(), None);
    }
}
```

#### 1.3 LabelExpression Helpers

**File**: `src/ast/expression.rs`

```rust
impl LabelExpression {
    /// Extract simple single label name (common case fast path).
    ///
    /// Returns `Some(name)` if this is a simple `LabelName` (possibly parenthesized),
    /// or `None` for complex expressions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::LabelExpression;
    /// let simple = LabelExpression::LabelName {
    ///     name: "Person".into(),
    ///     span: 0..6
    /// };
    /// assert_eq!(simple.as_simple_label(), Some(&SmolStr::new("Person")));
    ///
    /// let disjunction = /* :Person|Company */;
    /// assert_eq!(disjunction.as_simple_label(), None);
    /// ```
    pub fn as_simple_label(&self) -> Option<&SmolStr> {
        match self {
            LabelExpression::LabelName { name, .. } => Some(name),
            LabelExpression::Parenthesized { expression, .. } => {
                expression.as_simple_label()
            }
            _ => None,
        }
    }

    /// Extract all label names from a pure disjunction chain (A|B|C).
    ///
    /// Returns `Some(labels)` if this is a disjunction of simple label names,
    /// or `None` if the expression contains conjunction, negation, or other operations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::LabelExpression;
    /// let disjunction = /* :Person|Company|Organization */;
    /// assert_eq!(
    ///     disjunction.extract_disjunction_labels(),
    ///     Some(vec!["Person", "Company", "Organization"])
    /// );
    ///
    /// let conjunction = /* :Person&Active */;
    /// assert_eq!(conjunction.extract_disjunction_labels(), None);
    /// ```
    pub fn extract_disjunction_labels(&self) -> Option<Vec<&SmolStr>> {
        match self {
            LabelExpression::LabelName { name, .. } => Some(vec![name]),
            LabelExpression::Disjunction { left, right, .. } => {
                let mut labels = left.extract_disjunction_labels()?;
                labels.extend(right.extract_disjunction_labels()?);
                Some(labels)
            }
            LabelExpression::Parenthesized { expression, .. } => {
                expression.extract_disjunction_labels()
            }
            _ => None, // Conjunction, Negation, Wildcard not pure disjunction
        }
    }

    /// Check if this is a wildcard label expression (%).
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::LabelExpression;
    /// let wildcard = LabelExpression::Wildcard { span: 0..1 };
    /// assert!(wildcard.is_wildcard());
    ///
    /// let named = LabelExpression::LabelName {
    ///     name: "Person".into(),
    ///     span: 0..6
    /// };
    /// assert!(!named.is_wildcard());
    /// ```
    pub fn is_wildcard(&self) -> bool {
        matches!(self, LabelExpression::Wildcard { .. })
    }

    /// Count the number of distinct label names in this expression.
    ///
    /// Useful for estimating selectivity or choosing optimization strategies.
    pub fn label_count(&self) -> usize {
        match self {
            LabelExpression::LabelName { .. } => 1,
            LabelExpression::Disjunction { left, right, .. } |
            LabelExpression::Conjunction { left, right, .. } => {
                left.label_count() + right.label_count()
            }
            LabelExpression::Negation { operand, .. } |
            LabelExpression::Parenthesized { expression: operand, .. } => {
                operand.label_count()
            }
            LabelExpression::Wildcard { .. } => 0, // Wildcard matches anything
        }
    }
}
```

**Tests**: `tests/ast_helpers_test.rs`

```rust
#[cfg(test)]
mod label_expression_tests {
    use super::*;

    #[test]
    fn as_simple_label_basic() {
        let expr = LabelExpression::LabelName {
            name: SmolStr::new("Person"),
            span: 0..6,
        };
        assert_eq!(expr.as_simple_label(), Some(&SmolStr::new("Person")));
    }

    #[test]
    fn as_simple_label_parenthesized() {
        let expr = LabelExpression::Parenthesized {
            expression: Box::new(LabelExpression::LabelName {
                name: SmolStr::new("Person"),
                span: 1..7,
            }),
            span: 0..8,
        };
        assert_eq!(expr.as_simple_label(), Some(&SmolStr::new("Person")));
    }

    #[test]
    fn extract_disjunction_labels_chain() {
        let expr = parse_label_expr(":Person|Company|Organization");
        let labels = expr.extract_disjunction_labels().unwrap();
        assert_eq!(labels.len(), 3);
        assert!(labels.contains(&&SmolStr::new("Person")));
        assert!(labels.contains(&&SmolStr::new("Company")));
        assert!(labels.contains(&&SmolStr::new("Organization")));
    }

    #[test]
    fn extract_disjunction_labels_with_conjunction_returns_none() {
        let expr = parse_label_expr(":Person&Active");
        assert_eq!(expr.extract_disjunction_labels(), None);
    }

    #[test]
    fn is_wildcard() {
        let wildcard = LabelExpression::Wildcard { span: 0..1 };
        assert!(wildcard.is_wildcard());

        let named = LabelExpression::LabelName {
            name: SmolStr::new("Person"),
            span: 0..6,
        };
        assert!(!named.is_wildcard());
    }
}
```

#### 1.4 GraphPatternQuantifier Helpers

**File**: `src/ast/query.rs`

```rust
impl GraphPatternQuantifier {
    /// Get the maximum hop count for this quantifier.
    ///
    /// Returns `Some(max)` for bounded quantifiers, or `None` for unbounded (`*`, `+`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::GraphPatternQuantifier;
    /// let fixed = GraphPatternQuantifier::Fixed { count: 3, span: 0..1 };
    /// assert_eq!(fixed.max_hops(), Some(3));
    ///
    /// let range = GraphPatternQuantifier::General {
    ///     min: Some(2),
    ///     max: Some(5),
    ///     span: 0..5
    /// };
    /// assert_eq!(range.max_hops(), Some(5));
    ///
    /// let star = GraphPatternQuantifier::Star { span: 0..1 };
    /// assert_eq!(star.max_hops(), None); // unbounded
    /// ```
    pub fn max_hops(&self) -> Option<u32> {
        match self {
            GraphPatternQuantifier::Star { .. } => None, // unbounded
            GraphPatternQuantifier::Plus { .. } => None, // unbounded
            GraphPatternQuantifier::QuestionMark { .. } => Some(1),
            GraphPatternQuantifier::Fixed { count, .. } => Some(*count),
            GraphPatternQuantifier::General { max, .. } => *max,
        }
    }

    /// Get the minimum hop count for this quantifier.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::GraphPatternQuantifier;
    /// let plus = GraphPatternQuantifier::Plus { span: 0..1 };
    /// assert_eq!(plus.min_hops(), 1);
    ///
    /// let star = GraphPatternQuantifier::Star { span: 0..1 };
    /// assert_eq!(star.min_hops(), 0);
    ///
    /// let range = GraphPatternQuantifier::General {
    ///     min: Some(2),
    ///     max: Some(5),
    ///     span: 0..5
    /// };
    /// assert_eq!(range.min_hops(), 2);
    /// ```
    pub fn min_hops(&self) -> u32 {
        match self {
            GraphPatternQuantifier::Star { .. } => 0,
            GraphPatternQuantifier::Plus { .. } => 1,
            GraphPatternQuantifier::QuestionMark { .. } => 0,
            GraphPatternQuantifier::Fixed { count, .. } => *count,
            GraphPatternQuantifier::General { min, .. } => min.unwrap_or(0),
        }
    }

    /// Check if this quantifier is unbounded (no maximum).
    pub fn is_unbounded(&self) -> bool {
        self.max_hops().is_none()
    }

    /// Check if this quantifier allows zero repetitions.
    pub fn allows_zero(&self) -> bool {
        self.min_hops() == 0
    }
}
```

**Tests**: `tests/ast_helpers_test.rs`

```rust
#[cfg(test)]
mod quantifier_tests {
    use super::*;

    #[test]
    fn star_quantifier() {
        let q = GraphPatternQuantifier::Star { span: 0..1 };
        assert_eq!(q.min_hops(), 0);
        assert_eq!(q.max_hops(), None);
        assert!(q.is_unbounded());
        assert!(q.allows_zero());
    }

    #[test]
    fn plus_quantifier() {
        let q = GraphPatternQuantifier::Plus { span: 0..1 };
        assert_eq!(q.min_hops(), 1);
        assert_eq!(q.max_hops(), None);
        assert!(q.is_unbounded());
        assert!(!q.allows_zero());
    }

    #[test]
    fn fixed_quantifier() {
        let q = GraphPatternQuantifier::Fixed { count: 5, span: 0..3 };
        assert_eq!(q.min_hops(), 5);
        assert_eq!(q.max_hops(), Some(5));
        assert!(!q.is_unbounded());
    }

    #[test]
    fn general_quantifier() {
        let q = GraphPatternQuantifier::General {
            min: Some(2),
            max: Some(10),
            span: 0..5,
        };
        assert_eq!(q.min_hops(), 2);
        assert_eq!(q.max_hops(), Some(10));
        assert!(!q.is_unbounded());
    }
}
```

---

### Phase 2: Analysis Helpers (Pattern Traversal)

#### 2.1 Variable Collection

**File**: `src/ast/query.rs`

```rust
use std::collections::HashSet;

impl GraphPattern {
    /// Extract all bound variable names from this pattern.
    ///
    /// Useful for scope checking and demand analysis.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::GraphPattern;
    /// let pattern = parse_pattern("(a:Person)-[e:KNOWS]->(b:Company)");
    /// let vars = pattern.bound_variables();
    /// assert!(vars.contains(&SmolStr::new("a")));
    /// assert!(vars.contains(&SmolStr::new("e")));
    /// assert!(vars.contains(&SmolStr::new("b")));
    /// ```
    pub fn bound_variables(&self) -> HashSet<&SmolStr> {
        let mut vars = HashSet::new();
        for path in &self.paths.patterns {
            path.collect_variables(&mut vars);
        }
        vars
    }

    /// Check if this is a simple single-chain pattern (MVP fast path).
    ///
    /// Returns `true` if this pattern contains exactly one path pattern
    /// and that path is a simple linear chain (no union/alternation).
    pub fn is_single_chain(&self) -> bool {
        self.paths.patterns.len() == 1 &&
        self.paths.patterns[0].is_simple_chain()
    }
}

impl PathPattern {
    /// Collect all variable bindings in this path.
    pub fn collect_variables<'a>(&'a self, vars: &mut HashSet<&'a SmolStr>) {
        // Path variable (if using path variable declaration syntax)
        if let Some(var_decl) = &self.variable_declaration {
            vars.insert(&var_decl.variable);
        }

        // Element variables
        self.expression.collect_variables(vars);
    }

    /// Check if this is a simple chain (no alternation, no complex nesting).
    pub fn is_simple_chain(&self) -> bool {
        matches!(self.expression, PathPatternExpression::Term(_))
    }
}

impl PathPatternExpression {
    /// Recursively collect all variable bindings.
    pub fn collect_variables<'a>(&'a self, vars: &mut HashSet<&'a SmolStr>) {
        match self {
            PathPatternExpression::Term(term) => term.collect_variables(vars),
            PathPatternExpression::Union { left, right, .. } => {
                left.collect_variables(vars);
                right.collect_variables(vars);
            }
            PathPatternExpression::Alternation { alternatives, .. } => {
                for alt in alternatives {
                    alt.collect_variables(vars);
                }
            }
        }
    }
}

impl PathTerm {
    /// Collect variables from all factors in this term.
    pub fn collect_variables<'a>(&'a self, vars: &mut HashSet<&'a SmolStr>) {
        for factor in &self.factors {
            factor.collect_variables(vars);
        }
    }

    /// Extract the chain of elements as a flat sequence.
    ///
    /// Returns `Some(vec)` if this is a simple linear chain with no quantifiers
    /// or nested patterns. Returns `None` for complex patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::PathTerm;
    /// let term = parse_term("(a)-[:KNOWS]->(b)-[:WORKS_AT]->(c)");
    /// let chain = term.as_simple_chain().unwrap();
    /// assert_eq!(chain.len(), 5); // a, KNOWS, b, WORKS_AT, c
    /// ```
    pub fn as_simple_chain(&self) -> Option<Vec<&ElementPattern>> {
        let mut elements = Vec::new();

        for factor in &self.factors {
            // Reject quantified patterns
            if factor.quantifier.is_some() {
                return None;
            }

            // Extract element pattern
            match &factor.primary {
                PathPrimary::ElementPattern(elem) => {
                    elements.push(elem.as_ref());
                }
                _ => return None, // Nested patterns not simple
            }
        }

        Some(elements)
    }
}

impl PathFactor {
    fn collect_variables<'a>(&'a self, vars: &mut HashSet<&'a SmolStr>) {
        match &self.primary {
            PathPrimary::ElementPattern(elem) => elem.collect_variables(vars),
            PathPrimary::ParenthesizedExpression(expr) => expr.collect_variables(vars),
            PathPrimary::SimplifiedExpression(_) => {
                // Simplified syntax doesn't bind variables
            }
        }
    }
}

impl ElementPattern {
    fn collect_variables<'a>(&'a self, vars: &mut HashSet<&'a SmolStr>) {
        match self {
            ElementPattern::Node(node) => {
                if let Some(var_decl) = &node.variable {
                    vars.insert(&var_decl.variable);
                }
            }
            ElementPattern::Edge(edge) => {
                if let Some(var_name) = edge.variable_name() {
                    vars.insert(var_name);
                }
            }
        }
    }
}
```

**Tests**: `tests/ast_analysis_test.rs`

```rust
#[cfg(test)]
mod variable_collection_tests {
    use super::*;

    #[test]
    fn bound_variables_simple_chain() {
        let gql = "MATCH (a:Person)-[e:KNOWS]->(b:Company) RETURN a.name";
        let ast = parse(gql).unwrap();
        let pattern = extract_first_match(&ast);

        let vars = pattern.bound_variables();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&SmolStr::new("a")));
        assert!(vars.contains(&SmolStr::new("e")));
        assert!(vars.contains(&SmolStr::new("b")));
    }

    #[test]
    fn bound_variables_anonymous_edge() {
        let gql = "MATCH (a)-[:KNOWS]->(b) RETURN a";
        let ast = parse(gql).unwrap();
        let pattern = extract_first_match(&ast);

        let vars = pattern.bound_variables();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&SmolStr::new("a")));
        assert!(vars.contains(&SmolStr::new("b")));
    }

    #[test]
    fn as_simple_chain_basic() {
        let gql = "MATCH (a)-[:KNOWS]->(b)-[:WORKS_AT]->(c) RETURN a";
        let ast = parse(gql).unwrap();
        let pattern = extract_first_match(&ast);

        let term = &pattern.paths.patterns[0].expression;
        if let PathPatternExpression::Term(t) = term {
            let chain = t.as_simple_chain().unwrap();
            assert_eq!(chain.len(), 5); // a, KNOWS, b, WORKS_AT, c
        } else {
            panic!("Expected term");
        }
    }

    #[test]
    fn as_simple_chain_with_quantifier_returns_none() {
        let gql = "MATCH (a)-[:KNOWS*1..3]->(b) RETURN a";
        let ast = parse(gql).unwrap();
        let pattern = extract_first_match(&ast);

        let term = &pattern.paths.patterns[0].expression;
        if let PathPatternExpression::Term(t) = term {
            assert!(t.as_simple_chain().is_none());
        } else {
            panic!("Expected term");
        }
    }
}
```

#### 2.2 Expression Analysis

**File**: `src/ast/expression.rs`

```rust
use std::collections::{HashMap, HashSet};

impl Expression {
    /// Collect all variable references in this expression.
    ///
    /// Useful for scope checking and determining which variables
    /// are accessed in WHERE clauses or RETURN projections.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::Expression;
    /// let expr = parse_expr("a.age > 30 AND b.city = 'SF'");
    /// let vars = expr.referenced_variables();
    /// assert!(vars.contains(&SmolStr::new("a")));
    /// assert!(vars.contains(&SmolStr::new("b")));
    /// ```
    pub fn referenced_variables(&self) -> HashSet<SmolStr> {
        let mut vars = HashSet::new();
        self.collect_referenced_variables(&mut vars);
        vars
    }

    fn collect_referenced_variables(&self, vars: &mut HashSet<SmolStr>) {
        match self {
            Expression::VariableRef(var, _) => {
                vars.insert(var.clone());
            }
            Expression::PropertyAccess { base, .. } => {
                base.collect_referenced_variables(vars);
            }
            Expression::BinaryOp(left, _, right, _) => {
                left.collect_referenced_variables(vars);
                right.collect_referenced_variables(vars);
            }
            Expression::LogicalOp(left, _, right, _) => {
                left.collect_referenced_variables(vars);
                right.collect_referenced_variables(vars);
            }
            Expression::UnaryOp(_, operand, _) => {
                operand.collect_referenced_variables(vars);
            }
            Expression::FunctionCall(call, _) => {
                for arg in &call.arguments {
                    arg.collect_referenced_variables(vars);
                }
            }
            Expression::IsNull(operand, _) |
            Expression::IsNotNull(operand, _) => {
                operand.collect_referenced_variables(vars);
            }
            Expression::CaseExpression(case, _) => {
                match case {
                    CaseExpression::Simple(simple) => {
                        simple.operand.collect_referenced_variables(vars);
                        for clause in &simple.when_clauses {
                            clause.when_operand.collect_referenced_variables(vars);
                            clause.result.collect_referenced_variables(vars);
                        }
                        if let Some(else_result) = &simple.else_result {
                            else_result.collect_referenced_variables(vars);
                        }
                    }
                    CaseExpression::Searched(searched) => {
                        for clause in &searched.when_clauses {
                            clause.condition.collect_referenced_variables(vars);
                            clause.result.collect_referenced_variables(vars);
                        }
                        if let Some(else_result) = &searched.else_result {
                            else_result.collect_referenced_variables(vars);
                        }
                    }
                }
            }
            Expression::InList { expr, list, .. } => {
                expr.collect_referenced_variables(vars);
                for item in list {
                    item.collect_referenced_variables(vars);
                }
            }
            // Literals and other leaf nodes don't reference variables
            Expression::Literal(_, _) |
            Expression::Parameter(_, _) => {}

            _ => {} // Handle other expression types as needed
        }
    }

    /// Extract property access as (variable_name, property_name) if this is
    /// a simple property access.
    ///
    /// Returns `Some((var, prop))` for expressions like `a.name` or `b.age`,
    /// or `None` for complex expressions or non-property-access expressions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::Expression;
    /// let expr = parse_expr("a.name");
    /// assert_eq!(expr.as_property_access(), Some(("a", "name")));
    ///
    /// let complex = parse_expr("foo(a).name");
    /// assert_eq!(complex.as_property_access(), None);
    /// ```
    pub fn as_property_access(&self) -> Option<(&SmolStr, &SmolStr)> {
        match self {
            Expression::PropertyAccess { base, property, .. } => {
                match base.as_ref() {
                    Expression::VariableRef(var, _) => Some((var, property)),
                    _ => None, // Complex base expression
                }
            }
            _ => None,
        }
    }

    /// Collect all property accesses in this expression tree.
    ///
    /// Returns a vector of (variable_name, property_name) pairs for all
    /// simple property accesses found in this expression.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::Expression;
    /// let expr = parse_expr("a.name = 'John' AND a.age > 30 AND b.city = 'SF'");
    /// let accesses = expr.collect_property_accesses();
    /// assert_eq!(accesses.len(), 3);
    /// assert!(accesses.contains(&(SmolStr::new("a"), SmolStr::new("name"))));
    /// assert!(accesses.contains(&(SmolStr::new("a"), SmolStr::new("age"))));
    /// assert!(accesses.contains(&(SmolStr::new("b"), SmolStr::new("city"))));
    /// ```
    pub fn collect_property_accesses(&self) -> Vec<(SmolStr, SmolStr)> {
        let mut accesses = Vec::new();
        self.collect_property_accesses_recursive(&mut accesses);
        accesses
    }

    fn collect_property_accesses_recursive(&self, accesses: &mut Vec<(SmolStr, SmolStr)>) {
        if let Some((var, prop)) = self.as_property_access() {
            accesses.push((var.clone(), prop.clone()));
        }

        match self {
            Expression::PropertyAccess { base, .. } => {
                base.collect_property_accesses_recursive(accesses);
            }
            Expression::BinaryOp(left, _, right, _) => {
                left.collect_property_accesses_recursive(accesses);
                right.collect_property_accesses_recursive(accesses);
            }
            Expression::LogicalOp(left, _, right, _) => {
                left.collect_property_accesses_recursive(accesses);
                right.collect_property_accesses_recursive(accesses);
            }
            Expression::UnaryOp(_, operand, _) => {
                operand.collect_property_accesses_recursive(accesses);
            }
            Expression::FunctionCall(call, _) => {
                for arg in &call.arguments {
                    arg.collect_property_accesses_recursive(accesses);
                }
            }
            Expression::CaseExpression(case, _) => {
                match case {
                    CaseExpression::Simple(simple) => {
                        simple.operand.collect_property_accesses_recursive(accesses);
                        for clause in &simple.when_clauses {
                            clause.when_operand.collect_property_accesses_recursive(accesses);
                            clause.result.collect_property_accesses_recursive(accesses);
                        }
                        if let Some(else_result) = &simple.else_result {
                            else_result.collect_property_accesses_recursive(accesses);
                        }
                    }
                    CaseExpression::Searched(searched) => {
                        for clause in &searched.when_clauses {
                            clause.condition.collect_property_accesses_recursive(accesses);
                            clause.result.collect_property_accesses_recursive(accesses);
                        }
                        if let Some(else_result) = &searched.else_result {
                            else_result.collect_property_accesses_recursive(accesses);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
```

**Tests**: `tests/ast_analysis_test.rs`

```rust
#[cfg(test)]
mod expression_analysis_tests {
    use super::*;

    #[test]
    fn referenced_variables_simple() {
        let expr = parse_expr("a.age > 30");
        let vars = expr.referenced_variables();
        assert_eq!(vars.len(), 1);
        assert!(vars.contains(&SmolStr::new("a")));
    }

    #[test]
    fn referenced_variables_complex() {
        let expr = parse_expr("a.age > 30 AND b.city = 'SF' OR c.active = true");
        let vars = expr.referenced_variables();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&SmolStr::new("a")));
        assert!(vars.contains(&SmolStr::new("b")));
        assert!(vars.contains(&SmolStr::new("c")));
    }

    #[test]
    fn as_property_access() {
        let expr = parse_expr("a.name");
        assert_eq!(
            expr.as_property_access(),
            Some((&SmolStr::new("a"), &SmolStr::new("name")))
        );
    }

    #[test]
    fn as_property_access_complex_base_returns_none() {
        let expr = parse_expr("foo(a).name");
        assert_eq!(expr.as_property_access(), None);
    }

    #[test]
    fn collect_property_accesses() {
        let expr = parse_expr("a.name = 'John' AND a.age > 30 AND b.city = 'SF'");
        let accesses = expr.collect_property_accesses();
        assert_eq!(accesses.len(), 3);

        let access_set: HashSet<_> = accesses.into_iter().collect();
        assert!(access_set.contains(&(SmolStr::new("a"), SmolStr::new("name"))));
        assert!(access_set.contains(&(SmolStr::new("a"), SmolStr::new("age"))));
        assert!(access_set.contains(&(SmolStr::new("b"), SmolStr::new("city"))));
    }
}
```

---

### Phase 3: MVP Validation (Compile-Time Guards)

#### 3.1 Validation Module

**File**: `src/ast/validation.rs`

```rust
//! MVP compatibility validation for AST nodes.
//!
//! This module provides validation methods that check whether query patterns
//! are compatible with the MVP compiler implementation. Non-compatible patterns
//! will be rejected with clear error messages at compile time.

use crate::ast::{
    GraphPattern, PathPattern, PathPatternExpression, PathTerm, PathFactor,
    ElementPattern, EdgePattern, EdgeDirection, GraphPatternQuantifier,
    PathPatternPrefix, LabelExpression,
};

/// Result of MVP compatibility check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MvpCompatibility {
    /// Pattern is compatible with MVP compiler
    Compatible,
    /// Pattern is not compatible, with explanation
    Incompatible(String),
}

impl MvpCompatibility {
    /// Check if this is compatible
    pub fn is_compatible(&self) -> bool {
        matches!(self, MvpCompatibility::Compatible)
    }

    /// Get the error message if incompatible
    pub fn error_message(&self) -> Option<&str> {
        match self {
            MvpCompatibility::Incompatible(msg) => Some(msg),
            MvpCompatibility::Compatible => None,
        }
    }
}

impl GraphPattern {
    /// Check if this graph pattern is MVP-compatible.
    ///
    /// # MVP Constraints
    ///
    /// - Only single path pattern per MATCH (no comma-separated patterns)
    /// - No shortest path prefixes (ALL SHORTEST, ANY SHORTEST, etc.)
    /// - Only outgoing edges (`->`)
    /// - Variable-length paths capped at 10 hops
    /// - No complex label expressions (conjunction, negation) on edges
    ///
    /// # Examples
    ///
    /// ```
    /// # use gql_parser::ast::{GraphPattern, MvpCompatibility};
    /// let pattern = parse_pattern("(a:Person)-[:KNOWS]->(b)");
    /// assert!(matches!(pattern.validate_mvp(), MvpCompatibility::Compatible));
    ///
    /// let multi_pattern = parse_pattern("(a)->(b), (c)->(d)");
    /// assert!(matches!(
    ///     multi_pattern.validate_mvp(),
    ///     MvpCompatibility::Incompatible(_)
    /// ));
    /// ```
    pub fn validate_mvp(&self) -> MvpCompatibility {
        // Check single pattern constraint
        if self.paths.patterns.len() > 1 {
            return MvpCompatibility::Incompatible(
                format!(
                    "Multiple path patterns in single MATCH not supported in MVP (found {})",
                    self.paths.patterns.len()
                )
            );
        }

        // Check the single pattern
        if let Some(pattern) = self.paths.patterns.first() {
            pattern.validate_mvp()
        } else {
            // Empty pattern is technically valid but useless
            MvpCompatibility::Compatible
        }
    }
}

impl PathPattern {
    /// Check if this path pattern is MVP-compatible.
    pub fn validate_mvp(&self) -> MvpCompatibility {
        // Check for shortest path prefix
        if let Some(prefix) = &self.prefix {
            use crate::ast::PathPatternPrefix;
            if matches!(prefix, PathPatternPrefix::PathSearch(_)) {
                return MvpCompatibility::Incompatible(
                    "Shortest path queries not supported in MVP. \
                     Consider using variable-length paths with explicit bounds instead."
                        .to_string()
                );
            }
        }

        // Check the path expression
        self.expression.validate_mvp()
    }
}

impl PathPatternExpression {
    fn validate_mvp(&self) -> MvpCompatibility {
        match self {
            PathPatternExpression::Term(term) => term.validate_mvp(),
            PathPatternExpression::Union { .. } => {
                MvpCompatibility::Incompatible(
                    "Path unions (|) not supported in MVP. \
                     Consider using UNION ALL to combine separate queries."
                        .to_string()
                )
            }
            PathPatternExpression::Alternation { .. } => {
                MvpCompatibility::Incompatible(
                    "Path multiset alternation (|+|) not supported in MVP."
                        .to_string()
                )
            }
        }
    }
}

impl PathTerm {
    fn validate_mvp(&self) -> MvpCompatibility {
        for factor in &self.factors {
            if let MvpCompatibility::Incompatible(msg) = factor.validate_mvp() {
                return MvpCompatibility::Incompatible(msg);
            }
        }
        MvpCompatibility::Compatible
    }
}

impl PathFactor {
    fn validate_mvp(&self) -> MvpCompatibility {
        // Check quantifier bounds
        if let Some(quantifier) = &self.quantifier {
            if let MvpCompatibility::Incompatible(msg) = quantifier.validate_mvp() {
                return MvpCompatibility::Incompatible(msg);
            }
        }

        // Check element pattern
        use crate::ast::PathPrimary;
        match &self.primary {
            PathPrimary::ElementPattern(elem) => elem.validate_mvp(),
            PathPrimary::ParenthesizedExpression(expr) => expr.validate_mvp(),
            PathPrimary::SimplifiedExpression(_) => {
                MvpCompatibility::Incompatible(
                    "Simplified path syntax not supported in MVP."
                        .to_string()
                )
            }
        }
    }
}

impl GraphPatternQuantifier {
    /// Check if this quantifier is MVP-compatible.
    ///
    /// MVP enforces a maximum of 10 hops for variable-length paths
    /// to prevent unbounded expansion.
    pub fn validate_mvp(&self) -> MvpCompatibility {
        const MAX_HOPS_MVP: u32 = 10;

        if let Some(max) = self.max_hops() {
            if max > MAX_HOPS_MVP {
                return MvpCompatibility::Incompatible(
                    format!(
                        "Variable-length paths capped at {} hops in MVP (requested: {}). \
                         Consider breaking into multiple queries or using a lower bound.",
                        MAX_HOPS_MVP, max
                    )
                );
            }
        } else {
            // Unbounded quantifiers (*, +)
            return MvpCompatibility::Incompatible(
                format!(
                    "Unbounded variable-length paths ({}) not supported in MVP. \
                     Use explicit bounds like {{1,{}}} instead.",
                    match self {
                        GraphPatternQuantifier::Star { .. } => "*",
                        GraphPatternQuantifier::Plus { .. } => "+",
                        _ => unreachable!(),
                    },
                    MAX_HOPS_MVP
                )
            );
        }

        MvpCompatibility::Compatible
    }
}

impl ElementPattern {
    fn validate_mvp(&self) -> MvpCompatibility {
        match self {
            ElementPattern::Node(node) => {
                // Node patterns have no MVP restrictions currently
                // (label expressions on nodes are fully supported)
                MvpCompatibility::Compatible
            }
            ElementPattern::Edge(edge) => edge.validate_mvp(),
        }
    }
}

impl EdgePattern {
    /// Check if this edge pattern is MVP-compatible.
    ///
    /// MVP only supports outgoing edges (`->`). Incoming edges require
    /// a CSC (Compressed Sparse Column) index which is not in MVP scope.
    pub fn validate_mvp(&self) -> MvpCompatibility {
        // Check direction
        if !self.is_outgoing() {
            let direction_str = match self {
                EdgePattern::Full(full) => format!("{:?}", full.direction),
                EdgePattern::Abbreviated(abbrev) => format!("{:?}", abbrev),
            };

            return MvpCompatibility::Incompatible(
                format!(
                    "Only outgoing edges (->) supported in MVP. \
                     Direction '{}' requires CSC index (not implemented). \
                     Consider reversing the pattern direction.",
                    direction_str
                )
            );
        }

        // Check label expression complexity (if present)
        if let EdgePattern::Full(full) = self {
            if let Some(label_expr) = &full.filler.label_expression {
                if let MvpCompatibility::Incompatible(msg) =
                    validate_edge_label_expression(label_expr)
                {
                    return MvpCompatibility::Incompatible(msg);
                }
            }
        }

        MvpCompatibility::Compatible
    }
}

/// Validate edge label expressions for MVP.
///
/// MVP supports:
/// - Simple labels: `:KNOWS`
/// - Disjunction: `:KNOWS|FRIEND` (implemented as post-filter)
/// - Wildcard: no label or `:%`
///
/// MVP rejects:
/// - Conjunction: `:KNOWS&:ACTIVE`
/// - Negation: `!:KNOWS`
fn validate_edge_label_expression(expr: &LabelExpression) -> MvpCompatibility {
    match expr {
        LabelExpression::LabelName { .. } |
        LabelExpression::Wildcard { .. } => MvpCompatibility::Compatible,

        LabelExpression::Disjunction { left, right, .. } => {
            // Recursively check both sides
            if let MvpCompatibility::Incompatible(msg) =
                validate_edge_label_expression(left)
            {
                return MvpCompatibility::Incompatible(msg);
            }
            if let MvpCompatibility::Incompatible(msg) =
                validate_edge_label_expression(right)
            {
                return MvpCompatibility::Incompatible(msg);
            }
            MvpCompatibility::Compatible
        }

        LabelExpression::Conjunction { .. } => {
            MvpCompatibility::Incompatible(
                "Label conjunction (:A&:B) on edges not supported in MVP. \
                 Consider using separate MATCH clauses or post-filtering in WHERE."
                    .to_string()
            )
        }

        LabelExpression::Negation { .. } => {
            MvpCompatibility::Incompatible(
                "Label negation (!:A) on edges not supported in MVP. \
                 Consider using WHERE clause with type checking instead."
                    .to_string()
            )
        }

        LabelExpression::Parenthesized { expression, .. } => {
            validate_edge_label_expression(expression)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_simple_pattern() {
        let pattern = parse_pattern("(a:Person)-[:KNOWS]->(b)");
        assert!(matches!(pattern.validate_mvp(), MvpCompatibility::Compatible));
    }

    #[test]
    fn validate_multiple_patterns_rejected() {
        let pattern = parse_pattern("(a)->(b), (c)->(d)");
        assert!(matches!(
            pattern.validate_mvp(),
            MvpCompatibility::Incompatible(_)
        ));
    }

    #[test]
    fn validate_incoming_edge_rejected() {
        let pattern = parse_pattern("(a)<-[:KNOWS]-(b)");
        let result = pattern.validate_mvp();
        assert!(matches!(result, MvpCompatibility::Incompatible(_)));
        assert!(result.error_message().unwrap().contains("CSC"));
    }

    #[test]
    fn validate_unbounded_quantifier_rejected() {
        let pattern = parse_pattern("(a)-[:KNOWS*]->(b)");
        let result = pattern.validate_mvp();
        assert!(matches!(result, MvpCompatibility::Incompatible(_)));
        assert!(result.error_message().unwrap().contains("Unbounded"));
    }

    #[test]
    fn validate_bounded_quantifier_within_limit() {
        let pattern = parse_pattern("(a)-[:KNOWS*1..5]->(b)");
        assert!(matches!(pattern.validate_mvp(), MvpCompatibility::Compatible));
    }

    #[test]
    fn validate_bounded_quantifier_exceeds_limit() {
        let pattern = parse_pattern("(a)-[:KNOWS*1..20]->(b)");
        let result = pattern.validate_mvp();
        assert!(matches!(result, MvpCompatibility::Incompatible(_)));
        assert!(result.error_message().unwrap().contains("capped at 10"));
    }

    #[test]
    fn validate_label_disjunction_accepted() {
        let pattern = parse_pattern("(a)-[:KNOWS|FRIEND]->(b)");
        assert!(matches!(pattern.validate_mvp(), MvpCompatibility::Compatible));
    }

    #[test]
    fn validate_label_conjunction_rejected() {
        let pattern = parse_pattern("(a)-[:KNOWS&ACTIVE]->(b)");
        let result = pattern.validate_mvp();
        assert!(matches!(result, MvpCompatibility::Incompatible(_)));
        assert!(result.error_message().unwrap().contains("conjunction"));
    }

    #[test]
    fn validate_shortest_path_rejected() {
        let pattern = parse_pattern("SHORTEST (a)-[:KNOWS]->(b)");
        let result = pattern.validate_mvp();
        assert!(matches!(result, MvpCompatibility::Incompatible(_)));
        assert!(result.error_message().unwrap().contains("Shortest path"));
    }
}
```

**Module Export**: Add to `src/ast/mod.rs`

```rust
pub mod validation;
pub use validation::MvpCompatibility;
```

---

### Phase 4: Demand Analysis (Optimization Support)

#### 4.1 Demand Analysis Module

**File**: `src/ast/demand.rs`

```rust
//! Property demand analysis for query optimization.
//!
//! This module analyzes which properties are actually needed for each variable
//! in a query, enabling the compiler to only fetch necessary columns from storage.

use std::collections::{HashMap, HashSet};
use smol_str::SmolStr;
use crate::ast::{
    GraphPattern, PathPattern, PathPatternExpression, PathTerm, PathFactor,
    PathPrimary, ElementPattern, NodePattern, EdgePattern,
    ReturnStatement, ReturnItem, Expression, ElementPropertySpecification,
};

/// Map from variable name to set of property names demanded by the query.
///
/// # Example
///
/// For query `MATCH (a:Person)-[:KNOWS]->(b) WHERE a.age > 30 RETURN a.name, b.city`:
///
/// ```text
/// {
///   "a": {"age", "name"},
///   "b": {"city"}
/// }
/// ```
pub type PropertyDemands = HashMap<SmolStr, HashSet<SmolStr>>;

/// Analyze which properties are actually needed for each variable.
///
/// This enables the compiler to only project necessary columns, avoiding
/// unnecessary joins and data movement.
///
/// # Examples
///
/// ```
/// # use gql_parser::ast::demand::analyze_property_demands;
/// let gql = "MATCH (a:Person)-[:KNOWS]->(b:Company) \
///            WHERE a.age > 30 AND b.founded < 2000 \
///            RETURN a.name, b.name";
/// let ast = parse(gql).unwrap();
/// let pattern = &ast.statements[0].match_pattern;
/// let return_stmt = &ast.statements[0].return_statement;
///
/// let demands = analyze_property_demands(pattern, return_stmt.as_ref());
///
/// assert_eq!(demands.get("a").unwrap().len(), 2); // age, name
/// assert!(demands.get("a").unwrap().contains("age"));
/// assert!(demands.get("a").unwrap().contains("name"));
///
/// assert_eq!(demands.get("b").unwrap().len(), 2); // founded, name
/// assert!(demands.get("b").unwrap().contains("founded"));
/// assert!(demands.get("b").unwrap().contains("name"));
/// ```
pub fn analyze_property_demands(
    pattern: &GraphPattern,
    return_stmt: Option<&ReturnStatement>,
) -> PropertyDemands {
    let mut demands: PropertyDemands = HashMap::new();

    // Collect from pattern WHERE clause
    if let Some(where_clause) = &pattern.where_clause {
        collect_from_expression(&where_clause.condition, &mut demands);
    }

    // Collect from node/edge property specs and WHERE predicates
    for path in &pattern.paths.patterns {
        collect_from_path_pattern(path, &mut demands);
    }

    // Collect from RETURN
    if let Some(ret) = return_stmt {
        collect_from_return(ret, &mut demands);
    }

    demands
}

/// Collect property demands from a path pattern.
fn collect_from_path_pattern(
    pattern: &PathPattern,
    demands: &mut PropertyDemands,
) {
    match &pattern.expression {
        PathPatternExpression::Term(term) => {
            collect_from_path_term(term, demands);
        }
        PathPatternExpression::Union { left, right, .. } => {
            collect_from_path_pattern_expression(left, demands);
            collect_from_path_pattern_expression(right, demands);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for alt in alternatives {
                collect_from_path_term(alt, demands);
            }
        }
    }
}

fn collect_from_path_pattern_expression(
    expr: &PathPatternExpression,
    demands: &mut PropertyDemands,
) {
    match expr {
        PathPatternExpression::Term(term) => collect_from_path_term(term, demands),
        PathPatternExpression::Union { left, right, .. } => {
            collect_from_path_pattern_expression(left, demands);
            collect_from_path_pattern_expression(right, demands);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for alt in alternatives {
                collect_from_path_term(alt, demands);
            }
        }
    }
}

fn collect_from_path_term(
    term: &PathTerm,
    demands: &mut PropertyDemands,
) {
    for factor in &term.factors {
        match &factor.primary {
            PathPrimary::ElementPattern(elem) => {
                match elem.as_ref() {
                    ElementPattern::Node(node) => {
                        collect_from_node_pattern(node, demands);
                    }
                    ElementPattern::Edge(edge) => {
                        collect_from_edge_pattern(edge, demands);
                    }
                }
            }
            PathPrimary::ParenthesizedExpression(expr) => {
                collect_from_path_pattern_expression(expr, demands);
            }
            PathPrimary::SimplifiedExpression(_) => {
                // Simplified syntax doesn't have property specs
            }
        }
    }
}

fn collect_from_node_pattern(
    node: &NodePattern,
    demands: &mut PropertyDemands,
) {
    // Properties in {key: expr} specs
    if let Some(props) = &node.properties {
        collect_from_property_spec(props, demands);
    }

    // Properties in WHERE predicates
    if let Some(where_pred) = &node.where_clause {
        collect_from_expression(&where_pred.condition, demands);
    }
}

fn collect_from_edge_pattern(
    edge: &EdgePattern,
    demands: &mut PropertyDemands,
) {
    // Similar collection for edge properties if your AST supports them
    if let EdgePattern::Full(full) = edge {
        if let Some(props) = &full.filler.properties {
            collect_from_property_spec(props, demands);
        }
        if let Some(where_pred) = &full.filler.where_clause {
            collect_from_expression(&where_pred.condition, demands);
        }
    }
}

fn collect_from_property_spec(
    props: &ElementPropertySpecification,
    demands: &mut PropertyDemands,
) {
    for pair in &props.properties {
        // The key itself might be used in comparisons
        // The value expression might reference other properties
        collect_from_expression(&pair.value, demands);
    }
}

fn collect_from_return(
    ret: &ReturnStatement,
    demands: &mut PropertyDemands,
) {
    for item in &ret.items {
        collect_from_expression(&item.expression, demands);
    }
}

fn collect_from_expression(
    expr: &Expression,
    demands: &mut PropertyDemands,
) {
    // Use the helper method from Phase 2
    for (var, prop) in expr.collect_property_accesses() {
        demands.entry(var)
            .or_insert_with(HashSet::new)
            .insert(prop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_simple_query() {
        let gql = "MATCH (a:Person) WHERE a.age > 30 RETURN a.name";
        let ast = parse(gql).unwrap();
        let pattern = extract_pattern(&ast);
        let return_stmt = extract_return(&ast);

        let demands = analyze_property_demands(pattern, Some(return_stmt));

        let a_demands = demands.get(&SmolStr::new("a")).unwrap();
        assert_eq!(a_demands.len(), 2);
        assert!(a_demands.contains(&SmolStr::new("age")));
        assert!(a_demands.contains(&SmolStr::new("name")));
    }

    #[test]
    fn analyze_multi_variable_query() {
        let gql = "MATCH (a:Person)-[:KNOWS]->(b:Company) \
                   WHERE a.age > 30 AND b.founded < 2000 \
                   RETURN a.name, b.name, b.city";
        let ast = parse(gql).unwrap();
        let pattern = extract_pattern(&ast);
        let return_stmt = extract_return(&ast);

        let demands = analyze_property_demands(pattern, Some(return_stmt));

        let a_demands = demands.get(&SmolStr::new("a")).unwrap();
        assert_eq!(a_demands.len(), 2);
        assert!(a_demands.contains(&SmolStr::new("age")));
        assert!(a_demands.contains(&SmolStr::new("name")));

        let b_demands = demands.get(&SmolStr::new("b")).unwrap();
        assert_eq!(b_demands.len(), 3);
        assert!(b_demands.contains(&SmolStr::new("founded")));
        assert!(b_demands.contains(&SmolStr::new("name")));
        assert!(b_demands.contains(&SmolStr::new("city")));
    }

    #[test]
    fn analyze_property_spec_demands() {
        let gql = "MATCH (a:Person {age: 30, city: 'SF'}) RETURN a.name";
        let ast = parse(gql).unwrap();
        let pattern = extract_pattern(&ast);
        let return_stmt = extract_return(&ast);

        let demands = analyze_property_demands(pattern, Some(return_stmt));

        let a_demands = demands.get(&SmolStr::new("a")).unwrap();
        // Property specs don't create demands themselves (they're filters)
        // Only the RETURN creates a demand for "name"
        assert_eq!(a_demands.len(), 1);
        assert!(a_demands.contains(&SmolStr::new("name")));
    }

    #[test]
    fn analyze_no_demands() {
        let gql = "MATCH (a:Person) RETURN a"; // Returning the node itself
        let ast = parse(gql).unwrap();
        let pattern = extract_pattern(&ast);
        let return_stmt = extract_return(&ast);

        let demands = analyze_property_demands(pattern, Some(return_stmt));

        // No property accesses, so no demands
        assert!(demands.get(&SmolStr::new("a")).is_none() ||
                demands.get(&SmolStr::new("a")).unwrap().is_empty());
    }
}
```

**Module Export**: Add to `src/ast/mod.rs`

```rust
pub mod demand;
```

---

## Testing Strategy

### Unit Tests

Each helper method should have dedicated unit tests covering:

1. **Happy path**: Normal expected usage
2. **Edge cases**: Empty patterns, missing variables, complex nesting
3. **Error cases**: Invalid patterns that should return None or specific errors

**Test Files:**
- `tests/ast_helpers_test.rs` - Convenience accessors (Phase 1)
- `tests/ast_analysis_test.rs` - Analysis helpers (Phase 2)
- `tests/ast_validation_test.rs` - MVP validation (Phase 3)
- `tests/ast_demand_test.rs` - Demand analysis (Phase 4)

### Integration Tests

Create integration tests that parse real GQL queries and validate the helpers:

**File**: `tests/integration/ast_enhancements_test.rs`

```rust
#[test]
fn end_to_end_simple_query() {
    let gql = "MATCH (a:Person {age > 30})-[:KNOWS]->(b:Company) \
               WHERE b.founded < 2000 \
               RETURN a.name, b.name";

    let ast = parse(gql).unwrap();
    let pattern = extract_first_match(&ast);

    // Validation
    assert!(pattern.validate_mvp().is_compatible());

    // Variable collection
    let vars = pattern.bound_variables();
    assert_eq!(vars.len(), 2);

    // Chain extraction
    assert!(pattern.is_single_chain());

    // Demand analysis
    let demands = analyze_property_demands(pattern, extract_return(&ast));
    assert!(demands.get("a").unwrap().contains("name"));
    assert!(demands.get("b").unwrap().contains("founded"));
}
```

### Regression Tests

Run existing parser test suite to ensure no breaking changes:

```bash
cargo test --all
```

### Performance Tests

Benchmark the overhead of helper methods (should be negligible):

**File**: `benches/ast_helpers_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_variable_collection(c: &mut Criterion) {
    let pattern = parse_complex_pattern();

    c.bench_function("bound_variables", |b| {
        b.iter(|| {
            black_box(pattern.bound_variables())
        })
    });
}

criterion_group!(benches, benchmark_variable_collection);
criterion_main!(benches);
```

---

## Timeline and Milestones

### Week 1: Foundation (Phase 1)

**Days 1-2**: Convenience accessors
- [ ] Implement `NodePattern` helpers
- [ ] Implement `EdgePattern` helpers
- [ ] Write unit tests

**Days 3-4**: Label and quantifier helpers
- [ ] Implement `LabelExpression` helpers
- [ ] Implement `GraphPatternQuantifier` helpers
- [ ] Write unit tests

**Day 5**: Testing and documentation
- [ ] Write integration tests
- [ ] Add rustdoc examples
- [ ] Update SPRINT documentation

### Week 2: Analysis (Phases 2-3)

**Days 1-2**: Variable and expression analysis
- [ ] Implement variable collection
- [ ] Implement expression analysis
- [ ] Write unit tests

**Days 3-4**: MVP validation
- [ ] Implement validation module
- [ ] Write comprehensive validation tests
- [ ] Test with edge cases

**Day 5**: Integration testing
- [ ] End-to-end integration tests
- [ ] Regression testing
- [ ] Performance benchmarks

### Week 3: Optimization (Phase 4)

**Days 1-2**: Demand analysis
- [ ] Implement demand analysis module
- [ ] Write unit tests

**Days 3-4**: Compiler integration prep
- [ ] Create example compiler usage
- [ ] Write documentation
- [ ] Prepare migration guide

**Day 5**: Final review
- [ ] Code review
- [ ] Documentation review
- [ ] Merge to main

---

## Success Criteria

### Functional Requirements

- [ ] All helper methods implemented with tests
- [ ] 100% test coverage for new code
- [ ] Zero breaking changes to existing API
- [ ] All existing tests pass

### Performance Requirements

- [ ] Helper methods add <1% overhead to parsing
- [ ] Analysis operations complete in <10ms for typical queries
- [ ] No memory leaks or allocations in hot paths

### Documentation Requirements

- [ ] All public methods have rustdoc comments
- [ ] All methods have usage examples
- [ ] Integration guide for compiler developers
- [ ] Migration notes in SPRINT15.md

### Usability Requirements

- [ ] Compiler code using helpers is 50%+ shorter than manual traversal
- [ ] Error messages are clear and actionable
- [ ] API is intuitive (minimal surprises)

---

## Migration Guide for Compiler Developers

### Before (Manual Traversal)

```rust
// Extracting variable name manually
let var_name = match &node.variable {
    Some(var_decl) => &var_decl.variable,
    None => return Err("No variable"),
};

// Checking label manually
let label_id = match &node.label_expression {
    Some(LabelExpression::LabelName { name, .. }) => {
        catalog.lookup(name)?
    }
    Some(_) => return Err("Complex label"),
    None => return Err("No label"),
};

// Collecting variables manually
fn collect_vars(pattern: &GraphPattern) -> HashSet<&SmolStr> {
    let mut vars = HashSet::new();
    for path in &pattern.paths.patterns {
        // 30+ lines of recursive traversal...
    }
    vars
}
```

### After (Using Helpers)

```rust
// Extracting variable name
let var_name = node.variable_name()
    .ok_or("No variable")?;

// Checking label
let label_id = node.simple_label()
    .and_then(|name| catalog.lookup(name))
    .ok_or("No simple label")?;

// Collecting variables
let vars = pattern.bound_variables();
```

### Result

- **50% less code**
- **Clearer intent**
- **Fewer bugs**
- **Easier maintenance**

---

## Appendix: File Change Summary

### New Files

```
src/ast/validation.rs          (~400 lines)
src/ast/demand.rs              (~250 lines)
tests/ast_helpers_test.rs      (~500 lines)
tests/ast_analysis_test.rs     (~400 lines)
tests/ast_validation_test.rs   (~300 lines)
tests/ast_demand_test.rs       (~200 lines)
benches/ast_helpers_bench.rs   (~100 lines)
```

### Modified Files

```
src/ast/mod.rs                 (+10 lines - exports)
src/ast/query.rs               (+250 lines - helpers)
src/ast/expression.rs          (+200 lines - helpers)
```

### Total Addition

- **~2,600 lines** of well-tested, documented code
- **Zero breaking changes**
- **Significant compiler simplification**

---

## Next Steps

After completion of this enhancement:

1. **Sprint 16**: Begin compiler implementation using these helpers
2. **Sprint 17**: Implement ExpandOutExec using validated patterns
3. **Sprint 18**: End-to-end integration with DataFusion

This enhancement is the foundation for a clean, maintainable compiler architecture.
