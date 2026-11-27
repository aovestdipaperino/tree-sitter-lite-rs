//! Rust port of tree-sitter-ruby's external scanner (src/scanner.c).
//!
//! Locale: the C calls `iswspace`, `iswalpha`, `iswalnum`, `iswdigit`, `iswlower` and `iswupper`
//! on the 32-bit lookahead. A program that never calls `setlocale` runs in the C locale, so these
//! are ASCII only here (see the `isw*` helpers below). `strchr` and `memchr` on a character are
//! ported with the C's `char` truncation of the lookahead.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const LINE_BREAK: usize = 0;
const NO_LINE_BREAK: usize = 1;
const SIMPLE_SYMBOL: usize = 2;
const STRING_START: usize = 3;
const SYMBOL_START: usize = 4;
const SUBSHELL_START: usize = 5;
const REGEX_START: usize = 6;
const STRING_ARRAY_START: usize = 7;
const SYMBOL_ARRAY_START: usize = 8;
const HEREDOC_BODY_START: usize = 9;
const STRING_CONTENT: usize = 10;
const HEREDOC_CONTENT: usize = 11;
const STRING_END: usize = 12;
const HEREDOC_BODY_END: usize = 13;
const HEREDOC_START: usize = 14;
const FORWARD_SLASH: usize = 15;
const BLOCK_AMPERSAND: usize = 16;
const SPLAT_STAR: usize = 17;
const UNARY_MINUS: usize = 18;
const UNARY_MINUS_NUM: usize = 19;
const BINARY_MINUS: usize = 20;
const BINARY_STAR: usize = 21;
const SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE: usize = 22;
const HASH_KEY_SYMBOL: usize = 23;
const IDENTIFIER_SUFFIX: usize = 24;
const CONSTANT_SUFFIX: usize = 25;
const HASH_SPLAT_STAR_STAR: usize = 26;
const BINARY_STAR_STAR: usize = 27;
const ELEMENT_REFERENCE_BRACKET: usize = 28;
const SHORT_INTERPOLATION: usize = 29;
const NONE: usize = 30;

/// `TREE_SITTER_SERIALIZATION_BUFFER_SIZE`
const SERIALIZATION_BUFFER_SIZE: u32 = 1024;

const NON_IDENTIFIER_CHARS: [u8; 36] = [
    0, b'\n', b'\r', b'\t', b' ', b':', b';', b'`', b'"', b'\'', b'@', b'$', b'#', b'.', b',',
    b'|', b'^', b'&', b'<', b'=', b'>', b'+', b'-', b'*', b'/', b'\\', b'%', b'?', b'!', b'~',
    b'(', b')', b'[', b']', b'{', b'}',
];

// The C locale: ASCII only.
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}
fn iswdigit(c: i32) -> bool {
    matches!(c, 48..=57)
}
fn iswlower(c: i32) -> bool {
    matches!(c, 97..=122)
}
fn iswupper(c: i32) -> bool {
    matches!(c, 65..=90)
}
fn iswalpha(c: i32) -> bool {
    iswlower(c) || iswupper(c)
}
fn iswalnum(c: i32) -> bool {
    iswalpha(c) || iswdigit(c)
}

#[derive(Clone, Copy, Default)]
struct Literal {
    type_: i32,
    open_delimiter: i32,
    close_delimiter: i32,
    nesting_depth: i32,
    allows_interpolation: bool,
}

#[derive(Default)]
struct Heredoc {
    /// `Array(char)`: signed bytes, as the C's `char` is signed on the supported platforms.
    word: Vec<i8>,
    end_word_indentation_allowed: bool,
    allows_interpolation: bool,
    started: bool,
}

#[derive(Default)]
struct Scanner {
    has_leading_whitespace: bool,
    literal_stack: Vec<Literal>,
    open_heredocs: Vec<Heredoc>,
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

/// `memchr(NON_IDENTIFIER_CHARS, c, ...) == NULL` with `c` a `char` (the low byte of the lookahead).
fn is_iden_char(c: i32) -> bool {
    let b = c as u8;
    !NON_IDENTIFIER_CHARS.contains(&b)
}

unsafe fn scan_operator(lexer: *mut TSLexer) -> bool {
    match (*lexer).lookahead {
        // <, <=, <<, <=>
        0x3c => {
            advance(lexer);
            if (*lexer).lookahead == '<' as i32 {
                advance(lexer);
            } else if (*lexer).lookahead == '=' as i32 {
                advance(lexer);
                if (*lexer).lookahead == '>' as i32 {
                    advance(lexer);
                }
            }
            true
        }
        // >, >=, >>
        0x3e => {
            advance(lexer);
            if (*lexer).lookahead == '>' as i32 || (*lexer).lookahead == '=' as i32 {
                advance(lexer);
            }
            true
        }
        // ==, ===, =~
        0x3d => {
            advance(lexer);
            if (*lexer).lookahead == '~' as i32 {
                advance(lexer);
                return true;
            }
            if (*lexer).lookahead == '=' as i32 {
                advance(lexer);
                if (*lexer).lookahead == '=' as i32 {
                    advance(lexer);
                }
                return true;
            }
            false
        }
        // +, -, ~, +@, -@, ~@
        0x2b | 0x2d | 0x7e => {
            advance(lexer);
            if (*lexer).lookahead == '@' as i32 {
                advance(lexer);
            }
            true
        }
        // ..
        0x2e => {
            advance(lexer);
            if (*lexer).lookahead == '.' as i32 {
                advance(lexer);
                return true;
            }
            false
        }
        // &, ^, |, /, %, `
        0x26 | 0x5e | 0x7c | 0x2f | 0x25 | 0x60 => {
            advance(lexer);
            true
        }
        // !, !=, !~
        0x21 => {
            advance(lexer);
            if (*lexer).lookahead == '=' as i32 || (*lexer).lookahead == '~' as i32 {
                advance(lexer);
            }
            true
        }
        // *, **
        0x2a => {
            advance(lexer);
            if (*lexer).lookahead == '*' as i32 {
                advance(lexer);
            }
            true
        }
        // [], []=
        0x5b => {
            advance(lexer);
            if (*lexer).lookahead == ']' as i32 {
                advance(lexer);
            } else {
                return false;
            }
            if (*lexer).lookahead == '=' as i32 {
                advance(lexer);
            }
            true
        }
        _ => false,
    }
}

unsafe fn scan_symbol_identifier(lexer: *mut TSLexer) -> bool {
    if (*lexer).lookahead == '@' as i32 {
        advance(lexer);
        if (*lexer).lookahead == '@' as i32 {
            advance(lexer);
        }
    } else if (*lexer).lookahead == '$' as i32 {
        advance(lexer);
    }

    if is_iden_char((*lexer).lookahead) {
        advance(lexer);
    } else if !scan_operator(lexer) {
        return false;
    }

    while is_iden_char((*lexer).lookahead) {
        advance(lexer);
    }

    if (*lexer).lookahead == '?' as i32 || (*lexer).lookahead == '!' as i32 {
        advance(lexer);
    }

    if (*lexer).lookahead == '=' as i32 {
        mark_end(lexer);
        advance(lexer);
        if (*lexer).lookahead != '>' as i32 {
            mark_end(lexer);
        }
    }

    true
}

unsafe fn scan_open_delimiter(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    literal: &mut Literal,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    match (*lexer).lookahead {
        0x22 => {
            literal.type_ = STRING_START as i32;
            literal.open_delimiter = (*lexer).lookahead;
            literal.close_delimiter = (*lexer).lookahead;
            literal.allows_interpolation = true;
            advance(lexer);
            true
        }
        0x27 => {
            literal.type_ = STRING_START as i32;
            literal.open_delimiter = (*lexer).lookahead;
            literal.close_delimiter = (*lexer).lookahead;
            literal.allows_interpolation = false;
            advance(lexer);
            true
        }
        0x60 => {
            if !valid(SUBSHELL_START) {
                return false;
            }
            literal.type_ = SUBSHELL_START as i32;
            literal.open_delimiter = (*lexer).lookahead;
            literal.close_delimiter = (*lexer).lookahead;
            literal.allows_interpolation = true;
            advance(lexer);
            true
        }
        0x2f => {
            if !valid(REGEX_START) {
                return false;
            }
            literal.type_ = REGEX_START as i32;
            literal.open_delimiter = (*lexer).lookahead;
            literal.close_delimiter = (*lexer).lookahead;
            literal.allows_interpolation = true;
            advance(lexer);
            if valid(FORWARD_SLASH) {
                if !scanner.has_leading_whitespace {
                    return false;
                }
                let la = (*lexer).lookahead;
                if la == ' ' as i32 || la == '\t' as i32 || la == '\n' as i32 || la == '\r' as i32 {
                    return false;
                }
                if la == '=' as i32 {
                    return false;
                }
            }
            true
        }
        0x25 => {
            advance(lexer);

            match (*lexer).lookahead {
                0x73 => {
                    // 's'
                    if !valid(SIMPLE_SYMBOL) {
                        return false;
                    }
                    literal.type_ = SYMBOL_START as i32;
                    literal.allows_interpolation = false;
                    advance(lexer);
                }
                0x72 => {
                    // 'r'
                    if !valid(REGEX_START) {
                        return false;
                    }
                    literal.type_ = REGEX_START as i32;
                    literal.allows_interpolation = true;
                    advance(lexer);
                }
                0x78 => {
                    // 'x'
                    if !valid(SUBSHELL_START) {
                        return false;
                    }
                    literal.type_ = SUBSHELL_START as i32;
                    literal.allows_interpolation = true;
                    advance(lexer);
                }
                0x71 => {
                    // 'q'
                    if !valid(STRING_START) {
                        return false;
                    }
                    literal.type_ = STRING_START as i32;
                    literal.allows_interpolation = false;
                    advance(lexer);
                }
                0x51 => {
                    // 'Q'
                    if !valid(STRING_START) {
                        return false;
                    }
                    literal.type_ = STRING_START as i32;
                    literal.allows_interpolation = true;
                    advance(lexer);
                }
                0x77 => {
                    // 'w'
                    if !valid(STRING_ARRAY_START) {
                        return false;
                    }
                    literal.type_ = STRING_ARRAY_START as i32;
                    literal.allows_interpolation = false;
                    advance(lexer);
                }
                0x69 => {
                    // 'i'
                    if !valid(SYMBOL_ARRAY_START) {
                        return false;
                    }
                    literal.type_ = SYMBOL_ARRAY_START as i32;
                    literal.allows_interpolation = false;
                    advance(lexer);
                }
                0x57 => {
                    // 'W'
                    if !valid(STRING_ARRAY_START) {
                        return false;
                    }
                    literal.type_ = STRING_ARRAY_START as i32;
                    literal.allows_interpolation = true;
                    advance(lexer);
                }
                0x49 => {
                    // 'I'
                    if !valid(SYMBOL_ARRAY_START) {
                        return false;
                    }
                    literal.type_ = SYMBOL_ARRAY_START as i32;
                    literal.allows_interpolation = true;
                    advance(lexer);
                }
                _ => {
                    if !valid(STRING_START) {
                        return false;
                    }
                    literal.type_ = STRING_START as i32;
                    literal.allows_interpolation = true;
                }
            }

            let la = (*lexer).lookahead;
            match la {
                0x28 => {
                    literal.open_delimiter = '(' as i32;
                    literal.close_delimiter = ')' as i32;
                }
                0x5b => {
                    literal.open_delimiter = '[' as i32;
                    literal.close_delimiter = ']' as i32;
                }
                0x7b => {
                    literal.open_delimiter = '{' as i32;
                    literal.close_delimiter = '}' as i32;
                }
                0x3c => {
                    literal.open_delimiter = '<' as i32;
                    literal.close_delimiter = '>' as i32;
                }
                // '\r' '\n' ' ' '\t'
                0x0d | 0x0a | 0x20 | 0x09 => {
                    // If the `/` operator is valid, then so is the `%` operator, which means that
                    // a `%` followed by whitespace should be considered an operator, not a
                    // percent string.
                    if valid(FORWARD_SLASH) {
                        return false;
                    }
                }
                // | ! # / \ @ $ % ^ & * ) ] } > + - ~ ` , . ? : ; _ " '
                0x7c | 0x21 | 0x23 | 0x2f | 0x5c | 0x40 | 0x24 | 0x25 | 0x5e | 0x26 | 0x2a
                | 0x29 | 0x5d | 0x7d | 0x3e | 0x2b | 0x2d | 0x7e | 0x60 | 0x2c | 0x2e | 0x3f
                | 0x3a | 0x3b | 0x5f | 0x22 | 0x27 => {
                    literal.open_delimiter = la;
                    literal.close_delimiter = la;
                }
                _ => return false,
            }

            advance(lexer);
            true
        }
        _ => false,
    }
}

unsafe fn scan_heredoc_word(lexer: *mut TSLexer, heredoc: &mut Heredoc) {
    let mut word: Vec<i8> = Vec::new();
    let mut quote: i32 = 0;

    match (*lexer).lookahead {
        0x27 | 0x22 | 0x60 => {
            quote = (*lexer).lookahead;
            advance(lexer);
            while (*lexer).lookahead != quote && !eof(lexer) {
                word.push((*lexer).lookahead as i8);
                advance(lexer);
            }
            advance(lexer);
        }
        _ => {
            if iswalnum((*lexer).lookahead) || (*lexer).lookahead == '_' as i32 {
                word.push((*lexer).lookahead as i8);
                advance(lexer);
                while iswalnum((*lexer).lookahead) || (*lexer).lookahead == '_' as i32 {
                    word.push((*lexer).lookahead as i8);
                    advance(lexer);
                }
            }
        }
    }

    heredoc.word = word;
    heredoc.allows_interpolation = quote != '\'' as i32;
}

unsafe fn scan_short_interpolation(
    lexer: *mut TSLexer,
    has_content: bool,
    content_symbol: usize,
) -> bool {
    let start = (*lexer).lookahead as u8;
    if start == b'@' || start == b'$' {
        if has_content {
            (*lexer).result_symbol = content_symbol as u16;
            return true;
        }
        mark_end(lexer);
        advance(lexer);
        let mut is_short_interpolation = false;
        if start == b'$' {
            // `strchr(set, lookahead)`: the int is converted to char, and a NUL (including the
            // end of input, whose lookahead is 0) matches the terminator of the literal.
            let b = (*lexer).lookahead as u8;
            if b == 0 || b"!@&`'+~=/\\,;.<>*$?:\"".contains(&b) {
                is_short_interpolation = true;
            } else if (*lexer).lookahead == '-' as i32 {
                advance(lexer);
                is_short_interpolation =
                    iswalpha((*lexer).lookahead) || (*lexer).lookahead == '_' as i32;
            } else {
                is_short_interpolation =
                    iswalnum((*lexer).lookahead) || (*lexer).lookahead == '_' as i32;
            }
        }
        if start == b'@' {
            if (*lexer).lookahead == '@' as i32 {
                advance(lexer);
            }
            is_short_interpolation =
                is_iden_char((*lexer).lookahead) && !iswdigit((*lexer).lookahead);
        }

        if is_short_interpolation {
            (*lexer).result_symbol = SHORT_INTERPOLATION as u16;
            return true;
        }
    }
    false
}

unsafe fn scan_whitespace(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    let heredoc_body_start_is_valid = !scanner.open_heredocs.is_empty()
        && !scanner.open_heredocs[0].started
        && valid(HEREDOC_BODY_START);
    let mut crossed_newline = false;

    loop {
        if !valid(NO_LINE_BREAK)
            && valid(LINE_BREAK)
            && ((*lexer).is_at_included_range_start)(lexer)
        {
            mark_end(lexer);
            (*lexer).result_symbol = LINE_BREAK as u16;
            return true;
        }

        match (*lexer).lookahead {
            0x20 | 0x09 => skip(scanner, lexer),
            0x0d => {
                if heredoc_body_start_is_valid {
                    (*lexer).result_symbol = HEREDOC_BODY_START as u16;
                    scanner.open_heredocs[0].started = true;
                    return true;
                } else {
                    skip(scanner, lexer);
                }
            }
            0x0a => {
                if heredoc_body_start_is_valid {
                    (*lexer).result_symbol = HEREDOC_BODY_START as u16;
                    scanner.open_heredocs[0].started = true;
                    return true;
                } else if !valid(NO_LINE_BREAK) && valid(LINE_BREAK) && !crossed_newline {
                    mark_end(lexer);
                    advance(lexer);
                    crossed_newline = true;
                } else {
                    skip(scanner, lexer);
                }
            }
            0x5c => {
                advance(lexer);
                if (*lexer).lookahead == '\r' as i32 {
                    skip(scanner, lexer);
                }
                if iswspace((*lexer).lookahead) {
                    skip(scanner, lexer);
                } else {
                    return false;
                }
            }
            _ => {
                if crossed_newline {
                    let la = (*lexer).lookahead;
                    if la != '.' as i32 && la != '&' as i32 && la != '#' as i32 {
                        (*lexer).result_symbol = LINE_BREAK as u16;
                    } else if la == '.' as i32 {
                        // Don't return LINE_BREAK for the call operator (`.`) but do return one for
                        // range operators (`..` and `...`)
                        advance(lexer);
                        if !eof(lexer) && (*lexer).lookahead == '.' as i32 {
                            (*lexer).result_symbol = LINE_BREAK as u16;
                        } else {
                            return false;
                        }
                    }
                }
                return true;
            }
        }
    }
}

unsafe fn skip(scanner: &mut Scanner, lexer: *mut TSLexer) {
    scanner.has_leading_whitespace = true;
    ((*lexer).advance)(lexer, true);
}

impl Scanner {
    fn reset(&mut self) {
        self.literal_stack = Vec::new();
        self.open_heredocs = Vec::new();
    }

    unsafe fn serialize(&self, buffer: *mut c_char) -> u32 {
        let mut size: u32 = 0;
        let put = |size: &mut u32, v: u8| {
            *buffer.add(*size as usize) = v as c_char;
            *size += 1;
        };

        if (self.literal_stack.len() as u32)
            .wrapping_mul(5)
            .wrapping_add(2)
            >= SERIALIZATION_BUFFER_SIZE
        {
            return 0;
        }

        put(&mut size, self.literal_stack.len() as u8);
        for literal in &self.literal_stack {
            put(&mut size, literal.type_ as u8);
            put(&mut size, literal.open_delimiter as u8);
            put(&mut size, literal.close_delimiter as u8);
            put(&mut size, literal.nesting_depth as u8);
            put(&mut size, literal.allows_interpolation as u8);
        }

        put(&mut size, self.open_heredocs.len() as u8);
        for heredoc in &self.open_heredocs {
            let wsize = heredoc.word.len() as u32;
            // The C bound is `size + 2 + word.size >= 1024` while it writes `4 + word.size` bytes,
            // which can write one byte past the 1024-byte buffer. Check the real size instead: the
            // result is the same (0, nothing serialized) wherever the C stays in bounds.
            if size.wrapping_add(4).wrapping_add(wsize) > SERIALIZATION_BUFFER_SIZE {
                return 0;
            }
            put(&mut size, heredoc.end_word_indentation_allowed as u8);
            put(&mut size, heredoc.allows_interpolation as u8);
            put(&mut size, heredoc.started as u8);
            put(&mut size, wsize as u8);
            core::ptr::copy_nonoverlapping(
                heredoc.word.as_ptr() as *const u8,
                buffer.add(size as usize) as *mut u8,
                wsize as usize,
            );
            size += wsize;
        }

        size
    }

    unsafe fn deserialize(&mut self, buffer: *const c_char, length: u32) {
        let mut size: usize = 0;
        self.has_leading_whitespace = false;
        self.reset();

        if length == 0 {
            return;
        }

        let mut next = || {
            let b = *buffer.add(size);
            size += 1;
            b
        };

        let literal_depth = next() as u8;
        for _ in 0..literal_depth {
            let mut literal = Literal::default();
            literal.type_ = next() as i32;
            literal.open_delimiter = (next() as u8) as i32;
            literal.close_delimiter = (next() as u8) as i32;
            literal.nesting_depth = (next() as u8) as i32;
            literal.allows_interpolation = next() != 0;
            self.literal_stack.push(literal);
        }

        let open_heredoc_count = next() as u8;
        for _ in 0..open_heredoc_count {
            let mut heredoc = Heredoc::default();
            heredoc.end_word_indentation_allowed = next() != 0;
            heredoc.allows_interpolation = next() != 0;
            heredoc.started = next() != 0;

            let word_length = next() as u8 as usize;
            let mut word = Vec::with_capacity(word_length);
            for _ in 0..word_length {
                word.push(next() as i8);
            }
            heredoc.word = word;
            self.open_heredocs.push(heredoc);
        }

        // The C ends with `assert(size == length)`, which serialize always satisfies; it is not
        // reproduced (an abort on corrupt input).
    }

    unsafe fn scan_heredoc_content(&mut self, lexer: *mut TSLexer) -> bool {
        let mut position_in_word: usize = 0;
        let mut look_for_heredoc_end = true;
        let mut has_content = false;

        loop {
            if position_in_word == self.open_heredocs[0].word.len() {
                if !has_content {
                    mark_end(lexer);
                }
                while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
                    advance(lexer);
                }
                if (*lexer).lookahead == '\n' as i32 || (*lexer).lookahead == '\r' as i32 {
                    if has_content {
                        (*lexer).result_symbol = HEREDOC_CONTENT as u16;
                    } else {
                        self.open_heredocs.remove(0);
                        (*lexer).result_symbol = HEREDOC_BODY_END as u16;
                    }
                    return true;
                }
                has_content = true;
                position_in_word = 0;
            }

            if eof(lexer) {
                mark_end(lexer);
                if has_content {
                    (*lexer).result_symbol = HEREDOC_CONTENT as u16;
                } else {
                    self.open_heredocs.remove(0);
                    (*lexer).result_symbol = HEREDOC_BODY_END as u16;
                }
                return true;
            }

            let heredoc = &self.open_heredocs[0];
            if (*lexer).lookahead == heredoc.word[position_in_word] as i32 && look_for_heredoc_end {
                advance(lexer);
                position_in_word += 1;
            } else {
                position_in_word = 0;
                look_for_heredoc_end = false;

                if heredoc.allows_interpolation && (*lexer).lookahead == '\\' as i32 {
                    if has_content {
                        (*lexer).result_symbol = HEREDOC_CONTENT as u16;
                        return true;
                    }
                    return false;
                }

                if heredoc.allows_interpolation && (*lexer).lookahead == '#' as i32 {
                    mark_end(lexer);
                    advance(lexer);
                    if (*lexer).lookahead == '{' as i32 {
                        if has_content {
                            (*lexer).result_symbol = HEREDOC_CONTENT as u16;
                            return true;
                        }
                        return false;
                    }
                    if scan_short_interpolation(lexer, has_content, HEREDOC_CONTENT) {
                        return true;
                    }
                } else if (*lexer).lookahead == '\r' as i32 || (*lexer).lookahead == '\n' as i32 {
                    if (*lexer).lookahead == '\r' as i32 {
                        advance(lexer);
                        if (*lexer).lookahead == '\n' as i32 {
                            advance(lexer);
                        }
                    } else {
                        advance(lexer);
                    }
                    has_content = true;
                    look_for_heredoc_end = true;
                    while (*lexer).lookahead == ' ' as i32 || (*lexer).lookahead == '\t' as i32 {
                        advance(lexer);
                        if !heredoc.end_word_indentation_allowed {
                            look_for_heredoc_end = false;
                        }
                    }
                    mark_end(lexer);
                } else {
                    has_content = true;
                    advance(lexer);
                    mark_end(lexer);
                }
            }
        }
    }

    unsafe fn scan_literal_content(&mut self, lexer: *mut TSLexer) -> bool {
        let last = self.literal_stack.len() - 1;
        let mut has_content = false;
        let ltype = self.literal_stack[last].type_;
        let stop_on_space =
            ltype == SYMBOL_ARRAY_START as i32 || ltype == STRING_ARRAY_START as i32;

        loop {
            if stop_on_space && iswspace((*lexer).lookahead) {
                if has_content {
                    mark_end(lexer);
                    (*lexer).result_symbol = STRING_CONTENT as u16;
                    return true;
                }
                return false;
            }
            let literal = &mut self.literal_stack[last];
            if (*lexer).lookahead == literal.close_delimiter {
                mark_end(lexer);
                if literal.nesting_depth == 1 {
                    if has_content {
                        (*lexer).result_symbol = STRING_CONTENT as u16;
                    } else {
                        advance(lexer);
                        if literal.type_ == REGEX_START as i32 {
                            while iswlower((*lexer).lookahead) {
                                advance(lexer);
                            }
                        }
                        self.literal_stack.pop();
                        (*lexer).result_symbol = STRING_END as u16;
                        mark_end(lexer);
                    }
                    return true;
                }
                literal.nesting_depth = literal.nesting_depth.wrapping_sub(1);
                advance(lexer);
            } else if (*lexer).lookahead == literal.open_delimiter {
                literal.nesting_depth = literal.nesting_depth.wrapping_add(1);
                advance(lexer);
            } else if literal.allows_interpolation && (*lexer).lookahead == '#' as i32 {
                mark_end(lexer);
                advance(lexer);
                if (*lexer).lookahead == '{' as i32 {
                    if has_content {
                        (*lexer).result_symbol = STRING_CONTENT as u16;
                        return true;
                    }
                    return false;
                }
                if scan_short_interpolation(lexer, has_content, STRING_CONTENT) {
                    return true;
                }
            } else if (*lexer).lookahead == '\\' as i32 {
                if literal.allows_interpolation {
                    if has_content {
                        mark_end(lexer);
                        (*lexer).result_symbol = STRING_CONTENT as u16;
                        return true;
                    }
                    return false;
                }
                advance(lexer);
                advance(lexer);
            } else if eof(lexer) {
                advance(lexer);
                mark_end(lexer);
                return false;
            } else {
                advance(lexer);
            }

            has_content = true;
        }
    }

    unsafe fn scan(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let valid = |i: usize| *valid_symbols.add(i);
        self.has_leading_whitespace = false;

        // Contents of literals, which match any character except for some close delimiter
        if !valid(STRING_START) {
            if (valid(STRING_CONTENT) || valid(STRING_END)) && !self.literal_stack.is_empty() {
                return self.scan_literal_content(lexer);
            }
            if (valid(HEREDOC_CONTENT) || valid(HEREDOC_BODY_END)) && !self.open_heredocs.is_empty()
            {
                return self.scan_heredoc_content(lexer);
            }
        }

        // Whitespace
        (*lexer).result_symbol = NONE as u16;
        if !scan_whitespace(self, lexer, valid_symbols) {
            return false;
        }
        if (*lexer).result_symbol != NONE as u16 {
            return true;
        }

        match (*lexer).lookahead {
            0x26 => {
                // '&'
                if valid(BLOCK_AMPERSAND) {
                    advance(lexer);
                    let la = (*lexer).lookahead;
                    if la != '&' as i32 && la != '.' as i32 && la != '=' as i32 && !iswspace(la) {
                        (*lexer).result_symbol = BLOCK_AMPERSAND as u16;
                        return true;
                    }
                    return false;
                }
            }
            0x3c => {
                // '<'
                if valid(SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE) {
                    advance(lexer);
                    if (*lexer).lookahead == '<' as i32 {
                        advance(lexer);
                        (*lexer).result_symbol = SINGLETON_CLASS_LEFT_ANGLE_LEFT_ANGLE as u16;
                        return true;
                    }
                    return false;
                }
            }
            0x2a => {
                // '*'
                if valid(SPLAT_STAR)
                    || valid(BINARY_STAR)
                    || valid(HASH_SPLAT_STAR_STAR)
                    || valid(BINARY_STAR_STAR)
                {
                    advance(lexer);
                    if (*lexer).lookahead == '=' as i32 {
                        return false;
                    }
                    if (*lexer).lookahead == '*' as i32 {
                        if valid(HASH_SPLAT_STAR_STAR) || valid(BINARY_STAR_STAR) {
                            advance(lexer);
                            if (*lexer).lookahead == '=' as i32 {
                                return false;
                            }
                            if valid(BINARY_STAR_STAR) && !self.has_leading_whitespace {
                                (*lexer).result_symbol = BINARY_STAR_STAR as u16;
                                return true;
                            }
                            if valid(HASH_SPLAT_STAR_STAR) && !iswspace((*lexer).lookahead) {
                                (*lexer).result_symbol = HASH_SPLAT_STAR_STAR as u16;
                                return true;
                            }
                            if valid(BINARY_STAR_STAR) {
                                (*lexer).result_symbol = BINARY_STAR_STAR as u16;
                                return true;
                            }
                            if valid(HASH_SPLAT_STAR_STAR) {
                                (*lexer).result_symbol = HASH_SPLAT_STAR_STAR as u16;
                                return true;
                            }
                            return false;
                        }
                        return false;
                    }
                    if valid(BINARY_STAR) && !self.has_leading_whitespace {
                        (*lexer).result_symbol = BINARY_STAR as u16;
                        return true;
                    }
                    if valid(SPLAT_STAR) && !iswspace((*lexer).lookahead) {
                        (*lexer).result_symbol = SPLAT_STAR as u16;
                        return true;
                    }
                    if valid(BINARY_STAR) {
                        (*lexer).result_symbol = BINARY_STAR as u16;
                        return true;
                    }
                    if valid(SPLAT_STAR) {
                        (*lexer).result_symbol = SPLAT_STAR as u16;
                        return true;
                    }
                    return false;
                }
            }
            0x2d => {
                // '-'
                if valid(UNARY_MINUS) || valid(UNARY_MINUS_NUM) || valid(BINARY_MINUS) {
                    advance(lexer);
                    if (*lexer).lookahead != '=' as i32 && (*lexer).lookahead != '>' as i32 {
                        if valid(UNARY_MINUS_NUM)
                            && (!valid(BINARY_STAR) || self.has_leading_whitespace)
                            && iswdigit((*lexer).lookahead)
                        {
                            (*lexer).result_symbol = UNARY_MINUS_NUM as u16;
                            return true;
                        }
                        if valid(UNARY_MINUS)
                            && self.has_leading_whitespace
                            && !iswspace((*lexer).lookahead)
                        {
                            (*lexer).result_symbol = UNARY_MINUS as u16;
                        } else if valid(BINARY_MINUS) {
                            (*lexer).result_symbol = BINARY_MINUS as u16;
                        } else {
                            (*lexer).result_symbol = UNARY_MINUS as u16;
                        }
                        return true;
                    }
                    return false;
                }
            }
            0x3a => {
                // ':'
                if valid(SYMBOL_START) {
                    let mut literal = Literal::default();
                    literal.type_ = SYMBOL_START as i32;
                    literal.nesting_depth = 1;
                    advance(lexer);

                    match (*lexer).lookahead {
                        0x22 => {
                            advance(lexer);
                            literal.open_delimiter = '"' as i32;
                            literal.close_delimiter = '"' as i32;
                            literal.allows_interpolation = true;
                            self.literal_stack.push(literal);
                            (*lexer).result_symbol = SYMBOL_START as u16;
                            return true;
                        }
                        0x27 => {
                            advance(lexer);
                            literal.open_delimiter = '\'' as i32;
                            literal.close_delimiter = '\'' as i32;
                            literal.allows_interpolation = false;
                            self.literal_stack.push(literal);
                            (*lexer).result_symbol = SYMBOL_START as u16;
                            return true;
                        }
                        _ => {
                            if scan_symbol_identifier(lexer) {
                                (*lexer).result_symbol = SIMPLE_SYMBOL as u16;
                                return true;
                            }
                        }
                    }

                    return false;
                }
            }
            0x5b => {
                // '[': an element reference if the bracket is not preceded by whitespace, or if an
                // arbitrary expression is not valid at the current position.
                if valid(ELEMENT_REFERENCE_BRACKET)
                    && (!self.has_leading_whitespace || !valid(STRING_START))
                {
                    advance(lexer);
                    (*lexer).result_symbol = ELEMENT_REFERENCE_BRACKET as u16;
                    return true;
                }
            }
            _ => {}
        }

        // Identifier suffixes
        if ((valid(HASH_KEY_SYMBOL) || valid(IDENTIFIER_SUFFIX))
            && (iswalpha((*lexer).lookahead) || (*lexer).lookahead == '_' as i32))
            || (valid(CONSTANT_SUFFIX) && iswupper((*lexer).lookahead))
        {
            let valid_identifier_symbol = if iswupper((*lexer).lookahead) {
                CONSTANT_SUFFIX
            } else {
                IDENTIFIER_SUFFIX
            };
            while iswalnum((*lexer).lookahead) || (*lexer).lookahead == '_' as i32 {
                advance(lexer);
            }

            if valid(HASH_KEY_SYMBOL) && (*lexer).lookahead == ':' as i32 {
                mark_end(lexer);
                advance(lexer);
                if (*lexer).lookahead != ':' as i32 {
                    (*lexer).result_symbol = HASH_KEY_SYMBOL as u16;
                    return true;
                }
            } else if valid(valid_identifier_symbol) && (*lexer).lookahead == '!' as i32 {
                advance(lexer);
                if (*lexer).lookahead != '=' as i32 {
                    (*lexer).result_symbol = valid_identifier_symbol as u16;
                    return true;
                }
            }

            return false;
        }

        // Open delimiters for literals
        if valid(STRING_START) {
            let mut literal = Literal::default();
            literal.nesting_depth = 1;

            if (*lexer).lookahead == '<' as i32 {
                advance(lexer);
                if (*lexer).lookahead != '<' as i32 {
                    return false;
                }
                advance(lexer);

                let mut heredoc = Heredoc::default();
                if (*lexer).lookahead == '-' as i32 || (*lexer).lookahead == '~' as i32 {
                    advance(lexer);
                    heredoc.end_word_indentation_allowed = true;
                }

                scan_heredoc_word(lexer, &mut heredoc);
                if heredoc.word.is_empty() {
                    return false;
                }
                self.open_heredocs.push(heredoc);
                (*lexer).result_symbol = HEREDOC_START as u16;
                return true;
            }

            if scan_open_delimiter(self, lexer, &mut literal, valid_symbols) {
                self.literal_stack.push(literal);
                (*lexer).result_symbol = literal.type_ as u16;
                return true;
            }
            return false;
        }

        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ruby_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ruby_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ruby_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    (&*(payload as *const Scanner)).serialize(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ruby_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    (&mut *(payload as *mut Scanner)).deserialize(buffer, length)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_ruby_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    (&mut *(payload as *mut Scanner)).scan(lexer, valid_symbols)
}
