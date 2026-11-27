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
#[cfg(feature = "lang-batch")]
pub mod batch;
#[cfg(feature = "lang-c")]
pub mod c;
#[cfg(feature = "lang-c_sharp")]
pub mod c_sharp;
#[cfg(feature = "lang-clojure")]
pub mod clojure;
#[cfg(feature = "lang-cobol")]
pub mod cobol;
#[cfg(feature = "lang-cpp")]
pub mod cpp;
#[cfg(feature = "lang-dart")]
pub mod dart;
#[cfg(feature = "lang-dockerfile")]
pub mod dockerfile;
#[cfg(feature = "lang-elixir")]
pub mod elixir;
#[cfg(feature = "lang-erlang")]
pub mod erlang;
#[cfg(feature = "lang-fortran")]
pub mod fortran;
#[cfg(feature = "lang-fsharp")]
pub mod fsharp;
#[cfg(feature = "lang-glsl")]
pub mod glsl;
#[cfg(feature = "lang-go")]
pub mod go;
#[cfg(feature = "lang-gwbasic")]
pub mod gwbasic;
#[cfg(feature = "lang-java")]
pub mod java;
#[cfg(feature = "lang-javascript")]
pub mod javascript;
#[cfg(feature = "lang-julia")]
pub mod julia;
#[cfg(feature = "lang-kotlin")]
pub mod kotlin;
#[cfg(feature = "lang-lean")]
pub mod lean;
#[cfg(feature = "lang-lua")]
pub mod lua;
#[cfg(feature = "lang-markdown")]
pub mod markdown;
#[cfg(feature = "lang-markdown_inline")]
pub mod markdown_inline;
#[cfg(feature = "lang-msbasic2")]
pub mod msbasic2;
#[cfg(feature = "lang-nix")]
pub mod nix;
#[cfg(feature = "lang-objc")]
pub mod objc;
#[cfg(feature = "lang-ocaml")]
pub mod ocaml;
#[cfg(feature = "lang-pascal")]
pub mod pascal;
#[cfg(feature = "lang-perl")]
pub mod perl;
#[cfg(feature = "lang-php")]
pub mod php;
#[cfg(feature = "lang-powershell")]
pub mod powershell;
#[cfg(feature = "lang-protobuf")]
pub mod protobuf;
#[cfg(feature = "lang-python")]
pub mod python;
#[cfg(feature = "lang-qbasic")]
pub mod qbasic;
#[cfg(feature = "lang-quint")]
pub mod quint;
#[cfg(feature = "lang-r")]
pub mod r;
#[cfg(feature = "lang-ruby")]
pub mod ruby;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-scala")]
pub mod scala;
#[cfg(feature = "lang-sql")]
pub mod sql;
#[cfg(feature = "lang-swift")]
pub mod swift;
#[cfg(feature = "lang-toml")]
pub mod toml;
#[cfg(feature = "lang-tsx")]
pub mod tsx;
#[cfg(feature = "lang-typescript")]
pub mod typescript;
#[cfg(feature = "lang-vbnet")]
pub mod vbnet;
#[cfg(feature = "lang-zig")]
pub mod zig;

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

#[cfg(feature = "lang-swift")]
pub fn language_swift() -> LanguageFn {
    swift::LANGUAGE
}

#[cfg(feature = "lang-qbasic")]
pub fn language_qbasic() -> LanguageFn {
    qbasic::LANGUAGE
}

#[cfg(feature = "lang-zig")]
pub fn language_zig() -> LanguageFn {
    zig::LANGUAGE
}

#[cfg(feature = "lang-pascal")]
pub fn language_pascal() -> LanguageFn {
    pascal::LANGUAGE
}

#[cfg(feature = "lang-objc")]
pub fn language_objc() -> LanguageFn {
    objc::LANGUAGE
}

#[cfg(feature = "lang-vbnet")]
pub fn language_vbnet() -> LanguageFn {
    vbnet::LANGUAGE
}

#[cfg(feature = "lang-batch")]
pub fn language_batch() -> LanguageFn {
    batch::LANGUAGE
}

#[cfg(feature = "lang-msbasic2")]
pub fn language_msbasic2() -> LanguageFn {
    msbasic2::LANGUAGE
}

#[cfg(feature = "lang-gwbasic")]
pub fn language_gwbasic() -> LanguageFn {
    gwbasic::LANGUAGE
}

#[cfg(feature = "lang-nix")]
pub fn language_nix() -> LanguageFn {
    nix::LANGUAGE
}

#[cfg(feature = "lang-glsl")]
pub fn language_glsl() -> LanguageFn {
    glsl::LANGUAGE_GLSL
}

#[cfg(feature = "lang-clojure")]
pub fn language_clojure() -> LanguageFn {
    clojure::LANGUAGE
}

#[cfg(feature = "lang-toml")]
pub fn language_toml() -> LanguageFn {
    toml::LANGUAGE
}

#[cfg(feature = "lang-powershell")]
pub fn language_powershell() -> LanguageFn {
    powershell::LANGUAGE
}

#[cfg(feature = "lang-quint")]
pub fn language_quint() -> LanguageFn {
    quint::LANGUAGE
}

#[cfg(feature = "lang-dockerfile")]
pub fn language_dockerfile() -> LanguageFn {
    dockerfile::LANGUAGE
}

#[cfg(feature = "lang-protobuf")]
pub fn language_protobuf() -> LanguageFn {
    protobuf::LANGUAGE
}

#[cfg(feature = "lang-ocaml")]
pub fn language_ocaml() -> LanguageFn {
    ocaml::LANGUAGE_OCAML
}

#[cfg(feature = "lang-erlang")]
pub fn language_erlang() -> LanguageFn {
    erlang::LANGUAGE
}

#[cfg(feature = "lang-sql")]
pub fn language_sql() -> LanguageFn {
    sql::LANGUAGE
}

#[cfg(feature = "lang-cobol")]
pub fn language_cobol() -> LanguageFn {
    cobol::LANGUAGE
}

#[cfg(feature = "lang-fsharp")]
pub fn language_fsharp() -> LanguageFn {
    fsharp::LANGUAGE_FSHARP
}

#[cfg(feature = "lang-lean")]
pub fn language_lean() -> LanguageFn {
    lean::LANGUAGE
}

#[cfg(feature = "lang-julia")]
pub fn language_julia() -> LanguageFn {
    julia::LANGUAGE
}

#[cfg(feature = "lang-elixir")]
pub fn language_elixir() -> LanguageFn {
    elixir::LANGUAGE
}

#[cfg(feature = "lang-markdown_inline")]
pub fn language_markdown_inline() -> LanguageFn {
    markdown_inline::LANGUAGE
}

#[cfg(feature = "lang-r")]
pub fn language_r() -> LanguageFn {
    r::LANGUAGE
}

#[cfg(feature = "lang-perl")]
pub fn language_perl() -> LanguageFn {
    perl::LANGUAGE
}

#[cfg(feature = "lang-fortran")]
pub fn language_fortran() -> LanguageFn {
    fortran::LANGUAGE
}

#[cfg(feature = "lang-markdown")]
pub fn language_markdown() -> LanguageFn {
    markdown::LANGUAGE
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
    #[cfg(feature = "lang-swift")]
    langs.push(("swift", language_swift()));
    #[cfg(feature = "lang-qbasic")]
    langs.push(("qbasic", language_qbasic()));
    #[cfg(feature = "lang-zig")]
    langs.push(("zig", language_zig()));
    #[cfg(feature = "lang-pascal")]
    langs.push(("pascal", language_pascal()));
    #[cfg(feature = "lang-objc")]
    langs.push(("objc", language_objc()));
    #[cfg(feature = "lang-vbnet")]
    langs.push(("vbnet", language_vbnet()));
    #[cfg(feature = "lang-batch")]
    langs.push(("batch", language_batch()));
    #[cfg(feature = "lang-msbasic2")]
    langs.push(("msbasic2", language_msbasic2()));
    #[cfg(feature = "lang-gwbasic")]
    langs.push(("gwbasic", language_gwbasic()));
    #[cfg(feature = "lang-nix")]
    langs.push(("nix", language_nix()));
    #[cfg(feature = "lang-glsl")]
    langs.push(("glsl", language_glsl()));
    #[cfg(feature = "lang-clojure")]
    langs.push(("clojure", language_clojure()));
    #[cfg(feature = "lang-toml")]
    langs.push(("toml", language_toml()));
    #[cfg(feature = "lang-powershell")]
    langs.push(("powershell", language_powershell()));
    #[cfg(feature = "lang-quint")]
    langs.push(("quint", language_quint()));
    #[cfg(feature = "lang-dockerfile")]
    langs.push(("dockerfile", language_dockerfile()));
    #[cfg(feature = "lang-protobuf")]
    langs.push(("protobuf", language_protobuf()));
    #[cfg(feature = "lang-ocaml")]
    langs.push(("ocaml", language_ocaml()));
    #[cfg(feature = "lang-erlang")]
    langs.push(("erlang", language_erlang()));
    #[cfg(feature = "lang-sql")]
    langs.push(("sql", language_sql()));
    #[cfg(feature = "lang-cobol")]
    langs.push(("cobol", language_cobol()));
    #[cfg(feature = "lang-fsharp")]
    langs.push(("fsharp", language_fsharp()));
    #[cfg(feature = "lang-lean")]
    langs.push(("lean", language_lean()));
    #[cfg(feature = "lang-julia")]
    langs.push(("julia", language_julia()));
    #[cfg(feature = "lang-elixir")]
    langs.push(("elixir", language_elixir()));
    #[cfg(feature = "lang-markdown_inline")]
    langs.push(("markdown_inline", language_markdown_inline()));
    #[cfg(feature = "lang-r")]
    langs.push(("r", language_r()));
    #[cfg(feature = "lang-perl")]
    langs.push(("perl", language_perl()));
    #[cfg(feature = "lang-fortran")]
    langs.push(("fortran", language_fortran()));
    #[cfg(feature = "lang-markdown")]
    langs.push(("markdown", language_markdown()));
    langs
}
