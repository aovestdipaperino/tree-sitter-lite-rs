//! Rust port of tree-sitter-r's external scanner (src/scanner.c of tree-sitter-r 1.3.0).
//!
//! Judgement calls relative to the C:
//! - `iswspace` / `iswalnum` are the C locale (ASCII only): `iswspace` is 9..=13 and 32,
//!   `iswalnum` is `0-9a-zA-Z`. Every other code point is handled by the C's own `x >= 128` test.
//! - `char` is signed, as on the platforms that matter (`closing_bracket`, `closing_quote`, `Scope`).
//! - The C `scopes_deserialize` trusts the serialized `count`: a corrupt buffer with
//!   `MAX_SCOPES_COUNT < count <= length` would `memcpy` past the heap allocation. Here such a
//!   count is rejected (the caller resets the payload), which gives the same result wherever the C
//!   stays in bounds.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const START: usize = 0;
const NEWLINE: usize = 1;
const SEMICOLON: usize = 2;
const RAW_STRING_OPEN: usize = 3;
const RAW_STRING_CONTENT: usize = 4;
const RAW_STRING_CLOSE: usize = 5;
const ELSE: usize = 6;
const OPEN_PAREN: usize = 7;
const CLOSE_PAREN: usize = 8;
const OPEN_BRACE: usize = 9;
const CLOSE_BRACE: usize = 10;
const OPEN_BRACKET: usize = 11;
const CLOSE_BRACKET: usize = 12;
const OPEN_BRACKET2: usize = 13;
const CLOSE_BRACKET2: usize = 14;
const ERROR_SENTINEL: usize = 15;

const TREE_SITTER_SERIALIZATION_BUFFER_SIZE: usize = 1024;

const RAW_STRING_CLOSING_BRACKET_SIZE: usize = 1;
const RAW_STRING_HYPHEN_COUNT_SIZE: usize = 1;
const RAW_STRING_CLOSING_QUOTE_SIZE: usize = 1;
const SCOPE_SIZE: usize = 1;
const COUNT_SIZE: usize = 4;

const MAX_SCOPES_COUNT: u32 = ((TREE_SITTER_SERIALIZATION_BUFFER_SIZE
    - RAW_STRING_CLOSING_BRACKET_SIZE
    - RAW_STRING_HYPHEN_COUNT_SIZE
    - RAW_STRING_CLOSING_QUOTE_SIZE
    - COUNT_SIZE)
    / SCOPE_SIZE) as u32;

type Scope = i8;
const SCOPE_TOP_LEVEL: Scope = 0;
const SCOPE_BRACE: Scope = 1;
const SCOPE_PAREN: Scope = 2;
const SCOPE_BRACKET: Scope = 3;
const SCOPE_BRACKET2: Scope = 4;

#[derive(Default)]
struct RawStringState {
    closing_bracket: i8,
    hyphen_count: u8,
    closing_quote: i8,
}

impl RawStringState {
    fn set(&mut self, closing_bracket: i8, hyphen_count: u8, closing_quote: i8) {
        self.closing_bracket = closing_bracket;
        self.hyphen_count = hyphen_count;
        self.closing_quote = closing_quote;
    }

    fn reset(&mut self) {
        self.set(0, 0, 0);
    }
}

struct Scopes {
    data: Box<[Scope]>,
    count: u32,
}

impl Scopes {
    fn new() -> Self {
        Scopes {
            data: vec![0; MAX_SCOPES_COUNT as usize].into_boxed_slice(),
            count: 0,
        }
    }

    fn reset(&mut self) {
        self.count = 0;
    }

    fn push(&mut self, scope: Scope) -> bool {
        if self.count >= MAX_SCOPES_COUNT {
            return false;
        }
        self.data[self.count as usize] = scope;
        self.count += 1;
        true
    }

    fn peek(&self) -> Scope {
        if self.count == 0 {
            SCOPE_TOP_LEVEL
        } else {
            self.data[self.count as usize - 1]
        }
    }

    fn pop(&mut self, expected: Scope) -> bool {
        if self.count == 0 {
            return false;
        }
        let scope = self.peek();
        self.count -= 1;
        scope == expected
    }
}

struct Payload {
    raw_string_state: RawStringState,
    scopes: Scopes,
}

impl Payload {
    fn reset(&mut self) {
        self.raw_string_state.reset();
        self.scopes.reset();
    }

    unsafe fn serialize(&self, buffer: *mut u8) -> u32 {
        let mut len = 0usize;
        *buffer.add(len) = self.raw_string_state.closing_bracket as u8;
        len += 1;
        *buffer.add(len) = self.raw_string_state.hyphen_count;
        len += 1;
        *buffer.add(len) = self.raw_string_state.closing_quote as u8;
        len += 1;
        let count = self.scopes.count.to_ne_bytes();
        core::ptr::copy_nonoverlapping(count.as_ptr(), buffer.add(len), COUNT_SIZE);
        len += COUNT_SIZE;
        if self.scopes.count > 0 {
            let n = self.scopes.count as usize * SCOPE_SIZE;
            core::ptr::copy_nonoverlapping(
                self.scopes.data.as_ptr() as *const u8,
                buffer.add(len),
                n,
            );
            len += n;
        }
        len as u32
    }

    unsafe fn deserialize(&mut self, buffer: *const u8, length: u32) -> bool {
        let mut p = buffer;
        let mut remaining = length;
        if (remaining as usize)
            < RAW_STRING_CLOSING_BRACKET_SIZE
                + RAW_STRING_HYPHEN_COUNT_SIZE
                + RAW_STRING_CLOSING_QUOTE_SIZE
        {
            return false;
        }
        self.raw_string_state.closing_bracket = *p as i8;
        p = p.add(1);
        self.raw_string_state.hyphen_count = *p;
        p = p.add(1);
        self.raw_string_state.closing_quote = *p as i8;
        p = p.add(1);
        remaining -= 3;

        if (remaining as usize) < COUNT_SIZE {
            return false;
        }
        let mut cb = [0u8; COUNT_SIZE];
        core::ptr::copy_nonoverlapping(p, cb.as_mut_ptr(), COUNT_SIZE);
        self.scopes.count = u32::from_ne_bytes(cb);
        p = p.add(COUNT_SIZE);
        remaining -= COUNT_SIZE as u32;

        if self.scopes.count > 0 {
            let scopes_size = self.scopes.count.wrapping_mul(SCOPE_SIZE as u32);
            if remaining < scopes_size {
                return false;
            }
            // Bound the copy by the real capacity (the C would write past its allocation here).
            if self.scopes.count > MAX_SCOPES_COUNT {
                return false;
            }
            core::ptr::copy_nonoverlapping(
                p,
                self.scopes.data.as_mut_ptr() as *mut u8,
                scopes_size as usize,
            );
        }
        true
    }
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

/// `iswspace` in the C locale.
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswalnum` in the C locale.
fn iswalnum(c: i32) -> bool {
    matches!(c, 48..=57 | 65..=90 | 97..=122)
}

unsafe fn consume_whitespace_and_ignored_newlines(lexer: *mut TSLexer, scopes: &Scopes) {
    while iswspace((*lexer).lookahead) {
        if (*lexer).lookahead != '\n' as i32 {
            skip(lexer);
            continue;
        }
        let scope = scopes.peek();
        if scope == SCOPE_PAREN || scope == SCOPE_BRACKET || scope == SCOPE_BRACKET2 {
            skip(lexer);
            continue;
        }
        break;
    }
}

fn is_identifier_continuation(x: i32) -> bool {
    iswalnum(x) || x == '_' as i32 || x == '.' as i32 || x >= 128
}

unsafe fn scan_else(lexer: *mut TSLexer) -> bool {
    for &c in b"else" {
        if (*lexer).lookahead != c as i32 {
            return false;
        }
        advance(lexer);
    }
    if is_identifier_continuation((*lexer).lookahead) {
        return false;
    }
    mark_end(lexer);
    (*lexer).result_symbol = ELSE as u16;
    true
}

unsafe fn scan_else_with_leading_newlines(lexer: *mut TSLexer) -> bool {
    while iswspace((*lexer).lookahead) {
        if (*lexer).lookahead != '\n' as i32 {
            skip(lexer);
            continue;
        }
        skip(lexer);
        mark_end(lexer);
        (*lexer).result_symbol = NEWLINE as u16;
    }
    if (*lexer).lookahead == '#' as i32 {
        return false;
    }
    scan_else(lexer);
    true
}

unsafe fn scan_raw_string_open(lexer: *mut TSLexer, st: &mut RawStringState) -> bool {
    let prefix = (*lexer).lookahead;
    if prefix != 'r' as i32 && prefix != 'R' as i32 {
        return false;
    }
    advance(lexer);

    let closing_quote = (*lexer).lookahead as i8;
    if closing_quote != b'"' as i8 && closing_quote != b'\'' as i8 {
        return false;
    }
    advance(lexer);

    let mut hyphen_count: u8 = 0;
    while (*lexer).lookahead == '-' as i32 {
        if hyphen_count == u8::MAX {
            return false;
        }
        advance(lexer);
        hyphen_count += 1;
    }

    let opening_bracket = (*lexer).lookahead;
    let closing_bracket: i8;
    if opening_bracket == '(' as i32 {
        closing_bracket = b')' as i8;
        advance(lexer);
    } else if opening_bracket == '[' as i32 {
        closing_bracket = b']' as i8;
        advance(lexer);
    } else if opening_bracket == '{' as i32 {
        closing_bracket = b'}' as i8;
        advance(lexer);
    } else {
        return false;
    }

    mark_end(lexer);
    (*lexer).result_symbol = RAW_STRING_OPEN as u16;
    st.set(closing_bracket, hyphen_count, closing_quote);
    true
}

unsafe fn scan_raw_string_content_or_close(lexer: *mut TSLexer, st: &RawStringState) -> bool {
    let closing_bracket = st.closing_bracket as i32;
    let hyphen_count = st.hyphen_count;
    let closing_quote = st.closing_quote as i32;

    let mut any_content = false;

    while !eof(lexer) {
        if (*lexer).lookahead != closing_bracket {
            advance(lexer);
            any_content = true;
            continue;
        }

        mark_end(lexer);
        advance(lexer);

        let mut matched_hyphens = true;
        for _ in 0..hyphen_count {
            if (*lexer).lookahead != '-' as i32 {
                matched_hyphens = false;
                break;
            }
            advance(lexer);
        }

        if !matched_hyphens {
            any_content = true;
            continue;
        }

        if (*lexer).lookahead != closing_quote {
            any_content = true;
            continue;
        }

        advance(lexer);

        if any_content {
            (*lexer).result_symbol = RAW_STRING_CONTENT as u16;
        } else {
            mark_end(lexer);
            (*lexer).result_symbol = RAW_STRING_CLOSE as u16;
        }
        return true;
    }
    false
}

unsafe fn scan_raw_string_close(lexer: *mut TSLexer, st: &RawStringState) -> bool {
    advance(lexer);
    for _ in 0..st.hyphen_count {
        advance(lexer);
    }
    advance(lexer);
    mark_end(lexer);
    (*lexer).result_symbol = RAW_STRING_CLOSE as u16;
    true
}

unsafe fn scan_semicolon(lexer: *mut TSLexer) -> bool {
    advance(lexer);
    mark_end(lexer);
    (*lexer).result_symbol = SEMICOLON as u16;
    true
}

unsafe fn scan_newline(lexer: *mut TSLexer) -> bool {
    advance(lexer);
    mark_end(lexer);
    (*lexer).result_symbol = NEWLINE as u16;
    true
}

unsafe fn scan_open_block(
    lexer: *mut TSLexer,
    scopes: &mut Scopes,
    scope: Scope,
    symbol: usize,
) -> bool {
    if !scopes.push(scope) {
        return false;
    }
    advance(lexer);
    mark_end(lexer);
    (*lexer).result_symbol = symbol as u16;
    true
}

unsafe fn scan_close_block(
    lexer: *mut TSLexer,
    scopes: &mut Scopes,
    scope: Scope,
    symbol: usize,
) -> bool {
    if !scopes.pop(scope) {
        return false;
    }
    advance(lexer);
    mark_end(lexer);
    (*lexer).result_symbol = symbol as u16;
    true
}

unsafe fn scan_open_bracket_or_bracket2(
    lexer: *mut TSLexer,
    scopes: &mut Scopes,
    valid: *const bool,
) -> bool {
    advance(lexer);

    if *valid.add(OPEN_BRACKET2) && (*lexer).lookahead == '[' as i32 {
        if !scopes.push(SCOPE_BRACKET2) {
            return false;
        }
        advance(lexer);
        mark_end(lexer);
        (*lexer).result_symbol = OPEN_BRACKET2 as u16;
        return true;
    }

    if *valid.add(OPEN_BRACKET) {
        if !scopes.push(SCOPE_BRACKET) {
            return false;
        }
        mark_end(lexer);
        (*lexer).result_symbol = OPEN_BRACKET as u16;
        return true;
    }

    false
}

unsafe fn scan_close_bracket2(lexer: *mut TSLexer, scopes: &mut Scopes) -> bool {
    advance(lexer);
    if (*lexer).lookahead != ']' as i32 {
        return false;
    }
    scan_close_block(lexer, scopes, SCOPE_BRACKET2, CLOSE_BRACKET2)
}

unsafe fn scan(lexer: *mut TSLexer, p: &mut Payload, valid: *const bool) -> bool {
    if *valid.add(ERROR_SENTINEL) {
        return false;
    }

    if *valid.add(START) {
        (*lexer).result_symbol = START as u16;
        return true;
    }

    if *valid.add(RAW_STRING_CONTENT) {
        return scan_raw_string_content_or_close(lexer, &p.raw_string_state);
    } else if *valid.add(RAW_STRING_CLOSE) {
        return scan_raw_string_close(lexer, &p.raw_string_state);
    }

    consume_whitespace_and_ignored_newlines(lexer, &p.scopes);

    let la = (*lexer).lookahead;
    if *valid.add(SEMICOLON) && la == ';' as i32 {
        scan_semicolon(lexer)
    } else if *valid.add(OPEN_PAREN) && la == '(' as i32 {
        scan_open_block(lexer, &mut p.scopes, SCOPE_PAREN, OPEN_PAREN)
    } else if *valid.add(CLOSE_PAREN) && la == ')' as i32 {
        scan_close_block(lexer, &mut p.scopes, SCOPE_PAREN, CLOSE_PAREN)
    } else if *valid.add(OPEN_BRACE) && la == '{' as i32 {
        scan_open_block(lexer, &mut p.scopes, SCOPE_BRACE, OPEN_BRACE)
    } else if *valid.add(CLOSE_BRACE) && la == '}' as i32 {
        scan_close_block(lexer, &mut p.scopes, SCOPE_BRACE, CLOSE_BRACE)
    } else if (*valid.add(OPEN_BRACKET) || *valid.add(OPEN_BRACKET2)) && la == '[' as i32 {
        scan_open_bracket_or_bracket2(lexer, &mut p.scopes, valid)
    } else if *valid.add(CLOSE_BRACKET) && la == ']' as i32 && p.scopes.peek() == SCOPE_BRACKET {
        scan_close_block(lexer, &mut p.scopes, SCOPE_BRACKET, CLOSE_BRACKET)
    } else if *valid.add(CLOSE_BRACKET2) && la == ']' as i32 && p.scopes.peek() == SCOPE_BRACKET2 {
        scan_close_bracket2(lexer, &mut p.scopes)
    } else if *valid.add(RAW_STRING_OPEN) && (la == 'r' as i32 || la == 'R' as i32) {
        scan_raw_string_open(lexer, &mut p.raw_string_state)
    } else if *valid.add(ELSE) && la == 'e' as i32 {
        scan_else(lexer)
    } else if *valid.add(ELSE) && p.scopes.peek() == SCOPE_BRACE && la == '\n' as i32 {
        scan_else_with_leading_newlines(lexer)
    } else if *valid.add(NEWLINE) && la == '\n' as i32 {
        scan_newline(lexer)
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_r_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Payload {
        raw_string_state: RawStringState::default(),
        scopes: Scopes::new(),
    })) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_r_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Payload));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_r_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    if payload.is_null() {
        return false;
    }
    scan(lexer, &mut *(payload as *mut Payload), valid_symbols)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_r_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    if payload.is_null() {
        return 0;
    }
    (*(payload as *mut Payload)).serialize(buffer as *mut u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_r_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    if payload.is_null() {
        return;
    }
    let p = &mut *(payload as *mut Payload);
    if length == 0 {
        p.reset();
        return;
    }
    if !p.deserialize(buffer as *const u8, length) {
        p.reset();
    }
}
