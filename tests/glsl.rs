//! glsl: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-glsl")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_a_shader() {
    let mut p = Parser::new();
    p.set_language(&g::language_glsl().into()).unwrap();
    let src = "#version 450\nlayout(location = 0) in vec3 pos;\nuniform mat4 mvp;\nvoid main() {\n    gl_Position = mvp * vec4(pos, 1.0);\n}\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
    assert_eq!(tree.root_node().kind(), "translation_unit");
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_glsl().into()).unwrap();
    let golden = include_str!("golden/glsl.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "glsl: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "glsl: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::glsl::LANGUAGE_GLSL.into();
    assert!(g::glsl::NODE_TYPES.trim_start().starts_with('['));
    Query::new(&lang, g::glsl::HIGHLIGHTS_QUERY)
        .unwrap_or_else(|e| panic!("glsl highlights query: {e}"));
}
