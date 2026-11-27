//! Rust port of tree-sitter-julia's external scanner (src/scanner.c).
//!
//! The scanner is stateless: create returns NULL, serialize writes nothing, deserialize reads nothing.
//! No locale function (`iswspace` and friends) is used by the C.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const BLOCK_COMMENT_REST: usize = 0;
const IMMEDIATE_PAREN: usize = 1;
const IMMEDIATE_BRACKET: usize = 2;
const IMMEDIATE_BRACE: usize = 3;
const IMMEDIATE_STRING_START: usize = 4;
const IMMEDIATE_COMMAND_START: usize = 5;
const CONTENT_CMD_1: usize = 6;
const CONTENT_CMD_1_RAW: usize = 7;
const CONTENT_CMD_3: usize = 8;
const CONTENT_CMD_3_RAW: usize = 9;
const CONTENT_STR_1: usize = 10;
const CONTENT_STR_1_RAW: usize = 11;
const CONTENT_STR_3: usize = 12;
const CONTENT_STR_3_RAW: usize = 13;
const END_CMD: usize = 14;
const END_STR: usize = 15;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_julia_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_julia_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_julia_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_julia_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

unsafe fn scan_content(
    lexer: *mut TSLexer,
    content_symbol: usize,
    end_char: i32,
    n_delim: u32,
    interp: bool,
) -> bool {
    let end_symbol = if end_char == '"' as i32 {
        END_STR
    } else {
        END_CMD
    };
    let mut has_content = false;
    loop {
        let mut next = (*lexer).lookahead;
        if next == 0 {
            break;
        }
        mark_end(lexer);
        if interp && (next == '$' as i32 || next == '\\' as i32) {
            (*lexer).result_symbol = content_symbol as u16;
            return has_content;
        } else if next == '\\' as i32 {
            // Parse backslash in raw strings (check escaped delimiters and '\\')
            advance(lexer);
            next = (*lexer).lookahead;
            if next == end_char || next == '\\' as i32 {
                (*lexer).result_symbol = content_symbol as u16;
                return has_content;
            }
        } else {
            let mut is_end_delimiter = true;
            for _ in 1..=n_delim {
                if (*lexer).lookahead == end_char {
                    advance(lexer);
                } else {
                    is_end_delimiter = false;
                    break;
                }
            }
            if is_end_delimiter {
                if has_content {
                    (*lexer).result_symbol = content_symbol as u16;
                    return true;
                } else {
                    mark_end(lexer);
                    (*lexer).result_symbol = end_symbol as u16;
                    return true;
                }
            }
        }
        advance(lexer);
        has_content = true;
    }
    false
}

unsafe fn scan_block_comment(lexer: *mut TSLexer) -> bool {
    // NOTE: The first `#=` is scanned by tree-sitter
    let mut after_eq = false;
    let mut nesting_depth: u32 = 1;
    loop {
        match (*lexer).lookahead {
            c if c == '=' as i32 => {
                advance(lexer);
                after_eq = true;
            }
            c if c == '#' as i32 => {
                advance(lexer);
                if after_eq {
                    after_eq = false;
                    nesting_depth = nesting_depth.wrapping_sub(1);
                    if nesting_depth == 0 {
                        (*lexer).result_symbol = BLOCK_COMMENT_REST as u16;
                        return true;
                    }
                } else {
                    after_eq = false;
                    if (*lexer).lookahead == '=' as i32 {
                        advance(lexer);
                        nesting_depth = nesting_depth.wrapping_add(1);
                    }
                }
            }
            0 => return false,
            _ => {
                advance(lexer);
                after_eq = false;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_julia_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    let la = (*lexer).lookahead;
    if valid(IMMEDIATE_PAREN) && la == '(' as i32 {
        (*lexer).result_symbol = IMMEDIATE_PAREN as u16;
        return true;
    } else if valid(IMMEDIATE_BRACKET) && la == '[' as i32 {
        (*lexer).result_symbol = IMMEDIATE_BRACKET as u16;
        return true;
    } else if valid(IMMEDIATE_BRACE) && la == '{' as i32 {
        (*lexer).result_symbol = IMMEDIATE_BRACE as u16;
        return true;
    } else if valid(IMMEDIATE_STRING_START) && la == '"' as i32 {
        (*lexer).result_symbol = IMMEDIATE_STRING_START as u16;
        return true;
    } else if valid(IMMEDIATE_COMMAND_START) && la == '`' as i32 {
        (*lexer).result_symbol = IMMEDIATE_COMMAND_START as u16;
        return true;
    }

    if valid(BLOCK_COMMENT_REST) && scan_block_comment(lexer) {
        return true;
    }

    let dq = '"' as i32;
    let bt = '`' as i32;
    if valid(CONTENT_STR_1) && scan_content(lexer, CONTENT_STR_1, dq, 1, true) {
        return true;
    }
    if valid(CONTENT_STR_3) && scan_content(lexer, CONTENT_STR_3, dq, 3, true) {
        return true;
    }
    if valid(CONTENT_CMD_1) && scan_content(lexer, CONTENT_CMD_1, bt, 1, true) {
        return true;
    }
    if valid(CONTENT_CMD_3) && scan_content(lexer, CONTENT_CMD_3, bt, 3, true) {
        return true;
    }
    if valid(CONTENT_STR_1_RAW) && scan_content(lexer, CONTENT_STR_1_RAW, dq, 1, false) {
        return true;
    }
    if valid(CONTENT_STR_3_RAW) && scan_content(lexer, CONTENT_STR_3_RAW, dq, 3, false) {
        return true;
    }
    if valid(CONTENT_CMD_1_RAW) && scan_content(lexer, CONTENT_CMD_1_RAW, bt, 1, false) {
        return true;
    }
    if valid(CONTENT_CMD_3_RAW) && scan_content(lexer, CONTENT_CMD_3_RAW, bt, 3, false) {
        return true;
    }
    false
}
