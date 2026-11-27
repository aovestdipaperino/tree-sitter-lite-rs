pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_elixir() -> *const ();
}

/// The Elixir grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_elixir) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/elixir/node-types.json");

/// `queries/highlights.scm` of the upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/elixir/queries/highlights.scm");
/// `queries/injections.scm` of the upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/elixir/queries/injections.scm");
/// `queries/tags.scm` of the upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/elixir/queries/tags.scm");
