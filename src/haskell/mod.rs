pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_haskell() -> *const ();
}

/// The Haskell grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_haskell) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/haskell/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/haskell/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/haskell/queries/injections.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/haskell/queries/locals.scm");
