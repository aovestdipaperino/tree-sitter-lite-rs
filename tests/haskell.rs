//! haskell: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-haskell")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_layout_operators_and_comments() {
    let mut p = Parser::new();
    p.set_language(&g::language_haskell().into()).unwrap();
    // Layout (where, do, case, let), a qualified operator, a left section, a nested block comment
    // and a pragma: all of these go through the external scanner.
    let src = "{-# LANGUAGE LambdaCase #-}\nmodule M (f) where\n\n{- a {- nested -} b -}\nimport qualified Data.Map as M\n\nf :: Int -> IO ()\nf x = do\n  let y = case x of\n        0 -> 1\n        _ -> x M.! 2\n  print (y +)\n  where\n    z = 3\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "haskell");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_haskell().into()).unwrap();
    let golden = include_str!("golden/haskell.golden");
    // A tree can contain a newline (a MISSING "\n" node), so a record may span lines: a new record
    // starts at a line of the form `<hex>\t<sexp>`.
    let is_start = |l: &str| {
        l.split_once('\t')
            .is_some_and(|(h, _)| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    let mut records: Vec<String> = Vec::new();
    for line in golden.lines() {
        match records.last_mut() {
            Some(last) if !is_start(line) => {
                last.push('\n');
                last.push_str(line);
            }
            _ => records.push(line.to_string()),
        }
    }
    let mut n = 0;
    for (ln, rec) in records.iter().enumerate() {
        let (hex, expected) = rec.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "haskell: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "haskell: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::haskell::LANGUAGE.into();
    assert!(g::haskell::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("injections", g::haskell::INJECTIONS_QUERY),
        ("locals", g::haskell::LOCALS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("haskell {name} query: {e}"));
    }
}

/// The `highlights.scm` shipped by tree-sitter-haskell 0.23.1 does not compile as a whole against
/// this crate's ABI 15 language (tree-sitter 0.26.13): three patterns give `decl` (a supertype with
/// no fields of its own) the fields `name:`/`names:` of `bind`, `function` and `signature`. The
/// upstream crate's own ABI 14 `parser.c` accepts the file under the same runtime, so this is a
/// consequence of regenerating at ABI 15; the file is bundled unchanged. This test pins the
/// behaviour: those three patterns (by the line where the error is reported) are the only ones that
/// fail, and the rest compiles.
#[test]
fn bundled_highlights_query_compiles_but_for_three_upstream_patterns() {
    let lang: Language = g::haskell::LANGUAGE.into();
    let mut lines: Vec<String> = g::haskell::HIGHLIGHTS_QUERY
        .lines()
        .map(String::from)
        .collect();
    let mut failing_rows = Vec::new();
    loop {
        let q = lines.join("\n");
        match Query::new(&lang, &q) {
            Ok(_) => break,
            Err(e) => {
                failing_rows.push(e.row + 1);
                assert!(
                    failing_rows.len() <= 3,
                    "more failing patterns than upstream's: {e}"
                );
                // Blank the pattern (the run of non-empty lines) that contains the error.
                let mut s = e.row;
                while s > 0 && !lines[s - 1].trim().is_empty() {
                    s -= 1;
                }
                let mut t = e.row;
                while t < lines.len() && !lines[t].trim().is_empty() {
                    t += 1;
                }
                for l in &mut lines[s..t] {
                    l.clear();
                }
            }
        }
    }
    assert_eq!(failing_rows, [134, 153, 171]);
}
