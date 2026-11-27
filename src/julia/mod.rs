pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_julia() -> *const ();
}

/// The Julia grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_julia) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/julia/node-types.json");

/// `queries/highlights.scm` from the released upstream crate (not exported as a constant there).
pub const HIGHLIGHT_QUERY: &str = include_str!("../../data/julia/queries/highlights.scm");
