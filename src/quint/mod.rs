pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_quint() -> *const ();
}

/// The quint grammar, shaped like the large crate's `quint::LANGUAGE`.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_quint) };

/// The grammar's `node-types.json` (from zdavison/tree-sitter-quint @ 5155d17).
pub const NODE_TYPES: &str = include_str!("../../data/quint/node-types.json");

/// `queries/highlights.scm` from zdavison/tree-sitter-quint @ 5155d17.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/quint/queries/highlights.scm");
