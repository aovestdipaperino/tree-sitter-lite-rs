//! objc: a smoke parse, the golden trees recorded from the original C parser, and the bundled data.
#![cfg(feature = "lang-objc")]
use tree_sitter::{Language, Parser, Query};
use tree_sitter_grammars_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn parses_interfaces_messages_and_blocks() {
    let mut p = Parser::new();
    p.set_language(&g::language_objc().into()).unwrap();
    let src = "@interface A : NSObject\n@property (nonatomic, copy) NSString *n;\n- (void)go:(int)x;\n@end\n@implementation A\n- (void)go:(int)x {\n  [self.n length];\n  NSArray *a = @[@1, @\"s\"];\n  void (^b)(int) = ^(int i) { };\n}\n@end\n";
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
    p.set_language(&g::language_objc().into()).unwrap();
    let golden = include_str!("golden/objc.golden");
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
            "objc: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "objc: only {n} golden cases");
}

#[test]
fn bundled_queries_compile() {
    let lang: Language = g::objc::LANGUAGE.into();
    assert!(g::objc::NODE_TYPES.trim_start().starts_with('['));
    for (name, q) in [
        ("folds", g::objc::FOLDS_QUERY),
        ("highlights", g::objc::HIGHLIGHTS_QUERY),
        ("indents", g::objc::INDENTS_QUERY),
        ("injections", g::objc::INJECTIONS_QUERY),
        ("locals", g::objc::LOCALS_QUERY),
    ] {
        Query::new(&lang, q).unwrap_or_else(|e| panic!("objc {name} query: {e}"));
    }
}
