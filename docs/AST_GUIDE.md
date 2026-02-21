# AST Guide

## Root Model

- `Program`: top-level container of statements
- `Statement`: query/mutation/session/transaction/catalog variants
- query statements store a `Query` tree

## Query Shape

`Query` can be:

- `Linear`: clause pipeline
- `Composite`: set operator over two queries (`UNION`, `EXCEPT`, `INTERSECT`, `OTHERWISE`)
- `Parenthesized`

`LinearQuery` contains ordered primitive clauses and an optional result clause.

## Core Query Clauses

Primitive query clauses include:

- `MATCH`
- `CALL`
- `FILTER`
- `LET`
- `FOR`
- `ORDER BY`/paging
- `SELECT`

Result clause:

- `RETURN` or `FINISH`

## Pattern Nodes

Graph pattern structures are under `ast::query` and include:

- `GraphPattern`
- `PathPattern`
- `NodePattern`
- `EdgePattern`
- `LabelExpression`

## Expression Nodes

Expressions are defined in `ast::expression::Expression` and cover:

- literals
- variable/property references
- operators
- function calls and aggregate functions
- predicates
- collection constructors

## Spans

Major nodes store `Span` values (`Range<usize>`), and `Spanned<T>` is available for generic span-carrying values.

## Traversal

Use visitor APIs for traversal:

- `Visit`
- `VisitMut`
- walk helpers in `ast::visit` and `ast::visit_mut`

These support early exit via `ControlFlow`.
