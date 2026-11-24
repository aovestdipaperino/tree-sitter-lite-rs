//! Rust port of tree-sitter-javascript's external scanner (src/scanner.c).
//!
//! The C version calls `iswspace`/`iswalpha`/`iswdigit`; under the "C" locale those are
//! ASCII-only, which is what an embedded parser always sees. They are reproduced here exactly.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const AUTOMATIC_SEMICOLON: usize = 0;
const TEMPLATE_CHARS: usize = 1;
const TERNARY_QMARK: usize = 2;
const HTML_COMMENT: usize = 3;
const LOGICAL_OR: usize = 4;
const ESCAPE_SEQUENCE: usize = 5;
const REGEX_PATTERN: usize = 6;
const JSX_TEXT: usize = 7;

#[inline]
fn is_space(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
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
fn is_line_sep(c: i32) -> bool {
    c == b'\n' as i32 || c == 0x2028 || c == 0x2029
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_javascript_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_javascript_external_scanner_destroy(_p: *mut c_void) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_javascript_external_scanner_serialize(
    _p: *mut c_void,
    _b: *mut c_char,
) -> u32 {
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_javascript_external_scanner_deserialize(
    _p: *mut c_void,
    _b: *const c_char,
    _n: u32,
) {
}

unsafe fn scan_template_chars(l: *mut TSLexer) -> bool {
    (*l).result_symbol = TEMPLATE_CHARS as u16;
    let mut has_content = false;
    loop {
        mark_end(l);
        match la(l) {
            96 => return has_content, // `
            0 => return false,
            36 => {
                // $
                advance(l);
                if la(l) == b'{' as i32 {
                    return has_content;
                }
            }
            92 => return has_content, // backslash
            _ => advance(l),
        }
        has_content = true;
    }
}

#[derive(PartialEq)]
enum Ws {
    Reject,
    NoNewline,
    Accept,
}

unsafe fn scan_whitespace_and_comments(
    l: *mut TSLexer,
    scanned_comment: &mut bool,
    consume: bool,
) -> Ws {
    let mut saw_block_newline = false;
    loop {
        while is_space(la(l)) {
            skip(l);
        }
        if la(l) == b'/' as i32 {
            skip(l);
            if la(l) == b'/' as i32 {
                skip(l);
                while la(l) != 0 && !is_line_sep(la(l)) {
                    skip(l);
                }
                *scanned_comment = true;
            } else if la(l) == b'*' as i32 {
                skip(l);
                while la(l) != 0 {
                    if la(l) == b'*' as i32 {
                        skip(l);
                        if la(l) == b'/' as i32 {
                            skip(l);
                            *scanned_comment = true;
                            if la(l) != b'/' as i32 && !consume {
                                return if saw_block_newline {
                                    Ws::Accept
                                } else {
                                    Ws::NoNewline
                                };
                            }
                            break;
                        }
                    } else if is_line_sep(la(l)) {
                        saw_block_newline = true;
                        skip(l);
                    } else {
                        skip(l);
                    }
                }
            } else {
                return Ws::Reject;
            }
        } else {
            return Ws::Accept;
        }
    }
}

unsafe fn scan_automatic_semicolon(
    l: *mut TSLexer,
    comment_condition: bool,
    scanned_comment: &mut bool,
) -> bool {
    (*l).result_symbol = AUTOMATIC_SEMICOLON as u16;
    mark_end(l);
    loop {
        if la(l) == 0 {
            return true;
        }
        if la(l) == b'/' as i32 {
            let result = scan_whitespace_and_comments(l, scanned_comment, false);
            if result == Ws::Reject {
                return false;
            }
            if result == Ws::Accept
                && comment_condition
                && la(l) != b',' as i32
                && la(l) != b'=' as i32
            {
                return true;
            }
        }
        if la(l) == b'}' as i32 {
            return true;
        }
        if ((*l).is_at_included_range_start)(l) {
            return true;
        }
        if is_line_sep(la(l)) {
            break;
        }
        if !is_space(la(l)) {
            return false;
        }
        skip(l);
    }
    skip(l);
    if scan_whitespace_and_comments(l, scanned_comment, true) == Ws::Reject {
        return false;
    }
    let c = la(l);
    if [
        b'`', b',', b':', b';', b'*', b'%', b'>', b'<', b'=', b'[', b'(', b'?', b'^', b'|', b'&',
        b'/',
    ]
    .iter()
    .any(|&x| x as i32 == c)
    {
        return false;
    }
    match c as u8 as char {
        _ if c > 0xff => {}
        '.' => {
            skip(l);
            return is_digit(la(l));
        }
        '+' => {
            skip(l);
            return la(l) == b'+' as i32;
        }
        '-' => {
            skip(l);
            return la(l) == b'-' as i32;
        }
        '!' => {
            skip(l);
            return la(l) != b'=' as i32;
        }
        'i' => {
            skip(l);
            if la(l) != b'n' as i32 {
                return true;
            }
            skip(l);
            if !is_alpha(la(l)) {
                return false;
            }
            for &ch in b"stanceof" {
                if la(l) != ch as i32 {
                    return true;
                }
                skip(l);
            }
            if !is_alpha(la(l)) {
                return false;
            }
        }
        _ => {}
    }
    true
}

unsafe fn scan_ternary_qmark(l: *mut TSLexer) -> bool {
    while is_space(la(l)) {
        skip(l);
    }
    if la(l) == b'?' as i32 {
        advance(l);
        if la(l) == b'?' as i32 {
            return false;
        }
        mark_end(l);
        (*l).result_symbol = TERNARY_QMARK as u16;
        if la(l) == b'.' as i32 {
            advance(l);
            return is_digit(la(l));
        }
        return true;
    }
    false
}

unsafe fn scan_html_comment(l: *mut TSLexer) -> bool {
    while is_space(la(l)) || la(l) == 0x2028 || la(l) == 0x2029 {
        skip(l);
    }
    if la(l) == b'<' as i32 {
        for &ch in b"<!--" {
            if la(l) != ch as i32 {
                return false;
            }
            advance(l);
        }
    } else if la(l) == b'-' as i32 {
        for &ch in b"-->" {
            if la(l) != ch as i32 {
                return false;
            }
            advance(l);
        }
    } else {
        return false;
    }
    while la(l) != 0 && !is_line_sep(la(l)) {
        advance(l);
    }
    (*l).result_symbol = HTML_COMMENT as u16;
    mark_end(l);
    true
}

unsafe fn scan_jsx_text(l: *mut TSLexer) -> bool {
    let mut saw_text = false;
    let mut at_newline = false;
    while la(l) != 0
        && la(l) != b'<' as i32
        && la(l) != b'>' as i32
        && la(l) != b'{' as i32
        && la(l) != b'}' as i32
        && la(l) != b'&' as i32
    {
        let is_wspace = is_space(la(l));
        if la(l) == b'\n' as i32 {
            at_newline = true;
        } else {
            at_newline &= is_wspace;
            if !at_newline {
                saw_text = true;
            }
        }
        advance(l);
    }
    (*l).result_symbol = JSX_TEXT as u16;
    saw_text
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_javascript_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    if valid(TEMPLATE_CHARS) {
        if valid(AUTOMATIC_SEMICOLON) {
            return false;
        }
        return scan_template_chars(lexer);
    }
    if valid(JSX_TEXT) && scan_jsx_text(lexer) {
        return true;
    }
    if valid(AUTOMATIC_SEMICOLON) {
        let mut scanned_comment = false;
        let ret = scan_automatic_semicolon(lexer, !valid(LOGICAL_OR), &mut scanned_comment);
        if !ret && !scanned_comment && valid(TERNARY_QMARK) && la(lexer) == b'?' as i32 {
            return scan_ternary_qmark(lexer);
        }
        return ret;
    }
    if valid(TERNARY_QMARK) {
        return scan_ternary_qmark(lexer);
    }
    if valid(HTML_COMMENT) && !valid(LOGICAL_OR) && !valid(ESCAPE_SEQUENCE) && !valid(REGEX_PATTERN)
    {
        return scan_html_comment(lexer);
    }
    false
}
