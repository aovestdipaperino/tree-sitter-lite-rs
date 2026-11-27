//! protobuf: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-protobuf")]
use tree_sitter::{Language, Parser};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_a_proto3_file() {
    let mut p = Parser::new();
    p.set_language(&g::language_protobuf().into()).unwrap();
    let src = "syntax = \"proto3\";\n// a comment\npackage demo.v1;\nimport \"other.proto\";\noption java_package = \"com.demo\";\nenum Kind { UNKNOWN = 0; A = 1 [deprecated = true]; }\nmessage Req {\n  string name = 1;\n  repeated int32 ids = 2 [(x.y) = 3];\n  map<string, .demo.v1.Req> kids = 3;\n  oneof pick { bytes raw = 4; double num = 5; }\n  reserved 6, 8 to max;\n}\nservice Svc { rpc Get(Req) returns (stream Req); }\n";
    let tree = p.parse(src, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "{}",
        tree.root_node().to_sexp()
    );
}

#[test]
fn golden_trees_match_the_c_parser() {
    let mut p = Parser::new();
    p.set_language(&g::language_protobuf().into()).unwrap();
    let golden = include_str!("golden/protobuf.golden");
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "protobuf: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "protobuf: only {n} golden cases");
}

#[test]
fn bundled_data_is_present() {
    // The upstream crate at this commit bundles no queries; only node-types.json is exported.
    let _lang: Language = g::protobuf::LANGUAGE.into();
    assert!(g::protobuf::NODE_TYPES.trim_start().starts_with('['));
}
