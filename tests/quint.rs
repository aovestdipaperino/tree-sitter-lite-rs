//! quint: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-quint")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_a_quint_module() {
    let mut p = Parser::new();
    p.set_language(&g::language_quint().into()).unwrap();
    let src = "module counter {\n  var n: int\n  action init = n' = 0\n  action step = n' = n + 1\n  val Pair: Map[int, int] = Map()\n  // a comment\n  run t = match n { _ => true }\n}\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_quint().into()).unwrap();
    let golden = include_str!("golden/quint.golden");
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
            "quint: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "quint: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::quint::LANGUAGE.into();
    assert!(g::quint::NODE_TYPES.trim_start().starts_with('['));
    Query::new(&lang, g::quint::HIGHLIGHTS_QUERY)
        .unwrap_or_else(|e| panic!("quint highlights query: {e}"));
}
