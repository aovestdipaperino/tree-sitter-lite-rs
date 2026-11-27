pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_protobuf() -> *const ();
}

/// The protobuf grammar (yusdacra/tree-sitter-protobuf at 7f873da), shaped like the upstream
/// crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_protobuf) };

/// The grammar's `node-types.json` (identical in the vendored copy and upstream). The upstream
/// crate at this commit bundles no queries.
pub const NODE_TYPES: &str = include_str!("../../data/protobuf/node-types.json");
