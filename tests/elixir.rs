//! Elixir: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-elixir")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_strings_heredocs_and_pipelines() {
    let mut p = Parser::new();
    p.set_language(&g::language_elixir().into()).unwrap();
    let src = "defmodule M do\n  def f(x) do\n    x\n    |> Enum.map(&(&1 + 1))\n    |> then(fn y -> \"v = #{y}\" end)\n  end\n\n  @doc \"\"\"\n  doc #{1}\n  \"\"\"\n  def g, do: :\"quoted atom\" ++ ~s(sigil #{1})\nend\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "source");
    let sexp = tree.root_node().to_sexp();
    assert!(
        sexp.contains("quoted_content") && sexp.contains("interpolation") && sexp.contains("sigil")
    );
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_elixir().into()).unwrap();
    let golden = include_str!("golden/elixir.golden");
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
            "elixir: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "elixir: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::elixir::LANGUAGE.into();
    assert!(g::elixir::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::elixir::HIGHLIGHTS_QUERY),
        ("injections", g::elixir::INJECTIONS_QUERY),
        ("tags", g::elixir::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("elixir {name} query: {e}"));
    }
}
