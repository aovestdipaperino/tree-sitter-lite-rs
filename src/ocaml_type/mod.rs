pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_ocaml_type() -> *const ();
}

/// The upstream crate's `type` grammar (OCaml type expressions), shaped like the upstream crate's constant.
/// Its external scanner is the one of the `ocaml` grammar (shared, see `scanner.rs`).
pub const LANGUAGE_OCAML_TYPE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_ocaml_type) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const TYPE_NODE_TYPES: &str = include_str!("../../data/ocaml_type/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/ocaml_type/queries/highlights.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/ocaml_type/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/ocaml_type/queries/tags.scm");
