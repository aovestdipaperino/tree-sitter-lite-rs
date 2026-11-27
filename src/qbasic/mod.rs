pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_qbasic() -> *const ();
}

/// The qbasic grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_qbasic) };

/// The grammar's `node-types.json` (from the released upstream crate's `grammar-src/`).
pub const NODE_TYPES: &str = include_str!("../../data/qbasic/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/qbasic/queries/highlights.scm");
