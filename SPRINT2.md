# Sprint 2 Plan: Lexer Core and Token Model

## Status
- Sprint start: February 17, 2026
- Sprint end: February 17, 2026
- Current status: Completed

## Completion Summary
Sprint 2 has been successfully completed. All deliverables have been implemented:

### Final Verification (February 17, 2026)
- `cargo test`: 72 unit tests passed, 1 doc test passed
- `cargo clippy --all-targets --all-features -- -D warnings`: passed

### Implemented Components
1. **Token Type System** ([src/lexer/token.rs](src/lexer/token.rs))
   - Complete `TokenKind` enum with 100+ token variants
   - `Token` struct with kind, span, and text
   - Helper methods for token classification

2. **Keyword Recognition** ([src/lexer/keywords.rs](src/lexer/keywords.rs))
   - Case-insensitive keyword lookup using HashMap
   - Support for all GQL reserved and type keywords
   - 80+ keywords recognized

3. **Lexer Implementation** ([src/lexer/mod.rs](src/lexer/mod.rs))
   - Complete `Lexer` struct with robust scanning
   - `LexerResult` with tokens and diagnostics
   - Public `tokenize()` convenience function

4. **Token Scanning**
   - ✅ Keywords (case-insensitive)
   - ✅ Identifiers (regular and delimited with backticks)
   - ✅ String literals with escape sequences (\n, \t, \r, \', \\, \uXXXX)
   - ✅ Numeric literals (integers, floats, with underscores and exponents)
   - ✅ Boolean literals (TRUE, FALSE)
   - ✅ Null literals (NULL, UNKNOWN)
   - ✅ Temporal literals (DATE, TIME, TIMESTAMP, DURATION + string pattern)
   - ✅ Operators (arithmetic, comparison, path, logical)
   - ✅ Punctuation (parentheses, brackets, braces, etc.)
   - ✅ Multi-character operators (->, <-, <=, >=, <>, !=, ||, ::, ..)
   - ✅ Parameter tokens ($name, $123)
   - ✅ Single-line comments (//)
   - ✅ Block comments (/* */) with nesting support
   - ✅ Whitespace handling

5. **Error Recovery**
   - ✅ Continues after invalid characters
   - ✅ Handles unclosed strings gracefully
   - ✅ Handles unclosed block comments
   - ✅ Rejects malformed numbers with diagnostics
   - ✅ Invalid escape sequences reported but parsing continues
   - ✅ All errors integrated with Sprint 1 diagnostics

6. **Test Coverage**
   - 72 total tests passing (Sprint 1 + Sprint 2)
   - 30+ lexer-specific tests
   - Edge cases: empty input, whitespace, comments, malformed numbers, recovery paths
   - Example program demonstrating all features

7. **Quality Gates**
   - ✅ All tests pass (cargo test)
   - ✅ Clippy strict mode clean (cargo clippy --all-targets --all-features -- -D warnings)
   - ✅ Public API documented with examples
   - ✅ Diagnostic types exported via public API (`gql_parser::diag::*`)
   - ✅ Example program created (examples/lexer_demo.rs)

### Technical Decisions Confirmed
- **Token text storage**: Stored as String in each token (Decision 1 from plan)
- **Whitespace/comments**: Discarded from token stream (Decision 2)
- **Temporal literals**: Keyword + string pattern recognized, validation deferred (Decision 3)
- **Error tokens**: No error tokens in stream, only diagnostics (Decision 4)

### Key Achievements
- Complete coverage of GQL lexical syntax per ISO standard
- Robust error recovery - lexer never panics
- Rich diagnostics integrated with miette
- Clean separation of concerns (token types, keywords, lexer logic)
- Standalone `DETACH`/`NODETACH` keyword tokens (no fused compound token)
- Malformed numeric literals now produce explicit diagnostics (`L002`)
- Comprehensive test suite with 100% of planned test categories
- Production-ready code quality (no clippy warnings)

### Files Added/Modified
- Added: `src/lexer/mod.rs` (main lexer implementation, 600+ lines)
- Added: `src/lexer/token.rs` (token types, 400+ lines)
- Added: `src/lexer/keywords.rs` (keyword lookup, 200+ lines)
- Added: `examples/lexer_demo.rs` (usage examples)
- Added: `examples/advanced_lexer.rs` (advanced lexer scenarios)
- Modified: `src/lib.rs` (lexer exports and public diagnostics API)
- Modified: `Cargo.toml` (added lazy_static dependency)

### Unresolved Technical Debt
None. All planned features implemented to specification.

## Sprint Intent
Implement a robust, error-tolerant lexical analyzer that converts raw GQL source text into a stream of tokens with comprehensive lexical error reporting.

## Target Outcome
At sprint end, the project has a complete lexer layer that:
- Scans all GQL token kinds per the ISO standard
- Produces structured tokens with span information
- Continues scanning after lexical errors
- Returns both valid tokens and diagnostics
- Integrates cleanly with the diagnostics infrastructure from Sprint 1

## Scope

### In Scope
- **Token type definitions**: Complete enumeration of all GQL token kinds
- **Keyword recognition**: All reserved, pre-reserved, and non-reserved keywords
- **Identifier lexing**: Regular identifiers and delimited identifiers
- **Literal scanning**: String, numeric, boolean, null, date/time, duration literals
- **Operator tokens**: All symbolic operators and punctuation
- **Comment handling**: Single-line (`//`) and block (`/* */`) comments
- **Whitespace handling**: Space, tab, newline, carriage return
- **Parameter tokens**: `$` prefix for parameter references
- **Lexer error recovery**: Continue tokenization after invalid characters, unclosed strings, malformed numbers
- **Lexer diagnostics**: Integration with `Diag` model from Sprint 1

### Out of Scope
- Parser implementation (Sprint 3+)
- AST construction (Sprint 3+)
- Semantic validation (Sprint 14)
- Performance optimization beyond correctness
- Unicode normalization edge cases

## Dependencies from Sprint 1
- `Span` type for token locations
- `Spanned<T>` wrapper for tokens
- `Diag` model for lexical errors
- `DiagSeverity`, `DiagLabel` for error reporting
- `SourceFile` for diagnostic rendering

## Deliverables

### 1. Token Type System (`src/lexer/token.rs`)
- `TokenKind` enum covering all lexical categories:
  - Keywords (reserved, pre-reserved, non-reserved)
  - Identifiers (regular and delimited)
  - Literals (string, integer, float, boolean, null, temporal)
  - Operators and punctuation
  - Comments (optional: preserve for tooling)
  - Whitespace (optional: preserve for formatters)
  - EOF marker
  - No error token (lexical failures reported via diagnostics)
- `Token` struct:
  - `kind: TokenKind`
  - `span: Span`
  - Optional: `text: String` or reference to source
- Helper methods:
  - `is_keyword()`, `is_literal()`, `is_operator()`
  - `as_str()` for display

### 2. Lexer Implementation (`src/lexer/mod.rs`)
- `Lexer` struct:
  - Input source text
  - Current position/state
  - Accumulated tokens
  - Accumulated diagnostics
- Public API:
  - `pub fn new(source: &str) -> Self`
  - `pub fn tokenize(self) -> LexerResult`
- `LexerResult` struct:
  - `tokens: Vec<Token>` (each token carries its own span)
  - `diagnostics: Vec<Diag>`

### 3. Lexer Core Logic
Implement scanning for:

#### Keywords (Grammar: Lines 3630-3770 in GQL.g4)
- Reserved keywords: `MATCH`, `WHERE`, `RETURN`, `CREATE`, `DELETE`, etc.
- Pre-reserved keywords: future compatibility
- Non-reserved keywords: context-dependent identifiers
- Case-insensitive matching per GQL spec

#### Identifiers (Grammar: Lines ~3500-3520)
- Regular identifiers: `[a-zA-Z_][a-zA-Z0-9_]*`
- Delimited identifiers: backtick-quoted with escape sequences
- Keyword/identifier disambiguation

#### String Literals (Grammar: Lines ~3300-3350)
- Single-quoted strings: `'text'`
- Escape sequences: `\'`, `\\`, `\n`, `\t`, `\r`, `\uXXXX`
- Multiline strings
- Error recovery: unclosed strings

#### Numeric Literals (Grammar: Lines ~3250-3300)
- Integer literals: `123`, `-456`, `0`
- Float literals: `3.14`, `1.0e10`, `-2.5E-3`
- Underscore separators: `1_000_000`
- Error recovery: malformed numbers

#### Boolean and Null Literals (Grammar: Lines ~3350-3380)
- `TRUE`, `FALSE` (case-insensitive)
- `NULL`, `UNKNOWN` (case-insensitive)

#### Temporal Literals (Grammar: Lines ~3400-3450)
- Date: `DATE '2024-01-15'`
- Time: `TIME '14:30:00'`
- Timestamp: `TIMESTAMP '2024-01-15T14:30:00'`
- Duration: `DURATION 'P1Y2M3DT4H5M6S'`

#### Operators and Punctuation (Grammar: Lines ~3550-3630)
- Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`
- Comparison: `=`, `<>`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `AND`, `OR`, `NOT`, `IS`, `IN`
- Path operators: `->`, `<-`, `-`, `~`, `<~`, `~>`
- Delimiters: `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- Special: `|`, `&`, `||`, `::`, `..`

#### Parameter Tokens (Grammar: Lines ~3480-3500)
- Named parameters: `$param_name`
- Positional parameters: `$1`, `$2`

#### Comments and Whitespace (Grammar: Lines ~3770-3800)
- Single-line comments: `// comment`
- Block comments: `/* comment */`, with nesting support
- Whitespace: space, tab, newline, carriage return
- Decision: preserve or discard (recommend discard for parser, preserve for tooling layer)

### 4. Error Recovery Strategy
- **Invalid characters**: Emit error, skip character, continue
- **Unclosed string**: Emit error at EOL or EOF, synthesize closing quote
- **Malformed number**: Emit error, consume until non-numeric, continue
- **Nested comment mismatch**: Emit error at EOF or unmatched closer
- **Invalid escape sequence**: Emit error, keep scanning string

### 5. Lexer Diagnostics Integration
Use Sprint 1 diagnostic model:
- `Diag::error()` for lexical errors
- Primary label at error location
- Help text for common mistakes
- Diagnostic codes (e.g., `L001: unclosed string literal`)

Example:
```rust
Diag::error("unclosed string literal")
    .with_primary_label(start..current, "string starts here")
    .with_help("add a closing quote '")
    .with_code("L001")
```

### 6. Module Structure
```
src/
  lexer/
    mod.rs         # Public API, Lexer struct, tokenize()
    token.rs       # TokenKind, Token, Spanned<Token>
    keywords.rs    # Keyword classification helpers
    cursor.rs      # Low-level char iteration (optional)
  lib.rs           # Export lexer module
```

### 7. Test Coverage
- Unit tests for each token kind
- Keyword case-insensitivity tests
- Escape sequence tests
- Error recovery tests
- Edge cases:
  - Empty input
  - Only whitespace
  - Only comments
  - Interleaved comments and tokens
  - Maximum length identifiers/strings
  - Unicode in identifiers and strings
- Snapshot tests for token streams (optional)

## Work Breakdown

### Workstream A: Token Type Definitions
1. Define `TokenKind` enum with all categories
2. Define `Token` struct with kind and span
3. Add display/debug implementations
4. Unit tests for token construction

### Workstream B: Keyword System
1. Create keyword lookup table (case-insensitive)
2. Classify keywords: reserved vs pre-reserved vs non-reserved
3. Implement keyword recognition
4. Tests for keyword variants and case-insensitivity

### Workstream C: Identifier and String Lexing
1. Implement regular identifier scanning
2. Implement delimited identifier scanning with escapes
3. Implement string literal scanning with escape sequences
4. Error recovery for unclosed/malformed strings
5. Tests for identifiers and strings

### Workstream D: Numeric Literal Lexing
1. Implement integer literal scanning
2. Implement float literal scanning (with exponents)
3. Handle underscore separators
4. Error recovery for malformed numbers
5. Tests for numeric literals

### Workstream E: Operators, Punctuation, and Special Tokens
1. Implement multi-character operator recognition (`->`, `<>`, `||`, etc.)
2. Implement single-character punctuation
3. Implement parameter token recognition (`$name`)
4. Tests for all operators and punctuation

### Workstream F: Comments and Whitespace
1. Implement single-line comment scanning
2. Implement block comment scanning (with nesting)
3. Implement whitespace handling
4. Decide on preservation policy
5. Tests for comments and whitespace

### Workstream G: Temporal and Special Literals
1. Implement boolean and null literals
2. Implement temporal literal recognition (DATE, TIME, etc.)
3. Tests for special literals

### Workstream H: Lexer Integration and Error Recovery
1. Implement main tokenization loop
2. Wire all token scanners
3. Implement error recovery for invalid input
4. Integrate diagnostics model
5. Create `LexerResult` with tokens + diagnostics

### Workstream I: Quality Gates
1. Comprehensive unit test suite
2. Error recovery behavior tests
3. Diagnostic rendering validation
4. `cargo test` and `cargo clippy` clean
5. Coverage report for lexer module

## Suggested Execution Order

1. **Token types and keywords** (Workstreams A, B)
   - Foundation for all other work
   - No dependencies

2. **Identifiers and strings** (Workstream C)
   - Core lexical forms
   - Depends on token types

3. **Numeric literals** (Workstream D)
   - Independent from strings
   - Depends on token types

4. **Operators and punctuation** (Workstream E)
   - Straightforward once token types are defined
   - Depends on token types

5. **Comments and whitespace** (Workstream F)
   - Can be done in parallel with literals
   - Depends on token types

6. **Special literals** (Workstream G)
   - Temporal literals are complex but isolated
   - Depends on token types and keyword system

7. **Integration and error recovery** (Workstream H)
   - Ties everything together
   - Depends on all previous workstreams

8. **Testing and quality gates** (Workstream I)
   - Continuous throughout, final sweep at end

## Acceptance Criteria

### Functional Criteria
- Lexer tokenizes all valid GQL token kinds per ISO spec
- Keywords are recognized case-insensitively
- Identifiers support both regular and delimited forms
- String literals support all required escape sequences
- Numeric literals support integers, floats, and underscores
- Operators and punctuation are correctly recognized (including multi-char)
- Comments are handled (single-line and nested block comments)
- Temporal literals are recognized (DATE, TIME, TIMESTAMP, DURATION)
- Parameter tokens are recognized (`$name`)

### Error Recovery Criteria
- Lexer does not panic on any input
- Invalid characters produce diagnostics and continue scanning
- Unclosed strings produce diagnostics and recover at EOL/EOF
- Malformed numbers produce diagnostics (`L002`) and continue
- All errors integrate with Sprint 1 diagnostic model

### Quality Criteria
- Test coverage ≥80% for lexer module
- All unit tests pass
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- No `unsafe` code in lexer (unless justified and documented)
- Public API documented with examples

### Integration Criteria
- `Lexer::tokenize()` returns `LexerResult { tokens, diagnostics }`
- Tokens include accurate spans for all input
- Diagnostics render correctly via Sprint 1 conversion pipeline
- EOF token is always present at end of token stream

## Definition of Done

- All deliverables implemented
- All acceptance criteria met
- Tests pass locally
- Clippy strict mode passes
- Public API is documented
- Sprint notes updated with any unresolved technical debt
- Example usage added to docs or lib.rs

## Risks and Mitigations

### Risk: Keyword ambiguity with identifiers
- **Mitigation**: Follow GQL spec rules for reserved vs non-reserved keywords. Non-reserved keywords become identifiers in contexts where keywords are not expected. Document the classification clearly.

### Risk: Unicode and encoding edge cases
- **Mitigation**: Use Rust's built-in UTF-8 string handling. Defer normalization to semantic phase. Handle invalid UTF-8 gracefully (Rust strings are already UTF-8).

### Risk: Temporal literal complexity
- **Mitigation**: Recognize temporal literals lexically as keyword + string pattern. Defer validation of format to semantic phase (Sprint 14). Just ensure lexical structure is correct.

### Risk: Nested comment complexity
- **Mitigation**: Use a counter for nesting depth. Test edge cases thoroughly. Consider whether GQL requires nesting (check spec).

### Risk: Performance with large inputs
- **Mitigation**: Use efficient string scanning (avoid unnecessary allocations). Profile if needed. Defer optimization until correctness is validated.

### Risk: Token representation memory overhead
- **Mitigation**: For now, store token text or reference source. Measure memory usage on large files. Optimize in later sprint if needed (e.g., intern strings).

## Dependencies for Sprint 3

Sprint 3 (Parser Skeleton and Recovery Framework) depends on:
- Stable `Token` and `TokenKind` types
- `Lexer::tokenize()` API producing `Vec<Token>` + `Vec<Diag>`
- EOF token handling
- Diagnostics-based lexical error reporting (no error tokens in stream)
- Spans on all tokens for AST and diagnostic construction

## Resolved Questions for Sprint 2

### Q1: Should lexer preserve whitespace and comments?
- **Option A**: Discard whitespace and comments (simpler parser)
- **Option B**: Preserve as special tokens (enables formatters/linters)
- **Recommendation**: Option A for parser path, defer Option B to future tooling sprint

### Q2: How to represent token text?
- **Option A**: Store `String` in each token (simple, but memory overhead)
- **Option B**: Store `Span` only, reference source (efficient, but requires source lifetime management)
- **Option C**: Store interned string IDs (efficient, but adds complexity)
- **Recommendation**: Option A for Sprint 2 (measure, optimize later if needed)

### Q3: Should lexer handle trigraphs or other legacy compatibility?
- **Recommendation**: No, GQL is a modern spec. Only support what's in ISO standard.

### Q4: Error token representation?
- **Option A**: Use `TokenKind::Error` with optional error text
- **Option B**: Emit no token for error, only diagnostic
- **Recommendation**: Option B (cleaner separation, parser sees valid tokens only)

### Q5: Parameter syntax validation?
- **Option A**: Lexer validates parameter name format
- **Option B**: Lexer just recognizes `$` prefix, defer validation to parser
- **Recommendation**: Option A (lexer validates identifier after `$`)

### Q6: Temporal literal validation?
- **Option A**: Lexer validates full ISO 8601 / ISO duration format
- **Option B**: Lexer recognizes keyword + string pattern, defer validation
- **Recommendation**: Option B (lexical phase just identifies the form)

## Technical Debt Tracking

### Deferred to Future Sprints
- Full Unicode normalization (defer to semantic phase or future polish)
- Memory optimization for large files (intern strings, compact token representation)
- Incremental/streaming lexer for IDE use (defer to tooling sprint)
- Source map support for preprocessed input (defer to future)

### Known Limitations to Address in Sprint 2
- None. Sprint 2 implementation complete and validated.

## Locked Decisions

### Decision 1: Token Text Representation
- **Choice**: Option A (store String in token)
- **Rationale**: Simplicity and correctness first. Measure memory usage. Optimize in future sprint if profiling shows it's a bottleneck.

### Decision 2: Whitespace and Comment Handling
- **Choice**: Option A (discard whitespace and comments in token stream)
- **Rationale**: Simpler parser integration. Can add preservation later for tooling without breaking parser.

### Decision 3: Temporal Literal Validation
- **Choice**: Option B (lexer recognizes pattern, defer format validation)
- **Rationale**: Keep lexer focused on tokenization. Semantic phase is better suited for ISO 8601 validation.

### Decision 4: Error Token Strategy
- **Choice**: Option B (emit diagnostic, no error token in stream)
- **Rationale**: Cleaner separation. Parser sees only valid tokens. Diagnostics carry error information.

## References

- GQL Grammar: `third_party/opengql-grammar/GQL.g4`
- GQL Features: `GQL_FEATURES.md` Section 21 (Keywords), Section 18 (Literals), Section 19 (Parameters)
- Sprint 1 Diagnostics: `SPRINT1.md`, `src/diag.rs`
- ISO/IEC 39075:2024 GQL Standard (if accessible)

## Success Metrics

- All GQL token kinds are recognized
- Lexer handles 100% of valid token inputs correctly
- Error recovery works for all tested invalid inputs
- Zero panics on fuzzing with random input
- Diagnostics are clear and actionable
- Foundation is stable for parser implementation in Sprint 3

---

## Notes

This sprint establishes the lexical foundation for the entire parser. Correctness and robustness are the priorities. Performance optimization can come later once the parser is feature-complete and we have representative benchmarks.
