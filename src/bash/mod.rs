pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_bash() -> *const ();
}

/// The bash grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_bash) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/bash/node-types.json");

/// `queries/highlights.scm` from the released upstream crate (exported there as `HIGHLIGHT_QUERY`).
pub const HIGHLIGHT_QUERY: &str = include_str!("../../data/bash/queries/highlights.scm");
