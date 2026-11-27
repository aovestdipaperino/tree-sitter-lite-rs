pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_markdown_inline() -> *const ();
}

/// The markdown inline grammar, shaped like the large crate's `markdown::inline::LANGUAGE`.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_markdown_inline) };

/// The same constant under the name the upstream crate's `bindings/rust/lib.rs` gives it.
pub const INLINE_LANGUAGE: LanguageFn = LANGUAGE;

/// The grammar's `node-types.json` (tree-sitter-markdown v0.5.3), upstream name `NODE_TYPES_INLINE`.
pub const NODE_TYPES: &str = include_str!("../../data/markdown_inline/node-types.json");

/// Upstream name of [`NODE_TYPES`].
pub const NODE_TYPES_INLINE: &str = NODE_TYPES;

/// `tree-sitter-markdown-inline/queries/highlights.scm` (v0.5.3), upstream name `HIGHLIGHT_QUERY_INLINE`.
pub const HIGHLIGHTS_QUERY: &str =
    include_str!("../../data/markdown_inline/queries/highlights.scm");

/// Upstream name of [`HIGHLIGHTS_QUERY`].
pub const HIGHLIGHT_QUERY_INLINE: &str = HIGHLIGHTS_QUERY;

/// `tree-sitter-markdown-inline/queries/injections.scm` (v0.5.3), upstream name `INJECTION_QUERY_INLINE`.
pub const INJECTIONS_QUERY: &str =
    include_str!("../../data/markdown_inline/queries/injections.scm");

/// Upstream name of [`INJECTIONS_QUERY`].
pub const INJECTION_QUERY_INLINE: &str = INJECTIONS_QUERY;
