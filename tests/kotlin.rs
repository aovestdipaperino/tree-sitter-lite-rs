//! kotlin: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-kotlin")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_strings_comments_and_classes() {
    let mut p = Parser::new();
    p.set_language(&g::language_kotlin().into()).unwrap();
    let src = "package a.b\nimport x.Y\n/* c /* nested */ */\nclass A(val n: Int) {\n    fun f(x: Int) = \"v=${x + 1} $n\"\n}\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "source_file");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_kotlin().into()).unwrap();
    let golden = include_str!("golden/kotlin.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "kotlin: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "kotlin: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::kotlin::LANGUAGE.into();
    assert!(g::kotlin::NODE_TYPES.trim_start().starts_with('['));
    assert!(!g::kotlin::HIGHLIGHTS_QUERY.is_empty());
    Query::new(&lang, g::kotlin::HIGHLIGHTS_QUERY)
        .unwrap_or_else(|e| panic!("kotlin highlights query: {e}"));
}
