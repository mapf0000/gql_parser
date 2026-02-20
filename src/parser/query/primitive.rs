//! Primitive query statement parsing.
//!
//! This module handles parsing of primitive query statements including:
//! - USE clauses
//! - MATCH statements (simple and optional)
//! - FILTER statements
//! - LET statements
//! - FOR statements

use crate::ast::query::*;
use crate::ast::references::BindingVariable;
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::patterns::parse_graph_pattern;
use crate::parser::procedure::parse_call_procedure_statement;
use crate::parser::types::parse_value_type_prefix;
use smol_str::SmolStr;

use super::{
    parse_expression_with_diags, is_query_clause_boundary, skip_to_query_clause_boundary,
    ParseResult,
};

// Import functions from sibling modules that are needed by parse_primitive_query_statement
use super::result::parse_select_statement;
use super::pagination::parse_order_by_and_page_statement;

/// Parses a primitive query statement.
pub(crate) fn parse_primitive_query_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<PrimitiveQueryStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match &tokens[*pos].kind {
        TokenKind::Match => {
            let (match_opt, diags) = parse_match_statement(tokens, pos);
            (match_opt.map(PrimitiveQueryStatement::Match), diags)
        }
        TokenKind::Call => {
            let (call_opt, diags) = parse_call_procedure_statement(tokens, pos);
            (call_opt.map(PrimitiveQueryStatement::Call), diags)
        }
        TokenKind::Optional => {
            if matches!(tokens.get(*pos + 1).map(|t| &t.kind), Some(TokenKind::Call)) {
                let (call_opt, diags) = parse_call_procedure_statement(tokens, pos);
                (call_opt.map(PrimitiveQueryStatement::Call), diags)
            } else {
                let (match_opt, diags) = parse_match_statement(tokens, pos);
                (match_opt.map(PrimitiveQueryStatement::Match), diags)
            }
        }
        TokenKind::Filter => {
            let (filter_opt, diags) = parse_filter_statement(tokens, pos);
            (filter_opt.map(PrimitiveQueryStatement::Filter), diags)
        }
        TokenKind::Let => {
            let (let_opt, diags) = parse_let_statement(tokens, pos);
            (let_opt.map(PrimitiveQueryStatement::Let), diags)
        }
        TokenKind::For => {
            let (for_opt, diags) = parse_for_statement(tokens, pos);
            (for_opt.map(PrimitiveQueryStatement::For), diags)
        }
        TokenKind::Order => {
            let (order_opt, diags) = parse_order_by_and_page_statement(tokens, pos);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Limit | TokenKind::Offset | TokenKind::Skip => {
            let (order_opt, diags) = parse_order_by_and_page_statement(tokens, pos);
            (
                order_opt.map(PrimitiveQueryStatement::OrderByAndPage),
                diags,
            )
        }
        TokenKind::Select => {
            let (select_opt, diags) = parse_select_statement(tokens, pos);
            (
                select_opt.map(|select| PrimitiveQueryStatement::Select(Box::new(select))),
                diags,
            )
        }
        TokenKind::With => {
            let (select_opt, diags) = parse_select_statement(tokens, pos);
            (
                select_opt.map(|select| PrimitiveQueryStatement::Select(Box::new(select))),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

// ============================================================================
// USE Clause (Task 23)
// ============================================================================

/// Parses a USE clause.
pub(crate) fn parse_use_graph_clause(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<UseGraphClause> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Use) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse graph expression
    let (graph_opt, mut expr_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expr_diags);

    let graph = match graph_opt {
        Some(g) => g,
        None => {
            diags.push(
                Diag::error("Expected graph expression after USE").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(UseGraphClause {
            graph,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Match Statements (Task 15)
// ============================================================================

/// Parses a MATCH statement (simple or optional).
fn parse_match_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<MatchStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    if matches!(tokens[*pos].kind, TokenKind::Optional) {
        let (opt_match, diags) = parse_optional_match_statement(tokens, pos);
        (opt_match.map(MatchStatement::Optional), diags)
    } else if matches!(tokens[*pos].kind, TokenKind::Match) {
        let (simple_match, diags) = parse_simple_match_statement(tokens, pos);
        (
            simple_match.map(|simple| MatchStatement::Simple(Box::new(simple))),
            diags,
        )
    } else {
        (None, vec![])
    }
}

/// Parses a simple MATCH statement.
fn parse_simple_match_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<SimpleMatchStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Match) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse graph pattern with integration-level progress guarantees.
    let (pattern_opt, mut pattern_diags) = parse_graph_pattern_checked(tokens, pos);
    diags.append(&mut pattern_diags);

    let pattern = match pattern_opt {
        Some(p) => p,
        None => {
            diags.push(
                Diag::error("Expected graph pattern after MATCH").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected pattern here",
                ),
            );
            return (None, diags);
        }
    };

    let end = pattern.span.end;

    (
        Some(SimpleMatchStatement {
            pattern,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an OPTIONAL MATCH statement.
fn parse_optional_match_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<OptionalMatchStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Optional) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse optional operand
    let (operand_opt, mut operand_diags) = parse_optional_operand(tokens, pos);
    diags.append(&mut operand_diags);

    let operand = match operand_opt {
        Some(op) => op,
        None => {
            diags.push(
                Diag::error("Expected MATCH, '{', or '(' after OPTIONAL").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected operand here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(OptionalMatchStatement {
            operand,
            span: start..end,
        }),
        diags,
    )
}

/// Parses an optional operand (MATCH, block, or parenthesized block).
fn parse_optional_operand(tokens: &[Token], pos: &mut usize) -> ParseResult<OptionalOperand> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match &tokens[*pos].kind {
        TokenKind::Match => {
            *pos += 1;
            let (pattern_opt, diags) = parse_graph_pattern_checked(tokens, pos);
            (
                pattern_opt.map(|pattern| OptionalOperand::Match {
                    pattern: Box::new(pattern),
                }),
                diags,
            )
        }
        TokenKind::LBrace => {
            *pos += 1;
            let (statements, mut diags) = parse_match_statement_block(tokens, pos);

            if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RBrace) {
                *pos += 1;
            } else {
                diags.push(
                    Diag::error("Expected '}' to close OPTIONAL block").with_primary_label(
                        tokens.get(*pos).map(|t| t.span.clone()).unwrap_or(0..0),
                        "expected '}' here",
                    ),
                );
            }

            (Some(OptionalOperand::Block { statements }), diags)
        }
        TokenKind::LParen => {
            *pos += 1;
            let (statements, mut diags) = parse_match_statement_block(tokens, pos);

            if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RParen) {
                *pos += 1;
            } else {
                diags.push(
                    Diag::error("Expected ')' to close OPTIONAL parenthesized block")
                        .with_primary_label(
                            tokens.get(*pos).map(|t| t.span.clone()).unwrap_or(0..0),
                            "expected ')' here",
                        ),
                );
            }

            (
                Some(OptionalOperand::ParenthesizedBlock { statements }),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

/// Parses a match statement block (multiple MATCH statements).
fn parse_match_statement_block(
    tokens: &[Token],
    pos: &mut usize,
) -> (Vec<MatchStatement>, Vec<Diag>) {
    let mut statements = Vec::new();
    let mut diags = Vec::new();

    while *pos < tokens.len() {
        if matches!(tokens[*pos].kind, TokenKind::RBrace | TokenKind::RParen) {
            break;
        }

        let (stmt_opt, mut stmt_diags) = parse_match_statement(tokens, pos);
        diags.append(&mut stmt_diags);

        match stmt_opt {
            Some(stmt) => statements.push(stmt),
            None => break,
        }
    }

    (statements, diags)
}

/// Wrapper for graph-pattern parsing with progress and diagnostic guarantees.
fn parse_graph_pattern_checked(tokens: &[Token], pos: &mut usize) -> ParseResult<GraphPattern> {
    let start = *pos;
    let (pattern_opt, mut diags) = parse_graph_pattern(tokens, pos);

    if pattern_opt.is_some() && *pos == start {
        diags.push(
            Diag::error("Graph pattern parser succeeded without consuming input")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "pattern parser stalled here",
                ),
        );
        skip_to_query_clause_boundary(tokens, pos);
        return (None, diags);
    }

    if pattern_opt.is_none() && *pos == start && diags.is_empty() {
        diags.push(
            Diag::error("Expected graph pattern").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected pattern here",
            ),
        );
    }

    (pattern_opt, diags)
}

// ============================================================================
// Filter Statements (Task 16)
// ============================================================================

/// Parses a FILTER statement.
fn parse_filter_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<FilterStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Filter) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Check for optional WHERE keyword
    let where_optional = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Where) {
        *pos += 1;
        true
    } else {
        false
    };

    // Parse condition expression
    let (condition_opt, mut cond_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut cond_diags);

    let condition = match condition_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected search condition after FILTER").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected condition here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(FilterStatement {
            where_optional,
            condition,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// Let Statements (Task 17)
// ============================================================================

/// Parses a LET statement.
fn parse_let_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<LetStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Let) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse variable definitions (comma-separated)
    let mut bindings = Vec::new();

    loop {
        let (binding_opt, mut binding_diags) = parse_let_variable_definition(tokens, pos);
        diags.append(&mut binding_diags);

        match binding_opt {
            Some(binding) => bindings.push(binding),
            None => break,
        }

        // Check for comma (more bindings)
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
        } else {
            break;
        }
    }

    if bindings.is_empty() {
        diags.push(
            Diag::error("Expected variable definition after LET").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected variable definition here",
            ),
        );
        return (None, diags);
    }

    let end = bindings.last().map(|b| b.span.end).unwrap_or(start);

    (
        Some(LetStatement {
            bindings,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a LET variable definition.
fn parse_let_variable_definition(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<LetVariableDefinition> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse variable name
    let variable = match tokens.get(*pos) {
        Some(token) => match &token.kind {
            TokenKind::Identifier(name) => {
                *pos += 1;
                BindingVariable {
                    name: name.clone(),
                    span: token.span.clone(),
                }
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                *pos += 1;
                BindingVariable {
                    name: SmolStr::new(kind.to_string()),
                    span: token.span.clone(),
                }
            }
            _ => {
                diags.push(
                    Diag::error("Expected variable name in LET definition")
                        .with_primary_label(token.span.clone(), "expected identifier here"),
                );
                return (None, diags);
            }
        },
        None => return (None, diags),
    };

    // Parse optional type annotation
    let type_annotation =
        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::DoubleColon) {
            *pos += 1;
            match parse_value_type_prefix(&tokens[*pos..]) {
                Ok((value_type, consumed)) => {
                    *pos += consumed;
                    Some(value_type)
                }
                Err(err) => {
                    diags.push(*err);
                    None
                }
            }
        } else {
            None
        };

    // Expect '='
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Eq) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected '=' in LET variable definition").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected '=' here",
            ),
        );
        return (None, diags);
    }

    // Parse value expression
    let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut value_diags);

    let value = match value_opt {
        Some(v) => v,
        None => {
            diags.push(
                Diag::error("Expected value expression in LET definition").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(LetVariableDefinition {
            variable,
            type_annotation,
            value,
            span: start..end,
        }),
        diags,
    )
}

// ============================================================================
// For Statements (Task 18)
// ============================================================================

/// Parses a FOR statement.
fn parse_for_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<ForStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::For) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    // Parse FOR item
    let (item_opt, mut item_diags) = parse_for_item(tokens, pos);
    diags.append(&mut item_diags);

    let item = match item_opt {
        Some(i) => i,
        None => {
            diags.push(
                Diag::error("Expected FOR item specification").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected 'variable IN expression' here",
                ),
            );
            return (None, diags);
        }
    };

    // Parse optional WITH ORDINALITY/OFFSET
    let ordinality_or_offset = if *pos < tokens.len()
        && matches!(tokens[*pos].kind, TokenKind::With)
    {
        *pos += 1;
        let (ordinality_opt, mut ordinality_diags) = parse_for_ordinality_or_offset(tokens, pos);
        diags.append(&mut ordinality_diags);
        ordinality_opt
    } else {
        None
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(ForStatement {
            item,
            ordinality_or_offset,
            span: start..end,
        }),
        diags,
    )
}

/// Parses a FOR item (variable IN collection).
fn parse_for_item(tokens: &[Token], pos: &mut usize) -> ParseResult<ForItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    // Parse binding variable
    let binding_variable = match tokens.get(*pos) {
        Some(token) => match &token.kind {
            TokenKind::Identifier(name) => {
                *pos += 1;
                BindingVariable {
                    name: name.clone(),
                    span: token.span.clone(),
                }
            }
            kind if kind.is_non_reserved_identifier_keyword() => {
                *pos += 1;
                BindingVariable {
                    name: SmolStr::new(kind.to_string()),
                    span: token.span.clone(),
                }
            }
            _ => {
                diags.push(
                    Diag::error("Expected variable name in FOR statement")
                        .with_primary_label(token.span.clone(), "expected identifier here"),
                );
                return (None, diags);
            }
        },
        None => return (None, diags),
    };

    // Expect IN keyword
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::In) {
        *pos += 1;
    } else {
        diags.push(
            Diag::error("Expected IN keyword in FOR statement").with_primary_label(
                tokens
                    .get(*pos)
                    .map(|t| t.span.clone())
                    .unwrap_or(start..start),
                "expected 'IN' here",
            ),
        );
        return (None, diags);
    }

    // Parse collection expression
    let (collection_opt, mut coll_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut coll_diags);

    let collection = match collection_opt {
        Some(c) => c,
        None => {
            diags.push(
                Diag::error("Expected collection expression in FOR statement").with_primary_label(
                    tokens
                        .get(*pos)
                        .map(|t| t.span.clone())
                        .unwrap_or(start..start),
                    "expected expression here",
                ),
            );
            return (None, diags);
        }
    };

    let end = tokens
        .get(pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(start);

    (
        Some(ForItem {
            binding_variable,
            collection,
            span: start..end,
        }),
        diags,
    )
}

/// Parses WITH ORDINALITY or WITH OFFSET clause.
fn parse_for_ordinality_or_offset(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<ForOrdinalityOrOffset> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map(|t| t.span.start).unwrap_or(0);

    if *pos >= tokens.len() {
        diags.push(
            Diag::error("Expected ORDINALITY or OFFSET after WITH")
                .with_primary_label(start..start, "expected ORDINALITY or OFFSET here"),
        );
        return (None, diags);
    }

    let expects_ordinality = match &tokens[*pos].kind {
        TokenKind::Ordinality => {
            *pos += 1;
            true
        }
        TokenKind::Offset => {
            *pos += 1;
            false
        }
        _ => {
            diags.push(
                Diag::error("Expected ORDINALITY or OFFSET after WITH").with_primary_label(
                    tokens[*pos].span.clone(),
                    "expected ORDINALITY or OFFSET here",
                ),
            );
            skip_to_query_clause_boundary(tokens, pos);
            return (None, diags);
        }
    };

    // AS is optional in this clause.
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::As) {
        *pos += 1;
    }

    let Some(token) = tokens.get(*pos) else {
        let keyword = if expects_ordinality {
            "ORDINALITY"
        } else {
            "OFFSET"
        };
        diags.push(
            Diag::error(format!("Expected variable name after WITH {keyword}"))
                .with_primary_label(start..start, "expected identifier here"),
        );
        return (None, diags);
    };

    let variable = match &token.kind {
        TokenKind::Identifier(name) => {
            let variable = BindingVariable {
                name: name.clone(),
                span: token.span.clone(),
            };
            *pos += 1;
            variable
        }
        kind if kind.is_non_reserved_identifier_keyword() => {
            let variable = BindingVariable {
                name: SmolStr::new(kind.to_string()),
                span: token.span.clone(),
            };
            *pos += 1;
            variable
        }
        _ => {
            let keyword = if expects_ordinality {
                "ORDINALITY"
            } else {
                "OFFSET"
            };
            diags.push(
                Diag::error(format!("Expected variable name after WITH {keyword}"))
                    .with_primary_label(token.span.clone(), "expected identifier here"),
            );
            if !is_query_clause_boundary(&token.kind) {
                skip_to_query_clause_boundary(tokens, pos);
            }
            return (None, diags);
        }
    };

    if expects_ordinality {
        (Some(ForOrdinalityOrOffset::Ordinality { variable }), diags)
    } else {
        (Some(ForOrdinalityOrOffset::Offset { variable }), diags)
    }
}
