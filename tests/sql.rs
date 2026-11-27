//! sql: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-sql")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_dollar_quoted_strings() {
    let mut p = Parser::new();
    p.set_language(&g::language_sql().into()).unwrap();
    let src = "CREATE FUNCTION f() RETURNS int AS $body$ SELECT 1; $body$ LANGUAGE sql;\nSELECT $$a$$, $x$b$x$ FROM t;\n";
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
    p.set_language(&g::language_sql().into()).unwrap();
    let golden = include_str!("golden/sql.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "sql: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "sql: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::sql::LANGUAGE.into();
    assert!(g::sql::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::sql::HIGHLIGHTS_QUERY),
        ("indents", g::sql::INDENTS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("sql {name} query: {e}"));
    }
}
