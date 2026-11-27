//! Rust port of tree-sitter-toml-ng's external scanner (src/scanner.c, tree-sitter-toml-ng 0.7.0).
//!
//! The scanner is stateless (create returns NULL, serialize writes nothing) and uses no locale
//! functions: it only compares the lookahead against fixed ASCII characters.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const LINE_ENDING_OR_EOF: usize = 0;
const MULTILINE_BASIC_STRING_CONTENT: usize = 1;
const MULTILINE_BASIC_STRING_END: usize = 2;
const MULTILINE_LITERAL_STRING_CONTENT: usize = 3;
const MULTILINE_LITERAL_STRING_END: usize = 4;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_toml_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_toml_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_toml_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_toml_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

unsafe fn scan_multiline_string_end(
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
    delimiter: i32,
    content_symbol: usize,
    end_symbol: usize,
) -> bool {
    let l = &mut *lexer;
    if !*valid_symbols.add(end_symbol) || l.lookahead != delimiter {
        return false;
    }

    (l.advance)(lexer, false);
    (l.mark_end)(lexer);

    if (*lexer).lookahead != delimiter {
        (*lexer).result_symbol = content_symbol as u16;
        return true;
    }

    (l.advance)(lexer, false);

    if (*lexer).lookahead != delimiter {
        (l.mark_end)(lexer);
        (*lexer).result_symbol = content_symbol as u16;
        return true;
    }

    (l.advance)(lexer, false);

    if (*lexer).lookahead != delimiter {
        (l.mark_end)(lexer);
        (*lexer).result_symbol = end_symbol as u16;
        return true;
    }

    (*lexer).result_symbol = content_symbol as u16;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_toml_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    if scan_multiline_string_end(
        lexer,
        valid_symbols,
        '"' as i32,
        MULTILINE_BASIC_STRING_CONTENT,
        MULTILINE_BASIC_STRING_END,
    ) || scan_multiline_string_end(
        lexer,
        valid_symbols,
        '\'' as i32,
        MULTILINE_LITERAL_STRING_CONTENT,
        MULTILINE_LITERAL_STRING_END,
    ) {
        return true;
    }

    if *valid_symbols.add(LINE_ENDING_OR_EOF) {
        (*lexer).result_symbol = LINE_ENDING_OR_EOF as u16;

        while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
            ((*lexer).advance)(lexer, true);
        }

        if (*lexer).lookahead == 0 || (*lexer).lookahead == '\n' as i32 {
            return true;
        }

        if (*lexer).lookahead == '\r' as i32 {
            ((*lexer).advance)(lexer, true);
            if (*lexer).lookahead == '\n' as i32 {
                return true;
            }
        }
    }

    false
}
