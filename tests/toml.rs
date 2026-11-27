//! toml: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-toml")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_tables_and_multiline_strings() {
    let mut p = Parser::new();
    p.set_language(&g::language_toml().into()).unwrap();
    let src = "[server]\nname = \"a\"\nbody = \"\"\"\nline \"\"quoted\"\"\n\"\"\"\nraw = '''\nx\n'''\nports = [80, 443]\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "document");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("table") && sexp.contains("array"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_toml().into()).unwrap();
    let golden = include_str!("golden/toml.golden");
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
            "toml: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "toml: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::toml::LANGUAGE.into();
    assert!(g::toml::NODE_TYPES.trim_start().starts_with('['));
    Query::new(&lang, g::toml::HIGHLIGHTS_QUERY)
        .unwrap_or_else(|e| panic!("toml highlights query: {e}"));
}
