# Architecture

## Goal
Pure-Rust ISO GQL parser with resilient recovery and rich diagnostics.

## Stack
- `logos`: lexer implementation
- `chumsky`: parser control flow and grammar composition
- `miette`: rendered diagnostics at API boundaries
- `smol_str`: compact string storage in token payloads and future AST identifiers
- custom `ast` + internal `diag` model

## Pipeline
1. Lexing (`src/lexer/mod.rs`)
   - Input: `&str`
   - Output: `LexerResult { tokens: Vec<Token>, diagnostics: Vec<Diag> }`
   - Behavior: continues after lexical failures; always emits EOF token.
2. Parsing (`src/parser/mod.rs`, `src/parser/program.rs`)
   - Input: lexer tokens
   - Output: `ParseResult { ast: Option<Program>, diagnostics: Vec<Diag> }`
   - Behavior: parses statement skeleton with `chumsky`, synchronizes at top-level boundaries, avoids panic.
3. Diagnostics (`src/diag.rs`)
   - Internal representation: `Diag`
   - Rendering bridge: conversion helpers to `miette::Report`.

## Core Data Structures
- `Token { kind: TokenKind, span: Span }`
- `TokenKind` stores textual payload with `SmolStr` for identifiers/literals/parameters.
- `Program` root AST with top-level `Statement` nodes.

## Recovery Strategy
- Lexing: invalid characters, malformed literals, and unterminated constructs generate diagnostics while scanning continues.
- Parsing: top-level synchronization resumes at `;`, statement starts, or EOF.
- Semicolons are separators only and do not produce empty AST statements.

## Current Scope
- Stable lexer coverage for keywords, literals, operators, comments, and basic error handling.
- Parser skeleton for top-level query/mutation/catalog statement families.
- Deeper clause/expression/pattern grammar is staged in future sprints.

## Non-Goals (Current Phase)
- Semantic validation and name/type resolution.
- Full ISO conformance for all grammar families.

## Evolution Path
- Expand chumsky grammar incrementally by clause family (Sprints 4+).
- Keep AST/parser separation strict.
- Add semantic lowering/validation as a dedicated post-parse phase.
