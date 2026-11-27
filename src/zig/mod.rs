pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_zig() -> *const ();
}

/// The zig grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_zig) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/zig/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/zig/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/zig/queries/injections.scm");
