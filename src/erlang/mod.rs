pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_erlang() -> *const ();
}

/// The erlang grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_erlang) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/erlang/node-types.json");

/// `queries/highlights.scm` from the released upstream crate (exported there as `HIGHLIGHTS_QUERY`).
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/erlang/queries/highlights.scm");
