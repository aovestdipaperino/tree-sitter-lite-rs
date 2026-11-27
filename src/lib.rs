//! Pure-Rust tree-sitter grammars: no C sources, no `build.rs`.
//!
//! Parse tables and lexers are transpiled from the upstream generated `parser.c` by `c2rs`;
//! external scanners are hand-ported. Each grammar exports the usual `tree_sitter_<name>` symbol.
//!
//! Grammars are selected with cargo features: one `lang-<name>` per language, and three tiers that
//! bundle them (`lite`, the default; `medium`, which includes `lite`; and `full`, which includes
//! `medium`). [`all_languages`] lists whatever is enabled.
#![allow(clippy::all)]
pub mod abi;
#[cfg(feature = "lang-bash")]
pub mod bash;
#[cfg(feature = "lang-c")]
pub mod c;
#[cfg(feature = "lang-c_sharp")]
pub mod c_sharp;
#[cfg(feature = "lang-cpp")]
pub mod cpp;
#[cfg(feature = "lang-dart")]
pub mod dart;
#[cfg(feature = "lang-go")]
pub mod go;
#[cfg(feature = "lang-java")]
pub mod java;
#[cfg(feature = "lang-javascript")]
pub mod javascript;
#[cfg(feature = "lang-kotlin")]
pub mod kotlin;
#[cfg(feature = "lang-lua")]
pub mod lua;
#[cfg(feature = "lang-php")]
pub mod php;
#[cfg(feature = "lang-python")]
pub mod python;
#[cfg(feature = "lang-ruby")]
pub mod ruby;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-scala")]
pub mod scala;
#[cfg(feature = "lang-tsx")]
pub mod tsx;
#[cfg(feature = "lang-typescript")]
pub mod typescript;

use tree_sitter_language::LanguageFn;

#[cfg(feature = "lang-c")]
pub fn language_c() -> LanguageFn {
    c::LANGUAGE
}

#[cfg(feature = "lang-go")]
pub fn language_go() -> LanguageFn {
    go::LANGUAGE
}

#[cfg(feature = "lang-java")]
pub fn language_java() -> LanguageFn {
    java::LANGUAGE
}

#[cfg(feature = "lang-javascript")]
pub fn language_javascript() -> LanguageFn {
    javascript::LANGUAGE
}

#[cfg(feature = "lang-lua")]
pub fn language_lua() -> LanguageFn {
    lua::LANGUAGE
}

#[cfg(feature = "lang-python")]
pub fn language_python() -> LanguageFn {
    python::LANGUAGE
}

#[cfg(feature = "lang-rust")]
pub fn language_rust() -> LanguageFn {
    rust::LANGUAGE
}

#[cfg(feature = "lang-tsx")]
pub fn language_tsx() -> LanguageFn {
    tsx::LANGUAGE
}

#[cfg(feature = "lang-typescript")]
pub fn language_typescript() -> LanguageFn {
    typescript::LANGUAGE_TYPESCRIPT
}

#[cfg(feature = "lang-dart")]
pub fn language_dart() -> LanguageFn {
    dart::LANGUAGE
}

#[cfg(feature = "lang-cpp")]
pub fn language_cpp() -> LanguageFn {
    cpp::LANGUAGE
}

#[cfg(feature = "lang-scala")]
pub fn language_scala() -> LanguageFn {
    scala::LANGUAGE
}

#[cfg(feature = "lang-kotlin")]
pub fn language_kotlin() -> LanguageFn {
    kotlin::LANGUAGE
}

#[cfg(feature = "lang-ruby")]
pub fn language_ruby() -> LanguageFn {
    ruby::LANGUAGE
}

#[cfg(feature = "lang-c_sharp")]
pub fn language_c_sharp() -> LanguageFn {
    c_sharp::LANGUAGE
}

#[cfg(feature = "lang-bash")]
pub fn language_bash() -> LanguageFn {
    bash::LANGUAGE
}

#[cfg(feature = "lang-php")]
pub fn language_php() -> LanguageFn {
    php::LANGUAGE_PHP
}

/// The enabled languages as `(name, language)` pairs.
pub fn all_languages() -> Vec<(&'static str, LanguageFn)> {
    #[allow(unused_mut)]
    let mut langs = Vec::new();
    #[cfg(feature = "lang-c")]
    langs.push(("c", language_c()));
    #[cfg(feature = "lang-go")]
    langs.push(("go", language_go()));
    #[cfg(feature = "lang-java")]
    langs.push(("java", language_java()));
    #[cfg(feature = "lang-javascript")]
    langs.push(("javascript", language_javascript()));
    #[cfg(feature = "lang-lua")]
    langs.push(("lua", language_lua()));
    #[cfg(feature = "lang-python")]
    langs.push(("python", language_python()));
    #[cfg(feature = "lang-rust")]
    langs.push(("rust", language_rust()));
    #[cfg(feature = "lang-tsx")]
    langs.push(("tsx", language_tsx()));
    #[cfg(feature = "lang-typescript")]
    langs.push(("typescript", language_typescript()));
    #[cfg(feature = "lang-dart")]
    langs.push(("dart", language_dart()));
    #[cfg(feature = "lang-cpp")]
    langs.push(("cpp", language_cpp()));
    #[cfg(feature = "lang-scala")]
    langs.push(("scala", language_scala()));
    #[cfg(feature = "lang-kotlin")]
    langs.push(("kotlin", language_kotlin()));
    #[cfg(feature = "lang-ruby")]
    langs.push(("ruby", language_ruby()));
    #[cfg(feature = "lang-c_sharp")]
    langs.push(("c_sharp", language_c_sharp()));
    #[cfg(feature = "lang-bash")]
    langs.push(("bash", language_bash()));
    #[cfg(feature = "lang-php")]
    langs.push(("php", language_php()));
    langs
}
