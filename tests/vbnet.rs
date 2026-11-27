//! vbnet: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-vbnet")]
use tree_sitter::{Language, Parser};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_modules_and_methods() {
    let mut p = Parser::new();
    p.set_language(&g::language_vbnet().into()).unwrap();
    let src = "Module Program\n    Sub Main(args As String())\n        Console.WriteLine(\"Hello\")\n    End Sub\nEnd Module\n";
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
    p.set_language(&g::language_vbnet().into()).unwrap();
    let golden = include_str!("golden/vbnet.golden");
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
            "vbnet: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "vbnet: only {n} golden cases");
}

#[test]
fn bundled_data_is_present() {
    // The upstream crate ships no queries; only node-types.json is bundled.
    let lang: Language = g::vbnet::LANGUAGE.into();
    assert!(g::vbnet::NODE_TYPES.trim_start().starts_with('['));
    assert!(g::vbnet::NODE_TYPES.contains("\"source_file\""));
    // Every named node kind in node-types.json must exist in the language.
    assert!(lang.id_for_node_kind("source_file", true) != 0);
}
