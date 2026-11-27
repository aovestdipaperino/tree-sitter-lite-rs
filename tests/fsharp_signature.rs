//! fsharp_signature: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-fsharp_signature")]
use tree_sitter::{Language, Parser, Query, QueryErrorKind};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_offside_signatures_and_comments() {
    let mut p = Parser::new();
    p.set_language(&g::language_fsharp_signature().into())
        .unwrap();
    let src = "namespace N\n\nmodule M =\n    (* a (* nested *) comment *)\n    val f: int -> int\n    type T =\n        | A\n        | B of int\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "file");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("block_comment"), "{sexp}");
    assert!(sexp.contains("value_definition"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_fsharp_signature().into())
        .unwrap();
    let golden = include_str!("golden/fsharp_signature.golden");
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
            "fsharp_signature: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "fsharp_signature: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::fsharp_signature::LANGUAGE_SIGNATURE.into();
    assert!(
        g::fsharp_signature::SIGNATURE_NODE_TYPES
            .trim_start()
            .starts_with('[')
    );
    for (name, q) in [
        ("highlights", g::fsharp_signature::HIGHLIGHTS_QUERY),
        ("injections", g::fsharp_signature::INJECTIONS_QUERY),
        ("locals", g::fsharp_signature::LOCALS_QUERY),
        ("tags", g::fsharp_signature::TAGS_QUERY),
    ] {
        // The upstream crate ships one set of queries, written for the `fsharp` grammar. They are
        // bundled under the same names, but may name nodes this grammar does not have: that is
        // fine, a syntax error in a query is not.
        match Query::new(&lang, q) {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind,
                    QueryErrorKind::NodeType | QueryErrorKind::Field | QueryErrorKind::Structure
                ) => {}
            Err(e) => panic!("fsharp_signature {name} query: {e}"),
        }
    }
}
