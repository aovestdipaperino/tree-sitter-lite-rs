pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_perl() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for the perl grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_perl) };

/// The grammar's `node-types.json` (from the released upstream crate; it ships no query files).
pub const NODE_TYPES: &str = include_str!("../../data/perl/node-types.json");
