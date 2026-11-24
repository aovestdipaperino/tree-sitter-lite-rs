//! Rust port of tree-sitter-rust's external scanner (src/scanner.c).
//! `isw*` classification is ASCII-only, matching the "C" locale the C scanner runs under.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const STRING_CONTENT: usize = 0;
const STRING_CLOSE: usize = 1;
const RAW_STRING_LITERAL_START: usize = 2;
const RAW_STRING_LITERAL_CONTENT: usize = 3;
const RAW_STRING_LITERAL_END: usize = 4;
const FLOAT_LITERAL: usize = 5;
const BLOCK_OUTER_DOC_MARKER: usize = 6;
const BLOCK_INNER_DOC_MARKER: usize = 7;
const BLOCK_COMMENT_CONTENT: usize = 8;
const LINE_DOC_CONTENT: usize = 9;
const ERROR_SENTINEL: usize = 10;

struct Scanner {
    opening_hash_count: u8,
}

#[inline]
fn is_digit(c: i32) -> bool {
    matches!(c, 48..=57)
}
#[inline]
fn is_alpha(c: i32) -> bool {
    matches!(c, 65..=90 | 97..=122)
}
#[inline]
fn is_space(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}
#[inline]
fn is_num_char(c: i32) -> bool {
    c == b'_' as i32 || is_digit(c)
}
#[inline]
unsafe fn la(l: *mut TSLexer) -> i32 {
    (*l).lookahead
}
#[inline]
unsafe fn advance(l: *mut TSLexer) {
    ((*l).advance)(l, false)
}
#[inline]
unsafe fn skip(l: *mut TSLexer) {
    ((*l).advance)(l, true)
}
#[inline]
unsafe fn mark_end(l: *mut TSLexer) {
    ((*l).mark_end)(l)
}
#[inline]
unsafe fn eof(l: *mut TSLexer) -> bool {
    ((*l).eof)(l)
}
#[inline]
fn ch(c: u8) -> i32 {
    c as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_rust_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner {
        opening_hash_count: 0,
    })) as *mut c_void
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_rust_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_rust_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    *buffer = s.opening_hash_count as c_char;
    1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_rust_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    s.opening_hash_count = 0;
    if length == 1 {
        s.opening_hash_count = *buffer as u8;
    }
}

unsafe fn process_string(l: *mut TSLexer) -> bool {
    let mut has_content = false;
    loop {
        if la(l) == ch(b'"') || la(l) == ch(b'\\') {
            break;
        }
        if eof(l) {
            return false;
        }
        has_content = true;
        advance(l);
    }
    (*l).result_symbol = STRING_CONTENT as u16;
    mark_end(l);
    has_content
}

unsafe fn scan_raw_string_start(s: &mut Scanner, l: *mut TSLexer) -> bool {
    if la(l) == ch(b'b') || la(l) == ch(b'c') {
        advance(l);
    }
    if la(l) != ch(b'r') {
        return false;
    }
    advance(l);
    let mut opening_hash_count: u8 = 0;
    while la(l) == ch(b'#') {
        advance(l);
        opening_hash_count = opening_hash_count.wrapping_add(1);
    }
    if la(l) != ch(b'"') {
        return false;
    }
    advance(l);
    s.opening_hash_count = opening_hash_count;
    (*l).result_symbol = RAW_STRING_LITERAL_START as u16;
    true
}

unsafe fn scan_raw_string_content(s: &mut Scanner, l: *mut TSLexer) -> bool {
    loop {
        if eof(l) {
            return false;
        }
        if la(l) == ch(b'"') {
            mark_end(l);
            advance(l);
            let mut hash_count: u32 = 0;
            while la(l) == ch(b'#') && hash_count < s.opening_hash_count as u32 {
                advance(l);
                hash_count += 1;
            }
            if hash_count == s.opening_hash_count as u32 {
                (*l).result_symbol = RAW_STRING_LITERAL_CONTENT as u16;
                return true;
            }
        } else {
            advance(l);
        }
    }
}

unsafe fn scan_raw_string_end(s: &mut Scanner, l: *mut TSLexer) -> bool {
    advance(l);
    for _ in 0..s.opening_hash_count {
        advance(l);
    }
    (*l).result_symbol = RAW_STRING_LITERAL_END as u16;
    true
}

unsafe fn process_float_literal(l: *mut TSLexer) -> bool {
    (*l).result_symbol = FLOAT_LITERAL as u16;
    advance(l);
    while is_num_char(la(l)) {
        advance(l);
    }
    let mut has_fraction = false;
    let mut has_exponent = false;
    if la(l) == ch(b'.') {
        has_fraction = true;
        advance(l);
        if is_alpha(la(l)) {
            return false;
        }
        if la(l) == ch(b'.') {
            return false;
        }
        while is_num_char(la(l)) {
            advance(l);
        }
    }
    mark_end(l);
    if la(l) == ch(b'e') || la(l) == ch(b'E') {
        has_exponent = true;
        advance(l);
        if la(l) == ch(b'+') || la(l) == ch(b'-') {
            advance(l);
        }
        if !is_num_char(la(l)) {
            return true;
        }
        advance(l);
        while is_num_char(la(l)) {
            advance(l);
        }
        mark_end(l);
    }
    if !has_exponent && !has_fraction {
        return false;
    }
    if la(l) != ch(b'u') && la(l) != ch(b'i') && la(l) != ch(b'f') {
        return true;
    }
    advance(l);
    if !is_digit(la(l)) {
        return true;
    }
    while is_digit(la(l)) {
        advance(l);
    }
    mark_end(l);
    true
}

unsafe fn process_line_doc_content(l: *mut TSLexer) -> bool {
    (*l).result_symbol = LINE_DOC_CONTENT as u16;
    loop {
        if eof(l) {
            return true;
        }
        if la(l) == ch(b'\n') {
            advance(l);
            return true;
        }
        advance(l);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum St {
    LeftForwardSlash,
    LeftAsterisk,
    Continuing,
}

unsafe fn process_block_comment(l: *mut TSLexer, valid: &dyn Fn(usize) -> bool) -> bool {
    // `(char)lexer->lookahead` truncates to 8 bits.
    let mut first = la(l) as u8;
    if valid(BLOCK_INNER_DOC_MARKER) && first == b'!' {
        (*l).result_symbol = BLOCK_INNER_DOC_MARKER as u16;
        advance(l);
        return true;
    }
    if valid(BLOCK_OUTER_DOC_MARKER) && first == b'*' {
        advance(l);
        mark_end(l);
        if la(l) == ch(b'/') {
            return false;
        }
        if la(l) != ch(b'*') {
            (*l).result_symbol = BLOCK_OUTER_DOC_MARKER as u16;
            return true;
        }
    } else {
        advance(l);
    }

    if valid(BLOCK_COMMENT_CONTENT) {
        let mut state;
        let mut depth: u32 = 1;
        match first {
            b'*' => {
                state = St::LeftAsterisk;
                if la(l) == ch(b'/') {
                    return false;
                }
            }
            b'/' => state = St::LeftForwardSlash,
            _ => state = St::Continuing,
        }
        while !eof(l) && depth != 0 {
            first = la(l) as u8;
            match state {
                St::LeftForwardSlash => {
                    if first == b'*' {
                        depth = depth.wrapping_add(1);
                    }
                    state = St::Continuing;
                }
                St::LeftAsterisk => {
                    if first == b'*' {
                        mark_end(l);
                        state = St::LeftAsterisk;
                    } else {
                        if first == b'/' {
                            depth = depth.wrapping_sub(1);
                        }
                        state = St::Continuing;
                    }
                }
                St::Continuing => {
                    mark_end(l);
                    match first {
                        b'/' => state = St::LeftForwardSlash,
                        b'*' => state = St::LeftAsterisk,
                        _ => {}
                    }
                }
            }
            advance(l);
            if first == b'/' && depth != 0 {
                mark_end(l);
            }
        }
        (*l).result_symbol = BLOCK_COMMENT_CONTENT as u16;
        return true;
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_rust_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    if valid(ERROR_SENTINEL) {
        return false;
    }
    let scanner = &mut *(payload as *mut Scanner);

    if valid(BLOCK_COMMENT_CONTENT)
        || valid(BLOCK_INNER_DOC_MARKER)
        || valid(BLOCK_OUTER_DOC_MARKER)
    {
        return process_block_comment(lexer, &valid);
    }
    if valid(STRING_CONTENT) && !valid(FLOAT_LITERAL) && process_string(lexer) {
        return true;
    }
    if valid(STRING_CLOSE) && la(lexer) == ch(b'"') {
        advance(lexer);
        (*lexer).result_symbol = STRING_CLOSE as u16;
        mark_end(lexer);
        return true;
    }
    if valid(LINE_DOC_CONTENT) {
        return process_line_doc_content(lexer);
    }
    while is_space(la(lexer)) {
        skip(lexer);
    }
    if valid(RAW_STRING_LITERAL_START)
        && (la(lexer) == ch(b'r') || la(lexer) == ch(b'b') || la(lexer) == ch(b'c'))
    {
        return scan_raw_string_start(scanner, lexer);
    }
    if valid(RAW_STRING_LITERAL_CONTENT) {
        return scan_raw_string_content(scanner, lexer);
    }
    if valid(RAW_STRING_LITERAL_END) && la(lexer) == ch(b'"') {
        return scan_raw_string_end(scanner, lexer);
    }
    if valid(FLOAT_LITERAL) && is_digit(la(lexer)) {
        return process_float_literal(lexer);
    }
    false
}
