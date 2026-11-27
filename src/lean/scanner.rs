//! Rust port of the lean grammar's external scanner (grammar/scanner.c of arborium-lean 2.18.1,
//! identical to src/scanner.c of Julian/tree-sitter-lean @ efe6b87).
//!
//! The scanner is stateless (create returns NULL, serialize writes nothing) and uses no locale
//! functions: it only compares the lookahead against '\n' and the end-of-input marker 0.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const NEWLINE: usize = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lean_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lean_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lean_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lean_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lean_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    if *valid_symbols.add(NEWLINE) {
        let l = &mut *lexer;
        if l.lookahead == '\n' as i32 || l.lookahead == 0 {
            (l.advance)(lexer, true);
            (*lexer).result_symbol = NEWLINE as u16;
            return true;
        }
    }
    false
}
