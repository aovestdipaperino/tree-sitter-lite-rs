//! Rust port of tree-sitter-scala's external scanner (src/scanner.c).
//!
//! Locale: the C calls `iswspace` and `iswalnum` (from `<wctype.h>`) and the program never calls
//! `setlocale`, so they are the C locale: `iswspace` is 9..=13 and 32, `iswalnum` is ASCII
//! letters and digits only. Both are ported that way (`is_wspace`, `is_walnum`).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const AUTOMATIC_SEMICOLON: usize = 0;
const INDENT: u16 = 1;
const OUTDENT: usize = 2;
const COMMA_OUTDENT: usize = 3;
const SIMPLE_STRING_START: usize = 4;
const SIMPLE_STRING_MIDDLE: usize = 5;
const SIMPLE_MULTILINE_STRING_START: u16 = 6;
const INTERPOLATED_STRING_MIDDLE: usize = 7;
const INTERPOLATED_MULTILINE_STRING_MIDDLE: usize = 8;
const RAW_STRING_START: usize = 9;
const RAW_STRING_MIDDLE: usize = 10;
const RAW_STRING_MULTILINE_MIDDLE: usize = 11;
const SINGLE_LINE_STRING_END: u16 = 12;
const MULTILINE_STRING_END: usize = 13;
const ELSE: usize = 14;
const CATCH: usize = 15;
const FINALLY: usize = 16;
const EXTENDS: usize = 17;
const DERIVES: usize = 18;
const WITH: usize = 19;

/// `TREE_SITTER_SERIALIZATION_BUFFER_SIZE`
const SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// `{ Array(int16_t) indents; int16_t last_indentation_size, last_newline_count, last_column; }`
struct Scanner {
    indents: Vec<i16>,
    last_indentation_size: i16,
    last_newline_count: i16,
    last_column: i16,
}

/// `iswspace` in the C locale.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswalnum` in the C locale.
fn is_walnum(c: i32) -> bool {
    matches!(c, 48..=57 | 65..=90 | 97..=122)
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}

fn is_op_char(c: i32) -> bool {
    matches!(
        c as u8 as char,
        '!' | '#'
            | '%'
            | '&'
            | '*'
            | '+'
            | '-'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '\\'
            | '^'
            | '|'
            | '~'
            | ':'
    ) && (0..256).contains(&c)
}

#[derive(Clone, Copy, PartialEq)]
enum StringMode {
    Simple,
    Interpolated,
    Raw,
}

unsafe fn scan_string_content(
    lexer: *mut TSLexer,
    is_multiline: bool,
    string_mode: StringMode,
) -> bool {
    let mut closing_quote_count: u32 = 0;
    loop {
        let la = (*lexer).lookahead;
        if la == '"' as i32 {
            advance(lexer);
            closing_quote_count += 1;
            if !is_multiline {
                (*lexer).result_symbol = SINGLE_LINE_STRING_END;
                mark_end(lexer);
                return true;
            }
            if closing_quote_count >= 3 && (*lexer).lookahead != '"' as i32 {
                (*lexer).result_symbol = MULTILINE_STRING_END as u16;
                mark_end(lexer);
                return true;
            }
        } else if la == '$' as i32 && string_mode != StringMode::Simple {
            (*lexer).result_symbol = match string_mode {
                StringMode::Interpolated => {
                    if is_multiline {
                        INTERPOLATED_MULTILINE_STRING_MIDDLE
                    } else {
                        INTERPOLATED_STRING_MIDDLE
                    }
                }
                StringMode::Raw => {
                    if is_multiline {
                        RAW_STRING_MULTILINE_MIDDLE
                    } else {
                        RAW_STRING_MIDDLE
                    }
                }
                StringMode::Simple => unreachable!(),
            } as u16;
            mark_end(lexer);
            return true;
        } else {
            closing_quote_count = 0;
            if la == '\\' as i32 {
                if is_multiline || string_mode == StringMode::Raw {
                    advance(lexer);
                    if !is_multiline
                        && string_mode == StringMode::Raw
                        && ((*lexer).lookahead == '"' as i32 || (*lexer).lookahead == '\\' as i32)
                    {
                        advance(lexer);
                    }
                } else {
                    (*lexer).result_symbol = if string_mode == StringMode::Simple {
                        SIMPLE_STRING_MIDDLE
                    } else {
                        INTERPOLATED_STRING_MIDDLE
                    } as u16;
                    mark_end(lexer);
                    return true;
                }
            } else if la == '\n' as i32 && !is_multiline {
                return false;
            } else if eof(lexer) {
                return false;
            } else {
                advance(lexer);
            }
        }
    }
}

unsafe fn detect_comment_start(lexer: *mut TSLexer) -> bool {
    mark_end(lexer);
    if (*lexer).lookahead == '/' as i32 {
        advance(lexer);
        if (*lexer).lookahead == '/' as i32 || (*lexer).lookahead == '*' as i32 {
            return true;
        }
    }
    false
}

unsafe fn scan_word(lexer: *mut TSLexer, word: &[u8]) -> bool {
    for &w in word {
        if (*lexer).lookahead != w as i32 {
            return false;
        }
        advance(lexer);
    }
    !is_walnum((*lexer).lookahead)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_scala_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner {
        indents: Vec::new(),
        last_indentation_size: -1,
        last_newline_count: 0,
        last_column: -1,
    })) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_scala_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_scala_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    if (s.indents.len() + 3) * 2 > SERIALIZATION_BUFFER_SIZE {
        return 0;
    }
    let mut size = 0usize;
    let mut put = |v: i16| {
        core::ptr::copy_nonoverlapping(v.to_ne_bytes().as_ptr(), (buffer as *mut u8).add(size), 2);
        size += 2;
    };
    put(s.last_indentation_size);
    put(s.last_newline_count);
    put(s.last_column);
    for &i in &s.indents {
        put(i);
    }
    size as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_scala_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    s.indents.clear();
    s.last_indentation_size = -1;
    s.last_column = -1;
    s.last_newline_count = 0;
    if length == 0 {
        return;
    }
    // The C reads three int16 unconditionally and ends with `assert(size == length)`. A length
    // that is not an even number of at least 6 bytes is a failed assert (an abort) there, after
    // reading out of bounds; tree-sitter never produces one, so abort here without the read.
    assert!(
        length >= 6 && length % 2 == 0,
        "scala: bad serialized length"
    );
    let read = |off: usize| core::ptr::read_unaligned((buffer as *const u8).add(off) as *const i16);
    s.last_indentation_size = read(0);
    s.last_newline_count = read(2);
    s.last_column = read(4);
    let mut size = 6usize;
    while size < length as usize {
        s.indents.push(read(size));
        size += 2;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_scala_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    let scanner = &mut *(payload as *mut Scanner);
    let prev: i16 = match scanner.indents.last() {
        Some(&p) => p,
        None => -1,
    };
    let mut newline_count: i16 = 0;
    let mut indentation_size: i16 = 0;

    while is_wspace((*lexer).lookahead) {
        if (*lexer).lookahead == '\n' as i32 {
            newline_count = newline_count.wrapping_add(1);
            indentation_size = 0;
        } else {
            indentation_size = indentation_size.wrapping_add(1);
        }
        skip(lexer);
    }

    if valid(COMMA_OUTDENT) && (*lexer).lookahead == ',' as i32 && prev != -1 {
        scanner.indents.pop();
        mark_end(lexer);
        (*lexer).result_symbol = COMMA_OUTDENT as u16;
        return true;
    }

    let la = (*lexer).lookahead;
    if valid(OUTDENT)
        && (la == 0
            || (prev != -1 && (la == ')' as i32 || la == ']' as i32 || la == '}' as i32))
            || (scanner.last_indentation_size != -1
                && prev != -1
                && scanner.last_indentation_size < prev))
    {
        scanner.indents.pop();
        (*lexer).result_symbol = OUTDENT as u16;
        return true;
    }
    scanner.last_indentation_size = -1;

    if valid(INDENT as usize)
        && newline_count > 0
        && (scanner.indents.is_empty() || indentation_size > *scanner.indents.last().unwrap())
    {
        if detect_comment_start(lexer) {
            return false;
        }
        scanner.indents.push(indentation_size);
        (*lexer).result_symbol = INDENT;
        return true;
    }

    if valid(OUTDENT)
        && ((*lexer).lookahead == 0 || (newline_count > 0 && prev != -1 && indentation_size < prev))
    {
        scanner.indents.pop();
        (*lexer).result_symbol = OUTDENT as u16;
        mark_end(lexer);
        if detect_comment_start(lexer) {
            return false;
        }
        scanner.last_indentation_size = indentation_size;
        scanner.last_newline_count = newline_count;
        if eof(lexer) {
            scanner.last_column = -1;
        } else {
            scanner.last_column = ((*lexer).get_column)(lexer) as i16;
        }
        return true;
    }

    let is_eof = eof(lexer);
    if (scanner.last_newline_count > 0 && (is_eof && scanner.last_column == -1))
        || (!is_eof && ((*lexer).get_column)(lexer) == scanner.last_column as i32 as u32)
    {
        newline_count = newline_count.wrapping_add(scanner.last_newline_count);
    }
    scanner.last_newline_count = 0;

    if valid(AUTOMATIC_SEMICOLON) && newline_count > 0 {
        mark_end(lexer);
        (*lexer).result_symbol = AUTOMATIC_SEMICOLON as u16;

        if (*lexer).lookahead == '.' as i32 {
            return false;
        }

        if (*lexer).lookahead == '/' as i32 {
            advance(lexer);
            if (*lexer).lookahead == '/' as i32 {
                return false;
            }
            if (*lexer).lookahead == '*' as i32 {
                advance(lexer);
                while !eof(lexer) {
                    if (*lexer).lookahead == '*' as i32 {
                        advance(lexer);
                        if (*lexer).lookahead == '/' as i32 {
                            advance(lexer);
                            break;
                        }
                    } else {
                        advance(lexer);
                    }
                }
                while is_wspace((*lexer).lookahead) {
                    if (*lexer).lookahead == '\n' as i32 || (*lexer).lookahead == '\r' as i32 {
                        return false;
                    }
                    skip(lexer);
                }
                return true;
            }
        }

        if valid(ELSE) {
            return !scan_word(lexer, b"else");
        }
        if valid(CATCH) && scan_word(lexer, b"catch") {
            return false;
        }
        if valid(FINALLY) && scan_word(lexer, b"finally") {
            return false;
        }
        if valid(EXTENDS) && scan_word(lexer, b"extends") {
            return false;
        }
        if valid(WITH) && scan_word(lexer, b"with") {
            return false;
        }
        if valid(DERIVES) && scan_word(lexer, b"derives") {
            return false;
        }

        if newline_count > 1 {
            return true;
        }

        if is_op_char((*lexer).lookahead) || (*lexer).lookahead == '`' as i32 {
            if is_op_char((*lexer).lookahead) {
                advance(lexer);
                while is_op_char((*lexer).lookahead) {
                    advance(lexer);
                }
                let mut found_space = false;
                while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
                    advance(lexer);
                    found_space = true;
                }
                if found_space && !is_wspace((*lexer).lookahead) && !eof(lexer) {
                    return false;
                }
            } else if (*lexer).lookahead == '`' as i32 {
                advance(lexer);
                while (*lexer).lookahead != '`' as i32 && !eof(lexer) {
                    advance(lexer);
                }
                if (*lexer).lookahead == '`' as i32 {
                    advance(lexer);
                    let mut found_space = false;
                    while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
                        advance(lexer);
                        found_space = true;
                    }
                    if found_space && !is_wspace((*lexer).lookahead) && !eof(lexer) {
                        return false;
                    }
                }
            }
        }

        return true;
    }

    while is_wspace((*lexer).lookahead) {
        if (*lexer).lookahead == '\n' as i32 {
            newline_count = newline_count.wrapping_add(1);
        }
        skip(lexer);
    }

    if valid(SIMPLE_STRING_START) && (*lexer).lookahead == '"' as i32 {
        advance(lexer);
        mark_end(lexer);

        if (*lexer).lookahead == '"' as i32 {
            advance(lexer);
            if (*lexer).lookahead == '"' as i32 {
                advance(lexer);
                (*lexer).result_symbol = SIMPLE_MULTILINE_STRING_START;
                mark_end(lexer);
                return true;
            }
        }

        (*lexer).result_symbol = SIMPLE_STRING_START as u16;
        return true;
    }

    if valid(RAW_STRING_START) && (*lexer).lookahead == 'r' as i32 {
        advance(lexer);
        if (*lexer).lookahead == 'a' as i32 {
            advance(lexer);
            if (*lexer).lookahead == 'w' as i32 {
                advance(lexer);
                if (*lexer).lookahead == '"' as i32 {
                    mark_end(lexer);
                    (*lexer).result_symbol = RAW_STRING_START as u16;
                    return true;
                }
            }
        }
    }

    if valid(SIMPLE_STRING_MIDDLE) {
        return scan_string_content(lexer, false, StringMode::Simple);
    }
    if valid(INTERPOLATED_STRING_MIDDLE) {
        return scan_string_content(lexer, false, StringMode::Interpolated);
    }
    if valid(RAW_STRING_MIDDLE) {
        return scan_string_content(lexer, false, StringMode::Raw);
    }
    if valid(RAW_STRING_MULTILINE_MIDDLE) {
        return scan_string_content(lexer, true, StringMode::Raw);
    }
    if valid(INTERPOLATED_MULTILINE_STRING_MIDDLE) {
        return scan_string_content(lexer, true, StringMode::Interpolated);
    }
    if valid(MULTILINE_STRING_END) {
        return scan_string_content(lexer, true, StringMode::Simple);
    }

    false
}
