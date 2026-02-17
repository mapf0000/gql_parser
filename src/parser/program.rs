//! Program structure and statement parsing.

use crate::ast::{
    CallCatalogModifyingProcedureStatement, CatalogStatement, CatalogStatementKind, CommitCommand,
    CreateGraphStatement, CreateGraphTypeStatement, CreateSchemaStatement, DropGraphStatement,
    DropGraphTypeStatement, DropSchemaStatement, Expression, GraphReference, GraphTypeReference,
    GraphTypeSource, GraphTypeSpec, MutationStatement, ProcedureReference, Program, QueryStatement,
    RollbackCommand, SchemaReference, SessionCloseCommand, SessionCommand, SessionResetCommand,
    SessionResetTarget, SessionSetCommand, SessionSetGraphClause, SessionSetParameterClause,
    SessionSetSchemaClause, SessionSetTimeZoneClause, SessionStatement, Span,
    StartTransactionCommand, Statement, TransactionAccessMode, TransactionCharacteristics,
    TransactionCommand, TransactionMode, TransactionStatement,
};
use crate::diag::Diag;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::query::parse_query;
use crate::parser::references as reference_parser;
use smol_str::SmolStr;

type ParseError = Box<Diag>;
type ParseOutcome<T> = Result<T, ParseError>;
type StatementParseOutcome = (Option<Statement>, Vec<Diag>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SyntaxToken {
    QueryStart,
    MutationStart,
    SessionStart,
    TransactionStart,
    CatalogStart,
    Semicolon,
    Eof,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StatementClass {
    Query,
    Mutation,
    Session,
    Transaction,
    Catalog,
}

pub(crate) fn parse_program_tokens(tokens: &[Token], source_len: usize) -> (Program, Vec<Diag>) {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let start_cursor = cursor;
        match classify(&tokens[cursor].kind) {
            SyntaxToken::Eof => break,
            SyntaxToken::Semicolon => {
                cursor += 1;
            }
            SyntaxToken::Other => {
                diagnostics.push(
                    Diag::error("unexpected token in statement")
                        .with_primary_label(
                            tokens[cursor].span.clone(),
                            format!("unexpected {}", tokens[cursor].kind),
                        )
                        .with_code("P003"),
                );
                cursor = synchronize_top_level(tokens, cursor + 1);
            }
            start => {
                let class = syntax_to_statement_class(start);
                let end = find_statement_end(tokens, cursor, class);
                let statement_tokens = &tokens[cursor..end];
                let (statement_opt, mut statement_diags) = parse_statement(class, statement_tokens);
                diagnostics.append(&mut statement_diags);
                if let Some(statement) = statement_opt {
                    statements.push(statement);
                }
                cursor = end;
            }
        }

        // Safety net to guarantee forward progress even on parser contract bugs.
        if cursor == start_cursor {
            cursor += 1;
        }
    }

    let program_span = compute_program_span(tokens, source_len);
    (
        Program {
            statements,
            span: program_span,
        },
        diagnostics,
    )
}

fn classify(kind: &TokenKind) -> SyntaxToken {
    match kind {
        TokenKind::Match
        | TokenKind::Optional
        | TokenKind::Use
        | TokenKind::Filter
        | TokenKind::Let
        | TokenKind::For
        | TokenKind::Order
        | TokenKind::Limit
        | TokenKind::Offset
        | TokenKind::Return
        | TokenKind::Finish
        | TokenKind::Select
        | TokenKind::From => SyntaxToken::QueryStart,
        TokenKind::Insert | TokenKind::Delete => SyntaxToken::MutationStart,
        TokenKind::Session => SyntaxToken::SessionStart,
        TokenKind::Start | TokenKind::Commit | TokenKind::Rollback => SyntaxToken::TransactionStart,
        TokenKind::Create | TokenKind::Drop | TokenKind::Call => SyntaxToken::CatalogStart,
        TokenKind::Semicolon => SyntaxToken::Semicolon,
        TokenKind::Eof => SyntaxToken::Eof,
        _ => SyntaxToken::Other,
    }
}

fn syntax_to_statement_class(token: SyntaxToken) -> StatementClass {
    match token {
        SyntaxToken::QueryStart => StatementClass::Query,
        SyntaxToken::MutationStart => StatementClass::Mutation,
        SyntaxToken::SessionStart => StatementClass::Session,
        SyntaxToken::TransactionStart => StatementClass::Transaction,
        SyntaxToken::CatalogStart => StatementClass::Catalog,
        SyntaxToken::Semicolon | SyntaxToken::Eof | SyntaxToken::Other => {
            unreachable!("only statement-start syntax tokens are converted")
        }
    }
}

fn find_statement_end(tokens: &[Token], start: usize, class: StatementClass) -> usize {
    let mut cursor = start + 1;
    while cursor < tokens.len() {
        match classify(&tokens[cursor].kind) {
            SyntaxToken::Semicolon | SyntaxToken::Eof => return cursor,
            SyntaxToken::MutationStart
            | SyntaxToken::SessionStart
            | SyntaxToken::TransactionStart
            | SyntaxToken::CatalogStart => return cursor,
            SyntaxToken::QueryStart if !matches!(class, StatementClass::Query) => return cursor,
            SyntaxToken::Other => {
                cursor += 1;
            }
            SyntaxToken::QueryStart => {
                cursor += 1;
            }
        }
    }
    cursor
}

fn parse_statement(class: StatementClass, tokens: &[Token]) -> StatementParseOutcome {
    match class {
        StatementClass::Query => parse_query_statement(tokens),
        StatementClass::Mutation => (
            Some(Statement::Mutation(Box::new(MutationStatement {
                span: slice_span(tokens),
            }))),
            Vec::new(),
        ),
        StatementClass::Session => parse_non_query_statement(tokens, parse_session_statement),
        StatementClass::Transaction => {
            parse_non_query_statement(tokens, parse_transaction_statement)
        }
        StatementClass::Catalog => parse_non_query_statement(tokens, parse_catalog_statement),
    }
}

fn parse_query_statement(tokens: &[Token]) -> StatementParseOutcome {
    let mut pos = 0usize;
    let (query_opt, mut diags) = parse_query(tokens, &mut pos);

    match query_opt {
        Some(query) => {
            let span = query.span().clone();
            (
                Some(Statement::Query(Box::new(QueryStatement { query, span }))),
                diags,
            )
        }
        None => {
            if diags.is_empty() {
                diags.push(
                    Diag::error("expected query statement")
                        .with_primary_label(slice_span(tokens), "expected query statement")
                        .with_code("P004"),
                );
            }
            (None, diags)
        }
    }
}

fn parse_non_query_statement(
    tokens: &[Token],
    parse: impl Fn(&[Token]) -> ParseOutcome<Statement>,
) -> StatementParseOutcome {
    match parse(tokens) {
        Ok(statement) => (Some(statement), Vec::new()),
        Err(diag) => (None, vec![*diag]),
    }
}

fn parse_session_statement(tokens: &[Token]) -> ParseOutcome<Statement> {
    let span = slice_span(tokens);
    if tokens.len() < 2 {
        return Err(expected_token_diag(
            tokens,
            1,
            "SET, RESET, or CLOSE",
            "SESSION statement",
        ));
    }

    let command = match tokens[1].kind {
        TokenKind::Set => parse_session_set_command(tokens, 2)?,
        TokenKind::Reset => parse_session_reset_command(tokens, 2)?,
        TokenKind::Close => {
            if tokens.len() != 2 {
                return Err(unexpected_token_diag(tokens, 2, "SESSION CLOSE"));
            }
            SessionCommand::Close(SessionCloseCommand { span: span.clone() })
        }
        _ => {
            return Err(expected_token_diag(
                tokens,
                1,
                "SET, RESET, or CLOSE",
                "SESSION statement",
            ));
        }
    };

    Ok(Statement::Session(Box::new(SessionStatement {
        command,
        span,
    })))
}

fn parse_session_set_command(tokens: &[Token], mut cursor: usize) -> ParseOutcome<SessionCommand> {
    let stmt_span = slice_span(tokens);
    if cursor >= tokens.len() {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "SCHEMA, GRAPH, TIME ZONE, VALUE, or TABLE",
            "SESSION SET",
        ));
    }

    if matches!(tokens[cursor].kind, TokenKind::Schema) {
        cursor += 1;
        let (schema_reference, next_cursor) =
            parse_schema_reference_until(tokens, cursor, |_| false, "schema reference")?;
        if next_cursor < tokens.len() {
            return Err(unexpected_token_diag(
                tokens,
                next_cursor,
                "SESSION SET SCHEMA",
            ));
        }
        return Ok(SessionCommand::Set(SessionSetCommand::Schema(
            SessionSetSchemaClause {
                schema_reference,
                span: stmt_span,
            },
        )));
    }

    if matches!(tokens[cursor].kind, TokenKind::Time) {
        cursor += 1;
        if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Zone) {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "ZONE",
                "SESSION SET TIME",
            ));
        }
        cursor += 1;
        if cursor >= tokens.len() {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "time zone value",
                "SESSION SET TIME ZONE",
            ));
        }
        // Parse the time zone expression
        let value_tokens = &tokens[cursor..];
        let value = crate::parser::expression::parse_expression(value_tokens)?;
        return Ok(SessionCommand::Set(SessionSetCommand::TimeZone(
            SessionSetTimeZoneClause {
                value,
                span: stmt_span,
            },
        )));
    }

    if matches!(tokens[cursor].kind, TokenKind::Value) {
        return parse_session_set_value_parameter(tokens, cursor + 1);
    }

    if matches!(tokens[cursor].kind, TokenKind::Binding | TokenKind::Table) {
        return parse_session_set_binding_table_parameter(tokens, cursor);
    }

    if matches!(tokens[cursor].kind, TokenKind::Property | TokenKind::Graph) {
        return parse_session_set_graph_or_graph_parameter(tokens, cursor);
    }

    Err(expected_token_diag(
        tokens,
        cursor,
        "SCHEMA, GRAPH, TIME ZONE, VALUE, or TABLE",
        "SESSION SET",
    ))
}

fn parse_session_set_graph_or_graph_parameter(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<SessionCommand> {
    let stmt_span = slice_span(tokens);
    let property = if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Property) {
        cursor += 1;
        true
    } else {
        false
    };

    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Graph) {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "GRAPH",
            "SESSION SET GRAPH",
        ));
    }
    cursor += 1;

    if cursor >= tokens.len() {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "graph reference or $parameter",
            "SESSION SET GRAPH",
        ));
    }

    if starts_session_parameter_name(tokens, cursor) {
        let (name, next_cursor) =
            parse_session_parameter_name(tokens, cursor, "SESSION SET GRAPH parameter")?;
        let value =
            parse_initializer_expression(tokens, next_cursor, "SESSION SET GRAPH parameter")?;
        return Ok(SessionCommand::Set(SessionSetCommand::Parameter(
            SessionSetParameterClause::GraphParameter {
                name,
                value,
                span: stmt_span,
            },
        )));
    }

    let (graph_reference, next_cursor) =
        parse_graph_reference_until(tokens, cursor, |_| false, "graph reference")?;
    if next_cursor < tokens.len() {
        return Err(unexpected_token_diag(
            tokens,
            next_cursor,
            "SESSION SET GRAPH",
        ));
    }
    Ok(SessionCommand::Set(SessionSetCommand::Graph(
        SessionSetGraphClause {
            property,
            graph_reference,
            span: stmt_span,
        },
    )))
}

fn parse_session_set_binding_table_parameter(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<SessionCommand> {
    let stmt_span = slice_span(tokens);
    if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Binding) {
        cursor += 1;
    }

    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Table) {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "TABLE",
            "SESSION SET TABLE",
        ));
    }
    cursor += 1;

    let (name, next_cursor) =
        parse_session_parameter_name(tokens, cursor, "SESSION SET TABLE parameter")?;
    let value = parse_initializer_expression(tokens, next_cursor, "SESSION SET TABLE parameter")?;

    Ok(SessionCommand::Set(SessionSetCommand::Parameter(
        SessionSetParameterClause::BindingTableParameter {
            name,
            value,
            span: stmt_span,
        },
    )))
}

fn parse_session_set_value_parameter(
    tokens: &[Token],
    cursor: usize,
) -> ParseOutcome<SessionCommand> {
    let stmt_span = slice_span(tokens);
    let (name, next_cursor) =
        parse_session_parameter_name(tokens, cursor, "SESSION SET VALUE parameter")?;
    let value = parse_initializer_expression(tokens, next_cursor, "SESSION SET VALUE parameter")?;

    Ok(SessionCommand::Set(SessionSetCommand::Parameter(
        SessionSetParameterClause::ValueParameter {
            name,
            value,
            span: stmt_span,
        },
    )))
}

fn parse_session_reset_command(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<SessionCommand> {
    let stmt_span = slice_span(tokens);
    let target = if cursor >= tokens.len() {
        SessionResetTarget::All
    } else if matches!(tokens[cursor].kind, TokenKind::All) {
        cursor += 1;
        if cursor >= tokens.len() {
            SessionResetTarget::All
        } else if is_identifier_word(&tokens[cursor].kind, "PARAMETERS") {
            cursor += 1;
            SessionResetTarget::Parameters
        } else if matches!(tokens[cursor].kind, TokenKind::Characteristics) {
            cursor += 1;
            SessionResetTarget::Characteristics
        } else {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "PARAMETERS or CHARACTERISTICS",
                "SESSION RESET ALL",
            ));
        }
    } else if is_identifier_word(&tokens[cursor].kind, "PARAMETERS") {
        cursor += 1;
        SessionResetTarget::Parameters
    } else if matches!(tokens[cursor].kind, TokenKind::Characteristics) {
        cursor += 1;
        SessionResetTarget::Characteristics
    } else if matches!(tokens[cursor].kind, TokenKind::Schema) {
        cursor += 1;
        SessionResetTarget::Schema
    } else if matches!(tokens[cursor].kind, TokenKind::Property) {
        cursor += 1;
        if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Graph) {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "GRAPH",
                "SESSION RESET PROPERTY",
            ));
        }
        cursor += 1;
        SessionResetTarget::Graph
    } else if matches!(tokens[cursor].kind, TokenKind::Graph) {
        cursor += 1;
        SessionResetTarget::Graph
    } else if matches!(tokens[cursor].kind, TokenKind::Time) {
        cursor += 1;
        if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Zone) {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "ZONE",
                "SESSION RESET TIME",
            ));
        }
        cursor += 1;
        SessionResetTarget::TimeZone
    } else if is_identifier_word(&tokens[cursor].kind, "PARAMETER") {
        cursor += 1;
        if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Parameter(_)) {
            cursor += 1;
        }
        SessionResetTarget::Parameters
    } else if matches!(tokens[cursor].kind, TokenKind::Parameter(_)) {
        cursor += 1;
        SessionResetTarget::Parameters
    } else {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "RESET target",
            "SESSION RESET",
        ));
    };

    if cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, cursor, "SESSION RESET"));
    }

    Ok(SessionCommand::Reset(SessionResetCommand {
        target,
        span: stmt_span,
    }))
}

fn starts_session_parameter_name(tokens: &[Token], cursor: usize) -> bool {
    if cursor >= tokens.len() {
        return false;
    }
    matches!(tokens[cursor].kind, TokenKind::Parameter(_))
        || (cursor + 2 < tokens.len()
            && matches!(tokens[cursor].kind, TokenKind::If)
            && matches!(tokens[cursor + 1].kind, TokenKind::Not)
            && matches!(tokens[cursor + 2].kind, TokenKind::Exists))
}

fn parse_session_parameter_name(
    tokens: &[Token],
    mut cursor: usize,
    context: &str,
) -> ParseOutcome<(SmolStr, usize)> {
    if cursor + 2 < tokens.len()
        && matches!(tokens[cursor].kind, TokenKind::If)
        && matches!(tokens[cursor + 1].kind, TokenKind::Not)
        && matches!(tokens[cursor + 2].kind, TokenKind::Exists)
    {
        cursor += 3;
    }

    if cursor >= tokens.len() {
        return Err(expected_token_diag(tokens, cursor, "$parameter", context));
    }

    match &tokens[cursor].kind {
        TokenKind::Parameter(name) => Ok((name.clone(), cursor + 1)),
        _ => Err(expected_token_diag(tokens, cursor, "$parameter", context)),
    }
}

/// Parses an expression from the token stream starting at the given cursor position.
/// Expects an '=' token at cursor, then parses the expression after it.
fn parse_initializer_expression(
    tokens: &[Token],
    cursor: usize,
    context: &str,
) -> ParseOutcome<Expression> {
    if cursor >= tokens.len() {
        return Err(expected_token_diag(tokens, cursor, "=", context));
    }

    if !matches!(tokens[cursor].kind, TokenKind::Eq) {
        return Err(expected_token_diag(tokens, cursor, "=", context));
    }

    let expr_start = cursor + 1;
    if expr_start >= tokens.len() {
        return Err(expected_token_diag(
            tokens,
            expr_start,
            "expression",
            context,
        ));
    }

    let expr_tokens = &tokens[expr_start..];
    crate::parser::expression::parse_expression(expr_tokens)
}

fn parse_transaction_statement(tokens: &[Token]) -> ParseOutcome<Statement> {
    let span = slice_span(tokens);
    if tokens.is_empty() {
        return Err(expected_token_diag(
            tokens,
            0,
            "transaction command",
            "transaction statement",
        ));
    }

    let command = match tokens[0].kind {
        TokenKind::Start => parse_start_transaction_command(tokens)?,
        TokenKind::Commit => parse_commit_command(tokens)?,
        TokenKind::Rollback => parse_rollback_command(tokens)?,
        _ => {
            return Err(expected_token_diag(
                tokens,
                0,
                "START, COMMIT, or ROLLBACK",
                "transaction statement",
            ));
        }
    };

    Ok(Statement::Transaction(Box::new(TransactionStatement {
        command,
        span,
    })))
}

fn parse_start_transaction_command(tokens: &[Token]) -> ParseOutcome<TransactionCommand> {
    if tokens.len() < 2 || !matches!(tokens[1].kind, TokenKind::Transaction) {
        return Err(expected_token_diag(
            tokens,
            1,
            "TRANSACTION",
            "START statement",
        ));
    }

    let mut cursor = 2usize;
    let characteristics = if cursor < tokens.len() {
        let (characteristics, next_cursor) = parse_transaction_characteristics(tokens, cursor)?;
        cursor = next_cursor;
        Some(characteristics)
    } else {
        None
    };

    if cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, cursor, "START TRANSACTION"));
    }

    Ok(TransactionCommand::Start(StartTransactionCommand {
        characteristics,
        span: slice_span(tokens),
    }))
}

fn parse_transaction_characteristics(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<(TransactionCharacteristics, usize)> {
    let start = cursor;
    let mut modes = Vec::new();

    loop {
        if cursor + 1 >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Read) {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "READ ONLY or READ WRITE",
                "transaction characteristics",
            ));
        }

        let access_mode = if matches!(tokens[cursor + 1].kind, TokenKind::Only) {
            TransactionAccessMode::ReadOnly
        } else if matches!(tokens[cursor + 1].kind, TokenKind::Write) {
            TransactionAccessMode::ReadWrite
        } else {
            return Err(expected_token_diag(
                tokens,
                cursor + 1,
                "ONLY or WRITE",
                "READ mode",
            ));
        };

        modes.push(TransactionMode::AccessMode(access_mode));
        cursor += 2;

        if cursor >= tokens.len() {
            break;
        }
        if !matches!(tokens[cursor].kind, TokenKind::Comma) {
            return Err(unexpected_token_diag(
                tokens,
                cursor,
                "transaction characteristics",
            ));
        }
        cursor += 1;
        if cursor >= tokens.len() {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "transaction mode",
                "transaction characteristics",
            ));
        }
    }

    Ok((
        TransactionCharacteristics {
            modes,
            span: span_for_segment(tokens, start, cursor),
        },
        cursor,
    ))
}

fn parse_commit_command(tokens: &[Token]) -> ParseOutcome<TransactionCommand> {
    let mut cursor = 1usize;
    let work = if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Work) {
        cursor += 1;
        true
    } else {
        false
    };

    if cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, cursor, "COMMIT"));
    }

    Ok(TransactionCommand::Commit(CommitCommand {
        work,
        span: slice_span(tokens),
    }))
}

fn parse_rollback_command(tokens: &[Token]) -> ParseOutcome<TransactionCommand> {
    let mut cursor = 1usize;
    let work = if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Work) {
        cursor += 1;
        true
    } else {
        false
    };

    if cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, cursor, "ROLLBACK"));
    }

    Ok(TransactionCommand::Rollback(RollbackCommand {
        work,
        span: slice_span(tokens),
    }))
}

fn parse_catalog_statement(tokens: &[Token]) -> ParseOutcome<Statement> {
    let span = slice_span(tokens);
    if tokens.is_empty() {
        return Err(expected_token_diag(
            tokens,
            0,
            "catalog command",
            "catalog statement",
        ));
    }

    let kind = match tokens[0].kind {
        TokenKind::Create => parse_create_catalog_statement(tokens)?,
        TokenKind::Drop => parse_drop_catalog_statement(tokens)?,
        TokenKind::Call => parse_call_catalog_statement(tokens)?,
        _ => {
            return Err(expected_token_diag(
                tokens,
                0,
                "CREATE, DROP, or CALL",
                "catalog statement",
            ));
        }
    };

    Ok(Statement::Catalog(Box::new(CatalogStatement {
        kind,
        span,
    })))
}

fn parse_create_catalog_statement(tokens: &[Token]) -> ParseOutcome<CatalogStatementKind> {
    let mut cursor = 1usize;
    let mut or_replace = false;

    if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Or) {
        or_replace = true;
        cursor += 1;
        if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Replace) {
            return Err(expected_token_diag(tokens, cursor, "REPLACE", "CREATE OR"));
        }
        cursor += 1;
    }

    if cursor >= tokens.len() {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "SCHEMA or GRAPH",
            "CREATE statement",
        ));
    }

    if matches!(tokens[cursor].kind, TokenKind::Schema) {
        cursor += 1;
        let if_not_exists = consume_if_not_exists(tokens, &mut cursor);
        let (schema, next_cursor) =
            parse_schema_reference_until(tokens, cursor, |_| false, "schema reference")?;
        if next_cursor < tokens.len() {
            return Err(unexpected_token_diag(tokens, next_cursor, "CREATE SCHEMA"));
        }
        return Ok(CatalogStatementKind::CreateSchema(CreateSchemaStatement {
            or_replace,
            if_not_exists,
            schema,
            span: slice_span(tokens),
        }));
    }

    let property = if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Property) {
        cursor += 1;
        true
    } else {
        false
    };

    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Graph) {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "GRAPH",
            "CREATE statement",
        ));
    }
    cursor += 1;

    if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Type) {
        cursor += 1;
        let if_not_exists = consume_if_not_exists(tokens, &mut cursor);
        let (graph_type, next_cursor) = parse_graph_type_reference_until(
            tokens,
            cursor,
            is_graph_type_source_start,
            "graph type name",
        )?;
        cursor = next_cursor;

        let mut source = None;
        if cursor < tokens.len() {
            let source_start = cursor;
            if matches!(tokens[cursor].kind, TokenKind::As) {
                cursor += 1;
            }

            if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Copy) {
                cursor += 1;
                if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Of) {
                    return Err(expected_token_diag(tokens, cursor, "OF", "COPY clause"));
                }
                cursor += 1;
                let (copy_ref, next_cursor) =
                    parse_graph_type_reference_until(tokens, cursor, |_| false, "graph type")?;
                source = Some(GraphTypeSource::AsCopyOf {
                    graph_type: copy_ref,
                    span: span_for_segment(tokens, source_start, next_cursor),
                });
                cursor = next_cursor;
            } else {
                source = Some(GraphTypeSource::Detailed {
                    span: span_for_segment(tokens, source_start, tokens.len()),
                });
                cursor = tokens.len();
            }
        }

        if cursor < tokens.len() {
            return Err(unexpected_token_diag(tokens, cursor, "CREATE GRAPH TYPE"));
        }

        return Ok(CatalogStatementKind::CreateGraphType(
            CreateGraphTypeStatement {
                property,
                or_replace,
                if_not_exists,
                graph_type,
                source,
                span: slice_span(tokens),
            },
        ));
    }

    let if_not_exists = consume_if_not_exists(tokens, &mut cursor);
    let (graph, next_cursor) =
        parse_graph_reference_until(tokens, cursor, is_graph_type_spec_start, "graph name")?;
    cursor = next_cursor;

    let mut graph_type_spec = None;
    if cursor < tokens.len() && !matches!(tokens[cursor].kind, TokenKind::As) {
        let (spec, next_cursor) = parse_graph_type_spec(tokens, cursor)?;
        graph_type_spec = Some(spec);
        cursor = next_cursor;
    }

    if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::As) {
        let (source_graph, next_cursor, source_span) = parse_as_copy_of_graph(tokens, cursor)?;
        graph_type_spec = Some(GraphTypeSpec::AsCopyOf {
            graph: source_graph,
            span: source_span,
        });
        cursor = next_cursor;
    }

    if graph_type_spec.is_none() {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "graph type specification",
            "CREATE GRAPH",
        ));
    }
    if cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, cursor, "CREATE GRAPH"));
    }

    Ok(CatalogStatementKind::CreateGraph(CreateGraphStatement {
        property,
        or_replace,
        if_not_exists,
        graph,
        graph_type_spec,
        span: slice_span(tokens),
    }))
}

fn parse_graph_type_spec(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<(GraphTypeSpec, usize)> {
    let start = cursor;

    if cursor < tokens.len()
        && (matches!(tokens[cursor].kind, TokenKind::Typed)
            || is_identifier_word(&tokens[cursor].kind, "TYPED"))
    {
        cursor += 1;
        if cursor >= tokens.len() {
            return Err(expected_token_diag(
                tokens,
                cursor,
                "graph type specification",
                "TYPED clause",
            ));
        }
    }

    if matches!(tokens[cursor].kind, TokenKind::Any) {
        cursor += 1;
        if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Property) {
            cursor += 1;
        }
        if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Graph) {
            cursor += 1;
        }
        return Ok((
            GraphTypeSpec::Open {
                span: span_for_segment(tokens, start, cursor),
            },
            cursor,
        ));
    }

    if matches!(tokens[cursor].kind, TokenKind::Like) {
        cursor += 1;
        let (graph, next_cursor) = parse_graph_reference_until(
            tokens,
            cursor,
            |kind| matches!(kind, TokenKind::As),
            "LIKE graph",
        )?;
        return Ok((
            GraphTypeSpec::Like {
                graph,
                span: span_for_segment(tokens, start, next_cursor),
            },
            next_cursor,
        ));
    }

    if matches!(tokens[cursor].kind, TokenKind::Of) {
        cursor += 1;
        let (graph_type, next_cursor) = parse_graph_type_reference_until(
            tokens,
            cursor,
            |kind| matches!(kind, TokenKind::As),
            "graph type reference",
        )?;
        return Ok((
            GraphTypeSpec::Of {
                graph_type,
                span: span_for_segment(tokens, start, next_cursor),
            },
            next_cursor,
        ));
    }

    if matches!(
        tokens[cursor].kind,
        TokenKind::DoubleColon | TokenKind::LBrace
    ) {
        cursor += 1;
        while cursor < tokens.len() && !matches!(tokens[cursor].kind, TokenKind::As) {
            cursor += 1;
        }
        return Ok((
            GraphTypeSpec::Open {
                span: span_for_segment(tokens, start, cursor),
            },
            cursor,
        ));
    }

    let (graph_type, next_cursor) = parse_graph_type_reference_until(
        tokens,
        cursor,
        |kind| matches!(kind, TokenKind::As),
        "graph type reference",
    )?;
    Ok((
        GraphTypeSpec::Of {
            graph_type,
            span: span_for_segment(tokens, start, next_cursor),
        },
        next_cursor,
    ))
}

fn parse_as_copy_of_graph(
    tokens: &[Token],
    mut cursor: usize,
) -> ParseOutcome<(GraphReference, usize, Span)> {
    let start = cursor;
    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::As) {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "AS",
            "AS COPY OF clause",
        ));
    }
    cursor += 1;
    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Copy) {
        return Err(expected_token_diag(tokens, cursor, "COPY", "AS clause"));
    }
    cursor += 1;
    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Of) {
        return Err(expected_token_diag(tokens, cursor, "OF", "COPY clause"));
    }
    cursor += 1;

    let (graph, next_cursor) = parse_graph_reference_until(
        tokens,
        cursor,
        |_| false,
        "graph reference after AS COPY OF",
    )?;
    Ok((
        graph,
        next_cursor,
        span_for_segment(tokens, start, next_cursor),
    ))
}

fn parse_drop_catalog_statement(tokens: &[Token]) -> ParseOutcome<CatalogStatementKind> {
    let mut cursor = 1usize;
    if cursor >= tokens.len() {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "SCHEMA or GRAPH",
            "DROP statement",
        ));
    }

    if matches!(tokens[cursor].kind, TokenKind::Schema) {
        cursor += 1;
        let if_exists = consume_if_exists(tokens, &mut cursor);
        let (schema, next_cursor) =
            parse_schema_reference_until(tokens, cursor, |_| false, "schema reference")?;
        if next_cursor < tokens.len() {
            return Err(unexpected_token_diag(tokens, next_cursor, "DROP SCHEMA"));
        }
        return Ok(CatalogStatementKind::DropSchema(DropSchemaStatement {
            if_exists,
            schema,
            span: slice_span(tokens),
        }));
    }

    let property = if matches!(tokens[cursor].kind, TokenKind::Property) {
        cursor += 1;
        true
    } else {
        false
    };

    if cursor >= tokens.len() || !matches!(tokens[cursor].kind, TokenKind::Graph) {
        return Err(expected_token_diag(
            tokens,
            cursor,
            "GRAPH",
            "DROP statement",
        ));
    }
    cursor += 1;

    if cursor < tokens.len() && matches!(tokens[cursor].kind, TokenKind::Type) {
        cursor += 1;
        let if_exists = consume_if_exists(tokens, &mut cursor);
        let (graph_type, next_cursor) =
            parse_graph_type_reference_until(tokens, cursor, |_| false, "graph type reference")?;
        if next_cursor < tokens.len() {
            return Err(unexpected_token_diag(
                tokens,
                next_cursor,
                "DROP GRAPH TYPE",
            ));
        }
        return Ok(CatalogStatementKind::DropGraphType(
            DropGraphTypeStatement {
                property,
                if_exists,
                graph_type,
                span: slice_span(tokens),
            },
        ));
    }

    let if_exists = consume_if_exists(tokens, &mut cursor);
    let (graph, next_cursor) =
        parse_graph_reference_until(tokens, cursor, |_| false, "graph reference")?;
    if next_cursor < tokens.len() {
        return Err(unexpected_token_diag(tokens, next_cursor, "DROP GRAPH"));
    }

    Ok(CatalogStatementKind::DropGraph(DropGraphStatement {
        property,
        if_exists,
        graph,
        span: slice_span(tokens),
    }))
}

fn parse_call_catalog_statement(tokens: &[Token]) -> ParseOutcome<CatalogStatementKind> {
    if tokens.len() < 2 {
        return Err(expected_token_diag(
            tokens,
            1,
            "procedure name",
            "CALL statement",
        ));
    }

    let (procedure, next_cursor) = parse_procedure_reference_until(
        tokens,
        1,
        |kind| matches!(kind, TokenKind::LParen),
        "procedure name",
    )?;

    // Procedure argument parsing is deferred; accept any trailing (...) payload.
    if next_cursor < tokens.len() {
        if !matches!(tokens[next_cursor].kind, TokenKind::LParen) {
            return Err(unexpected_token_diag(tokens, next_cursor, "CALL"));
        }
        if !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::RParen)) {
            return Err(expected_token_diag(
                tokens,
                tokens.len(),
                ")",
                "CALL argument list",
            ));
        }
    }

    Ok(CatalogStatementKind::CallCatalogModifyingProcedure(
        CallCatalogModifyingProcedureStatement {
            procedure,
            span: slice_span(tokens),
        },
    ))
}

fn parse_schema_reference_until<F>(
    tokens: &[Token],
    start: usize,
    stop: F,
    context: &str,
) -> ParseOutcome<(SchemaReference, usize)>
where
    F: Fn(&TokenKind) -> bool,
{
    parse_reference_until(
        tokens,
        start,
        stop,
        context,
        reference_parser::parse_schema_reference,
    )
}

fn parse_graph_reference_until<F>(
    tokens: &[Token],
    start: usize,
    stop: F,
    context: &str,
) -> ParseOutcome<(GraphReference, usize)>
where
    F: Fn(&TokenKind) -> bool,
{
    parse_reference_until(
        tokens,
        start,
        stop,
        context,
        reference_parser::parse_graph_reference,
    )
}

fn parse_graph_type_reference_until<F>(
    tokens: &[Token],
    start: usize,
    stop: F,
    context: &str,
) -> ParseOutcome<(GraphTypeReference, usize)>
where
    F: Fn(&TokenKind) -> bool,
{
    parse_reference_until(
        tokens,
        start,
        stop,
        context,
        reference_parser::parse_graph_type_reference,
    )
}

fn parse_procedure_reference_until<F>(
    tokens: &[Token],
    start: usize,
    stop: F,
    context: &str,
) -> ParseOutcome<(ProcedureReference, usize)>
where
    F: Fn(&TokenKind) -> bool,
{
    parse_reference_until(
        tokens,
        start,
        stop,
        context,
        reference_parser::parse_procedure_reference,
    )
}

fn parse_reference_until<T, F, P>(
    tokens: &[Token],
    start: usize,
    stop: F,
    context: &str,
    parse: P,
) -> ParseOutcome<(T, usize)>
where
    F: Fn(&TokenKind) -> bool,
    P: Fn(&[Token]) -> Result<T, Box<Diag>>,
{
    if start >= tokens.len() {
        return Err(expected_token_diag(tokens, start, context, context));
    }

    for end in (start + 1..=tokens.len()).rev() {
        if end < tokens.len() && !stop(&tokens[end].kind) {
            continue;
        }
        if let Ok(reference) = parse(&tokens[start..end]) {
            return Ok((reference, end));
        }
    }

    Err(expected_token_diag(tokens, start, context, context))
}

fn is_graph_type_spec_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Any
            | TokenKind::Like
            | TokenKind::Of
            | TokenKind::As
            | TokenKind::LBrace
            | TokenKind::DoubleColon
            | TokenKind::Typed
    ) || is_identifier_word(kind, "TYPED")
}

fn is_graph_type_source_start(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::As
            | TokenKind::Copy
            | TokenKind::Like
            | TokenKind::LBrace
            | TokenKind::DoubleColon
    )
}

fn consume_if_not_exists(tokens: &[Token], cursor: &mut usize) -> bool {
    if *cursor + 2 < tokens.len()
        && matches!(tokens[*cursor].kind, TokenKind::If)
        && matches!(tokens[*cursor + 1].kind, TokenKind::Not)
        && matches!(tokens[*cursor + 2].kind, TokenKind::Exists)
    {
        *cursor += 3;
        true
    } else {
        false
    }
}

fn consume_if_exists(tokens: &[Token], cursor: &mut usize) -> bool {
    if *cursor + 1 < tokens.len()
        && matches!(tokens[*cursor].kind, TokenKind::If)
        && matches!(tokens[*cursor + 1].kind, TokenKind::Exists)
    {
        *cursor += 2;
        true
    } else {
        false
    }
}

fn is_identifier_word(kind: &TokenKind, word: &str) -> bool {
    matches!(kind, TokenKind::Identifier(name) if name.eq_ignore_ascii_case(word))
}

fn expected_token_diag(
    tokens: &[Token],
    cursor: usize,
    expected: &str,
    context: &str,
) -> ParseError {
    let span = if cursor < tokens.len() {
        tokens[cursor].span.clone()
    } else {
        empty_span_after(tokens)
    };
    Box::new(
        Diag::error(format!("expected {expected} in {context}"))
            .with_primary_label(span, format!("expected {expected}"))
            .with_code("P004"),
    )
}

fn unexpected_token_diag(tokens: &[Token], cursor: usize, context: &str) -> ParseError {
    let (span, found) = if cursor < tokens.len() {
        (tokens[cursor].span.clone(), tokens[cursor].kind.to_string())
    } else {
        (empty_span_after(tokens), "end of statement".to_string())
    };
    Box::new(
        Diag::error(format!("unexpected token in {context}"))
            .with_primary_label(span, format!("unexpected {found}"))
            .with_code("P004"),
    )
}

fn slice_span(tokens: &[Token]) -> Span {
    if let (Some(first), Some(last)) = (tokens.first(), tokens.last()) {
        first.span.start..last.span.end
    } else {
        0..0
    }
}

fn empty_span_after(tokens: &[Token]) -> Span {
    let end = tokens.last().map_or(0, |token| token.span.end);
    end..end
}

fn span_for_segment(tokens: &[Token], start: usize, end: usize) -> Span {
    if start < end {
        tokens[start].span.start..tokens[end - 1].span.end
    } else if start > 0 {
        let edge = tokens[start - 1].span.end;
        edge..edge
    } else {
        0..0
    }
}

fn synchronize_top_level(tokens: &[Token], mut cursor: usize) -> usize {
    while cursor < tokens.len() {
        match classify(&tokens[cursor].kind) {
            SyntaxToken::Semicolon => {
                while cursor < tokens.len()
                    && matches!(classify(&tokens[cursor].kind), SyntaxToken::Semicolon)
                {
                    cursor += 1;
                }
                return cursor;
            }
            SyntaxToken::QueryStart
            | SyntaxToken::MutationStart
            | SyntaxToken::SessionStart
            | SyntaxToken::TransactionStart
            | SyntaxToken::CatalogStart
            | SyntaxToken::Eof => return cursor,
            SyntaxToken::Other => {
                cursor += 1;
            }
        }
    }
    cursor
}

fn compute_program_span(tokens: &[Token], source_len: usize) -> Span {
    let mut start = None;
    let mut end = None;

    for token in tokens {
        if matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if start.is_none() {
            start = Some(token.span.start);
        }
        end = Some(token.span.end);
    }

    match (start, end) {
        (Some(start), Some(end)) => start..end,
        _ => source_len..source_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::query::{
        LinearQuery, PrimitiveQueryStatement, PrimitiveResultStatement, Query,
    };
    use crate::lexer::tokenize;

    fn parse_source(source: &str) -> (Program, Vec<Diag>) {
        let lex = tokenize(source);
        parse_program_tokens(&lex.tokens, source.len())
    }

    #[test]
    fn parse_empty_program() {
        let (program, diagnostics) = parse_source("");
        assert!(diagnostics.is_empty());
        assert!(program.statements.is_empty());
        assert_eq!(program.span, 0..0);
    }

    #[test]
    fn parse_skips_semicolons_without_empty_nodes() {
        let (program, diagnostics) = parse_source(";;;");
        assert!(diagnostics.is_empty());
        assert!(program.statements.is_empty());
    }

    #[test]
    fn parse_multiple_statements_without_semicolons() {
        let source = "MATCH (n) RETURN n INSERT (n) CREATE SCHEMA /foo";
        let (program, diagnostics) = parse_source(source);

        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 3);
        assert!(matches!(program.statements[0], Statement::Query(_)));
        assert!(matches!(program.statements[1], Statement::Mutation(_)));
        assert!(matches!(program.statements[2], Statement::Catalog(_)));
    }

    #[test]
    fn parse_return_query_builds_real_query_ast() {
        let (program, diagnostics) = parse_source("RETURN 1");
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);

        let Statement::Query(stmt) = &program.statements[0] else {
            panic!("expected query statement");
        };

        let Query::Linear(LinearQuery::Ambient(query)) = &stmt.query else {
            panic!("expected ambient linear query");
        };
        assert!(query.primitive_statements.is_empty());
        assert!(matches!(
            query.result_statement.as_deref(),
            Some(PrimitiveResultStatement::Return(_))
        ));
    }

    #[test]
    fn parse_select_from_match_stays_single_query_statement() {
        let source = "SELECT * FROM MATCH (n) RETURN n";
        let (program, diagnostics) = parse_source(source);

        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        assert_eq!(program.statements.len(), 1);

        let Statement::Query(stmt) = &program.statements[0] else {
            panic!("expected query statement");
        };
        let Query::Linear(LinearQuery::Ambient(query)) = &stmt.query else {
            panic!("expected ambient linear query");
        };
        assert!(matches!(
            query.primitive_statements.first(),
            Some(PrimitiveQueryStatement::Select(_))
        ));
    }

    #[test]
    fn parse_use_graph_query_start_is_accepted() {
        let source = "USE GRAPH g MATCH (n) RETURN n";
        let (program, diagnostics) = parse_source(source);

        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Query(_)));
    }

    #[test]
    fn parse_optional_match_query_start_is_accepted() {
        let source = "OPTIONAL MATCH (n) RETURN n";
        let (program, diagnostics) = parse_source(source);

        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);
        assert!(matches!(program.statements[0], Statement::Query(_)));
    }

    #[test]
    fn parse_recovers_after_invalid_top_level_token() {
        let source = "x MATCH (n) RETURN n";
        let (program, diagnostics) = parse_source(source);

        assert_eq!(program.statements.len(), 1);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("unexpected token"));
    }

    #[test]
    fn parse_session_set_schema_statement() {
        let (program, diagnostics) = parse_source("SESSION SET SCHEMA /myschema");
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);

        let Statement::Session(stmt) = &program.statements[0] else {
            panic!("expected session statement");
        };
        let SessionCommand::Set(SessionSetCommand::Schema(_)) = &stmt.command else {
            panic!("expected SESSION SET SCHEMA");
        };
    }

    #[test]
    fn parse_session_set_schema_plain_identifier_statement() {
        let (program, diagnostics) = parse_source("SESSION SET SCHEMA myschema");
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);

        let Statement::Session(stmt) = &program.statements[0] else {
            panic!("expected session statement");
        };
        let SessionCommand::Set(SessionSetCommand::Schema(_)) = &stmt.command else {
            panic!("expected SESSION SET SCHEMA");
        };
    }

    #[test]
    fn parse_session_set_value_parameter_statement() {
        let source = "SESSION SET VALUE IF NOT EXISTS $exampleProperty = DATE '2022-10-10'";
        let (program, diagnostics) = parse_source(source);
        assert!(diagnostics.is_empty());

        let Statement::Session(stmt) = &program.statements[0] else {
            panic!("expected session statement");
        };
        let SessionCommand::Set(SessionSetCommand::Parameter(
            SessionSetParameterClause::ValueParameter { name, .. },
        )) = &stmt.command
        else {
            panic!("expected SESSION SET VALUE parameter");
        };
        assert_eq!(name, "exampleProperty");
    }

    #[test]
    fn parse_session_reset_and_close_statements() {
        let source = "SESSION RESET TIME ZONE; SESSION CLOSE";
        let (program, diagnostics) = parse_source(source);
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 2);

        let Statement::Session(reset) = &program.statements[0] else {
            panic!("expected session reset");
        };
        let SessionCommand::Reset(SessionResetCommand { target, .. }) = &reset.command else {
            panic!("expected SESSION RESET");
        };
        assert!(matches!(target, SessionResetTarget::TimeZone));

        let Statement::Session(close) = &program.statements[1] else {
            panic!("expected session close");
        };
        assert!(matches!(close.command, SessionCommand::Close(_)));
    }

    #[test]
    fn parse_invalid_session_statement_reports_diagnostic() {
        let (program, diagnostics) = parse_source("SESSION banana");
        assert!(program.statements.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("expected"));
    }

    #[test]
    fn parse_transaction_statements() {
        let source = "START TRANSACTION READ ONLY, READ WRITE; COMMIT WORK; ROLLBACK WORK";
        let (program, diagnostics) = parse_source(source);
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 3);

        let Statement::Transaction(start_stmt) = &program.statements[0] else {
            panic!("expected transaction statement");
        };
        let TransactionCommand::Start(start) = &start_stmt.command else {
            panic!("expected START TRANSACTION");
        };
        assert!(start.characteristics.is_some());
        assert_eq!(start.characteristics.as_ref().unwrap().modes.len(), 2);

        let Statement::Transaction(commit_stmt) = &program.statements[1] else {
            panic!("expected transaction statement");
        };
        let TransactionCommand::Commit(commit) = &commit_stmt.command else {
            panic!("expected COMMIT");
        };
        assert!(commit.work);

        let Statement::Transaction(rollback_stmt) = &program.statements[2] else {
            panic!("expected transaction statement");
        };
        let TransactionCommand::Rollback(rollback) = &rollback_stmt.command else {
            panic!("expected ROLLBACK");
        };
        assert!(rollback.work);
    }

    #[test]
    fn parse_create_drop_schema_statements() {
        let source = "CREATE OR REPLACE SCHEMA IF NOT EXISTS /foo; DROP SCHEMA IF EXISTS /foo";
        let (program, diagnostics) = parse_source(source);
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 2);

        let Statement::Catalog(create_stmt) = &program.statements[0] else {
            panic!("expected catalog statement");
        };
        let CatalogStatementKind::CreateSchema(create_schema) = &create_stmt.kind else {
            panic!("expected CREATE SCHEMA");
        };
        assert!(create_schema.or_replace);
        assert!(create_schema.if_not_exists);

        let Statement::Catalog(drop_stmt) = &program.statements[1] else {
            panic!("expected catalog statement");
        };
        let CatalogStatementKind::DropSchema(drop_schema) = &drop_stmt.kind else {
            panic!("expected DROP SCHEMA");
        };
        assert!(drop_schema.if_exists);
    }

    #[test]
    fn parse_create_graph_and_drop_graph_type_statements() {
        let source =
            "CREATE GRAPH mygraph ANY AS COPY OF srcgraph; DROP GRAPH TYPE IF EXISTS mytype";
        let (program, diagnostics) = parse_source(source);
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 2);

        let Statement::Catalog(create_stmt) = &program.statements[0] else {
            panic!("expected catalog statement");
        };
        let CatalogStatementKind::CreateGraph(create_graph) = &create_stmt.kind else {
            panic!("expected CREATE GRAPH");
        };
        assert!(matches!(
            create_graph.graph_type_spec,
            Some(GraphTypeSpec::AsCopyOf { .. })
        ));

        let Statement::Catalog(drop_stmt) = &program.statements[1] else {
            panic!("expected catalog statement");
        };
        let CatalogStatementKind::DropGraphType(drop_graph_type) = &drop_stmt.kind else {
            panic!("expected DROP GRAPH TYPE");
        };
        assert!(drop_graph_type.if_exists);
    }

    #[test]
    fn parse_call_catalog_procedure_statement() {
        let (program, diagnostics) = parse_source("CALL doMaintenance()");
        assert!(diagnostics.is_empty());
        assert_eq!(program.statements.len(), 1);

        let Statement::Catalog(stmt) = &program.statements[0] else {
            panic!("expected catalog statement");
        };
        let CatalogStatementKind::CallCatalogModifyingProcedure(call) = &stmt.kind else {
            panic!("expected CALL");
        };
        let ProcedureReference::CatalogQualified { name, .. } = &call.procedure else {
            panic!("expected parsed procedure reference");
        };
        assert_eq!(name.name, "doMaintenance");
    }
}
