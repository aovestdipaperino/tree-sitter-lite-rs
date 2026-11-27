//! lua: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-lua")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_block_strings_and_comments() {
    let mut p = Parser::new();
    p.set_language(&g::language_lua().into()).unwrap();
    let src = "local s = [==[a]]b]==] --[[ c ]]\nfunction f(x) return x .. s end\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "chunk");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_lua().into()).unwrap();
    let golden = include_str!("golden/lua.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "lua: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "lua: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::lua::LANGUAGE.into();
    assert!(g::lua::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::lua::HIGHLIGHTS_QUERY),
        ("injections", g::lua::INJECTIONS_QUERY),
        ("locals", g::lua::LOCALS_QUERY),
        ("tags", g::lua::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("lua {name} query: {e}"));
    }
}
