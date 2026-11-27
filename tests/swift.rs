//! swift: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-swift")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_swift_source() {
    let mut p = Parser::new();
    p.set_language(&g::language_swift().into()).unwrap();
    let src = "import Foundation\n\nstruct P {\n    var x: Int = 1\n    func f(_ a: Int) -> Int { return a + x }\n}\nlet s = #\"raw \\#(1) /* c */\"#\n";
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
    p.set_language(&g::language_swift().into()).unwrap();
    let golden = include_str!("golden/swift.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "swift: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "swift: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::swift::LANGUAGE.into();
    assert!(g::swift::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::swift::HIGHLIGHTS_QUERY),
        ("injections", g::swift::INJECTIONS_QUERY),
        ("locals", g::swift::LOCALS_QUERY),
        ("tags", g::swift::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("swift {name} query: {e}"));
    }
}
