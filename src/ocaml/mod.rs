pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_ocaml() -> *const ();
}

/// The ocaml grammar (the upstream crate's `ocaml` grammar), shaped like the upstream crate's
/// constant. The upstream `LANGUAGE_OCAML_INTERFACE` and `LANGUAGE_OCAML_TYPE` grammars are not
/// included.
pub const LANGUAGE_OCAML: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_ocaml) };

/// The ocaml grammar's `node-types.json` (from the released upstream crate).
pub const OCAML_NODE_TYPES: &str = include_str!("../../data/ocaml/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/ocaml/queries/highlights.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/ocaml/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/ocaml/queries/tags.scm");
