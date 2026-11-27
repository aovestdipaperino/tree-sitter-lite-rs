//! scala: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-scala")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_classes_and_strings() {
    let mut p = Parser::new();
    p.set_language(&g::language_scala().into()).unwrap();
    let src =
        "object Main {\n  def f(x: Int): String = s\"v=${x}\"\n  val t = \"\"\"a\nb\"\"\"\n}\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "compilation_unit");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_scala().into()).unwrap();
    let golden = include_str!("golden/scala.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "scala: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "scala: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::scala::LANGUAGE.into();
    assert!(g::scala::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::scala::HIGHLIGHTS_QUERY),
        ("locals", g::scala::LOCALS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("scala {name} query: {e}"));
    }
}
