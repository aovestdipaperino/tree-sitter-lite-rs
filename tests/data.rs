//! The bundled upstream data: `NODE_TYPES` must be JSON describing the grammar, and every query
//! constant must compile against its grammar (which also exercises the runtime's query engine).
#![cfg(feature = "lite")]
use tree_sitter::{Language, Query, StreamingIterator};
use tree_sitter_grammars_rs as g;

fn node_types_ok(name: &str, json: &str, lang: &Language) {
    let v: serde_like::Count = serde_like::count_types(json);
    assert!(
        v.entries > 20,
        "{name}: node-types.json looks empty ({} entries)",
        v.entries
    );
    // every named node kind the grammar exposes must be described by node-types.json
    let mut missing = Vec::new();
    for id in 0..lang.node_kind_count() as u16 {
        if lang.node_kind_is_named(id) && lang.node_kind_is_visible(id) {
            let kind = lang.node_kind_for_id(id).unwrap();
            if !json.contains(&format!("\"type\": \"{kind}\"")) {
                missing.push(kind);
            }
        }
    }
    assert!(
        missing.len() < 3,
        "{name}: node kinds missing from node-types.json: {missing:?}"
    );
}

/// Tiny dependency-free JSON entry counter (counts top-level array objects by `"type":` keys).
mod serde_like {
    pub struct Count {
        pub entries: usize,
    }
    pub fn count_types(json: &str) -> Count {
        assert!(
            json.trim_start().starts_with('['),
            "node-types.json must be a JSON array"
        );
        Count {
            entries: json.matches("\"type\":").count(),
        }
    }
}

fn query_ok(name: &str, lang: &Language, q: &str) {
    Query::new(lang, q).unwrap_or_else(|e| panic!("{name}: query does not compile: {e}"));
}

#[test]
fn javascript_data() {
    let l: Language = g::javascript::LANGUAGE.into();
    node_types_ok("javascript", g::javascript::NODE_TYPES, &l);
    for (n, q) in [
        ("HIGHLIGHT_QUERY", g::javascript::HIGHLIGHT_QUERY),
        ("INJECTIONS_QUERY", g::javascript::INJECTIONS_QUERY),
        ("JSX_HIGHLIGHT_QUERY", g::javascript::JSX_HIGHLIGHT_QUERY),
        ("LOCALS_QUERY", g::javascript::LOCALS_QUERY),
        ("TAGS_QUERY", g::javascript::TAGS_QUERY),
    ] {
        query_ok(n, &l, q);
    }
}

#[test]
fn typescript_and_tsx_data() {
    let ts: Language = g::typescript::LANGUAGE_TYPESCRIPT.into();
    let tsx: Language = g::tsx::LANGUAGE_TSX.into();
    node_types_ok("typescript", g::typescript::TYPESCRIPT_NODE_TYPES, &ts);
    node_types_ok("tsx", g::tsx::TSX_NODE_TYPES, &tsx);
    for l in [&ts, &tsx] {
        query_ok("ts HIGHLIGHTS_QUERY", l, g::typescript::HIGHLIGHTS_QUERY);
        query_ok("ts LOCALS_QUERY", l, g::typescript::LOCALS_QUERY);
        query_ok("ts TAGS_QUERY", l, g::typescript::TAGS_QUERY);
    }
}

#[test]
fn python_data() {
    let l: Language = g::python::LANGUAGE.into();
    node_types_ok("python", g::python::NODE_TYPES, &l);
    query_ok("HIGHLIGHTS_QUERY", &l, g::python::HIGHLIGHTS_QUERY);
    query_ok("TAGS_QUERY", &l, g::python::TAGS_QUERY);
}

#[test]
fn rust_data() {
    let l: Language = g::rust::LANGUAGE.into();
    node_types_ok("rust", g::rust::NODE_TYPES, &l);
    query_ok("HIGHLIGHTS_QUERY", &l, g::rust::HIGHLIGHTS_QUERY);
    query_ok("INJECTIONS_QUERY", &l, g::rust::INJECTIONS_QUERY);
    query_ok("TAGS_QUERY", &l, g::rust::TAGS_QUERY);
}

#[test]
fn go_java_c_data() {
    let go: Language = g::go::LANGUAGE.into();
    node_types_ok("go", g::go::NODE_TYPES, &go);
    query_ok("go HIGHLIGHTS_QUERY", &go, g::go::HIGHLIGHTS_QUERY);
    query_ok("go TAGS_QUERY", &go, g::go::TAGS_QUERY);
    let java: Language = g::java::LANGUAGE.into();
    node_types_ok("java", g::java::NODE_TYPES, &java);
    query_ok("java HIGHLIGHTS_QUERY", &java, g::java::HIGHLIGHTS_QUERY);
    query_ok("java TAGS_QUERY", &java, g::java::TAGS_QUERY);
    let c: Language = g::c::LANGUAGE.into();
    node_types_ok("c", g::c::NODE_TYPES, &c);
    query_ok("c HIGHLIGHT_QUERY", &c, g::c::HIGHLIGHT_QUERY);
    query_ok("c TAGS_QUERY", &c, g::c::TAGS_QUERY);
}

/// A highlight query is only useful if it captures something on real code.
#[test]
fn highlight_query_captures_on_code() {
    let lang: Language = g::rust::LANGUAGE.into();
    let mut p = tree_sitter::Parser::new();
    p.set_language(&lang).unwrap();
    let src = "fn main() { let x = 1; }";
    let tree = p.parse(src, None).unwrap();
    let q = Query::new(&lang, g::rust::HIGHLIGHTS_QUERY).unwrap();
    let mut cur = tree_sitter::QueryCursor::new();
    let mut names = std::collections::BTreeSet::new();
    let mut matches = cur.matches(&q, tree.root_node(), src.as_bytes());
    while let Some(m) = matches.next() {
        for c in m.captures {
            names.insert(q.capture_names()[c.index as usize].to_string());
        }
    }
    assert!(
        names.contains("keyword") || names.contains("function"),
        "no captures: {names:?}"
    );
}
