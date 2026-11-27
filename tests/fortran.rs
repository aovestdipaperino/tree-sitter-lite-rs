//! Fortran: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-fortran")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_continuations_literals_and_statements() {
    let mut p = Parser::new();
    p.set_language(&g::language_fortran().into()).unwrap();
    let src = "program demo\n  implicit none\n  integer :: i = 10\n  real :: x = 1.5e-3, y = 2.d0\n  character(len=20) :: s = \"it's &\n     & here\"\n  i = 3 + &\n      4 ! trailing comment\n  x = real(z'1F') + 1_8\n  print *, 'a''b', i; print *, x\nend program demo\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "translation_unit");
    let sexp = tree.root_node().to_sexp();
    assert!(
        sexp.contains("string_literal") && sexp.contains("number_literal"),
        "{sexp}"
    );
}

/// The C scanner stores the lookahead in a `char`, so U+4E2D (low byte 0x2D, `-`) is taken for a
/// preprocessor unary operator. Recorded from the C parser.
#[test]
fn char_truncation_of_the_lookahead_matches_the_c_scanner() {
    let mut p = Parser::new();
    p.set_language(&g::language_fortran().into()).unwrap();
    let tree = p.parse("\u{4E2D}", None).unwrap();
    assert_eq!(tree.root_node().to_sexp(), "(translation_unit (ERROR))");
}

/// The C `scan_int` recurses once per `&&` continuation; ours loops, so a very long chain of them
/// cannot overflow the stack.
#[test]
fn long_continuation_chains_do_not_overflow_the_stack() {
    let mut p = Parser::new();
    p.set_language(&g::language_fortran().into()).unwrap();
    let src = format!("x = {}1\n", "1&&".repeat(200_000));
    let tree = p.parse(&src, None).unwrap();
    assert_eq!(tree.root_node().kind(), "translation_unit");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_fortran().into()).unwrap();
    let golden = include_str!("golden/fortran.golden");
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
            "fortran: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "fortran: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::fortran::LANGUAGE.into();
    assert!(g::fortran::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("highlights", g::fortran::HIGHLIGHTS_QUERY),
        ("folds", g::fortran::FOLDS_QUERY),
        ("indents", g::fortran::INDENTS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("fortran {name} query: {e}"));
    }
}
