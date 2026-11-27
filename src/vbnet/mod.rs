pub mod parser;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_vb_dotnet() -> *const ();
}

/// The VB.NET grammar (tree-sitter-vb-dotnet), shaped like the upstream crate's constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_vb_dotnet) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const NODE_TYPES: &str = include_str!("../../data/vbnet/node-types.json");
