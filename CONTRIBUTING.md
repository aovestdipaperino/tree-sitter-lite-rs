# Contributing

Issues and pull requests are welcome.

- `src/<grammar>/parser.rs` is generated: change [c2rs](https://github.com/aovestdipaperino/c2rs)
  or `tools/regen.sh`, not the file. `tools/regen.sh` must reproduce every `parser.rs` byte for byte.
- `src/<grammar>/scanner.rs` is hand-ported from the upstream `scanner.c`. A change to a scanner
  must be checked against the C with the c2rs differential harness (`RS_SCANNER=... DIRECT=1`).
- Use the released crate versions of the grammars, not upstream HEAD (HEAD of tree-sitter-python has
  grammar changes after 0.25.0 that break downstream extraction tests).
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass.

## Adding a language to a tier

The full guide, with the decision tree for what sources you have and the scanner rules, is
[docs/ADDING_A_LANGUAGE.md](docs/ADDING_A_LANGUAGE.md). The checklist:

One language is one self-contained change; nothing outside its own files is shared except the
registration lines. The steps, with `lua` as the worked example (the first medium-tier grammar):

1. `c2rs <crate>/src/parser.c src/<name> --export-name tree_sitter_<name>`, then drop the generated
   `abi.rs`, set the header line and run `rustfmt`; add the language to `tools/regen.sh`. A parser.c
   in an older ABI or layout is first regenerated at ABI 15 (see the `regen` function there).
2. Hand-port `scanner.c` to `src/<name>/scanner.rs` (same `tree_sitter_<name>_external_scanner_*`
   symbols), keeping the C's control flow. Follow the C's integer widths; `iswspace` and friends are
   the C locale, so ASCII only.
3. `src/<name>/mod.rs` with `LANGUAGE` and the data constants the upstream crate exports; the data
   files go under `data/<name>/`.
4. Register it with `tools/register.py <name> --tier medium --const LANGUAGE --crate <crate-dir>
   --mode emit|regen`: it adds `lang-<name>` to `Cargo.toml` (and its tier), the `cfg`-gated `mod`,
   `language_<name>()` and the `all_languages()` entry in `src/lib.rs`, and the lines in
   `tools/regen.sh` and `tools/verify.sh`. It is idempotent.
5. Verify against the C with the c2rs harness (`PREGEN`, `RS_SCANNER`, `DIRECT=1`, `SOUP`, `FUZZ`,
   `PREFIX`, `INCR`, and `GOLDEN=<file>` to record `tests/golden/<name>.golden`). The result must
   be `RESULT: PASS` with 0 mismatches.
6. `tests/<name>.rs`: a smoke parse, the golden trees, and that the bundled queries compile.
   `cargo build --no-default-features --features lang-<name>` must work on its own.
