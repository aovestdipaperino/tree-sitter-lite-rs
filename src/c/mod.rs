pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_c() -> *const ();
}

/// The c grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_c) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/c/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHT_QUERY: &str = include_str!("../../data/c/queries/highlights.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/c/queries/tags.scm");
