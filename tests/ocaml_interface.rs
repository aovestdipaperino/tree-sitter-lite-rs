//! ocaml_interface: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-ocaml_interface")]
use tree_sitter::{Language, Parser, Query, QueryErrorKind};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_comments_and_signatures() {
    let mut p = Parser::new();
    p.set_language(&g::language_ocaml_interface().into())
        .unwrap();
    let src = "(* a (* nested *) \"str *)\" *)\nmodule M : sig\n  val s : string\n  type t = A | B of int\nend\nval f : 'a list -> int\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "compilation_unit");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("(comment)"), "{sexp}");
    assert!(sexp.contains("module_definition"), "{sexp}");
    assert!(sexp.contains("value_specification"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_ocaml_interface().into())
        .unwrap();
    let golden = include_str!("golden/ocaml_interface.golden");
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
            "ocaml_interface: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "ocaml_interface: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::ocaml_interface::LANGUAGE_OCAML_INTERFACE.into();
    assert!(
        g::ocaml_interface::INTERFACE_NODE_TYPES
            .trim_start()
            .starts_with('[')
    );
    for (name, q) in [
        ("highlights", g::ocaml_interface::HIGHLIGHTS_QUERY),
        ("locals", g::ocaml_interface::LOCALS_QUERY),
        ("tags", g::ocaml_interface::TAGS_QUERY),
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
            Err(e) => panic!("ocaml_interface {name} query: {e}"),
        }
    }
}
