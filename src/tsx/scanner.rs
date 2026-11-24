//! Rust port of tree-sitter-typescript's shared external scanner (common/scanner.h).
//! The TypeScript and TSX grammars use the same logic; only the exported symbol prefix differs.
//! `isw*` classification is ASCII-only, matching the "C" locale the C scanner runs under.
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
const FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON: usize = 8;
#[allow(dead_code)]
const ERROR_RECOVERY: usize = 9;

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
fn ch(c: u8) -> i32 {
    c as i32
}

unsafe fn scan_template_chars(l: *mut TSLexer) -> bool {
    (*l).result_symbol = TEMPLATE_CHARS as u16;
    let mut has_content = false;
    loop {
        mark_end(l);
        match la(l) {
            96 => return has_content,
            0 => return false,
            36 => {
                advance(l);
                if la(l) == ch(b'{') {
                    return has_content;
                }
            }
            92 => return has_content,
            _ => advance(l),
        }
        has_content = true;
    }
}

unsafe fn scan_whitespace_and_comments(l: *mut TSLexer, scanned_comment: &mut bool) -> bool {
    loop {
        while is_space(la(l)) {
            skip(l);
        }
        if la(l) == ch(b'/') {
            skip(l);
            if la(l) == ch(b'/') {
                skip(l);
                while la(l) != 0 && la(l) != ch(b'\n') {
                    skip(l);
                }
                *scanned_comment = true;
            } else if la(l) == ch(b'*') {
                skip(l);
                while la(l) != 0 {
                    if la(l) == ch(b'*') {
                        skip(l);
                        if la(l) == ch(b'/') {
                            skip(l);
                            break;
                        }
                    } else {
                        skip(l);
                    }
                }
            } else {
                return false;
            }
        } else {
            return true;
        }
    }
}

unsafe fn scan_automatic_semicolon(
    l: *mut TSLexer,
    valid: &dyn Fn(usize) -> bool,
    scanned_comment: &mut bool,
) -> bool {
    (*l).result_symbol = AUTOMATIC_SEMICOLON as u16;
    mark_end(l);
    loop {
        if la(l) == 0 {
            return true;
        }
        if la(l) == ch(b'}') {
            // do { skip } while (iswspace)
            loop {
                skip(l);
                if !is_space(la(l)) {
                    break;
                }
            }
            if la(l) == ch(b':') {
                return valid(LOGICAL_OR);
            }
            return true;
        }
        if !is_space(la(l)) {
            return false;
        }
        if la(l) == ch(b'\n') {
            break;
        }
        skip(l);
    }
    skip(l);
    if !scan_whitespace_and_comments(l, scanned_comment) {
        return false;
    }
    let c = la(l);
    if [
        b'`', b',', b'.', b';', b'*', b'%', b'>', b'<', b'=', b'?', b'^', b'|', b'&', b'/', b':',
    ]
    .iter()
    .any(|&x| x as i32 == c)
    {
        return false;
    }
    if c == ch(b'{') {
        if valid(FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON) {
            return false;
        }
    } else if c == ch(b'(') || c == ch(b'[') {
        if valid(LOGICAL_OR) {
            return false;
        }
    } else if c == ch(b'+') {
        skip(l);
        return la(l) == ch(b'+');
    } else if c == ch(b'-') {
        skip(l);
        return la(l) == ch(b'-');
    } else if c == ch(b'!') {
        skip(l);
        return la(l) != ch(b'=');
    } else if c == ch(b'i') {
        skip(l);
        if la(l) != ch(b'n') {
            return true;
        }
        skip(l);
        if !is_alpha(la(l)) {
            return false;
        }
        for &x in b"stanceof" {
            if la(l) != x as i32 {
                return true;
            }
            skip(l);
        }
        if !is_alpha(la(l)) {
            return false;
        }
    }
    true
}

unsafe fn scan_ternary_qmark(l: *mut TSLexer) -> bool {
    while is_space(la(l)) {
        skip(l);
    }
    if la(l) == ch(b'?') {
        advance(l);
        // Optional chaining.
        if la(l) == ch(b'?') || la(l) == ch(b'.') {
            return false;
        }
        mark_end(l);
        (*l).result_symbol = TERNARY_QMARK as u16;
        // TypeScript optional arguments contain the ?: sequence, possibly with whitespace.
        while is_space(la(l)) {
            advance(l);
        }
        if la(l) == ch(b':') || la(l) == ch(b')') || la(l) == ch(b',') {
            return false;
        }
        if la(l) == ch(b'.') {
            advance(l);
            return is_digit(la(l));
        }
        return true;
    }
    false
}

unsafe fn scan_closing_comment(l: *mut TSLexer) -> bool {
    while is_space(la(l)) || la(l) == 0x2028 || la(l) == 0x2029 {
        skip(l);
    }
    if la(l) == ch(b'<') {
        for &x in b"<!--" {
            if la(l) != x as i32 {
                return false;
            }
            advance(l);
        }
    } else if la(l) == ch(b'-') {
        for &x in b"-->" {
            if la(l) != x as i32 {
                return false;
            }
            advance(l);
        }
    } else {
        return false;
    }
    while la(l) != 0 && la(l) != ch(b'\n') && la(l) != 0x2028 && la(l) != 0x2029 {
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
        && la(l) != ch(b'<')
        && la(l) != ch(b'>')
        && la(l) != ch(b'{')
        && la(l) != ch(b'}')
        && la(l) != ch(b'&')
    {
        let is_wspace = is_space(la(l));
        if la(l) == ch(b'\n') {
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

unsafe fn external_scanner_scan(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
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
    if valid(AUTOMATIC_SEMICOLON) || valid(FUNCTION_SIGNATURE_AUTOMATIC_SEMICOLON) {
        let mut scanned_comment = false;
        let ret = scan_automatic_semicolon(lexer, &valid, &mut scanned_comment);
        if !ret && !scanned_comment && valid(TERNARY_QMARK) && la(lexer) == ch(b'?') {
            return scan_ternary_qmark(lexer);
        }
        return ret;
    }
    if valid(TERNARY_QMARK) {
        return scan_ternary_qmark(lexer);
    }
    if valid(HTML_COMMENT) && !valid(LOGICAL_OR) && !valid(ESCAPE_SEQUENCE) && !valid(REGEX_PATTERN)
    {
        return scan_closing_comment(lexer);
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_tsx_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_tsx_external_scanner_destroy(_payload: *mut c_void) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_tsx_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_tsx_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_tsx_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    external_scanner_scan(lexer, valid_symbols)
}
