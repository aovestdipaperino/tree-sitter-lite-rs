//! Rust port of tree-sitter-powershell's external scanner (src/scanner.c).
//!
//! `iswspace` is ported as the C locale: ASCII only (space, \t, \n, \v, \f, \r).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const STATEMENT_TERMINATOR: usize = 0;

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

/// C locale `iswspace`.
fn iswspace(c: i32) -> bool {
    matches!(c, 0x20 | 0x09..=0x0d)
}

unsafe fn scan_statement_terminator(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    if *valid_symbols.add(STATEMENT_TERMINATOR) {
        (*lexer).result_symbol = STATEMENT_TERMINATOR as u16;
        // This token has no characters: everything is lookahead to determine its existence.
        ((*lexer).mark_end)(lexer);

        loop {
            let c = (*lexer).lookahead;
            if c == 0 {
                return true;
            }
            if c == '}' as i32 {
                return true;
            }
            if c == ';' as i32 {
                return true;
            }
            if c == ')' as i32 {
                return true;
            }
            if c == '\n' as i32 {
                return true;
            }
            if !iswspace(c) {
                return false;
            }
            skip(lexer);
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_powershell_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan_statement_terminator(payload, lexer, valid_symbols)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_powershell_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_powershell_external_scanner_destroy(_p: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_powershell_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_powershell_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}
