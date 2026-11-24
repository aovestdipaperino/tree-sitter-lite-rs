//! Golden tests: syntax trees recorded from the original C parsers (tree-sitter 0.26,
//! upstream grammars regenerated at ABI 15). Each line is `<hex input>\t<s-expression>`.
//! Inputs: grammar corpus + examples, real-world files, and random token soup.
use tree_sitter::Parser;
use tree_sitter_lite_rs as g;

fn unhex(h: &str) -> String {
    let bytes: Vec<u8> = (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

fn run(name: &str, lang: tree_sitter_language::LanguageFn, golden: &str) {
    let mut p = Parser::new();
    p.set_language(&lang.into()).unwrap();
    let mut n = 0;
    for (ln, line) in golden.lines().enumerate() {
        let (hex, expected) = line.split_once('\t').unwrap();
        let src = unhex(hex);
        let got = p.parse(&src, None).unwrap().root_node().to_sexp();
        assert_eq!(
            got, expected,
            "{name}: golden case {ln} differs; input = {src:?}"
        );
        n += 1;
    }
    assert!(n > 150, "{name}: only {n} golden cases");
}

macro_rules! golden {
    ($test:ident, $lang:ident, $file:literal) => {
        #[test]
        fn $test() {
            run(
                $file,
                g::$lang(),
                include_str!(concat!("golden/", $file, ".golden")),
            );
        }
    };
}
golden!(javascript, language_javascript, "javascript");
golden!(typescript, language_typescript, "typescript");
golden!(tsx, language_tsx, "tsx");
golden!(python, language_python, "python");
golden!(rust, language_rust, "rust");
golden!(go, language_go, "go");
golden!(java, language_java, "java");
golden!(c, language_c, "c");
