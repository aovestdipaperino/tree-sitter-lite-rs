//! php_only: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-php_only")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_heredoc_classes_and_strings() {
    let mut p = Parser::new();
    p.set_language(&g::language_php_only().into()).unwrap();
    let src = "<?php\nnamespace App;\nclass A {\n    public function f(int $x): string {\n        $s = <<<EOT\n        v: {$x} $x\n        EOT;\n        return \"a $s\";\n    }\n}\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "program");
    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("heredoc"), "{sexp}");
    assert!(sexp.contains("class_declaration"), "{sexp}");
    assert!(sexp.contains("encapsed_string"), "{sexp}");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_php_only().into()).unwrap();
    let golden = include_str!("golden/php_only.golden");
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
            "php_only: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "php_only: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::php_only::LANGUAGE_PHP_ONLY.into();
    assert!(
        g::php_only::PHP_ONLY_NODE_TYPES
            .trim_start()
            .starts_with('[')
    );
    for (name, q) in [
        ("highlights", g::php_only::HIGHLIGHTS_QUERY),
        ("injections", g::php_only::INJECTIONS_QUERY),
        ("tags", g::php_only::TAGS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("php_only {name} query: {e}"));
    }
}
