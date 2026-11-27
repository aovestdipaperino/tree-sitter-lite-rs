//! Targeted regressions and behaviours that exercise each ported scanner and the transpiled lexers.
#![cfg(feature = "lite")]
use tree_sitter::{InputEdit, Parser, Point};
use tree_sitter_grammars_rs as g;

fn sexp(lang: tree_sitter_language::LanguageFn, src: &str) -> String {
    let mut p = Parser::new();
    p.set_language(&lang.into()).unwrap();
    p.parse(src, None).unwrap().root_node().to_sexp()
}
fn ok(lang: tree_sitter_language::LanguageFn, src: &str) -> String {
    let s = sexp(lang, src);
    assert!(
        !s.contains("ERROR") && !s.contains("MISSING"),
        "unexpected error in {src:?}: {s}"
    );
    s
}

/// Regression: character ranges are 32-bit. A transpiler bug truncated them to 16 bits, which made
/// non-BMP characters (emoji) in identifiers a syntax error.
#[test]
fn non_bmp_characters_in_identifiers() {
    for src in ["x\u{1f600}", "r\u{1f600}", "let \u{1f600} = 1;"] {
        assert!(
            !sexp(g::language_javascript(), src).contains("ERROR"),
            "{src:?}"
        );
    }
}

#[test]
fn javascript_scanner() {
    let js = g::language_javascript;
    ok(js(), "a = `x${b}y\\n${`${c}`}`;");
    ok(js(), "a\n++b\n");
    ok(js(), "x = a ? b : c; y = a?.b; z = a ?? b;");
    ok(js(), "a = b\n/* c */\n(d)\n");
    ok(js(), "<!-- html comment\nx = 1\n--> also\n");
    ok(
        js(),
        "const e = <div a=\"1\">text {x} &amp; <b/>\n  more</div>;",
    );
    ok(js(), "x = a\nin b; y = a\ninstanceof b");
    // automatic semicolon before a decimal literal is inserted, before a call is not
    assert_ne!(ok(js(), "a\n.5"), ok(js(), "a\n.b"));
}

#[test]
fn typescript_scanner() {
    let ts = g::language_typescript;
    ok(ts(), "function f(a?: number, b?) {}");
    ok(ts(), "type F = ({a}: {a: number}) => number;");
    ok(ts(), "let x = a ? b : c;\nlet y = a?.b;");
    ok(
        ts(),
        "abstract class A { abstract f(): void\n g(): void {} }",
    );
    ok(ts(), "function f(): void\nfunction f() {}\n");
    ok(g::language_tsx(), "const e = <T,>(x: T) => <div>{x}</div>;");
}

#[test]
fn python_scanner() {
    let py = g::language_python;
    ok(py(), "if a:\n    b\n    if c:\n        d\ne\n");
    ok(py(), "if a:\n    b\n  # comment\n    c\n");
    ok(py(), "x = f\"{a}{{b}}{c!r:>{w}}\"\n");
    ok(
        py(),
        "x = rb'\\d' + br\"\\x\" + '''a\nb''' + \"\"\"c\"\"\"\n",
    );
    ok(py(), "x = (1,\n  2)\nif x:\n\ty\n");
    ok(py(), "x = 1 \\\n  + 2\n");
    ok(py(), "try:\n    pass\nexcept E:\n    pass\n");
    ok(py(), "x = r\"a\\\r\nb\"\n");
    ok(py(), "x = '\\N{DASH}' + b'\\N'\n");
    // dedent to column 0 at EOF
    ok(py(), "def f():\n    return 1");
}

#[test]
fn rust_scanner() {
    let rs = g::language_rust;
    ok(
        rs(),
        "fn f() { let a = r#\"x\"y\"#; let b = br##\"z\"#\"##; let c = cr\"q\"; }",
    );
    ok(rs(), "/* a /* nested */ b */ fn f() {}");
    ok(rs(), "/** outer */ /*! inner */ fn f() {}");
    ok(rs(), "/// line doc\n//! inner doc\nfn f() {}");
    ok(
        rs(),
        "fn f() { let _ = (1.0, 1., 1e5, 1.5e-3f64, 2u8, 1.max(2), 0..1, 1_000.0_0); }",
    );
    ok(
        rs(),
        "fn f() { let s = \"a\\\"b\\\\\"; let t = \"multi\nline\"; }",
    );
}

/// Incremental re-parsing exercises scanner state serialize/deserialize (Python's indent stack,
/// Rust's raw-string hash count) via the runtime's per-token state snapshots.
fn edit_and_reparse(
    lang: tree_sitter_language::LanguageFn,
    before: &str,
    at: usize,
    del: usize,
    ins: &str,
) {
    let mut p = Parser::new();
    p.set_language(&lang.into()).unwrap();
    let mut tree = p.parse(before, None).unwrap();
    let mut after = before.to_string();
    after.replace_range(at..at + del, ins);
    let pt = |s: &str, off: usize| {
        let pre = &s[..off];
        Point::new(
            pre.matches('\n').count(),
            off - pre.rfind('\n').map_or(0, |i| i + 1),
        )
    };
    tree.edit(&InputEdit {
        start_byte: at,
        old_end_byte: at + del,
        new_end_byte: at + ins.len(),
        start_position: pt(before, at),
        old_end_position: pt(before, at + del),
        new_end_position: pt(&after, at + ins.len()),
    });
    let incremental = p.parse(&after, Some(&tree)).unwrap().root_node().to_sexp();
    let fresh = p.parse(&after, None).unwrap().root_node().to_sexp();
    assert_eq!(
        incremental, fresh,
        "incremental != fresh for edit of {before:?}"
    );
}

#[test]
fn incremental_reparse_matches_fresh_parse() {
    edit_and_reparse(
        g::language_python(),
        "def f():\n    if a:\n        b\n    c\n",
        22,
        0,
        "    ",
    );
    edit_and_reparse(g::language_python(), "x = f\"{a}\"\ny = 1\n", 6, 0, "{");
    edit_and_reparse(
        g::language_rust(),
        "fn f() { let a = r##\"x\"##; let b = 1; }",
        21,
        1,
        "",
    );
    edit_and_reparse(
        g::language_rust(),
        "/* a */ fn f() {} fn g() {}",
        3,
        0,
        "/*",
    );
    edit_and_reparse(g::language_javascript(), "a = `x${b}`;\nc = d\n", 7, 0, "`");
    edit_and_reparse(
        g::language_typescript(),
        "let a = 1\nlet b = 2\n",
        9,
        1,
        ";\n",
    );
}

#[test]
fn every_grammar_has_stable_metadata() {
    for (name, lang) in [
        ("javascript", g::language_javascript()),
        ("typescript", g::language_typescript()),
        ("tsx", g::language_tsx()),
        ("python", g::language_python()),
        ("rust", g::language_rust()),
        ("go", g::language_go()),
        ("java", g::language_java()),
        ("c", g::language_c()),
    ] {
        let l: tree_sitter::Language = lang.into();
        assert_eq!(l.abi_version(), 15, "{name}");
        assert!(l.node_kind_count() > 100, "{name}");
        assert!(l.field_count() > 0, "{name}");
    }
}

#[test]
fn empty_and_pathological_inputs_do_not_panic() {
    let langs = [
        g::language_javascript(),
        g::language_typescript(),
        g::language_tsx(),
        g::language_python(),
        g::language_rust(),
        g::language_go(),
        g::language_java(),
        g::language_c(),
    ];
    let inputs = [
        "",
        "\n",
        "\0",
        "\u{ffff}",
        "\u{10ffff}",
        "((((((((((((((((((((",
        "`${`${`${",
        "\"\"\"",
        "r###\"",
        "/*",
        "\u{feff}",
        "\r\n\r\n",
        &"a ".repeat(5000),
    ];
    for lang in langs {
        for src in inputs {
            let mut p = Parser::new();
            p.set_language(&lang.into()).unwrap();
            let _ = p.parse(src, None).unwrap();
        }
    }
}
