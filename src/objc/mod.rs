pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_objc() -> *const ();
}

/// The objc grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_objc) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/objc/node-types.json");

/// `queries/folds.scm` from the released upstream crate.
pub const FOLDS_QUERY: &str = include_str!("../../data/objc/queries/folds.scm");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/objc/queries/highlights.scm");

/// `queries/indents.scm` from the released upstream crate.
pub const INDENTS_QUERY: &str = include_str!("../../data/objc/queries/indents.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/objc/queries/injections.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/objc/queries/locals.scm");
