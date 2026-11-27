//! Rust port of tree-sitter-sql's external scanner (src/scanner.c of tree-sitter-sequel 0.3.11).
//!
//! Locale functions: `iswspace` is ported as the C locale (ASCII only: 9..=13 and 32).
//!
//! The C keeps tags as NUL-terminated `char*` strings, filled with `add_char(.., (char)lookahead)`:
//! a code point is truncated to its low byte, and a low byte of 0 ends the string for `strlen` and
//! `strcmp`. The tags are kept here as the byte vectors the C builds (without the terminator) and
//! compared/measured through `cstr`, which reproduces that.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const DOLLAR_QUOTED_STRING_START_TAG: usize = 0;
const DOLLAR_QUOTED_STRING_END_TAG: usize = 1;
const DOLLAR_QUOTED_STRING: usize = 2;

/// `TREE_SITTER_SERIALIZATION_BUFFER_SIZE`
const SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// `typedef struct { char *start_tag; } LexerState;`
#[derive(Default)]
struct Scanner {
    start_tag: Option<Vec<u8>>,
}

/// The bytes `strlen` / `strcmp` would see: up to the first NUL. (A tag from `scan_dollar_string_tag`
/// or from `serialize` is always NUL-terminated in the C; a corrupt `deserialize` buffer without a
/// terminator would be read out of bounds there, here it ends at the buffer's end.)
fn cstr(v: &[u8]) -> &[u8] {
    match v.iter().position(|&b| b == 0) {
        Some(n) => &v[..n],
        None => v,
    }
}

/// `iswspace` in the C locale, which is what a program that never calls `setlocale` gets.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

unsafe fn advance(lexer: *mut TSLexer, skip: bool) {
    ((*lexer).advance)(lexer, skip);
}

unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}

unsafe fn skip_whitespaces(lexer: *mut TSLexer) {
    while is_wspace((*lexer).lookahead) {
        advance(lexer, true);
    }
}

/// A `$tag$` (or `$$`) at the lookahead, or None (the characters read so far stay consumed).
unsafe fn scan_dollar_string_tag(lexer: *mut TSLexer) -> Option<Vec<u8>> {
    let mut tag: Vec<u8> = Vec::new();
    if (*lexer).lookahead == '$' as i32 {
        tag.push(b'$');
        advance(lexer, false);
    } else {
        return None;
    }

    while (*lexer).lookahead != '$' as i32 && !is_wspace((*lexer).lookahead) && !eof(lexer) {
        // add_char(.., char c, ..): truncation to the low byte
        tag.push((*lexer).lookahead as u8);
        advance(lexer, false);
    }

    if (*lexer).lookahead == '$' as i32 {
        tag.push((*lexer).lookahead as u8);
        advance(lexer, false);
        Some(tag)
    } else {
        None
    }
}

impl Scanner {
    unsafe fn scan(&mut self, lexer: *mut TSLexer, valid: &dyn Fn(usize) -> bool) -> bool {
        if valid(DOLLAR_QUOTED_STRING_START_TAG) && self.start_tag.is_none() {
            skip_whitespaces(lexer);

            let start_tag = match scan_dollar_string_tag(lexer) {
                Some(t) => t,
                None => return false,
            };
            self.start_tag = Some(start_tag);
            (*lexer).result_symbol = DOLLAR_QUOTED_STRING_START_TAG as u16;
            return true;
        }

        if valid(DOLLAR_QUOTED_STRING_END_TAG) && self.start_tag.is_some() {
            skip_whitespaces(lexer);

            let end_tag = scan_dollar_string_tag(lexer);
            if let Some(end_tag) = end_tag {
                if cstr(&end_tag) == cstr(self.start_tag.as_ref().unwrap()) {
                    self.start_tag = None;
                    (*lexer).result_symbol = DOLLAR_QUOTED_STRING_END_TAG as u16;
                    return true;
                }
            }
            return false;
        }

        if valid(DOLLAR_QUOTED_STRING) {
            ((*lexer).mark_end)(lexer);
            skip_whitespaces(lexer);

            let start_tag = match scan_dollar_string_tag(lexer) {
                Some(t) => t,
                None => return false,
            };

            if let Some(cur) = &self.start_tag {
                if cstr(cur) == cstr(&start_tag) {
                    return false;
                }
            }

            loop {
                if eof(lexer) {
                    return false;
                }

                let end_tag = match scan_dollar_string_tag(lexer) {
                    Some(t) => t,
                    None => {
                        advance(lexer, false);
                        continue;
                    }
                };

                if cstr(&end_tag) == cstr(&start_tag) {
                    ((*lexer).mark_end)(lexer);
                    (*lexer).result_symbol = DOLLAR_QUOTED_STRING as u16;
                    return true;
                }
            }
        }

        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_sql_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_sql_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_sql_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    s.scan(lexer, &|i| *valid_symbols.add(i))
}

/// Like the C, a successful serialization clears the scanner's tag (`free` + `NULL`); a tag too
/// long for the buffer is not written (returns 0) and is kept.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_sql_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &mut *(payload as *mut Scanner);
    let tag = match &s.start_tag {
        Some(t) => t,
        None => return 0,
    };
    let body = cstr(tag);
    // + 1 for the '\0'
    let tag_length = body.len() + 1;
    if tag_length >= SERIALIZATION_BUFFER_SIZE {
        return 0;
    }
    core::ptr::copy_nonoverlapping(body.as_ptr(), buffer as *mut u8, body.len());
    *buffer.add(body.len()) = 0;
    s.start_tag = None;
    tag_length as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_sql_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    s.start_tag = None;
    // A length of 1 can't exists.
    if length > 1 {
        s.start_tag =
            Some(core::slice::from_raw_parts(buffer as *const u8, length as usize).to_vec());
    }
}
