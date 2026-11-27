//! Rust port of tree-sitter-dart-orchard's external scanner (src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const TEMPLATE_CHARS_SINGLE: usize = 0;
const TEMPLATE_CHARS_DOUBLE: usize = 1;
const TEMPLATE_CHARS_SINGLE_SINGLE: usize = 2;
const TEMPLATE_CHARS_DOUBLE_SINGLE: usize = 3;
const TEMPLATE_CHARS_RAW_SLASH: usize = 4;
const BLOCK_COMMENT: usize = 5;
const DOCUMENTATION_BLOCK_COMMENT: usize = 6;

/// `iswspace` in the C locale, which is what a program that never calls `setlocale` gets.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip_ws(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn scan_multiline_comments(lexer: *mut TSLexer) -> bool {
    let mut documentation_comment = false;
    advance(lexer);
    if (*lexer).lookahead != '*' as i32 {
        return false;
    }
    advance(lexer);
    if (*lexer).lookahead == '*' as i32 {
        documentation_comment = true;
    }

    let mut after_star = false;
    // `unsigned nesting_depth`
    let mut nesting_depth: u32 = 1;
    loop {
        match (*lexer).lookahead {
            0 => return false,
            42 => {
                // '*'
                advance(lexer);
                after_star = true;
            }
            47 => {
                // '/'
                if after_star {
                    advance(lexer);
                    after_star = false;
                    nesting_depth = nesting_depth.wrapping_sub(1);
                    if nesting_depth == 0 {
                        if !documentation_comment {
                            (*lexer).result_symbol = BLOCK_COMMENT as u16;
                        } else {
                            (*lexer).result_symbol = DOCUMENTATION_BLOCK_COMMENT as u16;
                        }
                        return true;
                    }
                } else {
                    advance(lexer);
                    after_star = false;
                    if (*lexer).lookahead == '*' as i32 {
                        nesting_depth = nesting_depth.wrapping_add(1);
                        advance(lexer);
                    }
                }
            }
            _ => {
                advance(lexer);
                after_star = false;
            }
        }
    }
}

unsafe fn scan_templates(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    if valid(TEMPLATE_CHARS_DOUBLE) {
        (*lexer).result_symbol = TEMPLATE_CHARS_DOUBLE as u16;
    } else if valid(TEMPLATE_CHARS_SINGLE) {
        (*lexer).result_symbol = TEMPLATE_CHARS_SINGLE as u16;
    } else if valid(TEMPLATE_CHARS_SINGLE_SINGLE) {
        (*lexer).result_symbol = TEMPLATE_CHARS_SINGLE_SINGLE as u16;
    } else {
        (*lexer).result_symbol = TEMPLATE_CHARS_DOUBLE_SINGLE as u16;
    }
    let mut has_content = false;
    loop {
        ((*lexer).mark_end)(lexer);
        match (*lexer).lookahead {
            39 | 34 => return has_content, // '\'' '"'
            10 => {
                if valid(TEMPLATE_CHARS_DOUBLE_SINGLE) || valid(TEMPLATE_CHARS_SINGLE_SINGLE) {
                    return false;
                }
                advance(lexer);
            }
            0 => return false,
            36 => return has_content, // '$'
            92 => {
                // '\\'
                if valid(TEMPLATE_CHARS_RAW_SLASH) {
                    (*lexer).result_symbol = TEMPLATE_CHARS_RAW_SLASH as u16;
                    advance(lexer);
                } else {
                    return has_content;
                }
            }
            _ => advance(lexer),
        }
        has_content = true;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dart_orchard_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dart_orchard_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dart_orchard_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dart_orchard_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dart_orchard_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    if valid(TEMPLATE_CHARS_DOUBLE)
        || valid(TEMPLATE_CHARS_SINGLE)
        || valid(TEMPLATE_CHARS_DOUBLE_SINGLE)
        || valid(TEMPLATE_CHARS_SINGLE_SINGLE)
    {
        return scan_templates(lexer, valid_symbols);
    }
    while is_wspace((*lexer).lookahead) {
        skip_ws(lexer);
    }

    if (*lexer).lookahead == '/' as i32 {
        return scan_multiline_comments(lexer);
    }
    false
}
