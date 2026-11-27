//! markdown: the block grammar of tree-sitter-markdown (tree-sitter-grammars/tree-sitter-markdown
//! v0.5.3, the `tree-sitter-markdown` grammar of the `tree-sitter-md` crate). The block parser
//! produces `(document ... (inline))` nodes; the inline grammar is a separate language.
pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_markdown() -> *const ();
}

/// The block grammar, shaped like the upstream crate's `LANGUAGE` constant.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_markdown) };

/// The block grammar's `node-types.json` (upstream exports it as `NODE_TYPES_BLOCK`).
pub const NODE_TYPES_BLOCK: &str = include_str!("../../data/markdown/node-types.json");

/// `tree-sitter-markdown/queries/highlights.scm` (upstream: `HIGHLIGHT_QUERY_BLOCK`).
pub const HIGHLIGHT_QUERY_BLOCK: &str = include_str!("../../data/markdown/queries/highlights.scm");

/// `tree-sitter-markdown/queries/injections.scm` (upstream: `INJECTION_QUERY_BLOCK`).
pub const INJECTION_QUERY_BLOCK: &str = include_str!("../../data/markdown/queries/injections.scm");
