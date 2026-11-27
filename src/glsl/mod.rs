pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_glsl() -> *const ();
}

/// The glsl grammar, shaped like the upstream crate's constant.
pub const LANGUAGE_GLSL: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_glsl) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/glsl/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/glsl/queries/highlights.scm");
