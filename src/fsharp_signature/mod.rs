pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_fsharp_signature() -> *const ();
}

/// The upstream crate's `fsharp_signature` grammar (F# `.fsi` signature files), shaped like the upstream crate's constant.
/// Its external scanner is the one of the `fsharp` grammar (shared, see `scanner.rs`).
pub const LANGUAGE_SIGNATURE: LanguageFn =
    unsafe { LanguageFn::from_raw(tree_sitter_fsharp_signature) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const SIGNATURE_NODE_TYPES: &str = include_str!("../../data/fsharp_signature/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str =
    include_str!("../../data/fsharp_signature/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str =
    include_str!("../../data/fsharp_signature/queries/injections.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/fsharp_signature/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/fsharp_signature/queries/tags.scm");
