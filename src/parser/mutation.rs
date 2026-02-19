//! Mutation statement parsing for ISO GQL data modification.

use crate::ast::Span;
use crate::ast::mutation::*;
use crate::ast::query::{
    ElementPropertySpecification, ElementVariableDeclaration, LabelSetSpecification,
    PrimitiveResultStatement, PropertyKeyValuePair,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::LegacyParseResult;
use crate::parser::procedure::parse_call_procedure_statement;
use crate::parser::query::{
    parse_expression_with_diags, parse_primitive_query_statement, parse_return_statement,
    parse_use_graph_clause,
};
use smol_str::SmolStr;

/// Parse result with optional value and diagnostics.
type ParseResult<T> = LegacyParseResult<T>;

/// Parses a linear data modifying statement starting at the given position.
pub fn parse_linear_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<LinearDataModifyingStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    if matches!(tokens[*pos].kind, TokenKind::Use) {
        let (focused, diags) = parse_focused_linear_data_modifying_statement(tokens, pos);
        return (focused.map(LinearDataModifyingStatement::Focused), diags);
    }

    let (ambient, diags) = parse_ambient_linear_data_modifying_statement(tokens, pos);
    (ambient.map(LinearDataModifyingStatement::Ambient), diags)
}

fn parse_focused_linear_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<FocusedLinearDataModifyingStatement> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (use_graph_clause_opt, mut use_diags) = parse_use_graph_clause(tokens, pos);
    diags.append(&mut use_diags);

    let Some(use_graph_clause) = use_graph_clause_opt else {
        return (None, diags);
    };

    let (statements, primitive_result_statement, mut body_diags, end) =
        parse_linear_data_modifying_body(tokens, pos, start);
    diags.append(&mut body_diags);

    if statements.is_empty() {
        diags.push(
            Diag::error("Expected data-accessing statement after USE clause")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(use_graph_clause.span.clone(), |token| token.span.clone()),
                    "expected statement here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    (
        Some(FocusedLinearDataModifyingStatement {
            use_graph_clause,
            statements,
            primitive_result_statement,
            span: start..end,
        }),
        diags,
    )
}

fn parse_ambient_linear_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<AmbientLinearDataModifyingStatement> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (statements, primitive_result_statement, mut body_diags, end) =
        parse_linear_data_modifying_body(tokens, pos, start);
    diags.append(&mut body_diags);

    if statements.is_empty() {
        diags.push(
            Diag::error("Expected data-modifying statement")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected statement here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    (
        Some(AmbientLinearDataModifyingStatement {
            statements,
            primitive_result_statement,
            span: start..end,
        }),
        diags,
    )
}

fn parse_linear_data_modifying_body(
    tokens: &[Token],
    pos: &mut usize,
    start: usize,
) -> (
    Vec<SimpleDataAccessingStatement>,
    Option<PrimitiveResultStatement>,
    Vec<Diag>,
    usize,
) {
    let mut diags = Vec::new();
    let mut statements = Vec::new();

    loop {
        if *pos >= tokens.len() {
            break;
        }

        if is_result_statement_start(&tokens[*pos].kind)
            || is_linear_mutation_boundary(&tokens[*pos].kind)
        {
            break;
        }

        let before = *pos;
        let (statement_opt, mut statement_diags) =
            parse_simple_data_accessing_statement(tokens, pos);
        diags.append(&mut statement_diags);

        match statement_opt {
            Some(statement) => statements.push(statement),
            None => {
                if *pos == before {
                    break;
                }
            }
        }
    }

    let primitive_result_statement =
        if *pos < tokens.len() && is_result_statement_start(&tokens[*pos].kind) {
            let (result_opt, mut result_diags) = parse_primitive_result_statement(tokens, pos);
            diags.append(&mut result_diags);
            result_opt
        } else {
            None
        };

    let end = end_after_last_consumed(tokens, *pos, start);
    (statements, primitive_result_statement, diags, end)
}

fn parse_simple_data_accessing_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<SimpleDataAccessingStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    // In mutation pipelines, CALL/OPTIONAL CALL is parsed as data-modifying call syntax.
    if matches!(tokens[*pos].kind, TokenKind::Call)
        || (matches!(tokens[*pos].kind, TokenKind::Optional)
            && matches!(
                tokens.get(*pos + 1).map(|token| &token.kind),
                Some(TokenKind::Call)
            ))
    {
        let (modifying_opt, diags) = parse_simple_data_modifying_statement(tokens, pos);
        return (
            modifying_opt.map(SimpleDataAccessingStatement::Modifying),
            diags,
        );
    }

    let checkpoint = *pos;
    let (query_opt, mut query_diags) = parse_primitive_query_statement(tokens, pos);
    if let Some(query_stmt) = query_opt {
        return (
            Some(SimpleDataAccessingStatement::Query(Box::new(query_stmt))),
            query_diags,
        );
    }
    if *pos != checkpoint {
        return (None, query_diags);
    }

    let (modifying_opt, mut modifying_diags) = parse_simple_data_modifying_statement(tokens, pos);
    query_diags.append(&mut modifying_diags);
    (
        modifying_opt.map(SimpleDataAccessingStatement::Modifying),
        query_diags,
    )
}

fn parse_simple_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<SimpleDataModifyingStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match tokens[*pos].kind {
        TokenKind::Insert
        | TokenKind::Set
        | TokenKind::Remove
        | TokenKind::Delete
        | TokenKind::Detach
        | TokenKind::Nodetach => {
            let (primitive_opt, diags) = parse_primitive_data_modifying_statement(tokens, pos);
            (
                primitive_opt.map(SimpleDataModifyingStatement::Primitive),
                diags,
            )
        }
        TokenKind::Call | TokenKind::Optional => {
            let (call_opt, diags) = parse_call_data_modifying_procedure_statement(tokens, pos);
            (
                call_opt.map(|call| SimpleDataModifyingStatement::Call(Box::new(call))),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

fn parse_primitive_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<PrimitiveDataModifyingStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match tokens[*pos].kind {
        TokenKind::Insert => {
            let (statement_opt, diags) = parse_insert_statement(tokens, pos);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Insert),
                diags,
            )
        }
        TokenKind::Set => {
            let (statement_opt, diags) = parse_set_statement(tokens, pos);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Set),
                diags,
            )
        }
        TokenKind::Remove => {
            let (statement_opt, diags) = parse_remove_statement(tokens, pos);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Remove),
                diags,
            )
        }
        TokenKind::Delete | TokenKind::Detach | TokenKind::Nodetach => {
            let (statement_opt, diags) = parse_delete_statement(tokens, pos);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Delete),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

fn parse_insert_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<InsertStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Insert) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let (pattern_opt, mut pattern_diags) = parse_insert_graph_pattern(tokens, pos);
    diags.append(&mut pattern_diags);

    let Some(pattern) = pattern_opt else {
        diags.push(
            Diag::error("Expected insert graph pattern after INSERT")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected insert pattern here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    };

    (
        Some(InsertStatement {
            span: start..pattern.span.end,
            pattern,
        }),
        diags,
    )
}

fn parse_insert_graph_pattern(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<InsertGraphPattern> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (first_opt, mut first_diags) = parse_insert_path_pattern(tokens, pos);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut paths = vec![first];

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
        *pos += 1;

        let (path_opt, mut path_diags) = parse_insert_path_pattern(tokens, pos);
        diags.append(&mut path_diags);

        let Some(path) = path_opt else {
            diags.push(
                Diag::error("Expected insert path pattern after ','")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "missing insert path pattern",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        paths.push(path);
    }

    let end = paths.last().map_or(start, |path| path.span.end);
    (
        Some(InsertGraphPattern {
            paths,
            span: start..end,
        }),
        diags,
    )
}

fn parse_insert_path_pattern(tokens: &[Token], pos: &mut usize) -> ParseResult<InsertPathPattern> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (node_opt, mut node_diags) = parse_insert_node_pattern(tokens, pos);
    diags.append(&mut node_diags);

    let Some(node) = node_opt else {
        return (None, diags);
    };

    let mut elements = vec![InsertElementPattern::Node(node)];

    loop {
        let before = *pos;
        let (edge_opt, mut edge_diags) = parse_insert_edge_pattern(tokens, pos);
        diags.append(&mut edge_diags);

        let Some(edge) = edge_opt else {
            if *pos == before {
                break;
            }
            continue;
        };

        elements.push(InsertElementPattern::Edge(edge));

        let (next_node_opt, mut next_node_diags) = parse_insert_node_pattern(tokens, pos);
        diags.append(&mut next_node_diags);

        let Some(next_node) = next_node_opt else {
            diags.push(
                Diag::error("Expected node pattern after insert edge")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected node pattern here",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        elements.push(InsertElementPattern::Node(next_node));
    }

    let end = elements.last().map_or(start, |pattern| pattern.span().end);
    (
        Some(InsertPathPattern {
            elements,
            span: start..end,
        }),
        diags,
    )
}

fn parse_insert_node_pattern(tokens: &[Token], pos: &mut usize) -> ParseResult<InsertNodePattern> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::LParen) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let filler = if *pos < tokens.len() && !matches!(tokens[*pos].kind, TokenKind::RParen) {
        let (filler_opt, mut filler_diags) = parse_insert_element_pattern_filler(tokens, pos);
        diags.append(&mut filler_diags);
        filler_opt
    } else {
        None
    };

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::RParen) {
        diags.push(
            Diag::error("Expected ')' to close insert node pattern")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected ')' here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    let end = tokens[*pos].span.end;
    *pos += 1;

    (
        Some(InsertNodePattern {
            filler,
            span: start..end,
        }),
        diags,
    )
}

fn parse_insert_edge_pattern(tokens: &[Token], pos: &mut usize) -> ParseResult<InsertEdgePattern> {
    if *pos + 1 >= tokens.len() {
        return (None, vec![]);
    }

    let mut diags = Vec::new();
    let start = tokens[*pos].span.start;

    if matches!(tokens[*pos].kind, TokenKind::LeftArrow)
        && matches!(tokens[*pos + 1].kind, TokenKind::LBracket)
    {
        *pos += 2;
        let (filler, mut filler_diags) = parse_optional_insert_edge_filler(tokens, pos, start);
        diags.append(&mut filler_diags);

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::RBracket) {
            diags.push(
                Diag::error("Expected ']' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected ']' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }
        *pos += 1;

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Minus) {
            diags.push(
                Diag::error("Expected '-' after '<-[...]' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected '-' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }

        let end = tokens[*pos].span.end;
        *pos += 1;

        return (
            Some(InsertEdgePattern::PointingLeft(InsertEdgePointingLeft {
                filler,
                span: start..end,
            })),
            diags,
        );
    }

    if matches!(tokens[*pos].kind, TokenKind::Minus)
        && matches!(tokens[*pos + 1].kind, TokenKind::LBracket)
    {
        *pos += 2;
        let (filler, mut filler_diags) = parse_optional_insert_edge_filler(tokens, pos, start);
        diags.append(&mut filler_diags);

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::RBracket) {
            diags.push(
                Diag::error("Expected ']' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected ']' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }
        *pos += 1;

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Arrow) {
            diags.push(
                Diag::error("Expected '->' after '-[...]' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected '->' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }

        let end = tokens[*pos].span.end;
        *pos += 1;

        return (
            Some(InsertEdgePattern::PointingRight(InsertEdgePointingRight {
                filler,
                span: start..end,
            })),
            diags,
        );
    }

    if matches!(tokens[*pos].kind, TokenKind::Tilde)
        && matches!(tokens[*pos + 1].kind, TokenKind::LBracket)
    {
        *pos += 2;
        let (filler, mut filler_diags) = parse_optional_insert_edge_filler(tokens, pos, start);
        diags.append(&mut filler_diags);

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::RBracket) {
            diags.push(
                Diag::error("Expected ']' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected ']' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }
        *pos += 1;

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Tilde) {
            diags.push(
                Diag::error("Expected '~' after '~[...]' in insert edge pattern")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "expected '~' here",
                    )
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }

        let end = tokens[*pos].span.end;
        *pos += 1;

        return (
            Some(InsertEdgePattern::Undirected(InsertEdgeUndirected {
                filler,
                span: start..end,
            })),
            diags,
        );
    }

    (None, diags)
}

fn parse_optional_insert_edge_filler(
    tokens: &[Token],
    pos: &mut usize,
    start: usize,
) -> (Option<InsertElementPatternFiller>, Vec<Diag>) {
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RBracket) {
        return (None, vec![]);
    }

    let (filler_opt, mut diags) = parse_insert_element_pattern_filler(tokens, pos);
    if filler_opt.is_none() {
        diags.push(
            Diag::error("Expected insert edge filler")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected edge filler here",
                )
                .with_code("P_MUT"),
        );
    }

    (filler_opt, diags)
}

fn parse_insert_element_pattern_filler(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<InsertElementPatternFiller> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);
    let mut consumed_any = false;

    let variable = parse_element_variable_declaration_opt(tokens, pos);
    if variable.is_some() {
        consumed_any = true;
    }

    let mut label_set = None;
    let mut use_is_keyword = false;
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Is | TokenKind::Colon) {
        use_is_keyword = matches!(tokens[*pos].kind, TokenKind::Is);
        *pos += 1;

        let (label_set_opt, mut label_diags) = parse_label_set_specification(tokens, pos);
        diags.append(&mut label_diags);

        if label_set_opt.is_some() {
            consumed_any = true;
        }
        label_set = label_set_opt;
    }

    let mut properties = None;
    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::LBrace) {
        let (properties_opt, mut prop_diags) =
            parse_element_property_specification(tokens, pos, false);
        diags.append(&mut prop_diags);
        if properties_opt.is_some() {
            consumed_any = true;
        }
        properties = properties_opt;
    }

    if !consumed_any {
        return (None, diags);
    }

    let end = end_after_last_consumed(tokens, *pos, start);
    (
        Some(InsertElementPatternFiller {
            variable,
            label_set,
            use_is_keyword,
            properties,
            span: start..end,
        }),
        diags,
    )
}

fn parse_set_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<SetStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Set) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let (items_opt, mut item_diags) = parse_set_item_list(tokens, pos);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected SET item list")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected set item here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    };

    (
        Some(SetStatement {
            span: start..items.span.end,
            items,
        }),
        diags,
    )
}

fn parse_set_item_list(tokens: &[Token], pos: &mut usize) -> ParseResult<SetItemList> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (first_opt, mut first_diags) = parse_set_item(tokens, pos);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
        *pos += 1;
        let (item_opt, mut item_diags) = parse_set_item(tokens, pos);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected SET item after ','")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "missing SET item",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        items.push(item);
    }

    let end = items.last().map_or(start, |item| item.span().end);
    (
        Some(SetItemList {
            items,
            span: start..end,
        }),
        diags,
    )
}

fn parse_set_item(tokens: &[Token], pos: &mut usize) -> ParseResult<SetItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let Some((element, element_span)) = parse_regular_identifier(tokens, pos) else {
        return (None, diags);
    };

    if *pos >= tokens.len() {
        diags.push(
            Diag::error("Expected '.', '=', or label assignment in SET item")
                .with_primary_label(element_span, "incomplete SET item")
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    match tokens[*pos].kind {
        TokenKind::Dot => {
            *pos += 1;
            let Some((property, property_span)) = parse_identifier(tokens, pos) else {
                diags.push(
                    Diag::error("Expected property name after '.' in SET item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected property name",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Eq) {
                diags.push(
                    Diag::error("Expected '=' in SET property item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(property_span, |token| token.span.clone()),
                            "expected '=' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }
            *pos += 1;

            let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, pos);
            diags.append(&mut value_diags);

            let Some(value) = value_opt else {
                diags.push(
                    Diag::error("Expected value expression in SET property item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected expression here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            let end = value.span().end;
            (
                Some(SetItem::Property(SetPropertyItem {
                    element,
                    property,
                    value,
                    span: start..end,
                })),
                diags,
            )
        }
        TokenKind::Eq => {
            *pos += 1;
            let (properties_opt, mut properties_diags) =
                parse_element_property_specification(tokens, pos, true);
            diags.append(&mut properties_diags);

            let Some(properties) = properties_opt else {
                diags.push(
                    Diag::error("Expected record literal in SET all-properties item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected '{...}' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            (
                Some(SetItem::AllProperties(SetAllPropertiesItem {
                    element,
                    span: start..properties.span.end,
                    properties,
                })),
                diags,
            )
        }
        TokenKind::Is | TokenKind::Colon => {
            let use_is_keyword = matches!(tokens[*pos].kind, TokenKind::Is);
            *pos += 1;

            let Some((label, label_span)) = parse_identifier(tokens, pos) else {
                diags.push(
                    Diag::error("Expected label name in SET label item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected label name",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            (
                Some(SetItem::Label(SetLabelItem {
                    element,
                    label,
                    use_is_keyword,
                    span: start..label_span.end,
                })),
                diags,
            )
        }
        _ => {
            diags.push(
                Diag::error("Expected '.', '=', or label assignment in SET item")
                    .with_primary_label(tokens[*pos].span.clone(), "expected '.', '=', IS, or ':'")
                    .with_code("P_MUT"),
            );
            (None, diags)
        }
    }
}

fn parse_remove_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<RemoveStatement> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Remove) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let (items_opt, mut item_diags) = parse_remove_item_list(tokens, pos);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected REMOVE item list")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected remove item here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    };

    (
        Some(RemoveStatement {
            span: start..items.span.end,
            items,
        }),
        diags,
    )
}

fn parse_remove_item_list(tokens: &[Token], pos: &mut usize) -> ParseResult<RemoveItemList> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (first_opt, mut first_diags) = parse_remove_item(tokens, pos);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
        *pos += 1;

        let (item_opt, mut item_diags) = parse_remove_item(tokens, pos);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected REMOVE item after ','")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "missing REMOVE item",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        items.push(item);
    }

    let end = items.last().map_or(start, |item| item.span().end);
    (
        Some(RemoveItemList {
            items,
            span: start..end,
        }),
        diags,
    )
}

fn parse_remove_item(tokens: &[Token], pos: &mut usize) -> ParseResult<RemoveItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let Some((element, _)) = parse_regular_identifier(tokens, pos) else {
        return (None, diags);
    };

    if *pos >= tokens.len() {
        diags.push(
            Diag::error("Expected property or label removal after element")
                .with_primary_label(start..start, "incomplete REMOVE item")
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    match tokens[*pos].kind {
        TokenKind::Dot => {
            *pos += 1;
            let Some((property, property_span)) = parse_identifier(tokens, pos) else {
                diags.push(
                    Diag::error("Expected property name in REMOVE property item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected property name",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            (
                Some(RemoveItem::Property(RemovePropertyItem {
                    element,
                    property,
                    span: start..property_span.end,
                })),
                diags,
            )
        }
        TokenKind::Is | TokenKind::Colon => {
            let use_is_keyword = matches!(tokens[*pos].kind, TokenKind::Is);
            *pos += 1;

            let Some((label, label_span)) = parse_identifier(tokens, pos) else {
                diags.push(
                    Diag::error("Expected label name in REMOVE label item")
                        .with_primary_label(
                            tokens
                                .get(*pos)
                                .map_or(start..start, |token| token.span.clone()),
                            "expected label name",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            (
                Some(RemoveItem::Label(RemoveLabelItem {
                    element,
                    label,
                    use_is_keyword,
                    span: start..label_span.end,
                })),
                diags,
            )
        }
        _ => {
            diags.push(
                Diag::error("Expected '.' or label assignment in REMOVE item")
                    .with_primary_label(tokens[*pos].span.clone(), "expected '.', IS, or ':'")
                    .with_code("P_MUT"),
            );
            (None, diags)
        }
    }
}

fn parse_delete_statement(tokens: &[Token], pos: &mut usize) -> ParseResult<DeleteStatement> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let detach_option = if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Detach) {
        *pos += 1;
        DetachOption::Detach
    } else if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Nodetach) {
        *pos += 1;
        DetachOption::NoDetach
    } else {
        DetachOption::Default
    };

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Delete) {
        diags.push(
            Diag::error("Expected DELETE keyword")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected DELETE here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }
    *pos += 1;

    let (items_opt, mut item_diags) = parse_delete_item_list(tokens, pos);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected DELETE item list")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected delete item here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    };

    (
        Some(DeleteStatement {
            detach_option,
            span: start..items.span.end,
            items,
        }),
        diags,
    )
}

fn parse_delete_item_list(tokens: &[Token], pos: &mut usize) -> ParseResult<DeleteItemList> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (first_opt, mut first_diags) = parse_delete_item(tokens, pos);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
        *pos += 1;

        let (item_opt, mut item_diags) = parse_delete_item(tokens, pos);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected DELETE item after ','")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "missing DELETE item",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        items.push(item);
    }

    let end = items.last().map_or(start, |item| item.span.end);
    (
        Some(DeleteItemList {
            items,
            span: start..end,
        }),
        diags,
    )
}

fn parse_delete_item(tokens: &[Token], pos: &mut usize) -> ParseResult<DeleteItem> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let (expression_opt, mut expression_diags) = parse_expression_with_diags(tokens, pos);
    diags.append(&mut expression_diags);

    let Some(expression) = expression_opt else {
        return (None, diags);
    };

    let end = expression.span().end;
    (
        Some(DeleteItem {
            expression,
            span: start..end,
        }),
        diags,
    )
}

fn parse_call_data_modifying_procedure_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<CallDataModifyingProcedureStatement> {
    let (call_opt, diags) = parse_call_procedure_statement(tokens, pos);
    let Some(call) = call_opt else {
        return (None, diags);
    };

    (
        Some(CallDataModifyingProcedureStatement {
            span: call.span.clone(),
            call,
        }),
        diags,
    )
}

fn parse_primitive_result_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<PrimitiveResultStatement> {
    if *pos >= tokens.len() {
        return (None, vec![]);
    }

    match tokens[*pos].kind {
        TokenKind::Return => {
            let (return_opt, diags) = parse_return_statement(tokens, pos);
            (return_opt.map(PrimitiveResultStatement::Return), diags)
        }
        TokenKind::Finish => {
            let span = tokens[*pos].span.clone();
            *pos += 1;
            (Some(PrimitiveResultStatement::Finish(span)), vec![])
        }
        _ => (None, vec![]),
    }
}

fn parse_label_set_specification(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<LabelSetSpecification> {
    let mut diags = Vec::new();
    let start = tokens.get(*pos).map_or(0, |token| token.span.start);

    let Some((first, first_span)) = parse_identifier(tokens, pos) else {
        return (None, diags);
    };

    let mut labels = vec![first];
    let mut end = first_span.end;

    while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Ampersand) {
        *pos += 1;

        let Some((next, next_span)) = parse_identifier(tokens, pos) else {
            diags.push(
                Diag::error("Expected label name after '&'")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(start..start, |token| token.span.clone()),
                        "missing label name",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        labels.push(next);
        end = next_span.end;
    }

    (
        Some(LabelSetSpecification {
            labels,
            span: start..end,
        }),
        diags,
    )
}

fn parse_element_property_specification(
    tokens: &[Token],
    pos: &mut usize,
    allow_empty: bool,
) -> ParseResult<ElementPropertySpecification> {
    let mut diags = Vec::new();

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::LBrace) {
        return (None, diags);
    }

    let start = tokens[*pos].span.start;
    *pos += 1;

    let mut properties = Vec::new();

    if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::RBrace) {
        let end = tokens[*pos].span.end;
        *pos += 1;
        if !allow_empty {
            diags.push(
                Diag::error("Expected at least one property key-value pair")
                    .with_primary_label(start..end, "empty property map is not allowed here")
                    .with_code("P_MUT"),
            );
            return (None, diags);
        }

        return (
            Some(ElementPropertySpecification {
                properties,
                span: start..end,
            }),
            diags,
        );
    }

    loop {
        let pair_start = tokens.get(*pos).map_or(start, |token| token.span.start);

        let Some((key, key_span)) = parse_identifier(tokens, pos) else {
            diags.push(
                Diag::error("Expected property name")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(pair_start..pair_start, |token| token.span.clone()),
                        "expected property name",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::Colon) {
            diags.push(
                Diag::error("Expected ':' after property name")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(key_span, |token| token.span.clone()),
                        "expected ':' here",
                    )
                    .with_code("P_MUT"),
            );
            break;
        }
        *pos += 1;

        let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, pos);
        diags.append(&mut value_diags);

        let Some(value) = value_opt else {
            diags.push(
                Diag::error("Expected property value expression")
                    .with_primary_label(
                        tokens
                            .get(*pos)
                            .map_or(pair_start..pair_start, |token| token.span.clone()),
                        "expected value expression",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        let value_span = value.span().clone();
        properties.push(PropertyKeyValuePair {
            key,
            value,
            span: pair_start..value_span.end,
        });

        if *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Comma) {
            *pos += 1;
            continue;
        }

        break;
    }

    if *pos >= tokens.len() || !matches!(tokens[*pos].kind, TokenKind::RBrace) {
        diags.push(
            Diag::error("Expected '}' to close property map")
                .with_primary_label(
                    tokens
                        .get(*pos)
                        .map_or(start..start, |token| token.span.clone()),
                    "expected '}' here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    let end = tokens[*pos].span.end;
    *pos += 1;

    if properties.is_empty() && !allow_empty {
        diags.push(
            Diag::error("Expected at least one property key-value pair")
                .with_primary_label(start..end, "empty property map is not allowed here")
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    (
        Some(ElementPropertySpecification {
            properties,
            span: start..end,
        }),
        diags,
    )
}

fn parse_element_variable_declaration_opt(
    tokens: &[Token],
    pos: &mut usize,
) -> Option<ElementVariableDeclaration> {
    let (variable, span) = parse_regular_identifier(tokens, pos)?;
    Some(ElementVariableDeclaration { variable, span })
}

fn parse_regular_identifier(tokens: &[Token], pos: &mut usize) -> Option<(SmolStr, Span)> {
    let token = tokens.get(*pos)?;
    let name = match &token.kind {
        TokenKind::Identifier(name) => name.clone(),
        kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
        _ => return None,
    };
    let span = token.span.clone();
    *pos += 1;
    Some((name, span))
}

fn parse_identifier(tokens: &[Token], pos: &mut usize) -> Option<(SmolStr, Span)> {
    let token = tokens.get(*pos)?;
    let name = match &token.kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => name.clone(),
        kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
        _ => return None,
    };
    let span = token.span.clone();
    *pos += 1;
    Some((name, span))
}

fn end_after_last_consumed(tokens: &[Token], pos: usize, fallback: usize) -> usize {
    if pos > 0 {
        tokens.get(pos - 1).map_or(fallback, |token| token.span.end)
    } else {
        fallback
    }
}

fn is_result_statement_start(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Return | TokenKind::Finish)
}

fn is_linear_mutation_boundary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::Session
            | TokenKind::Start
            | TokenKind::Commit
            | TokenKind::Rollback
            | TokenKind::Create
            | TokenKind::Drop
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_source(source: &str) -> ParseResult<LinearDataModifyingStatement> {
        let lex = tokenize(source);
        let mut pos = 0usize;
        parse_linear_data_modifying_statement(&lex.tokens, &mut pos)
    }

    #[test]
    fn parses_insert_set_remove_delete_chain() {
        let (statement_opt, diags) =
            parse_source("INSERT (n:Person {name: 'Alice'}) SET n.age = 30 REMOVE n:Old DELETE n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        assert_eq!(stmt.statements.len(), 4);
    }

    #[test]
    fn parses_focused_use_graph_mutation_with_return() {
        let (statement_opt, diags) = parse_source("USE myGraph INSERT (n) RETURN n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Focused(stmt)) = statement_opt else {
            panic!("expected focused statement");
        };

        assert_eq!(stmt.statements.len(), 1);
        assert!(matches!(
            stmt.primitive_result_statement,
            Some(PrimitiveResultStatement::Return(_))
        ));
    }

    #[test]
    fn parses_delete_with_expression_items() {
        let (statement_opt, diags) = parse_source("DELETE n, m.age, n.age + 1");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Primitive(
            PrimitiveDataModifyingStatement::Delete(delete),
        ))) = stmt.statements.first()
        else {
            panic!("expected DELETE statement");
        };

        assert_eq!(delete.items.items.len(), 3);
    }

    #[test]
    fn parses_set_all_properties_with_empty_map() {
        let (statement_opt, diags) = parse_source("SET n = {} ");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Primitive(
            PrimitiveDataModifyingStatement::Set(set_stmt),
        ))) = stmt.statements.first()
        else {
            panic!("expected SET statement");
        };

        let Some(SetItem::AllProperties(item)) = set_stmt.items.items.first() else {
            panic!("expected SET all-properties item");
        };
        assert!(item.properties.properties.is_empty());
    }

    #[test]
    fn parses_optional_call_statement() {
        let (statement_opt, diags) = parse_source("OPTIONAL CALL myProc(1, 2) YIELD x");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(call))) =
            stmt.statements.first()
        else {
            panic!("expected CALL statement");
        };

        assert!(call.call.optional);
        let crate::ast::ProcedureCall::Named(named) = &call.call.call else {
            panic!("expected named call");
        };
        assert_eq!(
            named
                .arguments
                .as_ref()
                .map_or(0, |args| args.arguments.len()),
            2
        );
        assert_eq!(
            named
                .yield_clause
                .as_ref()
                .map_or(0, |yield_clause| yield_clause.items.items.len()),
            1
        );
    }

    #[test]
    fn parses_inline_call_statement() {
        let (statement_opt, diags) = parse_source("INSERT (n) CALL { RETURN n }");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(call))) =
            stmt.statements.get(1)
        else {
            panic!("expected inline CALL statement");
        };

        assert!(!call.call.optional);
        assert!(matches!(
            call.call.call,
            crate::ast::ProcedureCall::Inline(_)
        ));
    }

    #[test]
    fn parses_inline_call_with_variable_scope() {
        let (statement_opt, diags) = parse_source("INSERT (n) CALL (n, m) { RETURN n }");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(LinearDataModifyingStatement::Ambient(stmt)) = statement_opt else {
            panic!("expected ambient statement");
        };

        let Some(SimpleDataAccessingStatement::Modifying(SimpleDataModifyingStatement::Call(call))) =
            stmt.statements.get(1)
        else {
            panic!("expected inline CALL statement");
        };

        let crate::ast::ProcedureCall::Inline(inline) = &call.call.call else {
            panic!("expected inline call");
        };
        let scope = inline
            .variable_scope
            .as_ref()
            .expect("expected inline variable scope");
        assert_eq!(scope.variables.len(), 2);
        assert_eq!(scope.variables[0].name.as_str(), "n");
        assert_eq!(scope.variables[1].name.as_str(), "m");
    }
}
