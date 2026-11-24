# Contributing

Issues and pull requests are welcome.

- `src/<grammar>/parser.rs` is generated: change [c2rs](https://github.com/aovestdipaperino/c2rs)
  or `tools/regen.sh`, not the file. `tools/regen.sh` must reproduce every `parser.rs` byte for byte.
- `src/<grammar>/scanner.rs` is hand-ported from the upstream `scanner.c`. A change to a scanner
  must be checked against the C with the c2rs differential harness (`RS_SCANNER=... DIRECT=1`).
- Use the released crate versions of the grammars, not upstream HEAD (HEAD of tree-sitter-python has
  grammar changes after 0.25.0 that break downstream extraction tests).
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass.
