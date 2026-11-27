pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_dockerfile() -> *const ();
}

/// The dockerfile grammar, shaped like the other languages' constants (the upstream crate 0.2.0
/// exports a `language()` function instead).
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_dockerfile) };

/// The grammar's `node-types.json` (from tag v0.2.0 of camdencheek/tree-sitter-dockerfile).
pub const NODE_TYPES: &str = include_str!("../../data/dockerfile/node-types.json");
