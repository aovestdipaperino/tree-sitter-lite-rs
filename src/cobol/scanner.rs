//! Rust port of tree-sitter-cobol's external scanner (src/scanner.c).
//!
//! The scanner is stateless: serialize writes nothing, deserialize reads nothing.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const WHITE_SPACES: usize = 0;
const LINE_PREFIX_COMMENT: usize = 1;
const LINE_SUFFIX_COMMENT: usize = 2;
const LINE_COMMENT: usize = 3;
const COMMENT_ENTRY: usize = 4;
const MULTILINE_STRING: usize = 5;

const NUMBER_OF_COMMENT_ENTRY_KEYWORDS: usize = 9;
static ANY_CONTENT_KEYWORD: [&[u8]; NUMBER_OF_COMMENT_ENTRY_KEYWORDS] = [
    b"author",
    b"installlation",
    b"date-written",
    b"date-compiled",
    b"security",
    b"identification division",
    b"environment division",
    b"data division",
    b"procedure division",
];

const MAX_KEYWORDS: usize = 16;

unsafe fn advance(lexer: *mut TSLexer, skip: bool) {
    ((*lexer).advance)(lexer, skip);
}

unsafe fn column(lexer: *mut TSLexer) -> u32 {
    ((*lexer).get_column)(lexer)
}

/// `iswspace` in the C locale (ASCII only: \t \n \v \f \r and space), plus `;` and `,`.
fn is_white_space(c: i32) -> bool {
    matches!(c, 9..=13 | 32) || c == b';' as i32 || c == b',' as i32
}

/// `towupper` in the C locale: ASCII letters only.
fn towupper(c: i32) -> i32 {
    if (b'a' as i32..=b'z' as i32).contains(&c) {
        c - 32
    } else {
        c
    }
}

/// `towlower` in the C locale: ASCII letters only.
fn towlower(c: i32) -> i32 {
    if (b'A' as i32..=b'Z' as i32).contains(&c) {
        c + 32
    } else {
        c
    }
}

unsafe fn start_with_word(lexer: *mut TSLexer, words: &[&[u8]], number_of_words: usize) -> bool {
    while (*lexer).lookahead == b' ' as i32 || (*lexer).lookahead == b'\t' as i32 {
        advance(lexer, true);
    }

    if number_of_words > MAX_KEYWORDS {
        return false;
    }
    // The C keeps one `char *` per keyword and increments all of them on every character, even
    // those whose match already failed, so it walks past the end of the string literal (an
    // out-of-bounds read whose value is never used, because `continue_check` is false by then).
    // Here one shared offset is used and a keyword is only read while its check is still true,
    // when the offset is within the string or at its terminator: same result, no overrun.
    let mut offset = 0usize;
    let mut continue_check = [true; MAX_KEYWORDS];

    loop {
        // At the end of the line
        if column(lexer) > 71 || (*lexer).lookahead == b'\n' as i32 || (*lexer).lookahead == 0 {
            return false;
        }

        // If all keyword matching fails, move to the end of the line
        let mut all_match_failed = true;
        for i in 0..number_of_words {
            if continue_check[i] {
                all_match_failed = false;
            }
        }

        if all_match_failed {
            while column(lexer) < 71
                && (*lexer).lookahead != b'\n' as i32
                && (*lexer).lookahead != 0
            {
                advance(lexer, true);
            }
            return false;
        }

        // If the head of the line matches any of specified keywords, return true;
        // `char c = lexer->lookahead` truncates to a (signed) char.
        let c = (*lexer).lookahead as i8 as i32;
        for i in 0..number_of_words {
            if continue_check[i] && words[i].get(offset).is_none() {
                return true;
            }
        }

        // matching keywords
        for i in 0..number_of_words {
            if continue_check[i] {
                let k = words[i][offset] as i32;
                continue_check[i] = c == towupper(k) || c == towlower(k);
            }
        }
        offset += 1;

        // next character
        advance(lexer, true);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_COBOL_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_COBOL_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_COBOL_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_COBOL_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_COBOL_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);

    if (*lexer).lookahead == 0 {
        return false;
    }

    if valid(WHITE_SPACES) {
        if is_white_space((*lexer).lookahead) {
            while is_white_space((*lexer).lookahead) {
                advance(lexer, true);
            }
            (*lexer).result_symbol = WHITE_SPACES as u16;
            ((*lexer).mark_end)(lexer);
            return true;
        }
    }

    if valid(LINE_PREFIX_COMMENT) && column(lexer) <= 5 {
        // The C has no end-of-input check here: when the input ends in a line shorter than six
        // columns, `advance` at end of input no longer moves the column and the C loops forever.
        // Stopping at end of input changes nothing wherever the C terminates.
        while column(lexer) <= 5 && (*lexer).lookahead != 0 {
            advance(lexer, true);
        }
        (*lexer).result_symbol = LINE_PREFIX_COMMENT as u16;
        ((*lexer).mark_end)(lexer);
        return true;
    }

    if valid(LINE_COMMENT) {
        if column(lexer) == 6 {
            if (*lexer).lookahead == b'*' as i32 || (*lexer).lookahead == b'/' as i32 {
                while (*lexer).lookahead != b'\n' as i32 && (*lexer).lookahead != 0 {
                    advance(lexer, true);
                }
                (*lexer).result_symbol = LINE_COMMENT as u16;
                ((*lexer).mark_end)(lexer);
                return true;
            } else {
                advance(lexer, true);
                ((*lexer).mark_end)(lexer);
                return false;
            }
        }
    }

    if valid(LINE_SUFFIX_COMMENT) {
        if column(lexer) >= 72 {
            while (*lexer).lookahead != b'\n' as i32 && (*lexer).lookahead != 0 {
                advance(lexer, true);
            }
            (*lexer).result_symbol = LINE_SUFFIX_COMMENT as u16;
            ((*lexer).mark_end)(lexer);
            return true;
        }
    }

    if valid(COMMENT_ENTRY) {
        if !start_with_word(
            lexer,
            &ANY_CONTENT_KEYWORD,
            NUMBER_OF_COMMENT_ENTRY_KEYWORDS,
        ) {
            ((*lexer).mark_end)(lexer);
            (*lexer).result_symbol = COMMENT_ENTRY as u16;
            return true;
        } else {
            return false;
        }
    }

    if valid(MULTILINE_STRING) {
        loop {
            if (*lexer).lookahead != b'"' as i32 {
                return false;
            }
            advance(lexer, false);
            while (*lexer).lookahead != b'"' as i32 && (*lexer).lookahead != 0 && column(lexer) < 72
            {
                advance(lexer, false);
            }
            if (*lexer).lookahead == b'"' as i32 {
                (*lexer).result_symbol = MULTILINE_STRING as u16;
                advance(lexer, false);
                ((*lexer).mark_end)(lexer);
                return true;
            }
            while (*lexer).lookahead != 0 && (*lexer).lookahead != b'\n' as i32 {
                advance(lexer, true);
            }
            if (*lexer).lookahead == 0 {
                return false;
            }
            advance(lexer, true);
            for _ in 0..=5 {
                if (*lexer).lookahead == 0 || (*lexer).lookahead == b'\n' as i32 {
                    return false;
                }
                advance(lexer, true);
            }

            if (*lexer).lookahead != b'-' as i32 {
                return false;
            }

            advance(lexer, true);
            while (*lexer).lookahead == b' ' as i32 && column(lexer) < 72 {
                advance(lexer, true);
            }
        }
    }

    false
}
