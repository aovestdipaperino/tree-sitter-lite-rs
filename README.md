# tree-sitter-grammars-rs

[![CI](https://github.com/aovestdipaperino/tree-sitter-grammars-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/aovestdipaperino/tree-sitter-grammars-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

Pure-Rust tree-sitter grammars, selected with cargo features. The `lite` tier (the default) is **javascript, typescript, tsx, python, rust, go, java and c**; `medium` adds lua, dart, cpp, c_sharp, scala, kotlin, ruby, swift, bash and php; `full` adds zig, pascal, vbnet, objc, batch, msbasic2, gwbasic, qbasic, nix, glsl, clojure, toml, powershell, quint, dockerfile, protobuf, ocaml, erlang, sql, cobol, fsharp, lean, julia, elixir, r, markdown, markdown_inline, perl, fortran and haskell.
No C sources, no `build.rs`, no C compiler needed to build the grammars.

The parse tables and lexers are transpiled from the released tree-sitter parsers by
[c2rs](https://github.com/aovestdipaperino/c2rs). The external scanners that most of the grammars
need are hand-ported to Rust. The result plugs into the normal `tree-sitter` runtime through the
usual `tree_sitter_<name>()` symbols.

```toml
[dependencies]
tree-sitter = "0.26"
tree-sitter-grammars-rs = { git = "https://github.com/aovestdipaperino/tree-sitter-grammars-rs" }
```

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_grammars_rs::python::LANGUAGE.into())?;
let tree = parser.parse("def f():\n    return 1\n", None).unwrap();
println!("{}", tree.root_node().to_sexp());
```

Each grammar module exposes the same constants the upstream crate does, under the same names:
`LANGUAGE` (`LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` for TypeScript), `NODE_TYPES`
(`TYPESCRIPT_NODE_TYPES` and `TSX_NODE_TYPES`), and the query constants the upstream crate has
(`HIGHLIGHTS_QUERY` or `HIGHLIGHT_QUERY`, `TAGS_QUERY`, `LOCALS_QUERY`, `INJECTIONS_QUERY`,
`JSX_HIGHLIGHT_QUERY`). The data files come from the same released crates and are bundled under
`data/`; `tests/data.rs` compiles every query against its grammar.

Want another language? See [docs/ADDING_A_LANGUAGE.md](docs/ADDING_A_LANGUAGE.md).

## Tiers and features

Every language has its own feature, `lang-<name>`, and three tiers bundle them: `lite` (default),
`medium` (includes `lite`) and `full` (includes `medium`). Take a tier, or only what you need:

```toml
tree-sitter-grammars-rs = { version = "0.1", default-features = false, features = ["lang-python", "lang-rust"] }
```

`all_languages()` returns the enabled languages as `(name, LanguageFn)` pairs. CI builds each
language on its own, so a feature never depends on another one by accident.

## Grammars

| Grammar | Source (released crate) | Scanner |
|---|---|---|
| javascript | tree-sitter-javascript 0.25.0 | ported (`src/javascript/scanner.rs`) |
| typescript, tsx | tree-sitter-typescript 0.23.2 (regenerated at ABI 15) | ported, shared logic |
| python | tree-sitter-python 0.25.0 | ported |
| rust | tree-sitter-rust 0.24.2 (regenerated at ABI 15) | ported |
| go | tree-sitter-go 0.25.0 | none needed |
| java | tree-sitter-java 0.23.5 (regenerated at ABI 15) | none needed |
| c | tree-sitter-c 0.24.2 | none needed |
| lua | tree-sitter-lua 0.5.0 | ported |
| dart | tree-sitter-dart-orchard 0.3.2 (regenerated at ABI 15) | ported |
| cpp | tree-sitter-cpp 0.23.4 (regenerated at ABI 15) | ported |
| c_sharp | tree-sitter-c-sharp 0.23.5 | ported |
| scala | tree-sitter-scala 0.25.1 | ported |
| kotlin | tree-sitter-kotlin-sg 0.4.1 (regenerated at ABI 15) | ported |
| ruby | tree-sitter-ruby 0.23.1 (regenerated at ABI 15) | ported |
| swift | tree-sitter-swift 0.7.4 | ported |
| bash | tree-sitter-bash 0.25.1 | ported |
| php | tree-sitter-php 0.24.2, the `php` grammar (regenerated at ABI 15) | ported |
| zig | tree-sitter-zig (amaanq) | none needed |
| pascal | tree-sitter-pascal | none needed |
| vbnet | tree-sitter-vb-dotnet (export `vb_dotnet`) | none needed |
| objc | tree-sitter-objc | none needed |
| batch, msbasic2, gwbasic, qbasic | legacy BASIC and batch grammars (`grammar-src/`) | ported |
| nix | tree-sitter-nix | ported |
| glsl | tree-sitter-glsl | none needed |
| clojure | tree-sitter-clojure-orchard | ported |
| toml | tree-sitter-toml-ng | ported |
| powershell | tree-sitter-powershell | ported |
| quint | tree-sitter-quint at 5155d17 (vendored `grammar.json`) | ported |
| dockerfile | tree-sitter-dockerfile 0.2.0 (vendored `grammar.json`) | ported |
| protobuf | tree-sitter-protobuf 0.0.1 (vendored `grammar.json`) | ported |
| ocaml | tree-sitter-ocaml, the `ocaml` grammar | ported |
| erlang | tree-sitter-erlang | ported |
| sql | tree-sitter-sequel | ported |
| cobol | tree-sitter-cobol 0.0.1 (vendored `grammar.json`) | ported |
| fsharp | tree-sitter-fsharp, the `fsharp` grammar | ported |
| lean | tree-sitter-lean (arborium) | ported |
| julia | tree-sitter-julia | ported |
| elixir | tree-sitter-elixir | ported |
| r | tree-sitter-r | ported |
| markdown, markdown_inline | tree-sitter-markdown 0.5.3 (vendored sources) | ported |
| perl | tree-sitter-perl | ported |
| fortran | tree-sitter-fortran | ported |
| haskell | tree-sitter-haskell 0.23.1 (regenerated at ABI 15) | ported |

The first eight are the `lite` tier; lua through php are `medium`; the rest are `full`. Not included yet: `php_only`, `ocaml_interface`, `ocaml_type` and `fsharp_signature`.

Known quirk: haskell's `highlights.scm` has three patterns (lines 134, 153, 171) that the ABI 15 regeneration rejects; injections and locals compile.

The grammars come from the released crate versions, not upstream HEAD: HEAD of tree-sitter-python
already has grammar changes after 0.25.0.

## Runtime versions

The golden trees were recorded with tree-sitter 0.26. Under 0.27.2 the inputs that contain ERROR
nodes recover differently (the runtime changed, not the grammars): the same 31 golden tests fail
with the upstream C 0.27.2 runtime and with the Rust port of it
([tree-sitter-runtime-rs](https://github.com/aovestdipaperino/tree-sitter-runtime-rs)), and everything
else passes on both.

## What is and is not Rust

The grammar side is pure Rust. The `tree-sitter` runtime that drives it (the incremental parse
loop) is still C, and is a dependency of anything that parses. So "no C" here means no C in the
grammar crates.

## How it is verified

Every grammar was compared against the original C with c2rs: all tables, the grammar corpora,
real-world files, byte-level mutants, input prefixes, incremental edits, random token soup, and a
direct comparison of the C and Rust lexers and scanners on a mock lexer (about 400 million lexer
calls and 16 million scanner scans, no mismatches). Line coverage of the Rust scanners and lexers
under those tests is 93 to 100 percent. Parse speed is within about 7 percent of C on real files,
faster on four of the eight grammars.

`tests/golden/` holds syntax trees recorded from the C parsers, so the crate checks itself without
needing C, and `tests/regress.rs` pins behaviours of each scanner. `tools/regen.sh` reproduces every
`parser.rs` byte for byte from the released crates, and the **Verify against C** workflow does that
on every push (failing if any committed file differs) and then runs the c2rs differential harness
on Linux for each grammar (`tools/verify.sh`). The unmodified tokensave integration suite
(3,088 tests) passes against this crate.

## Scanner notes

The C scanners call `iswspace`, `iswalpha` and `iswdigit`. Under the "C" locale, which an embedded
parser always sees, these are ASCII-only, and the ports reproduce exactly that, so behaviour does
not depend on the platform libc. The Python scanner differs from the C in one place: the C
serializer can write one byte past its 1024-byte buffer when many indent levels are nested, and the
port stops one write earlier instead. The Ruby scanner's heredoc serializer has the same kind of off-by-one in its bound (it can write one byte past the buffer), and the port checks the real size.

## Layout

| Path | What |
|---|---|
| `src/<grammar>/parser.rs` | generated by c2rs, do not edit |
| `src/<grammar>/scanner.rs` | hand-ported external scanner |
| `src/abi.rs` | ABI 15 types shared by all grammars |
| `tests/` | golden and regression tests |
| `data/` | `node-types.json` and queries bundled from the released upstream crates |
| `tools/regen.sh`, `tools/verify.sh` | reproduce the generated files; differential check against C |

## Star history

[![Star History Chart](https://api.star-history.com/svg?repos=aovestdipaperino/tree-sitter-grammars-rs&type=Date)](https://star-history.com/#aovestdipaperino/tree-sitter-grammars-rs&Date)

## License

[MIT](LICENSE). The parse tables, lexers and scanners are derived from MIT-licensed upstream
grammars; their notices are in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
