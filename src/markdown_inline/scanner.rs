//! Rust port of tree-sitter-markdown-inline's external scanner (tree-sitter-markdown v0.5.3,
//! `tree-sitter-markdown-inline/src/scanner.c`).
//!
//! The C has no `isw*` calls: `is_punctuation` is an ASCII range test and the other character
//! tests are comparisons with ASCII literals, so there is no locale dependence.
//!
//! Integer widths follow the C: the delimiter counters are `uint8_t` and wrap (`level`,
//! `star_count`, `underscore_count`), while `close_level` is a `size_t` that is compared with the
//! (possibly wrapped) `uint8_t` level.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const TRIGGER_ERROR: usize = 1;
const CODE_SPAN_START: usize = 2;
const CODE_SPAN_CLOSE: usize = 3;
const EMPHASIS_OPEN_STAR: usize = 4;
const EMPHASIS_OPEN_UNDERSCORE: usize = 5;
const EMPHASIS_CLOSE_STAR: usize = 6;
const EMPHASIS_CLOSE_UNDERSCORE: usize = 7;
const LAST_TOKEN_WHITESPACE: usize = 8;
const LAST_TOKEN_PUNCTUATION: usize = 9;
const STRIKETHROUGH_OPEN: usize = 10;
const STRIKETHROUGH_CLOSE: usize = 11;
const LATEX_SPAN_START: usize = 12;
const LATEX_SPAN_CLOSE: usize = 13;
const UNCLOSED_SPAN: usize = 14;
/// `ERROR` in the C enum.
const ERROR: u16 = 0;

/// Current delimiter run is opening.
const STATE_EMPHASIS_DELIMITER_IS_OPEN: u8 = 0x1 << 2;

/// Determines if a character is punctuation as defined by the markdown spec.
fn is_punctuation(chr: i32) -> bool {
    (b'!' as i32..=b'/' as i32).contains(&chr)
        || (b':' as i32..=b'@' as i32).contains(&chr)
        || (b'[' as i32..=b'`' as i32).contains(&chr)
        || (b'{' as i32..=b'~' as i32).contains(&chr)
}

/// `{ uint8_t state; uint8_t code_span_delimiter_length; uint8_t latex_span_delimiter_length;
/// uint8_t num_emphasis_delimiters_left; }`
#[derive(Default)]
struct Scanner {
    state: u8,
    code_span_delimiter_length: u8,
    latex_span_delimiter_length: u8,
    num_emphasis_delimiters_left: u8,
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}

/// Emit the error token, used to stop invalid parse branches.
unsafe fn error(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = ERROR;
    true
}

unsafe fn parse_leaf_delimiter(
    lexer: *mut TSLexer,
    delimiter_length: &mut u8,
    valid_symbols: *const bool,
    delimiter: u8,
    open_token: usize,
    close_token: usize,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    let mut level: u8 = 0;
    while (*lexer).lookahead == delimiter as i32 {
        advance(lexer);
        level = level.wrapping_add(1);
    }
    mark_end(lexer);
    if level == *delimiter_length && valid(close_token) {
        *delimiter_length = 0;
        (*lexer).result_symbol = close_token as u16;
        return true;
    }
    if valid(open_token) {
        // Parse ahead to check if there is a closing delimiter
        let mut close_level: usize = 0;
        while !eof(lexer) {
            if (*lexer).lookahead == delimiter as i32 {
                close_level += 1;
            } else {
                if close_level == level as usize {
                    // Found a matching delimiter
                    break;
                }
                close_level = 0;
            }
            advance(lexer);
        }
        if close_level == level as usize {
            *delimiter_length = level;
            (*lexer).result_symbol = open_token as u16;
            return true;
        }
        if valid(UNCLOSED_SPAN) {
            (*lexer).result_symbol = UNCLOSED_SPAN as u16;
            return true;
        }
    }
    false
}

/// `parse_star`, `parse_tilde` and `parse_underscore` are the same code in the C, differing only
/// in the delimiter character and the open and close tokens.
unsafe fn parse_run(
    s: &mut Scanner,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
    delimiter: u8,
    open_token: usize,
    close_token: usize,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    advance(lexer);
    // If `num_emphasis_delimiters_left` is not zero then we already decided that this should be
    // part of an emphasis delimiter run, so interpret it as such.
    if s.num_emphasis_delimiters_left > 0 {
        if (s.state & STATE_EMPHASIS_DELIMITER_IS_OPEN) != 0 && valid(open_token) {
            s.state &= !STATE_EMPHASIS_DELIMITER_IS_OPEN;
            (*lexer).result_symbol = open_token as u16;
            s.num_emphasis_delimiters_left -= 1;
            return true;
        }
        if valid(close_token) {
            (*lexer).result_symbol = close_token as u16;
            s.num_emphasis_delimiters_left -= 1;
            return true;
        }
    }
    mark_end(lexer);
    // Otherwise count the number of delimiters
    let mut count: u8 = 1;
    while (*lexer).lookahead == delimiter as i32 {
        count = count.wrapping_add(1);
        advance(lexer);
    }
    let la = (*lexer).lookahead;
    let line_end = la == b'\n' as i32 || la == b'\r' as i32 || eof(lexer);
    if valid(open_token) || valid(close_token) {
        // `count - 1` is computed as int and narrowed to uint8_t: 0 wraps to 255.
        s.num_emphasis_delimiters_left = count.wrapping_sub(1);
        let la = (*lexer).lookahead;
        let next_symbol_whitespace = line_end || la == b' ' as i32 || la == b'\t' as i32;
        let next_symbol_punctuation = is_punctuation(la);
        if valid(close_token)
            && !valid(LAST_TOKEN_WHITESPACE)
            && (!valid(LAST_TOKEN_PUNCTUATION) || next_symbol_punctuation || next_symbol_whitespace)
        {
            // Closing delimiters take precedence
            s.state &= !STATE_EMPHASIS_DELIMITER_IS_OPEN;
            (*lexer).result_symbol = close_token as u16;
            return true;
        }
        if !next_symbol_whitespace
            && (!next_symbol_punctuation
                || valid(LAST_TOKEN_PUNCTUATION)
                || valid(LAST_TOKEN_WHITESPACE))
        {
            s.state |= STATE_EMPHASIS_DELIMITER_IS_OPEN;
            (*lexer).result_symbol = open_token as u16;
            return true;
        }
    }
    false
}

unsafe fn scan(s: &mut Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    // A normal tree-sitter rule decided that the current branch is invalid and now "requests" an
    // error to stop the branch.
    if *valid_symbols.add(TRIGGER_ERROR) {
        return error(lexer);
    }
    match (*lexer).lookahead {
        0x60 => parse_leaf_delimiter(
            lexer,
            &mut s.code_span_delimiter_length,
            valid_symbols,
            b'`',
            CODE_SPAN_START,
            CODE_SPAN_CLOSE,
        ),
        0x24 => parse_leaf_delimiter(
            lexer,
            &mut s.latex_span_delimiter_length,
            valid_symbols,
            b'$',
            LATEX_SPAN_START,
            LATEX_SPAN_CLOSE,
        ),
        0x2a => parse_run(
            s,
            lexer,
            valid_symbols,
            b'*',
            EMPHASIS_OPEN_STAR,
            EMPHASIS_CLOSE_STAR,
        ),
        0x5f => parse_run(
            s,
            lexer,
            valid_symbols,
            b'_',
            EMPHASIS_OPEN_UNDERSCORE,
            EMPHASIS_CLOSE_UNDERSCORE,
        ),
        0x7e => parse_run(
            s,
            lexer,
            valid_symbols,
            b'~',
            STRIKETHROUGH_OPEN,
            STRIKETHROUGH_CLOSE,
        ),
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_inline_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_inline_external_scanner_destroy(
    payload: *mut c_void,
) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_inline_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    *buffer = s.state as c_char;
    *buffer.add(1) = s.code_span_delimiter_length as c_char;
    *buffer.add(2) = s.latex_span_delimiter_length as c_char;
    *buffer.add(3) = s.num_emphasis_delimiters_left as c_char;
    4
}

/// The C reads four bytes whenever `length > 0`, which overruns the buffer for a length of 1 to 3
/// (never produced by `serialize`, which always writes 4). Here bytes at or beyond `length` read as
/// 0; the result is identical wherever the C stays in bounds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_inline_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    s.state = 0;
    s.code_span_delimiter_length = 0;
    s.latex_span_delimiter_length = 0;
    s.num_emphasis_delimiters_left = 0;
    if length > 0 {
        let byte = |i: u32| {
            if i < length {
                *buffer.add(i as usize) as u8
            } else {
                0
            }
        };
        s.state = byte(0);
        s.code_span_delimiter_length = byte(1);
        s.latex_span_delimiter_length = byte(2);
        s.num_emphasis_delimiters_left = byte(3);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_inline_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(&mut *(payload as *mut Scanner), lexer, valid_symbols)
}
