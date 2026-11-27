pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_cpp() -> *const ();
}

/// The cpp grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_cpp) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/cpp/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHT_QUERY: &str = include_str!("../../data/cpp/queries/highlights.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/cpp/queries/tags.scm");
