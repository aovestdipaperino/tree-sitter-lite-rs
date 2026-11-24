pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_tsx() -> *const ();
}

/// The TSX grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };

/// The same grammar under the name the upstream `tree-sitter-typescript` crate uses.
pub const LANGUAGE_TSX: LanguageFn = LANGUAGE;

/// The grammar's `node-types.json` (from the released upstream crate).
pub const TSX_NODE_TYPES: &str = include_str!("../../data/tsx/node-types.json");
