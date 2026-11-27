pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_lean() -> *const ();
}

/// The lean grammar, as returned by `arborium_lean::language()` upstream.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_lean) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/lean/node-types.json");

/// `queries/highlights.scm` from the released upstream crate (exported there as `HIGHLIGHTS_QUERY`).
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/lean/queries/highlights.scm");

/// The injections query (empty upstream: no injections available).
pub const INJECTIONS_QUERY: &str = "";

/// The locals query (empty upstream: no locals available).
pub const LOCALS_QUERY: &str = "";
