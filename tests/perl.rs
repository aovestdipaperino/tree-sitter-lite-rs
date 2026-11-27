//! perl: a smoke parse, the golden trees recorded from the original C parser, and the bundled data (the upstream crate exports no queries).
#![cfg(feature = "lang-perl")]
use tree_sitter::{Language, Parser};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_heredocs_quotes_and_regexes() {
    let mut p = Parser::new();
    p.set_language(&g::language_perl().into()).unwrap();
    let src = "use strict;\nmy $x = 1;\nmy @w = qw(a b c);\nprint <<EOF;\nhello\nEOF\nmy $s = \"a b\";\n$s =~ s/a/b/g;\nif ($s =~ m{b}) { print q(x), qq{y {z} $x}; }\n$s =~ tr/a-z/A-Z/;\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "source_file");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("heredoc"), "{sexp}");
}

/// The C scanner recurses once per nested start delimiter inside a quote-like operator and
/// overflows its stack on a long run; the port iterates. It must terminate on such input.
#[test]
fn deeply_nested_delimiters_do_not_overflow() {
    let mut p = Parser::new();
    p.set_language(&g::language_perl().into()).unwrap();
    let src = format!("my $x = qq{{{}", "{".repeat(300_000));
    // Unterminated, so the parse ends in an error; the point is that it returns.
    assert!(p.parse(&src, None).is_some());
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_perl().into()).unwrap();
    let golden = include_str!("golden/perl.golden");
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
            "perl: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "perl: only {n} golden cases");
}

#[test]
fn bundled_data_is_present() {
    assert!(g::perl::NODE_TYPES.trim_start().starts_with('['));
    let _lang: Language = g::perl::LANGUAGE.into();
}
