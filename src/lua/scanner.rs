//! Rust port of tree-sitter-lua's external scanner (src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const BLOCK_COMMENT_START: usize = 0;
const BLOCK_COMMENT_CONTENT: usize = 1;
const BLOCK_COMMENT_END: usize = 2;
const BLOCK_STRING_START: usize = 3;
const BLOCK_STRING_CONTENT: usize = 4;
const BLOCK_STRING_END: usize = 5;

/// `{ char ending_char; uint8_t level_count; }`
#[derive(Default)]
struct Scanner {
    ending_char: u8,
    level_count: u8,
}

unsafe fn consume(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn consume_char(c: u8, lexer: *mut TSLexer) -> bool {
    if (*lexer).lookahead != c as i32 {
        return false;
    }
    consume(lexer);
    true
}

/// The C wraps the count in a `uint8_t`, so more than 255 `=` signs wrap around.
unsafe fn consume_and_count_char(c: u8, lexer: *mut TSLexer) -> u8 {
    let mut count = 0u8;
    while (*lexer).lookahead == c as i32 {
        count = count.wrapping_add(1);
        consume(lexer);
    }
    count
}

/// `iswspace` in the C locale, which is what a program that never calls `setlocale` gets.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

unsafe fn skip_whitespaces(lexer: *mut TSLexer) {
    while is_wspace((*lexer).lookahead) {
        skip(lexer);
    }
}

impl Scanner {
    fn reset(&mut self) {
        self.ending_char = 0;
        self.level_count = 0;
    }

    unsafe fn scan_block_start(&mut self, lexer: *mut TSLexer) -> bool {
        if consume_char(b'[', lexer) {
            let level = consume_and_count_char(b'=', lexer);
            if consume_char(b'[', lexer) {
                self.level_count = level;
                return true;
            }
        }
        false
    }

    unsafe fn scan_block_end(&mut self, lexer: *mut TSLexer) -> bool {
        if consume_char(b']', lexer) {
            let level = consume_and_count_char(b'=', lexer);
            if self.level_count == level && consume_char(b']', lexer) {
                return true;
            }
        }
        false
    }

    unsafe fn scan_block_content(&mut self, lexer: *mut TSLexer) -> bool {
        while (*lexer).lookahead != 0 {
            if (*lexer).lookahead == b']' as i32 {
                ((*lexer).mark_end)(lexer);
                if self.scan_block_end(lexer) {
                    return true;
                }
            } else {
                consume(lexer);
            }
        }
        false
    }

    unsafe fn scan_comment_start(&mut self, lexer: *mut TSLexer) -> bool {
        if consume_char(b'-', lexer) && consume_char(b'-', lexer) {
            ((*lexer).mark_end)(lexer);
            if self.scan_block_start(lexer) {
                ((*lexer).mark_end)(lexer);
                (*lexer).result_symbol = BLOCK_COMMENT_START as u16;
                return true;
            }
        }
        false
    }

    unsafe fn scan_comment_content(&mut self, lexer: *mut TSLexer) -> bool {
        if self.ending_char == 0 {
            // block comment
            if self.scan_block_content(lexer) {
                (*lexer).result_symbol = BLOCK_COMMENT_CONTENT as u16;
                return true;
            }
            return false;
        }
        while (*lexer).lookahead != 0 {
            if (*lexer).lookahead == self.ending_char as i32 {
                self.reset();
                (*lexer).result_symbol = BLOCK_COMMENT_CONTENT as u16;
                return true;
            }
            consume(lexer);
        }
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lua_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lua_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lua_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    *buffer = s.ending_char as c_char;
    *buffer.add(1) = s.level_count as c_char;
    2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lua_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    if length == 0 {
        return;
    }
    s.ending_char = *buffer as u8;
    if length == 1 {
        return;
    }
    s.level_count = *buffer.add(1) as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_lua_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    let valid = |i: usize| *valid_symbols.add(i);

    if valid(BLOCK_STRING_END) && s.scan_block_end(lexer) {
        s.reset();
        (*lexer).result_symbol = BLOCK_STRING_END as u16;
        return true;
    }
    if valid(BLOCK_STRING_CONTENT) && s.scan_block_content(lexer) {
        (*lexer).result_symbol = BLOCK_STRING_CONTENT as u16;
        return true;
    }
    if valid(BLOCK_COMMENT_END) && s.ending_char == 0 && s.scan_block_end(lexer) {
        s.reset();
        (*lexer).result_symbol = BLOCK_COMMENT_END as u16;
        return true;
    }
    if valid(BLOCK_COMMENT_CONTENT) && s.scan_comment_content(lexer) {
        return true;
    }

    skip_whitespaces(lexer);

    if valid(BLOCK_STRING_START) && s.scan_block_start(lexer) {
        (*lexer).result_symbol = BLOCK_STRING_START as u16;
        return true;
    }
    if valid(BLOCK_COMMENT_START) && s.scan_comment_start(lexer) {
        return true;
    }
    false
}
