pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_tsx() -> *const ();
}

/// The tsx grammar, shaped like the upstream `tree-sitter-tsx` crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_tsx) };
