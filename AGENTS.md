# GQL Parser Plan (Rust)

## Goal
Pure-Rust ISO GQL parser with high-quality (rustc-like) diagnostics.
Stack: `logos` (lexer) + `chumsky` (parser/recovery) + `miette` (rendering) + custom `ast`.

## Crates
- `logos` (tokenizer)
- `chumsky` (token-stream parser + recovery)
- `miette` (diagnostic reporting with labeled spans/snippets)
- `smol_str` (identifiers/keys in AST; cheap clones, fewer heap allocs)
- `thiserror` (optional: internal error types)

## Architecture
1. **Lexer**: `&str -> Vec<Token> + Vec<Diag>`
   - `Token { kind: TokenKind, span: Range<usize> }`
   - Emit diagnostics for unknown chars/unterminated strings, keep lexing.
2. **Parser**: `Vec<Token> -> (Option<Ast>, Vec<Diag>)`
   - Parse from tokens (not chars).
   - Use clause boundaries for recovery (`;`, `MATCH`, `WHERE`, `RETURN`, `WITH`, `)` etc.).
   - Prefer returning partial AST when possible.
3. **AST (custom)**:
   - Separate from parsing concerns; stable, typed enums/structs.
   - Store `Span` on major nodes: `Spanned<T> { node: T, span }`.
   - Use `SmolStr` for identifiers/labels/property keys.
   - Use `Box` for recursion; keep nodes small.
4. **Lower/Validate** (next phase): `Ast -> IR + Vec<Diag>`
   - Semantic checks (undefined vars, invalid patterns, etc.) as diagnostics.
5. **Diagnostics**
   - Internal `Diag { message, labels, help, notes, severity }` with spans.
   - Convert to `miette::Report` at the API boundary for rendering.

## Repo Layout
- `src/lexer.rs` (logos rules + TokenKind)
- `src/parser/mod.rs` (chumsky grammar, recovery helpers)
- `src/ast.rs` (AST + Span/Spanned)
- `src/diag.rs` (Diag model + miette conversion)
- `src/lib.rs` (public API: `parse(query: &str) -> ParseResult`)

## Public API
- `parse(&str) -> { ast: Option<Ast>, diags: Vec<miette::Report> }`
- Never panic on bad input; always return diagnostics (and partial AST if available).

## Code style
- Rust Editon 2024
- Write idiomatic rust, use modern langauge features and efficient abstractions
- Run clippy in strict mode and fix issues
- This is a greenfield project with no external consumers. Plan accordingly and make sound architectural descisions that ensure long term maintainability.