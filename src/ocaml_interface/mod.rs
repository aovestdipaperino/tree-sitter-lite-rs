pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_ocaml_interface() -> *const ();
}

/// The upstream crate's `interface` grammar (OCaml `.mli` files), shaped like the upstream crate's constant.
/// Its external scanner is the one of the `ocaml` grammar (shared, see `scanner.rs`).
pub const LANGUAGE_OCAML_INTERFACE: LanguageFn =
    unsafe { LanguageFn::from_raw(tree_sitter_ocaml_interface) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const INTERFACE_NODE_TYPES: &str = include_str!("../../data/ocaml_interface/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str =
    include_str!("../../data/ocaml_interface/queries/highlights.scm");

/// `queries/locals.scm` from the released upstream crate.
pub const LOCALS_QUERY: &str = include_str!("../../data/ocaml_interface/queries/locals.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/ocaml_interface/queries/tags.scm");
