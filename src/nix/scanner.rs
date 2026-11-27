//! Rust port of tree-sitter-nix's external scanner (src/scanner.c, tree-sitter-nix 0.3.0).
//!
//! The scanner is stateless (create returns NULL, serialize writes 0 bytes). It uses no locale
//! functions: every character class is spelled out explicitly in the C and here.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const STRING_FRAGMENT: usize = 0;
const INDENTED_STRING_FRAGMENT: usize = 1;
const PATH_START: usize = 2;
const PATH_FRAGMENT: usize = 3;
const DOLLAR_ESCAPE: usize = 4;
const INDENTED_DOLLAR_ESCAPE: usize = 5;

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

const fn ch(c: u8) -> i32 {
    c as i32
}

unsafe fn scan_dollar_escape(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = DOLLAR_ESCAPE as u16;
    advance(lexer);
    mark_end(lexer);
    (*lexer).lookahead == ch(b'$')
}

unsafe fn scan_indented_dollar_escape(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = INDENTED_DOLLAR_ESCAPE as u16;
    advance(lexer);
    mark_end(lexer);
    if (*lexer).lookahead == ch(b'$') {
        return true;
    }
    if (*lexer).lookahead == ch(b'\\') {
        advance(lexer);
        if (*lexer).lookahead == ch(b'$') {
            mark_end(lexer);
            return true;
        }
    }
    false
}

unsafe fn scan_string_fragment(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = STRING_FRAGMENT as u16;
    let mut has_content = false;
    loop {
        mark_end(lexer);
        match (*lexer).lookahead {
            c if c == ch(b'"') || c == ch(b'\\') => return has_content,
            c if c == ch(b'$') => {
                advance(lexer);
                if (*lexer).lookahead == ch(b'{') {
                    return has_content;
                } else if (*lexer).lookahead != ch(b'"') && (*lexer).lookahead != ch(b'\\') {
                    advance(lexer);
                }
            }
            0 => return false,
            _ => advance(lexer),
        }
        has_content = true;
    }
}

unsafe fn scan_indented_string_fragment(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = INDENTED_STRING_FRAGMENT as u16;
    let mut has_content = false;
    loop {
        mark_end(lexer);
        match (*lexer).lookahead {
            c if c == ch(b'$') => {
                advance(lexer);
                if (*lexer).lookahead == ch(b'{') {
                    return has_content;
                } else if (*lexer).lookahead != ch(b'\'') {
                    advance(lexer);
                }
            }
            c if c == ch(b'\'') => {
                advance(lexer);
                if (*lexer).lookahead == ch(b'\'') {
                    return has_content;
                }
            }
            0 => return false,
            _ => advance(lexer),
        }
        has_content = true;
    }
}

fn is_path_char(c: i32) -> bool {
    (c >= ch(b'0') && c <= ch(b'9'))
        || (c >= ch(b'a') && c <= ch(b'z'))
        || (c >= ch(b'A') && c <= ch(b'Z'))
        || c == ch(b'-')
        || c == ch(b'+')
        || c == ch(b'_')
        || c == ch(b'.')
        || c == ch(b'/')
}

unsafe fn scan_path_start(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = PATH_START as u16;

    let mut have_sep = false;
    let mut have_after_sep = false;
    let mut c = (*lexer).lookahead;

    while c == ch(b' ') || c == ch(b'\n') || c == ch(b'\r') || c == ch(b'\t') {
        skip(lexer);
        c = (*lexer).lookahead;
    }

    loop {
        mark_end(lexer);
        c = (*lexer).lookahead;

        if c == ch(b'/') {
            have_sep = true;
        } else if is_path_char(c) {
            if have_sep {
                have_after_sep = true;
            }
        } else if c == ch(b'$') {
            return have_sep;
        } else {
            return have_after_sep;
        }

        advance(lexer);
    }
}

unsafe fn scan_path_fragment(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = PATH_FRAGMENT as u16;
    let mut has_content = false;
    loop {
        mark_end(lexer);
        if !is_path_char((*lexer).lookahead) {
            return has_content;
        }
        advance(lexer);
        has_content = true;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_nix_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_nix_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);

    // Error recovery marks every symbol valid: consume nothing.
    if valid(STRING_FRAGMENT)
        && valid(INDENTED_STRING_FRAGMENT)
        && valid(PATH_START)
        && valid(PATH_FRAGMENT)
        && valid(DOLLAR_ESCAPE)
        && valid(INDENTED_DOLLAR_ESCAPE)
    {
        false
    } else if valid(STRING_FRAGMENT) {
        if (*lexer).lookahead == ch(b'\\') {
            return scan_dollar_escape(lexer);
        }
        scan_string_fragment(lexer)
    } else if valid(INDENTED_STRING_FRAGMENT) {
        if (*lexer).lookahead == ch(b'\'') {
            mark_end(lexer);
            advance(lexer);
            if (*lexer).lookahead == ch(b'\'') {
                return scan_indented_dollar_escape(lexer);
            }
        }
        scan_indented_string_fragment(lexer)
    } else if valid(PATH_FRAGMENT) && is_path_char((*lexer).lookahead) {
        scan_path_fragment(lexer)
    } else if valid(PATH_START) {
        scan_path_start(lexer)
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_nix_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_nix_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_nix_external_scanner_destroy(_payload: *mut c_void) {}
