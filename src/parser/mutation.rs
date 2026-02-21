//! Mutation statement parsing for ISO GQL data modification.

use crate::ast::Span;
use crate::ast::mutation::*;
use crate::ast::query::{
    ElementPropertySpecification, ElementVariableDeclaration, LabelSetSpecification,
    PrimitiveResultStatement, PropertyKeyValuePair,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::InternalParseResult;
use crate::parser::base::TokenStream;
use crate::parser::procedure::parse_call_procedure_statement;
use crate::parser::query::{
    parse_expression_with_diags, parse_primitive_query_statement, parse_return_statement,
    parse_use_graph_clause,
};
use smol_str::SmolStr;

/// Parse result with optional value and diagnostics.
type ParseResult<T> = InternalParseResult<T>;

/// Parses a linear data modifying statement starting at the given position.
pub fn parse_linear_data_modifying_statement(
    tokens: &[Token],
    pos: &mut usize,
) -> ParseResult<LinearDataModifyingStatement> {
    let mut stream = TokenStream::new(tokens);
    stream.set_position(*pos);

    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Check for optional USE clause
    let use_graph_clause = if stream.check(&TokenKind::Use) {
        let (use_graph_clause_opt, mut use_diags) = parse_use_graph_clause(&mut stream);
        diags.append(&mut use_diags);
        use_graph_clause_opt
    } else {
        None
    };

    let (statements, primitive_result_statement, mut body_diags, end) =
        parse_linear_data_modifying_body(&mut stream, start);
    diags.append(&mut body_diags);

    if statements.is_empty() {
        let error_msg = if use_graph_clause.is_some() {
            "Expected data-accessing statement after USE clause"
        } else {
            "Expected data-modifying statement"
        };
        diags.push(
            Diag::error(error_msg)
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected statement here",
                )
                .with_code("P_MUT"),
        );
        *pos = stream.position();
        return (None, diags);
    }

    *pos = stream.position();
    (
        Some(LinearDataModifyingStatement {
            use_graph_clause,
            statements,
            primitive_result_statement,
            span: start..end,
        }),
        diags,
    )
}

fn parse_linear_data_modifying_body(
    stream: &mut TokenStream,
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
        if stream.check(&TokenKind::Eof) {
            break;
        }

        if is_result_statement_start(&stream.current().kind)
            || is_linear_mutation_boundary(&stream.current().kind)
        {
            break;
        }

        let before = stream.position();
        let (statement_opt, mut statement_diags) =
            parse_simple_data_accessing_statement(stream);
        diags.append(&mut statement_diags);

        match statement_opt {
            Some(statement) => statements.push(statement),
            None => {
                if stream.position() == before {
                    break;
                }
            }
        }
    }

    let primitive_result_statement =
        if !stream.check(&TokenKind::Eof) && is_result_statement_start(&stream.current().kind) {
            let (result_opt, mut result_diags) = parse_primitive_result_statement(stream);
            diags.append(&mut result_diags);
            result_opt
        } else {
            None
        };

    let end = end_after_last_consumed(stream, start);
    (statements, primitive_result_statement, diags, end)
}

fn parse_simple_data_accessing_statement(
    stream: &mut TokenStream,
) -> ParseResult<SimpleDataAccessingStatement> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    // In mutation pipelines, CALL/OPTIONAL CALL is parsed as data-modifying call syntax.
    let tokens = stream.tokens();
    let pos = stream.position();
    if stream.check(&TokenKind::Call)
        || (stream.check(&TokenKind::Optional)
            && pos + 1 < tokens.len()
            && matches!(tokens[pos + 1].kind, TokenKind::Call))
    {
        let (modifying_opt, diags) = parse_simple_data_modifying_statement(stream);
        return (
            modifying_opt.map(SimpleDataAccessingStatement::Modifying),
            diags,
        );
    }

    let checkpoint = stream.position();
    let (query_opt, mut query_diags) = parse_primitive_query_statement(stream);
    if let Some(query_stmt) = query_opt {
        return (
            Some(SimpleDataAccessingStatement::Query(Box::new(query_stmt))),
            query_diags,
        );
    }
    if stream.position() != checkpoint {
        return (None, query_diags);
    }

    let (modifying_opt, mut modifying_diags) = parse_simple_data_modifying_statement(stream);
    query_diags.append(&mut modifying_diags);
    (
        modifying_opt.map(SimpleDataAccessingStatement::Modifying),
        query_diags,
    )
}

fn parse_simple_data_modifying_statement(
    stream: &mut TokenStream,
) -> ParseResult<SimpleDataModifyingStatement> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    match &stream.current().kind {
        TokenKind::Insert
        | TokenKind::Set
        | TokenKind::Remove
        | TokenKind::Delete
        | TokenKind::Detach
        | TokenKind::Nodetach => {
            let (primitive_opt, diags) = parse_primitive_data_modifying_statement(stream);
            (
                primitive_opt.map(SimpleDataModifyingStatement::Primitive),
                diags,
            )
        }
        TokenKind::Call | TokenKind::Optional => {
            let (call_opt, diags) = parse_call_data_modifying_procedure_statement(stream);
            (
                call_opt.map(|call| SimpleDataModifyingStatement::Call(Box::new(call))),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

fn parse_primitive_data_modifying_statement(
    stream: &mut TokenStream,
) -> ParseResult<PrimitiveDataModifyingStatement> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    match &stream.current().kind {
        TokenKind::Insert => {
            let (statement_opt, diags) = parse_insert_statement(stream);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Insert),
                diags,
            )
        }
        TokenKind::Set => {
            let (statement_opt, diags) = parse_set_statement(stream);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Set),
                diags,
            )
        }
        TokenKind::Remove => {
            let (statement_opt, diags) = parse_remove_statement(stream);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Remove),
                diags,
            )
        }
        TokenKind::Delete | TokenKind::Detach | TokenKind::Nodetach => {
            let (statement_opt, diags) = parse_delete_statement(stream);
            (
                statement_opt.map(PrimitiveDataModifyingStatement::Delete),
                diags,
            )
        }
        _ => (None, vec![]),
    }
}

fn parse_insert_statement(stream: &mut TokenStream) -> ParseResult<InsertStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Insert) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let (pattern_opt, mut pattern_diags) = parse_insert_graph_pattern(stream);
    diags.append(&mut pattern_diags);

    let Some(pattern) = pattern_opt else {
        diags.push(
            Diag::error("Expected insert graph pattern after INSERT")
                .with_primary_label(
                    stream.current().span.clone(),
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
    stream: &mut TokenStream,
) -> ParseResult<InsertGraphPattern> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let (first_opt, mut first_diags) = parse_insert_path_pattern(stream);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut paths = vec![first];

    while stream.check(&TokenKind::Comma) {
        stream.advance();

        let (path_opt, mut path_diags) = parse_insert_path_pattern(stream);
        diags.append(&mut path_diags);

        let Some(path) = path_opt else {
            diags.push(
                Diag::error("Expected insert path pattern after ','")
                    .with_primary_label(
                        stream.current().span.clone(),
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

fn parse_insert_path_pattern(stream: &mut TokenStream) -> ParseResult<InsertPathPattern> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let (node_opt, mut node_diags) = parse_insert_node_pattern(stream);
    diags.append(&mut node_diags);

    let Some(node) = node_opt else {
        return (None, diags);
    };

    let mut elements = vec![InsertElementPattern::Node(node)];

    loop {
        let before = stream.position();
        let (edge_opt, mut edge_diags) = parse_insert_edge_pattern(stream);
        diags.append(&mut edge_diags);

        let Some(edge) = edge_opt else {
            if stream.position() == before {
                break;
            }
            continue;
        };

        elements.push(InsertElementPattern::Edge(edge));

        let (next_node_opt, mut next_node_diags) = parse_insert_node_pattern(stream);
        diags.append(&mut next_node_diags);

        let Some(next_node) = next_node_opt else {
            diags.push(
                Diag::error("Expected node pattern after insert edge")
                    .with_primary_label(
                        stream.current().span.clone(),
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

fn parse_insert_node_pattern(stream: &mut TokenStream) -> ParseResult<InsertNodePattern> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::LParen) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let filler = if !stream.check(&TokenKind::RParen) {
        let (filler_opt, mut filler_diags) = parse_insert_element_pattern_filler(stream);
        diags.append(&mut filler_diags);
        filler_opt
    } else {
        None
    };

    if !stream.check(&TokenKind::RParen) {
        diags.push(
            Diag::error("Expected ')' to close insert node pattern")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected ')' here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    let end = stream.current().span.end;
    stream.advance();

    (
        Some(InsertNodePattern {
            filler,
            span: start..end,
        }),
        diags,
    )
}

fn parse_insert_edge_pattern(stream: &mut TokenStream) -> ParseResult<InsertEdgePattern> {
    // Need at least 2 tokens for edge patterns
    let tokens = stream.tokens();
    let pos = stream.position();
    if pos + 1 >= tokens.len() {
        return (None, vec![]);
    }

    let mut diags = Vec::new();
    let start = stream.current().span.start;

    if stream.check(&TokenKind::LeftArrow) {
        let next_token = &tokens[pos + 1];
        if matches!(next_token.kind, TokenKind::LBracket) {
            stream.advance();
            stream.advance();
            let (filler, mut filler_diags) = parse_optional_insert_edge_filler(stream);
            diags.append(&mut filler_diags);

            if !stream.check(&TokenKind::RBracket) {
                diags.push(
                    Diag::error("Expected ']' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected ']' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }
            stream.advance();

            if !stream.check(&TokenKind::Minus) {
                diags.push(
                    Diag::error("Expected '-' after '<-[...]' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected '-' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }

            let end = stream.current().span.end;
            stream.advance();

            return (
                Some(InsertEdgePattern::PointingLeft(InsertEdgePointingLeft {
                    filler,
                    span: start..end,
                })),
                diags,
            );
        }
    }

    if stream.check(&TokenKind::Minus) {
        let next_token = &tokens[pos + 1];
        if matches!(next_token.kind, TokenKind::LBracket) {
            stream.advance();
            stream.advance();
            let (filler, mut filler_diags) = parse_optional_insert_edge_filler(stream);
            diags.append(&mut filler_diags);

            if !stream.check(&TokenKind::RBracket) {
                diags.push(
                    Diag::error("Expected ']' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected ']' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }
            stream.advance();

            if !stream.check(&TokenKind::Arrow) {
                diags.push(
                    Diag::error("Expected '->' after '-[...]' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected '->' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }

            let end = stream.current().span.end;
            stream.advance();

            return (
                Some(InsertEdgePattern::PointingRight(InsertEdgePointingRight {
                    filler,
                    span: start..end,
                })),
                diags,
            );
        }
    }

    if stream.check(&TokenKind::Tilde) {
        let next_token = &tokens[pos + 1];
        if matches!(next_token.kind, TokenKind::LBracket) {
            stream.advance();
            stream.advance();
            let (filler, mut filler_diags) = parse_optional_insert_edge_filler(stream);
            diags.append(&mut filler_diags);

            if !stream.check(&TokenKind::RBracket) {
                diags.push(
                    Diag::error("Expected ']' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected ']' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }
            stream.advance();

            if !stream.check(&TokenKind::Tilde) {
                diags.push(
                    Diag::error("Expected '~' after '~[...]' in insert edge pattern")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected '~' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }

            let end = stream.current().span.end;
            stream.advance();

            return (
                Some(InsertEdgePattern::Undirected(InsertEdgeUndirected {
                    filler,
                    span: start..end,
                })),
                diags,
            );
        }
    }

    (None, diags)
}

fn parse_optional_insert_edge_filler(
    stream: &mut TokenStream,
) -> (Option<InsertElementPatternFiller>, Vec<Diag>) {
    if stream.check(&TokenKind::RBracket) {
        return (None, vec![]);
    }

    let (filler_opt, mut diags) = parse_insert_element_pattern_filler(stream);
    if filler_opt.is_none() {
        diags.push(
            Diag::error("Expected insert edge filler")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected edge filler here",
                )
                .with_code("P_MUT"),
        );
    }

    (filler_opt, diags)
}

fn parse_insert_element_pattern_filler(
    stream: &mut TokenStream,
) -> ParseResult<InsertElementPatternFiller> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;
    let mut consumed_any = false;

    let variable = parse_element_variable_declaration_opt(stream);
    if variable.is_some() {
        consumed_any = true;
    }

    let mut label_set = None;
    let mut use_is_keyword = false;
    if stream.check(&TokenKind::Is) || stream.check(&TokenKind::Colon) {
        use_is_keyword = stream.check(&TokenKind::Is);
        stream.advance();

        let (label_set_opt, mut label_diags) = parse_label_set_specification(stream);
        diags.append(&mut label_diags);

        if label_set_opt.is_some() {
            consumed_any = true;
        }
        label_set = label_set_opt;
    }

    let mut properties = None;
    if stream.check(&TokenKind::LBrace) {
        let (properties_opt, mut prop_diags) =
            parse_element_property_specification(stream, false);
        diags.append(&mut prop_diags);
        if properties_opt.is_some() {
            consumed_any = true;
        }
        properties = properties_opt;
    }

    if !consumed_any {
        return (None, diags);
    }

    // Get end position from stream's previous position
    let end = end_after_last_consumed(stream, start);
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

fn parse_set_statement(stream: &mut TokenStream) -> ParseResult<SetStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Set) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let (items_opt, mut item_diags) = parse_set_item_list(stream);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected SET item list")
                .with_primary_label(
                    stream.current().span.clone(),
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

fn parse_set_item_list(stream: &mut TokenStream) -> ParseResult<SetItemList> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let (first_opt, mut first_diags) = parse_set_item(stream);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while stream.check(&TokenKind::Comma) {
        stream.advance();
        let (item_opt, mut item_diags) = parse_set_item(stream);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected SET item after ','")
                    .with_primary_label(
                        stream.current().span.clone(),
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

fn parse_set_item(stream: &mut TokenStream) -> ParseResult<SetItem> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let Some((element, element_span)) = parse_regular_identifier(stream) else {
        return (None, diags);
    };

    if stream.check(&TokenKind::Eof) {
        diags.push(
            Diag::error("Expected '.', '=', or label assignment in SET item")
                .with_primary_label(element_span, "incomplete SET item")
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    match &stream.current().kind {
        TokenKind::Dot => {
            stream.advance();
            let Some((property, _property_span)) = parse_identifier(stream) else {
                diags.push(
                    Diag::error("Expected property name after '.' in SET item")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected property name",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            };

            if !stream.check(&TokenKind::Eq) {
                diags.push(
                    Diag::error("Expected '=' in SET property item")
                        .with_primary_label(
                            stream.current().span.clone(),
                            "expected '=' here",
                        )
                        .with_code("P_MUT"),
                );
                return (None, diags);
            }
            stream.advance();

            // Bridge to legacy expression parser
            let tokens = stream.tokens();
            let mut pos = stream.position();
            let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, &mut pos);
            stream.set_position(pos);
            diags.append(&mut value_diags);

            let Some(value) = value_opt else {
                diags.push(
                    Diag::error("Expected value expression in SET property item")
                        .with_primary_label(
                            stream.current().span.clone(),
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
            stream.advance();
            let (properties_opt, mut properties_diags) =
                parse_element_property_specification(stream, true);
            diags.append(&mut properties_diags);

            let Some(properties) = properties_opt else {
                diags.push(
                    Diag::error("Expected record literal in SET all-properties item")
                        .with_primary_label(
                            stream.current().span.clone(),
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
            let use_is_keyword = matches!(stream.current().kind, TokenKind::Is);
            stream.advance();

            let Some((label, label_span)) = parse_identifier(stream) else {
                diags.push(
                    Diag::error("Expected label name in SET label item")
                        .with_primary_label(
                            stream.current().span.clone(),
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
                    .with_primary_label(stream.current().span.clone(), "expected '.', '=', IS, or ':'")
                    .with_code("P_MUT"),
            );
            (None, diags)
        }
    }
}

fn parse_remove_statement(stream: &mut TokenStream) -> ParseResult<RemoveStatement> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::Remove) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let (items_opt, mut item_diags) = parse_remove_item_list(stream);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected REMOVE item list")
                .with_primary_label(
                    stream.current().span.clone(),
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

fn parse_remove_item_list(stream: &mut TokenStream) -> ParseResult<RemoveItemList> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let (first_opt, mut first_diags) = parse_remove_item(stream);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while stream.check(&TokenKind::Comma) {
        stream.advance();

        let (item_opt, mut item_diags) = parse_remove_item(stream);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected REMOVE item after ','")
                    .with_primary_label(
                        stream.current().span.clone(),
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

fn parse_remove_item(stream: &mut TokenStream) -> ParseResult<RemoveItem> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let Some((element, _)) = parse_regular_identifier(stream) else {
        return (None, diags);
    };

    if stream.check(&TokenKind::Eof) {
        diags.push(
            Diag::error("Expected property or label removal after element")
                .with_primary_label(start..start, "incomplete REMOVE item")
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    match &stream.current().kind {
        TokenKind::Dot => {
            stream.advance();
            let Some((property, property_span)) = parse_identifier(stream) else {
                diags.push(
                    Diag::error("Expected property name in REMOVE property item")
                        .with_primary_label(
                            stream.current().span.clone(),
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
            let use_is_keyword = matches!(stream.current().kind, TokenKind::Is);
            stream.advance();

            let Some((label, label_span)) = parse_identifier(stream) else {
                diags.push(
                    Diag::error("Expected label name in REMOVE label item")
                        .with_primary_label(
                            stream.current().span.clone(),
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
                    .with_primary_label(stream.current().span.clone(), "expected '.', IS, or ':'")
                    .with_code("P_MUT"),
            );
            (None, diags)
        }
    }
}

fn parse_delete_statement(stream: &mut TokenStream) -> ParseResult<DeleteStatement> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let detach_option = if stream.check(&TokenKind::Detach) {
        stream.advance();
        DetachOption::Detach
    } else if stream.check(&TokenKind::Nodetach) {
        stream.advance();
        DetachOption::NoDetach
    } else {
        DetachOption::Default
    };

    if !stream.check(&TokenKind::Delete) {
        diags.push(
            Diag::error("Expected DELETE keyword")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected DELETE here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }
    stream.advance();

    let (items_opt, mut item_diags) = parse_delete_item_list(stream);
    diags.append(&mut item_diags);

    let Some(items) = items_opt else {
        diags.push(
            Diag::error("Expected DELETE item list")
                .with_primary_label(
                    stream.current().span.clone(),
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

fn parse_delete_item_list(stream: &mut TokenStream) -> ParseResult<DeleteItemList> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let (first_opt, mut first_diags) = parse_delete_item(stream);
    diags.append(&mut first_diags);

    let Some(first) = first_opt else {
        return (None, diags);
    };

    let mut items = vec![first];

    while stream.check(&TokenKind::Comma) {
        stream.advance();

        let (item_opt, mut item_diags) = parse_delete_item(stream);
        diags.append(&mut item_diags);

        let Some(item) = item_opt else {
            diags.push(
                Diag::error("Expected DELETE item after ','")
                    .with_primary_label(
                        stream.current().span.clone(),
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

fn parse_delete_item(stream: &mut TokenStream) -> ParseResult<DeleteItem> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    // Bridge to legacy expression parser
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (expression_opt, mut expression_diags) = parse_expression_with_diags(tokens, &mut pos);
    stream.set_position(pos);
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
    stream: &mut TokenStream,
) -> ParseResult<CallDataModifyingProcedureStatement> {
    // Bridge to legacy procedure parser
    let tokens = stream.tokens();
    let mut pos = stream.position();
    let (call_opt, diags) = parse_call_procedure_statement(tokens, &mut pos);
    stream.set_position(pos);

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
    stream: &mut TokenStream,
) -> ParseResult<PrimitiveResultStatement> {
    if stream.check(&TokenKind::Eof) {
        return (None, vec![]);
    }

    match &stream.current().kind {
        TokenKind::Return => {
            let (return_opt, diags) = parse_return_statement(stream);
            (return_opt.map(PrimitiveResultStatement::Return), diags)
        }
        TokenKind::Finish => {
            let span = stream.current().span.clone();
            stream.advance();
            (Some(PrimitiveResultStatement::Finish(span)), vec![])
        }
        _ => (None, vec![]),
    }
}

fn parse_label_set_specification(
    stream: &mut TokenStream,
) -> ParseResult<LabelSetSpecification> {
    let mut diags = Vec::new();
    let start = stream.current().span.start;

    let Some((first, first_span)) = parse_identifier(stream) else {
        return (None, diags);
    };

    let mut labels = vec![first];
    let mut end = first_span.end;

    while stream.check(&TokenKind::Ampersand) {
        stream.advance();

        let Some((next, next_span)) = parse_identifier(stream) else {
            diags.push(
                Diag::error("Expected label name after '&'")
                    .with_primary_label(
                        stream.current().span.clone(),
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
    stream: &mut TokenStream,
    allow_empty: bool,
) -> ParseResult<ElementPropertySpecification> {
    let mut diags = Vec::new();

    if !stream.check(&TokenKind::LBrace) {
        return (None, diags);
    }

    let start = stream.current().span.start;
    stream.advance();

    let mut properties = Vec::new();

    if stream.check(&TokenKind::RBrace) {
        let end = stream.current().span.end;
        stream.advance();
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
        let pair_start = stream.current().span.start;

        let Some((key, _key_span)) = parse_identifier(stream) else {
            diags.push(
                Diag::error("Expected property name")
                    .with_primary_label(
                        stream.current().span.clone(),
                        "expected property name",
                    )
                    .with_code("P_MUT"),
            );
            break;
        };

        if !stream.check(&TokenKind::Colon) {
            diags.push(
                Diag::error("Expected ':' after property name")
                    .with_primary_label(
                        stream.current().span.clone(),
                        "expected ':' here",
                    )
                    .with_code("P_MUT"),
            );
            break;
        }
        stream.advance();

        // Bridge to legacy expression parser
        let tokens = stream.tokens();
        let mut pos = stream.position();
        let (value_opt, mut value_diags) = parse_expression_with_diags(tokens, &mut pos);
        stream.set_position(pos);
        diags.append(&mut value_diags);

        let Some(value) = value_opt else {
            diags.push(
                Diag::error("Expected property value expression")
                    .with_primary_label(
                        stream.current().span.clone(),
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

        if stream.check(&TokenKind::Comma) {
            stream.advance();
            continue;
        }

        break;
    }

    if !stream.check(&TokenKind::RBrace) {
        diags.push(
            Diag::error("Expected '}' to close property map")
                .with_primary_label(
                    stream.current().span.clone(),
                    "expected '}' here",
                )
                .with_code("P_MUT"),
        );
        return (None, diags);
    }

    let end = stream.current().span.end;
    stream.advance();

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
    stream: &mut TokenStream,
) -> Option<ElementVariableDeclaration> {
    let (variable, span) = parse_regular_identifier(stream)?;
    Some(ElementVariableDeclaration { variable, span })
}

fn parse_regular_identifier(stream: &mut TokenStream) -> Option<(SmolStr, Span)> {
    let token = stream.current();
    let name = match &token.kind {
        TokenKind::Identifier(name) => name.clone(),
        kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
        _ => return None,
    };
    let span = token.span.clone();
    stream.advance();
    Some((name, span))
}

fn parse_identifier(stream: &mut TokenStream) -> Option<(SmolStr, Span)> {
    let token = stream.current();
    let name = match &token.kind {
        TokenKind::Identifier(name) | TokenKind::DelimitedIdentifier(name) => name.clone(),
        kind if kind.is_non_reserved_identifier_keyword() => SmolStr::new(kind.to_string()),
        _ => return None,
    };
    let span = token.span.clone();
    stream.advance();
    Some((name, span))
}

fn end_after_last_consumed(stream: &TokenStream, fallback: usize) -> usize {
    let pos = stream.position();
    let tokens = stream.tokens();
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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");
        assert_eq!(stmt.statements.len(), 4);
    }

    #[test]
    fn parses_focused_use_graph_mutation_with_return() {
        let (statement_opt, diags) = parse_source("USE myGraph INSERT (n) RETURN n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_focused(), "expected focused statement");
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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");

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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");

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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");

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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");

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

        let Some(stmt) = statement_opt else {
            panic!("expected statement");
        };

        assert!(stmt.is_ambient(), "expected ambient statement");

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
