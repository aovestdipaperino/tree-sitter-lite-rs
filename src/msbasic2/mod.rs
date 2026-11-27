pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_msbasic2() -> *const ();
}

/// The msbasic2 grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_msbasic2) };

/// The grammar's `node-types.json` (from the released upstream crate's `grammar-src/`).
pub const NODE_TYPES: &str = include_str!("../../data/msbasic2/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/msbasic2/queries/highlights.scm");
