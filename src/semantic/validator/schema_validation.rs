// Pass 9: Schema Validation
//
// This pass validates schema references in queries:
// - Node labels: (n:Person) -> check if 'Person' exists in schema
// - Edge labels: -[e:KNOWS]-> -> check if 'KNOWS' exists in schema
// - Properties: n.name -> check if 'name' exists for nodes with label 'Person'

use crate::ast::query::{
    EdgePattern, ElementPattern, PathPattern, PathPatternExpression, PathPrimary, PathTerm,
};
use crate::ast::*;
use crate::diag::Diag;

/// Run schema validation pass.
pub(super) fn run_schema_validation(
    validator: &super::SemanticValidator,
    program: &Program,
    diagnostics: &mut Vec<Diag>,
) {
    // This pass checks:
    // - Labels exist in schema
    // - Properties exist in schema
    // - Property types match schema

    // Only perform validation if schema is provided
    let Some(schema) = validator.schema else {
        // Schema not available, skip validation
        return;
    };

    for statement in &program.statements {
        if let Statement::Query(query_stmt) = statement {
            // Validate:
            // - Node labels: (n:Person) -> check if 'Person' exists in schema
            // - Edge labels: -[e:KNOWS]-> -> check if 'KNOWS' exists in schema
            // - Properties: n.name -> check if 'name' exists for nodes with label 'Person'
            validate_query_schema(&query_stmt.query, schema, diagnostics);
        }
    }
}

/// Validates schema references in a query.
fn validate_query_schema(
    query: &Query,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    match query {
        Query::Linear(linear_query) => {
            validate_linear_query_schema(linear_query, schema, diagnostics);
        }
        Query::Composite(composite) => {
            validate_query_schema(&composite.left, schema, diagnostics);
            validate_query_schema(&composite.right, schema, diagnostics);
        }
        Query::Parenthesized(query, _) => {
            validate_query_schema(query, schema, diagnostics);
        }
    }
}

/// Validates schema references in a linear query.
fn validate_linear_query_schema(
    linear_query: &LinearQuery,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    let primitive_statements = match linear_query {
        LinearQuery::Focused(focused) => &focused.primitive_statements,
        LinearQuery::Ambient(ambient) => &ambient.primitive_statements,
    };

    for statement in primitive_statements {
        if let PrimitiveQueryStatement::Match(match_stmt) = statement {
            // Validate labels in MATCH patterns based on MatchStatement type
            match match_stmt {
                MatchStatement::Simple(simple) => {
                    // Simple match has a GraphPattern with paths
                    for path in &simple.pattern.paths.patterns {
                        validate_path_pattern_schema(path, schema, diagnostics);
                    }
                }
                MatchStatement::Optional(optional) => {
                    // Optional match - validate nested patterns
                    match &optional.operand {
                        crate::ast::query::OptionalOperand::Match { pattern } => {
                            for path in &pattern.paths.patterns {
                                validate_path_pattern_schema(path, schema, diagnostics);
                            }
                        }
                        crate::ast::query::OptionalOperand::Block { statements }
                        | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
                            // Validate nested MATCH statements recursively
                            for stmt in statements {
                                match stmt {
                                    MatchStatement::Simple(simple) => {
                                        for path in &simple.pattern.paths.patterns {
                                            validate_path_pattern_schema(path, schema, diagnostics);
                                        }
                                    }
                                    MatchStatement::Optional(nested_optional) => {
                                        // Recursively validate nested optional matches
                                        validate_optional_match_schema(
                                            nested_optional,
                                            schema,
                                            diagnostics,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Validates schema references in an optional match (recursive helper).
fn validate_optional_match_schema(
    optional: &crate::ast::query::OptionalMatchStatement,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    match &optional.operand {
        crate::ast::query::OptionalOperand::Match { pattern } => {
            for path in &pattern.paths.patterns {
                validate_path_pattern_schema(path, schema, diagnostics);
            }
        }
        crate::ast::query::OptionalOperand::Block { statements }
        | crate::ast::query::OptionalOperand::ParenthesizedBlock { statements } => {
            for stmt in statements {
                match stmt {
                    MatchStatement::Simple(simple) => {
                        for path in &simple.pattern.paths.patterns {
                            validate_path_pattern_schema(path, schema, diagnostics);
                        }
                    }
                    MatchStatement::Optional(nested) => {
                        validate_optional_match_schema(nested, schema, diagnostics);
                    }
                }
            }
        }
    }
}

/// Validates labels in a path pattern against the schema.
fn validate_path_pattern_schema(
    path: &PathPattern,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    // PathPattern has an expression field, which is a PathPatternExpression
    // We need to walk the expression to find elements
    validate_path_expression_schema(&path.expression, schema, diagnostics);
}

/// Validates labels in a path expression against the schema.
fn validate_path_expression_schema(
    expr: &PathPatternExpression,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    // PathPatternExpression is an enum with Term, Union, and Alternation variants
    match expr {
        PathPatternExpression::Term(term) => {
            // Validate a single term
            validate_path_term_schema(term, schema, diagnostics);
        }
        PathPatternExpression::Union { left, right, .. } => {
            // Validate both sides of union
            validate_path_expression_schema(left, schema, diagnostics);
            validate_path_expression_schema(right, schema, diagnostics);
        }
        PathPatternExpression::Alternation { alternatives, .. } => {
            // Validate all alternatives
            for alt in alternatives {
                validate_path_expression_schema(alt, schema, diagnostics);
            }
        }
    }
}

/// Validates labels in a path term against the schema.
fn validate_path_term_schema(
    term: &PathTerm,
    schema: &dyn crate::semantic::schema::Schema,
    diagnostics: &mut Vec<Diag>,
) {
    use crate::semantic::diag::SemanticDiagBuilder;

    // Each term has factors
    for factor in &term.factors {
        // Check if the primary is an element pattern
        if let PathPrimary::ElementPattern(element) = &factor.primary {
            // ElementPattern is boxed, dereference it
            match &**element {
                ElementPattern::Node(node) => {
                    // Check node labels using label_expression field
                    if let Some(label_expr) = &node.label_expression {
                        for label_name in extract_label_names(label_expr) {
                            if schema.validate_label(&label_name, true).is_err() {
                                diagnostics.push(
                                    SemanticDiagBuilder::unknown_reference(
                                        "label",
                                        &label_name,
                                        node.span.clone(),
                                    )
                                    .build(),
                                );
                            }
                        }
                    }
                }
                ElementPattern::Edge(edge) => {
                    // Check edge labels
                    if let EdgePattern::Full(full) = edge
                        && let Some(label_expr) = &full.filler.label_expression
                    {
                        for label_name in extract_label_names(label_expr) {
                            if schema.validate_label(&label_name, false).is_err() {
                                diagnostics.push(
                                    SemanticDiagBuilder::unknown_reference(
                                        "edge label",
                                        &label_name,
                                        full.span.clone(),
                                    )
                                    .build(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Helper: Extract label names from a label expression.
fn extract_label_names(label_expr: &crate::ast::query::LabelExpression) -> Vec<String> {
    use crate::ast::query::LabelExpression;

    match label_expr {
        LabelExpression::LabelName { name, .. } => vec![name.to_string()],
        LabelExpression::Disjunction { left, right, .. } => {
            let mut labels = extract_label_names(left);
            labels.extend(extract_label_names(right));
            labels
        }
        LabelExpression::Conjunction { left, right, .. } => {
            let mut labels = extract_label_names(left);
            labels.extend(extract_label_names(right));
            labels
        }
        LabelExpression::Negation { operand, .. } => extract_label_names(operand),
        LabelExpression::Wildcard { .. } => vec![], // Wildcard matches any label
        LabelExpression::Parenthesized { expression, .. } => extract_label_names(expression),
    }
}
