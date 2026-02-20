//! Graph pattern metadata extraction.

use std::collections::{BTreeMap, BTreeSet};

use smol_str::SmolStr;

use crate::ast::Expression;
use crate::ast::query::{
    EdgePattern, ElementPattern, GraphPattern, LabelExpression, PathFactor, PathPattern,
    PathPatternExpression, PathPatternPrefix, PathPrimary, PathSearch, PathTerm,
    ShortestPathSearch, SimplifiedPathPatternExpression,
};

/// Coarse-grained complexity classification for pattern label expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LabelExpressionComplexity {
    /// The pattern contains no label expressions.
    #[default]
    None,
    /// Only single labels or wildcards are used.
    Simple,
    /// Boolean operators (`|` / `&`) are used.
    Boolean,
    /// Unary negation (`!`) is used on simple labels.
    Negated,
    /// Nested combinations of boolean operators/negation are used.
    NestedBoolean,
}

/// Compiler-oriented metadata extracted from a graph pattern.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatternInfo {
    /// Number of node pattern occurrences.
    pub node_count: usize,
    /// Number of edge pattern occurrences.
    pub edge_count: usize,
    /// Number of top-level path patterns.
    pub path_count: usize,
    /// Number of label expressions encountered in node/edge patterns.
    pub label_expression_count: usize,
    /// Maximum label-expression complexity seen across the pattern.
    pub label_expression_complexity: LabelExpressionComplexity,
    /// Number of distinct node variables seen in the pattern.
    pub node_variable_count: usize,
    /// Number of connected components inferred from node variables per path.
    pub connected_component_count: usize,
    /// True when all discovered node variables are connected.
    pub is_fully_connected: bool,
}

impl PatternInfo {
    /// Extracts structural pattern metadata.
    pub fn analyze(pattern: &GraphPattern) -> Self {
        let mut analyzer = PatternAnalyzer::default();
        analyzer.analyze_pattern(pattern);
        analyzer.into_info(pattern.paths.patterns.len())
    }
}

#[derive(Debug, Default)]
struct PatternAnalyzer {
    node_count: usize,
    edge_count: usize,
    label_expression_count: usize,
    label_expression_complexity: LabelExpressionComplexity,
    node_indices: BTreeMap<SmolStr, usize>,
    parent: Vec<usize>,
}

impl PatternAnalyzer {
    fn into_info(self, path_count: usize) -> PatternInfo {
        let component_count = self.connected_component_count();

        PatternInfo {
            node_count: self.node_count,
            edge_count: self.edge_count,
            path_count,
            label_expression_count: self.label_expression_count,
            label_expression_complexity: self.label_expression_complexity,
            node_variable_count: self.node_indices.len(),
            connected_component_count: component_count,
            is_fully_connected: component_count <= 1,
        }
    }

    fn analyze_pattern(&mut self, pattern: &GraphPattern) {
        for path in &pattern.paths.patterns {
            self.analyze_path_pattern(path);
        }
    }

    fn analyze_path_pattern(&mut self, path: &PathPattern) {
        let mut vars_in_path = BTreeSet::new();

        self.analyze_path_expression(&path.expression, &mut vars_in_path);
        self.connect_path_variables(&vars_in_path);
    }

    fn analyze_path_expression(
        &mut self,
        expression: &PathPatternExpression,
        vars_in_path: &mut BTreeSet<SmolStr>,
    ) {
        match expression {
            PathPatternExpression::Union { left, right, .. } => {
                self.analyze_path_expression(left, vars_in_path);
                self.analyze_path_expression(right, vars_in_path);
            }
            PathPatternExpression::Alternation { alternatives, .. } => {
                for alternative in alternatives {
                    self.analyze_path_expression(alternative, vars_in_path);
                }
            }
            PathPatternExpression::Term(term) => {
                self.analyze_path_term(term, vars_in_path);
            }
        }
    }

    fn analyze_path_term(&mut self, term: &PathTerm, vars_in_path: &mut BTreeSet<SmolStr>) {
        for factor in &term.factors {
            self.analyze_path_factor(factor, vars_in_path);
        }
    }

    fn analyze_path_factor(&mut self, factor: &PathFactor, vars_in_path: &mut BTreeSet<SmolStr>) {
        match &factor.primary {
            PathPrimary::ElementPattern(element) => {
                self.analyze_element_pattern(element, vars_in_path);
            }
            PathPrimary::ParenthesizedExpression(expression) => {
                self.analyze_path_expression(expression, vars_in_path);
            }
            PathPrimary::SimplifiedExpression(expression) => {
                self.analyze_simplified_expression(expression);
            }
        }
    }

    fn analyze_simplified_expression(&mut self, expression: &SimplifiedPathPatternExpression) {
        match expression {
            SimplifiedPathPatternExpression::Contents(_)
            | SimplifiedPathPatternExpression::Questioned(_) => {}
            SimplifiedPathPatternExpression::Union(union) => {
                self.analyze_simplified_expression(&union.left);
                self.analyze_simplified_expression(&union.right);
            }
            SimplifiedPathPatternExpression::MultisetAlternation(alternation) => {
                for alternative in &alternation.alternatives {
                    self.analyze_simplified_expression(alternative);
                }
            }
            SimplifiedPathPatternExpression::Conjunction(conjunction) => {
                self.analyze_simplified_expression(&conjunction.left);
                self.analyze_simplified_expression(&conjunction.right);
            }
            SimplifiedPathPatternExpression::Concatenation(concatenation) => {
                for part in &concatenation.parts {
                    self.analyze_simplified_expression(part);
                }
            }
            SimplifiedPathPatternExpression::Quantified(quantified) => {
                self.analyze_simplified_expression(&quantified.pattern);
            }
            SimplifiedPathPatternExpression::DirectionOverride(direction_override) => {
                self.analyze_simplified_expression(&direction_override.pattern);
            }
            SimplifiedPathPatternExpression::Negation(negation) => {
                self.analyze_simplified_expression(&negation.pattern);
            }
        }
    }

    fn analyze_element_pattern(
        &mut self,
        element: &ElementPattern,
        vars_in_path: &mut BTreeSet<SmolStr>,
    ) {
        match element {
            ElementPattern::Node(node) => {
                self.node_count += 1;

                if let Some(variable) = &node.variable {
                    vars_in_path.insert(variable.variable.clone());
                    self.ensure_node_variable(variable.variable.clone());
                }

                if let Some(label_expression) = &node.label_expression {
                    self.observe_label_expression(label_expression);
                }
            }
            ElementPattern::Edge(edge) => {
                self.edge_count += 1;

                if let EdgePattern::Full(full_edge) = edge
                    && let Some(label_expression) = &full_edge.filler.label_expression
                {
                    self.observe_label_expression(label_expression);
                }
            }
        }
    }

    fn observe_label_expression(&mut self, expression: &LabelExpression) {
        self.label_expression_count += 1;
        self.label_expression_complexity = self
            .label_expression_complexity
            .max(classify_label_expression(expression));
    }

    fn ensure_node_variable(&mut self, variable: SmolStr) {
        if self.node_indices.contains_key(&variable) {
            return;
        }

        let index = self.parent.len();
        self.parent.push(index);
        self.node_indices.insert(variable, index);
    }

    fn connect_path_variables(&mut self, vars_in_path: &BTreeSet<SmolStr>) {
        let mut iter = vars_in_path.iter();
        let Some(first) = iter.next() else {
            return;
        };

        let first_idx = self.node_indices[first];
        for variable in iter {
            let idx = self.node_indices[variable];
            self.union(first_idx, idx);
        }
    }

    fn find_root(&self, mut index: usize) -> usize {
        while self.parent[index] != index {
            index = self.parent[index];
        }
        index
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find_root(left);
        let right_root = self.find_root(right);

        if left_root != right_root {
            self.parent[right_root] = left_root;
        }
    }

    fn connected_component_count(&self) -> usize {
        let mut roots = BTreeSet::new();
        for index in 0..self.parent.len() {
            roots.insert(self.find_root(index));
        }
        roots.len()
    }
}

fn classify_label_expression(expression: &LabelExpression) -> LabelExpressionComplexity {
    match expression {
        LabelExpression::LabelName { .. } | LabelExpression::Wildcard { .. } => {
            LabelExpressionComplexity::Simple
        }
        LabelExpression::Parenthesized {
            expression: inner, ..
        } => classify_label_expression(inner),
        LabelExpression::Negation { operand, .. } => match classify_label_expression(operand) {
            LabelExpressionComplexity::Simple => LabelExpressionComplexity::Negated,
            LabelExpressionComplexity::None => LabelExpressionComplexity::Negated,
            _ => LabelExpressionComplexity::NestedBoolean,
        },
        LabelExpression::Conjunction { left, right, .. }
        | LabelExpression::Disjunction { left, right, .. } => {
            let left_complexity = classify_label_expression(left);
            let right_complexity = classify_label_expression(right);

            if left_complexity <= LabelExpressionComplexity::Simple
                && right_complexity <= LabelExpressionComplexity::Simple
            {
                LabelExpressionComplexity::Boolean
            } else {
                LabelExpressionComplexity::NestedBoolean
            }
        }
    }
}

/// Collects root expression nodes contained in a graph pattern.
pub(crate) fn collect_pattern_expression_roots<'a>(
    pattern: &'a GraphPattern,
    roots: &mut Vec<&'a Expression>,
) {
    if let Some(where_clause) = &pattern.where_clause {
        roots.push(&where_clause.condition);
    }

    if let Some(yield_clause) = &pattern.yield_clause {
        for item in &yield_clause.items {
            roots.push(&item.expression);
        }
    }

    for path_pattern in &pattern.paths.patterns {
        collect_path_pattern_expression_roots(path_pattern, roots);
    }
}

fn collect_path_pattern_expression_roots<'a>(
    path_pattern: &'a PathPattern,
    roots: &mut Vec<&'a Expression>,
) {
    if let Some(prefix) = &path_pattern.prefix {
        collect_prefix_expression_roots(prefix, roots);
    }

    collect_path_expression_roots(&path_pattern.expression, roots);
}

fn collect_prefix_expression_roots<'a>(
    prefix: &'a PathPatternPrefix,
    roots: &mut Vec<&'a Expression>,
) {
    if let PathPatternPrefix::PathSearch(
        PathSearch::Shortest(ShortestPathSearch::CountedShortest { count, .. })
        | PathSearch::Shortest(ShortestPathSearch::CountedShortestGroups { count, .. }),
    ) = prefix
    {
        roots.push(count);
    }
}

fn collect_path_expression_roots<'a>(
    expression: &'a PathPatternExpression,
    roots: &mut Vec<&'a Expression>,
) {
    match expression {
        PathPatternExpression::Union { left, right, .. } => {
            collect_path_expression_roots(left, roots);
            collect_path_expression_roots(right, roots);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            for alternative in alternatives {
                collect_path_expression_roots(alternative, roots);
            }
        }
        PathPatternExpression::Term(term) => {
            collect_path_term_expression_roots(term, roots);
        }
    }
}

fn collect_path_term_expression_roots<'a>(term: &'a PathTerm, roots: &mut Vec<&'a Expression>) {
    for factor in &term.factors {
        collect_path_factor_expression_roots(factor, roots);
    }
}

fn collect_path_factor_expression_roots<'a>(
    factor: &'a PathFactor,
    roots: &mut Vec<&'a Expression>,
) {
    match &factor.primary {
        PathPrimary::ElementPattern(element) => collect_element_expression_roots(element, roots),
        PathPrimary::ParenthesizedExpression(expression) => {
            collect_path_expression_roots(expression, roots);
        }
        PathPrimary::SimplifiedExpression(_) => {}
    }
}

fn collect_element_expression_roots<'a>(
    element: &'a ElementPattern,
    roots: &mut Vec<&'a Expression>,
) {
    match element {
        ElementPattern::Node(node) => {
            if let Some(properties) = &node.properties {
                for pair in &properties.properties {
                    roots.push(&pair.value);
                }
            }

            if let Some(where_clause) = &node.where_clause {
                roots.push(&where_clause.condition);
            }
        }
        ElementPattern::Edge(edge) => {
            if let EdgePattern::Full(full_edge) = edge {
                if let Some(properties) = &full_edge.filler.properties {
                    for pair in &properties.properties {
                        roots.push(&pair.value);
                    }
                }

                if let Some(where_clause) = &full_edge.filler.where_clause {
                    roots.push(&where_clause.condition);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::pattern_info::{
        LabelExpressionComplexity, PatternInfo, collect_pattern_expression_roots,
    };
    use crate::ast::{MatchStatement, Statement};
    use crate::parse;

    fn first_graph_pattern(source: &str) -> crate::ast::GraphPattern {
        let parse_result = parse(source);
        let program = parse_result.ast.expect("expected AST");

        let Statement::Query(query_statement) = &program.statements[0] else {
            panic!("expected query statement");
        };

        let crate::ast::Query::Linear(crate::ast::LinearQuery::Ambient(ambient)) =
            &query_statement.query
        else {
            panic!("expected ambient query");
        };

        let crate::ast::PrimitiveQueryStatement::Match(match_statement) =
            &ambient.primitive_statements[0]
        else {
            panic!("expected MATCH statement");
        };

        match match_statement {
            MatchStatement::Simple(simple) => simple.pattern.clone(),
            MatchStatement::Optional(_) => panic!("expected simple MATCH"),
        }
    }

    #[test]
    fn pattern_info_reports_structure_and_connectivity() {
        let pattern = first_graph_pattern(
            "MATCH (a:Person)-[:KNOWS]->(b:Person), (b)-[:WORKS_AT]->(c:Company) RETURN a, b, c",
        );

        let info = PatternInfo::analyze(&pattern);

        assert_eq!(info.path_count, 2);
        assert_eq!(info.node_count, 4);
        assert_eq!(info.edge_count, 2);
        assert_eq!(info.connected_component_count, 1);
        assert!(info.is_fully_connected);
        assert_eq!(
            info.label_expression_complexity,
            LabelExpressionComplexity::Simple
        );
    }

    #[test]
    fn pattern_info_detects_disconnected_components_and_complex_labels() {
        let pattern = first_graph_pattern("MATCH (a:!(Person|Company)&%), (b:Company) RETURN a, b");

        let info = PatternInfo::analyze(&pattern);

        assert_eq!(info.path_count, 2);
        assert_eq!(info.connected_component_count, 2);
        assert!(!info.is_fully_connected);
        assert_eq!(
            info.label_expression_complexity,
            LabelExpressionComplexity::NestedBoolean
        );
    }

    #[test]
    fn collect_pattern_expression_roots_covers_pattern_conditions() {
        let pattern = first_graph_pattern(
            "MATCH (n:Person {age: 10} WHERE n.age > 5)-[:KNOWS {since: 2020} WHERE n.active = true]->(m) WHERE n.score > 10 RETURN n",
        );

        let mut roots = Vec::new();
        collect_pattern_expression_roots(&pattern, &mut roots);

        // node property + node where + graph-pattern where are always captured.
        assert_eq!(roots.len(), 3);
    }
}
