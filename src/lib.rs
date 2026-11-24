//! Pure-Rust tree-sitter grammars (lite tier): no C sources, no `build.rs`.
//! Parse tables and lexers are transpiled from the upstream generated `parser.c` by `c2rs`;
//! external scanners are hand-ported. Each grammar exports the usual `tree_sitter_<name>` symbol.
#![allow(clippy::all)]
pub mod abi;
pub mod c;
pub mod go;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod tsx;
pub mod typescript;

use tree_sitter_language::LanguageFn;

pub fn language_c() -> LanguageFn {
    c::LANGUAGE
}
pub fn language_go() -> LanguageFn {
    go::LANGUAGE
}
pub fn language_java() -> LanguageFn {
    java::LANGUAGE
}
pub fn language_javascript() -> LanguageFn {
    javascript::LANGUAGE
}
pub fn language_python() -> LanguageFn {
    python::LANGUAGE
}
pub fn language_rust() -> LanguageFn {
    rust::LANGUAGE
}
pub fn language_tsx() -> LanguageFn {
    tsx::LANGUAGE
}
pub fn language_typescript() -> LanguageFn {
    typescript::LANGUAGE_TYPESCRIPT
}
