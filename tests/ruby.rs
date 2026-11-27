//! ruby: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-ruby")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_heredocs_blocks_and_interpolation() {
    let mut p = Parser::new();
    p.set_language(&g::language_ruby().into()).unwrap();
    let src = "class A\n  def f(x) = x + 1\n  def g\n    s = \"v: #{f(2)} #@x\"\n    t = <<~EOS\n      hi #{s}\n    EOS\n    [1, 2].map { |n| n * 2 }\n    %w[a b]\n  end\nend\n";
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
    p.set_language(&g::language_ruby().into()).unwrap();
    let golden = include_str!("golden/ruby.golden");
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
            "ruby: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "ruby: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::ruby::LANGUAGE.into();
    assert!(g::ruby::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::ruby::HIGHLIGHTS_QUERY),
        ("locals", g::ruby::LOCALS_QUERY),
        ("tags", g::ruby::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("ruby {name} query: {e}"));
    }
}
