pub mod parser;
pub mod scanner;

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_php() -> *const ();
}

/// The php grammar (the upstream crate's `php` grammar, which accepts text before `<?php`),
/// shaped like the upstream crate's constant. The `php_only` grammar is not included.
pub const LANGUAGE_PHP: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_php) };

/// The php grammar's `node-types.json` (from the released upstream crate).
pub const PHP_NODE_TYPES: &str = include_str!("../../data/php/node-types.json");

/// `queries/highlights.scm` from the released upstream crate.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../data/php/queries/highlights.scm");

/// `queries/injections.scm` from the released upstream crate.
pub const INJECTIONS_QUERY: &str = include_str!("../../data/php/queries/injections.scm");

/// `queries/tags.scm` from the released upstream crate.
pub const TAGS_QUERY: &str = include_str!("../../data/php/queries/tags.scm");
