pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_typescript() -> *const ();
}

/// The typescript grammar, shaped like the upstream `tree-sitter-typescript` crate's constant.
pub const LANGUAGE_TYPESCRIPT: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_typescript) };
