pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_php_only() -> *const ();
}

/// The upstream crate's `php_only` grammar (php without the text before `<?php`), shaped like the upstream crate's constant.
/// Its external scanner is the one of the `php` grammar (shared, see `scanner.rs`).
pub const LANGUAGE_PHP_ONLY: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_php_only) };

/// The grammar's `node-types.json` (from the released upstream crate).
pub const PHP_ONLY_NODE_TYPES: &str = include_str!("../../data/php_only/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/php_only/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/php_only/queries/injections.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/php_only/queries/tags.scm");
