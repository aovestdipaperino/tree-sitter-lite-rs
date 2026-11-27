//! markdown (block grammar): a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-markdown")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_blocks_that_use_the_scanner() {
    let mut p = Parser::new();
    p.set_language(&g::language_markdown().into()).unwrap();
    let src = "---\ntitle: x\n---\n\n# Head\n\n> quote\n> more\n\n- a\n- b\n  1. c\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n<div>\nhtml\n</div>\n\n    indented\n\n***\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "document");
    let sexp = tree.root_node().to_sexp();
    for kind in [
        "minus_metadata",
        "atx_heading",
        "block_quote",
        "list",
        "fenced_code_block",
        "pipe_table",
        "html_block",
        "indented_code_block",
        "thematic_break",
    ] {
        assert!(sexp.contains(kind), "missing {kind} in {sexp}");
    }
}

/// The C scanner copies every open block into the 1024-byte serialization buffer and overruns it
/// past 254 open blocks; the port stops at what fits. Up to 254 the result is the C's (a clean
/// tree); beyond it (where the C's own `uint8_t` block counters also wrap) the tree is degraded,
/// but parsing and incremental re-parsing must not crash or corrupt memory.
#[test]
fn deeply_nested_blocks_do_not_overrun_the_serialization_buffer() {
    let mut p = Parser::new();
    p.set_language(&g::language_markdown().into()).unwrap();
    let fits = p
        .parse(format!("{}deep\n\nafter\n", "> ".repeat(254)), None)
        .unwrap();
    assert_eq!(fits.root_node().kind(), "document");
    assert!(!fits.root_node().has_error());
    let src = format!("{}deep\n\nafter\n", "> ".repeat(600));
    let tree = p.parse(&src, None).unwrap();
    let mut edited = tree.clone();
    edited.edit(&tree_sitter::InputEdit {
        start_byte: src.len() - 6,
        old_end_byte: src.len() - 6,
        new_end_byte: src.len() - 5,
        start_position: tree_sitter::Point::new(2, 0),
        old_end_position: tree_sitter::Point::new(2, 0),
        new_end_position: tree_sitter::Point::new(2, 1),
    });
    let src2 = format!("{}deep\n\nxafter\n", "> ".repeat(600));
    let tree2 = p.parse(&src2, Some(&edited)).unwrap();
    assert_eq!(tree2.root_node().byte_range().end, src2.len());
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_markdown().into()).unwrap();
    let golden = include_str!("golden/markdown.golden");
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
            "markdown: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "markdown: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::markdown::LANGUAGE.into();
    assert!(g::markdown::NODE_TYPES_BLOCK.trim_start().starts_with('['));
    Query::new(&lang, g::markdown::HIGHLIGHT_QUERY_BLOCK)
        .unwrap_or_else(|e| panic!("markdown highlights query: {e}"));
    Query::new(&lang, g::markdown::INJECTION_QUERY_BLOCK)
        .unwrap_or_else(|e| panic!("markdown injections query: {e}"));
}
