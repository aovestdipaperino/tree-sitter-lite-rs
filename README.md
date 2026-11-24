# tree-sitter-lite-rs

Pure-Rust tree-sitter grammars for the lite tier: javascript, typescript, tsx, python, rust, go,
java, c. No C sources and no `build.rs`.

| Grammar | Source (released crate) | Scanner |
|---|---|---|
| javascript | tree-sitter-javascript 0.25.0 | ported (`src/javascript/scanner.rs`) |
| typescript, tsx | tree-sitter-typescript 0.23.2 (regenerated at ABI 15) | ported, shared logic |
| python | tree-sitter-python 0.25.0 | ported |
| rust | tree-sitter-rust 0.24.2 (regenerated at ABI 15) | ported |
| go | tree-sitter-go 0.25.0 | none |
| java | tree-sitter-java 0.23.5 (regenerated at ABI 15) | none |
| c | tree-sitter-c 0.24.2 | none |

- `src/<grammar>/parser.rs`: transpiled from the released `parser.c` by [c2rs](https://github.com/aovestdipaperino/c2rs). Regenerate with `tools/regen.sh`.
- `src/<grammar>/scanner.rs`: hand-ported external scanners. The `isw*` classification of the C
  scanners is reproduced ASCII-only (the "C" locale), so behaviour is platform-independent.
- Use the released crate versions, not upstream HEAD: HEAD of tree-sitter-python has grammar
  changes after 0.25.0.

## Verification

Every grammar was differentially tested against the original C with c2rs: all tables, grammar
corpora, real-world files, byte-level mutants, prefixes, incremental edits, token soup, and a direct
lexer/scanner comparison on a mock lexer (about 400M lexer calls, 16M scanner scans, 0 mismatches).
`tests/golden/` holds syntax trees recorded from the C parsers so this crate verifies itself
without C. tokensave's full integration suite (3088 tests) passes against it.

The `tree-sitter` runtime that consumes these grammars is still C.
