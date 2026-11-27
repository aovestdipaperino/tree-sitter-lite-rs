//! Rust port of tree-sitter-ocaml's external scanner (common/scanner.h, included by
//! grammars/ocaml/src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn, unused_assignments)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// enum TokenType
const COMMENT: usize = 0;
const LEFT_QUOTED_STRING_DELIM: usize = 1;
const RIGHT_QUOTED_STRING_DELIM: usize = 2;
const STRING_DELIM: usize = 3;
const LINE_NUMBER_DIRECTIVE: usize = 4;
const NULL_CHARACTER: usize = 5;
const ERROR_SENTINEL: usize = 6;

const TREE_SITTER_SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// The C scanner: `quoted_string_id` is a `int32_t` array of `capacity` elements of which the first
/// `length` are in use. `buf.len()` is the capacity; memory the C leaves uninitialised (what a
/// `ts_realloc` returns beyond the copied bytes) reads as zero here.
#[derive(Default)]
struct Scanner {
    in_string: bool,
    quoted_string_id_length: usize,
    buf: Vec<i32>,
}

impl Scanner {
    fn quoted_string_id_clear(&mut self) {
        self.quoted_string_id_length = 0;
    }

    fn quoted_string_id_resize(&mut self, min_capacity: usize) {
        let mut capacity = self.buf.len();
        if capacity >= min_capacity {
            return;
        }
        if capacity < 16 {
            capacity = 16;
        }
        while capacity < min_capacity {
            capacity <<= 1;
        }
        self.buf.resize(capacity, 0);
    }

    /// `memcpy(quoted_string_id, buffer, length)`: `length` bytes (not elements) are copied, into a
    /// buffer of `capacity` elements, which is always large enough.
    unsafe fn quoted_string_id_assign(&mut self, buffer: *const u8, length: usize) {
        if length > 0 {
            self.quoted_string_id_resize(length);
            core::ptr::copy_nonoverlapping(buffer, self.buf.as_mut_ptr() as *mut u8, length);
        }
        self.quoted_string_id_length = length;
    }

    /// `memcpy(buffer, quoted_string_id, length)`: `length` bytes of the element array. The length
    /// is below the capacity in elements (invariant of resize/push/assign), so the read is in
    /// bounds.
    unsafe fn quoted_string_id_copy(&self, buffer: *mut u8) -> usize {
        let length = self.quoted_string_id_length;
        if length > 0 {
            debug_assert!(length <= self.buf.len() * 4);
            core::ptr::copy_nonoverlapping(self.buf.as_ptr() as *const u8, buffer, length);
        }
        length
    }

    fn quoted_string_id_push(&mut self, c: i32) {
        self.quoted_string_id_resize(self.quoted_string_id_length + 1);
        self.buf[self.quoted_string_id_length] = c;
        self.quoted_string_id_length += 1;
    }
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}

// The C calls iswspace, iswdigit and towupper. A program that never calls setlocale is in the C
// locale, where these are ASCII only.

/// `iswspace` in the C locale: tab, newline, vertical tab, form feed, carriage return, space.
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswdigit` in the C locale.
fn iswdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}

/// `towupper` in the C locale: ASCII lowercase letters only.
fn towupper(c: i32) -> i32 {
    if (b'a' as i32..=b'z' as i32).contains(&c) {
        c - 32
    } else {
        c
    }
}

unsafe fn scan_string(lexer: *mut TSLexer) {
    loop {
        match (*lexer).lookahead {
            0x5c => {
                advance(lexer);
                advance(lexer);
            }
            0x22 => {
                advance(lexer);
                return;
            }
            0 => {
                if eof(lexer) {
                    return;
                }
                advance(lexer);
            }
            _ => advance(lexer),
        }
    }
}

const LOWER_UTF8_CHARS: [i32; 35] = [
    0xdf, 0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee,
    0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
    0x153, 0x161, 0x17e,
];

const LOWER_UTF8_PAIRS: [[i32; 3]; 29] = [
    ['a' as i32, 0x300, 0xe0],
    ['a' as i32, 0x301, 0xe1],
    ['a' as i32, 0x302, 0xe2],
    ['a' as i32, 0x303, 0xe3],
    ['a' as i32, 0x308, 0xe4],
    ['a' as i32, 0x30a, 0xe5],
    ['c' as i32, 0x327, 0xe7],
    ['e' as i32, 0x300, 0xe8],
    ['e' as i32, 0x301, 0xe9],
    ['e' as i32, 0x302, 0xea],
    ['e' as i32, 0x308, 0xeb],
    ['i' as i32, 0x300, 0xec],
    ['i' as i32, 0x301, 0xed],
    ['i' as i32, 0x302, 0xee],
    ['i' as i32, 0x308, 0xef],
    ['n' as i32, 0x303, 0xf1],
    ['o' as i32, 0x300, 0xf2],
    ['o' as i32, 0x301, 0xf3],
    ['o' as i32, 0x302, 0xf4],
    ['o' as i32, 0x303, 0xf5],
    ['o' as i32, 0x308, 0xf6],
    ['s' as i32, 0x30c, 0x161],
    ['u' as i32, 0x300, 0xf9],
    ['u' as i32, 0x301, 0xfa],
    ['u' as i32, 0x302, 0xfb],
    ['u' as i32, 0x308, 0xfc],
    ['y' as i32, 0x301, 0xfd],
    ['y' as i32, 0x308, 0xff],
    ['z' as i32, 0x30c, 0x17e],
];

fn search_lower_utf8_char(c: i32) -> i32 {
    let mut base = 0usize;
    let mut range = LOWER_UTF8_CHARS.len();
    while range != 0 {
        let probe = base + (range >> 1);
        if c == LOWER_UTF8_CHARS[probe] {
            return LOWER_UTF8_CHARS[probe];
        } else if c > LOWER_UTF8_CHARS[probe] {
            base = probe + 1;
            range -= 1;
        }
        range >>= 1;
    }
    0
}

fn search_lower_utf8_pair(c: i32, diacritic: i32) -> i32 {
    let mut base = 0usize;
    let mut range = LOWER_UTF8_PAIRS.len();
    while range != 0 {
        let probe = &LOWER_UTF8_PAIRS[base + (range >> 1)];
        if c == probe[0] && diacritic == probe[1] {
            return probe[2];
        } else if c > probe[0] || (c == probe[0] && diacritic > probe[1]) {
            base = base + (range >> 1) + 1;
            range -= 1;
        }
        range >>= 1;
    }
    0
}

unsafe fn scan_quoted_string_delim_char(lexer: *mut TSLexer) -> i32 {
    let c = (*lexer).lookahead;

    if c == '|' as i32 {
        return 0;
    }

    if c == '_' as i32 {
        advance(lexer);
        return c;
    }

    if c >= 'a' as i32 && c <= 'z' as i32 {
        advance(lexer);
        let la = (*lexer).lookahead;
        if la >= 0x300 && la <= 0x327 {
            let r = search_lower_utf8_pair(c, la);
            if r != 0 {
                advance(lexer);
                return r;
            }
        }
        return c;
    }

    let r = search_lower_utf8_char(c);
    if r != 0 {
        advance(lexer);
        return r;
    }

    0
}

unsafe fn scan_left_quoted_string_delimiter(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    loop {
        let c = scan_quoted_string_delim_char(lexer);
        if c == 0 {
            break;
        }
        scanner.quoted_string_id_push(c);
    }

    if (*lexer).lookahead == '|' as i32 {
        advance(lexer);
        scanner.in_string = true;
        true
    } else {
        scanner.quoted_string_id_clear();
        false
    }
}

unsafe fn scan_right_quoted_string_delimiter(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    for i in 0..scanner.quoted_string_id_length {
        if scan_quoted_string_delim_char(lexer) != scanner.buf[i] {
            return false;
        }
    }

    if (*lexer).lookahead == '}' as i32 {
        scanner.in_string = false;
        scanner.quoted_string_id_clear();
        true
    } else {
        false
    }
}

unsafe fn scan_quoted_string(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    if !scan_left_quoted_string_delimiter(scanner, lexer) {
        return false;
    }

    loop {
        match (*lexer).lookahead {
            0x7c => {
                advance(lexer);
                if scan_right_quoted_string_delimiter(scanner, lexer) {
                    return true;
                }
            }
            0 => {
                if eof(lexer) {
                    return false;
                }
                advance(lexer);
            }
            _ => advance(lexer),
        }
    }
}

unsafe fn scan_character(lexer: *mut TSLexer) -> i32 {
    let mut last: i32 = 0;

    match (*lexer).lookahead {
        0x5c => {
            advance(lexer);
            if iswdigit((*lexer).lookahead) {
                advance(lexer);
                for _ in 0..2 {
                    if !iswdigit((*lexer).lookahead) {
                        return 0;
                    }
                    advance(lexer);
                }
            } else {
                match (*lexer).lookahead {
                    0x78 => {
                        advance(lexer);
                        for _ in 0..2 {
                            let la = (*lexer).lookahead;
                            if !iswdigit(la)
                                && (towupper(la) < 'A' as i32 || towupper(la) > 'F' as i32)
                            {
                                return 0;
                            }
                            advance(lexer);
                        }
                    }
                    0x6f => {
                        advance(lexer);
                        for _ in 0..3 {
                            let la = (*lexer).lookahead;
                            if !iswdigit(la) || la > '7' as i32 {
                                return 0;
                            }
                            advance(lexer);
                        }
                    }
                    0x27 | 0x22 | 0x5c | 0x6e | 0x74 | 0x62 | 0x72 | 0x20 => {
                        last = (*lexer).lookahead;
                        advance(lexer);
                    }
                    _ => return 0,
                }
            }
        }
        0x27 => {}
        0x0d => {
            advance(lexer);
            while (*lexer).lookahead == '\r' as i32 {
                advance(lexer);
            }
            if (*lexer).lookahead != '\n' as i32 {
                return 0;
            }
            advance(lexer);
        }
        0 => {
            if eof(lexer) {
                return 0;
            }
            advance(lexer);
        }
        _ => {
            if (*lexer).lookahead < 256 {
                last = (*lexer).lookahead;
                advance(lexer);
            } else {
                return 0;
            }
        }
    }

    if (*lexer).lookahead == '\'' as i32 {
        advance(lexer);
        return 0;
    }
    last
}

fn is_lowercase_ext(c: i32) -> bool {
    (c >= 'a' as i32 && c <= 'z' as i32) || c == '_' as i32 || c >= 192
}

fn is_identstart(c: i32) -> bool {
    is_lowercase_ext(c) || (c >= 'A' as i32 && c <= 'Z' as i32)
}

fn is_identchar(c: i32) -> bool {
    is_identstart(c) || (c >= '0' as i32 && c <= '9' as i32) || c == '\'' as i32
}

unsafe fn scan_identifier(lexer: *mut TSLexer) -> bool {
    if is_identstart((*lexer).lookahead) {
        advance(lexer);
        while is_identchar((*lexer).lookahead) {
            advance(lexer);
        }
        return true;
    }
    false
}

unsafe fn scan_extattrident(lexer: *mut TSLexer) -> bool {
    while scan_identifier(lexer) {
        if (*lexer).lookahead != '.' as i32 {
            return true;
        }
        advance(lexer);
    }
    false
}

unsafe fn scan_comment(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    let mut last: i32 = 0;

    if (*lexer).lookahead != '*' as i32 {
        return false;
    }
    advance(lexer);

    // `if (last) last = 0; else advance(lexer);`
    macro_rules! consume {
        () => {
            if last != 0 {
                last = 0;
            } else {
                advance(lexer);
            }
        };
    }

    loop {
        let sw = if last != 0 { last } else { (*lexer).lookahead };
        match sw {
            0x28 => {
                consume!();
                scan_comment(scanner, lexer);
            }
            0x2a => {
                consume!();
                if (*lexer).lookahead == ')' as i32 {
                    advance(lexer);
                    return true;
                }
            }
            0x27 => {
                consume!();
                last = scan_character(lexer);
            }
            0x22 => {
                consume!();
                scan_string(lexer);
            }
            0x7b => 'arm: {
                consume!();
                if (*lexer).lookahead == '%' as i32 {
                    advance(lexer);
                    if (*lexer).lookahead == '%' as i32 {
                        advance(lexer);
                    }
                    if scan_extattrident(lexer) {
                        while iswspace((*lexer).lookahead) {
                            advance(lexer);
                        }
                    } else {
                        break 'arm;
                    }
                }
                if scan_quoted_string(scanner, lexer) {
                    advance(lexer);
                }
            }
            0 => {
                if eof(lexer) {
                    return true;
                }
                consume!();
            }
            _ => {
                if scan_identifier(lexer) || last != 0 {
                    last = 0;
                } else {
                    advance(lexer);
                }
            }
        }
    }
}

unsafe fn serialize(scanner: &Scanner, buffer: *mut c_char) -> u32 {
    let buf = buffer as *mut u8;
    *buf = scanner.in_string as u8;
    if scanner.quoted_string_id_length < TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        (scanner.quoted_string_id_copy(buf.add(1)) + 1) as u32
    } else {
        1
    }
}

unsafe fn deserialize(scanner: &mut Scanner, buffer: *const c_char, length: u32) {
    if length > 0 {
        let buf = buffer as *const u8;
        scanner.in_string = *buf != 0;
        scanner.quoted_string_id_assign(buf.add(1), (length - 1) as usize);
    } else {
        scanner.in_string = false;
        scanner.quoted_string_id_clear();
    }
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);

    if !valid(ERROR_SENTINEL)
        && valid(LEFT_QUOTED_STRING_DELIM)
        && (is_lowercase_ext((*lexer).lookahead) || (*lexer).lookahead == '|' as i32)
    {
        (*lexer).result_symbol = LEFT_QUOTED_STRING_DELIM as u16;
        return scan_left_quoted_string_delimiter(scanner, lexer);
    }

    if !valid(ERROR_SENTINEL)
        && valid(RIGHT_QUOTED_STRING_DELIM)
        && (*lexer).lookahead == '|' as i32
    {
        advance(lexer);
        (*lexer).result_symbol = RIGHT_QUOTED_STRING_DELIM as u16;
        return scan_right_quoted_string_delimiter(scanner, lexer);
    }

    if scanner.in_string && valid(STRING_DELIM) && (*lexer).lookahead == '"' as i32 {
        advance(lexer);
        scanner.in_string = false;
        (*lexer).result_symbol = STRING_DELIM as u16;
        return true;
    }

    while iswspace((*lexer).lookahead) {
        skip(lexer);
    }

    if !scanner.in_string && valid(STRING_DELIM) && (*lexer).lookahead == '"' as i32 {
        advance(lexer);
        scanner.in_string = true;
        (*lexer).result_symbol = STRING_DELIM as u16;
        return true;
    }

    if !scanner.in_string
        && valid(LINE_NUMBER_DIRECTIVE)
        && (*lexer).lookahead == '#' as i32
        && ((*lexer).get_column)(lexer) == 0
    {
        advance(lexer);

        while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
            advance(lexer);
        }

        if !iswdigit((*lexer).lookahead) {
            return false;
        }
        while iswdigit((*lexer).lookahead) {
            advance(lexer);
        }

        while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
            advance(lexer);
        }

        if (*lexer).lookahead != '"' as i32 {
            return false;
        }
        advance(lexer);

        while (*lexer).lookahead != '\n' as i32
            && (*lexer).lookahead != '\r' as i32
            && (*lexer).lookahead != '"' as i32
            && !eof(lexer)
        {
            advance(lexer);
        }

        if (*lexer).lookahead != '"' as i32 {
            return false;
        }
        advance(lexer);

        while (*lexer).lookahead != '\n' as i32 && (*lexer).lookahead != '\r' as i32 && !eof(lexer)
        {
            advance(lexer);
        }

        (*lexer).result_symbol = LINE_NUMBER_DIRECTIVE as u16;
        return true;
    }

    if !scanner.in_string && valid(COMMENT) && (*lexer).lookahead == '(' as i32 {
        advance(lexer);
        (*lexer).result_symbol = COMMENT as u16;
        return scan_comment(scanner, lexer);
    }

    if valid(NULL_CHARACTER) && (*lexer).lookahead == 0 && !eof(lexer) {
        advance(lexer);
        (*lexer).result_symbol = NULL_CHARACTER as u16;
        return true;
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ocaml_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ocaml_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ocaml_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    serialize(&*(payload as *const Scanner), buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ocaml_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    deserialize(&mut *(payload as *mut Scanner), buffer, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ocaml_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(&mut *(payload as *mut Scanner), lexer, valid_symbols)
}
