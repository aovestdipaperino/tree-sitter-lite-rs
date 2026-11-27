pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_COBOL() -> *const ();
}

/// The COBOL grammar (yutaro-sakamoto/tree-sitter-cobol at 550020d), shaped like the upstream
/// crate's `language()`, as a [`LanguageFn`].
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_COBOL) };

/// The grammar's `node-types.json` (identical in the vendored copy and upstream). The upstream
/// crate exports no queries.
pub const NODE_TYPES: &str = include_str!("../../data/cobol/node-types.json");
