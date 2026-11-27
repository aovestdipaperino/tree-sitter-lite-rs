//! msbasic2: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-msbasic2")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_a_numbered_program() {
    let mut p = Parser::new();
    p.set_language(&g::language_msbasic2().into()).unwrap();
    let src = "10 PRINT \"HELLO WORLD\"
20 FOR I=1 TO 10: PRINT I: NEXT I
30 GOTO 10
";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "program");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_msbasic2().into()).unwrap();
    let golden = include_str!("golden/msbasic2.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "msbasic2: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "msbasic2: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::msbasic2::LANGUAGE.into();
    assert!(g::msbasic2::NODE_TYPES.trim_start().starts_with('['));
    Query::new(&lang, g::msbasic2::HIGHLIGHTS_QUERY)
        .unwrap_or_else(|e| panic!("msbasic2 highlights query: {e}"));
}
