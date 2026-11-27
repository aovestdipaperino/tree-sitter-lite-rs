//! cobol: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-cobol")]
use tree_sitter::{Language, Parser};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_fixed_format_programs_with_comments_and_continued_strings() {
    let mut p = Parser::new();
    p.set_language(&g::language_cobol().into()).unwrap();
    let src = concat!(
        "000100 IDENTIFICATION DIVISION.\n",
        "000200 PROGRAM-ID. HELLO.\n",
        "      * a comment line in column 7\n",
        "000300 PROCEDURE DIVISION.\n",
        "000400     DISPLAY \"HELLO, WORLD\".\n",
        "000500     STOP RUN.\n",
    );
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "start");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("program_name"), "{sexp}");
    assert!(sexp.contains("comment"), "{sexp}");
}

#[test]
fn input_ending_in_a_short_line_terminates() {
    // The C scanner loops forever here; the port stops at end of input.
    let mut p = Parser::new();
    p.set_language(&g::language_cobol().into()).unwrap();
    let _ = p
        .parse("000100 IDENTIFICATION DIVISION.\n  ", None)
        .unwrap();
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_cobol().into()).unwrap();
    let golden = include_str!("golden/cobol.golden");
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
            "cobol: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "cobol: only {n} golden cases");
}

#[test]
fn bundled_data_is_present() {
    // The upstream crate bundles no queries, only node-types.json.
    let _: Language = g::cobol::LANGUAGE.into();
    assert!(g::cobol::NODE_TYPES.trim_start().starts_with('['));
}
