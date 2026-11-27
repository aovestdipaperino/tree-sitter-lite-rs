//! r: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-r")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_raw_strings_else_and_brackets() {
    let mut p = Parser::new();
    p.set_language(&g::language_r().into()).unwrap();
    let src = "f <- function(x) {\n  if (x) 1\n  else 2\n}\ns <- r\"-(a)\"-)-\"\nm[[1]][2] <- x[[y[1]]]; z\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "program");
    let sexp = tree.root_node().to_sexp();
    assert!(
        sexp.contains("string_open") && sexp.contains("alternative"),
        "{sexp}"
    );
    assert!(sexp.contains("subset2"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_r().into()).unwrap();
    let golden = include_str!("golden/r.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "r: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "r: only {n} golden cases");
}

unsafe extern "C" {
    fn tree_sitter_r_external_scanner_create() -> *mut core::ffi::c_void;
    fn tree_sitter_r_external_scanner_destroy(p: *mut core::ffi::c_void);
    fn tree_sitter_r_external_scanner_serialize(
        p: *mut core::ffi::c_void,
        b: *mut core::ffi::c_char,
    ) -> u32;
    fn tree_sitter_r_external_scanner_deserialize(
        p: *mut core::ffi::c_void,
        b: *const core::ffi::c_char,
        n: u32,
    );
}

/// The C scanner trusts a serialized scope count and would write past its allocation when the
/// count exceeds the capacity; the port rejects it and resets to the empty state.
#[test]
fn corrupt_scope_count_resets_instead_of_overflowing() {
    unsafe {
        let p = tree_sitter_r_external_scanner_create();
        let mut buf = vec![0u8; 3 + 4 + 2000];
        buf[3..7].copy_from_slice(&2000u32.to_ne_bytes());
        tree_sitter_r_external_scanner_deserialize(p, buf.as_ptr() as *const _, buf.len() as u32);
        let mut out = [0 as core::ffi::c_char; 1024];
        let n = tree_sitter_r_external_scanner_serialize(p, out.as_mut_ptr());
        assert_eq!(n, 7);
        assert_eq!(&out[3..7], &[0, 0, 0, 0]);
        tree_sitter_r_external_scanner_destroy(p);
    }
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::r::LANGUAGE.into();
    assert!(g::r::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::r::HIGHLIGHTS_QUERY),
        ("locals", g::r::LOCALS_QUERY),
        ("tags", g::r::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("r {name} query: {e}"));
    }
}
