#![cfg(feature = "lite")]
use tree_sitter::Parser;
use tree_sitter_grammars_rs as g;

fn check(lang: tree_sitter_language::LanguageFn, src: &str, expect_kind: &str) {
    let mut p = Parser::new();
    p.set_language(&lang.into()).unwrap();
    let t = p.parse(src, None).unwrap();
    assert!(
        !t.root_node().has_error(),
        "parse error in {src:?}: {}",
        t.root_node().to_sexp()
    );
    assert_eq!(t.root_node().kind(), expect_kind);
}

#[test]
fn all_lite_grammars_parse() {
    check(
        g::language_javascript(),
        "const a = `x${b}`; x = a ?? b\nlet y = 1\n",
        "program",
    );
    check(
        g::language_typescript(),
        "interface A<T> { x?: T }\nlet a: A<number> = { x: 1 }\n",
        "program",
    );
    check(g::language_tsx(), "const e = <div>{a}</div>;\n", "program");
    check(
        g::language_python(),
        "def f(x):\n    return f\"{x}\" + 'a'\n\nclass A:\n    pass\n",
        "module",
    );
    check(
        g::language_rust(),
        "fn main() { let r = r#\"a\"#; /* /* n */ */ let f = 1.5e3f32; }\n",
        "source_file",
    );
    check(
        g::language_go(),
        "package main\nfunc main() { println(\"hi\") }\n",
        "source_file",
    );
    check(
        g::language_java(),
        "class A { int f(int x) { return x + 1; } }\n",
        "program",
    );
    check(
        g::language_c(),
        "int main(void) { return 0; }\n",
        "translation_unit",
    );
}
