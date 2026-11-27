//! ocaml_type: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-ocaml_type")]
use tree_sitter::{Language, Parser, Query, QueryErrorKind};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_comments_and_type_expressions() {
    let mut p = Parser::new();
    p.set_language(&g::language_ocaml_type().into()).unwrap();
    let src = "(* a (* nested *) \"str *)\" *) int -> 'a list -> (string * int) option";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "type");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("(comment)"), "{sexp}");
    assert!(sexp.contains("function_type"), "{sexp}");
    assert!(sexp.contains("tuple_type"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_ocaml_type().into()).unwrap();
    let golden = include_str!("golden/ocaml_type.golden");
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
            "ocaml_type: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "ocaml_type: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::ocaml_type::LANGUAGE_OCAML_TYPE.into();
    assert!(g::ocaml_type::TYPE_NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::ocaml_type::HIGHLIGHTS_QUERY),
        ("locals", g::ocaml_type::LOCALS_QUERY),
        ("tags", g::ocaml_type::TAGS_QUERY),
    ] {
        // The upstream crate ships one set of queries, written for the `ocaml` grammar. They are
        // bundled under the same names, but may name nodes this grammar does not have: that is
        // fine, a syntax error in a query is not.
        match Query::new(&lang, q) {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind,
                    QueryErrorKind::NodeType | QueryErrorKind::Field | QueryErrorKind::Structure
                ) => {}
            Err(e) => panic!("ocaml_type {name} query: {e}"),
        }
    }
}
