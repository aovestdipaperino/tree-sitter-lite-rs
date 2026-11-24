pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_javascript() -> *const ();
}

/// The javascript grammar, shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_javascript) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/javascript/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHT_QUERY: &str = include_str!("../../data/javascript/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/javascript/queries/injections.scm");

/// `queries/highlights-jsx.scm` from the released upstream crate.
pub const JSX_HIGHLIGHT_QUERY: &str =
    include_str!("../../data/javascript/queries/highlights-jsx.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/javascript/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/javascript/queries/tags.scm");
