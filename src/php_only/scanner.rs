//! The external scanner of this grammar is the one of the `php` grammar: both grammars include the
//! same upstream `common/scanner.h` and declare the same external tokens in the same order. The
//! exported symbols carry the grammar's own prefix, so each one forwards to the sibling's port
//! instead of copying it. The `lang-php_only` feature enables `lang-php` for this reason.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use crate::php as sibling;
use core::ffi::{c_char, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_only_external_scanner_create() -> *mut c_void {
    sibling::scanner::tree_sitter_php_external_scanner_create()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_only_external_scanner_destroy(payload: *mut c_void) {
    sibling::scanner::tree_sitter_php_external_scanner_destroy(payload)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_only_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    sibling::scanner::tree_sitter_php_external_scanner_serialize(payload, buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_only_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    sibling::scanner::tree_sitter_php_external_scanner_deserialize(payload, buffer, length)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_only_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    sibling::scanner::tree_sitter_php_external_scanner_scan(payload, lexer, valid_symbols)
}
