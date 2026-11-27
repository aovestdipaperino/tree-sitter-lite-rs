pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_sql() -> *const ();
}

/// The sql grammar (upstream crate `tree-sitter-sequel`, exported there as `LANGUAGE`).
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_sql) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/sql/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/sql/queries/highlights.scm");

/// `queries/indents.scm` from the released upstream crate (not exported by it).
pub const INDENTS_QUERY: &str = include_str!("../../data/sql/queries/indents.scm");
