pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_fsharp() -> *const ();
}

/// The fsharp grammar (the upstream crate's `fsharp` grammar), shaped like the upstream crate's
/// constant. The upstream `LANGUAGE_SIGNATURE` grammar (`fsharp_signature`) is not included.
pub const LANGUAGE_FSHARP: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_fsharp) };

/// The fsharp grammar's `node-types.json` (from the released upstream crate).
pub const FSHARP_NODE_TYPES: &str = include_str!("../../data/fsharp/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/fsharp/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/fsharp/queries/injections.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/fsharp/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/fsharp/queries/tags.scm");
