//! Rust port of tree-sitter-bash's external scanner (src/scanner.c, tree-sitter-bash 0.25.1).
//!
//! Locale: the C calls `iswspace`, `iswalpha`, `iswalnum`, `iswdigit` and `isdigit`. They are ported
//! as the C locale (ASCII only), which is what a program that never calls `setlocale` gets:
//! `is_wspace` (9..=13 and 32), `is_walpha`, `is_walnum`, `is_wdigit` (also used for `isdigit`).
//!
//! Quirks kept on purpose:
//! - `deserialize` never shrinks the heredoc array (it only reuses or appends entries), so stale
//!   heredocs past the restored count survive, exactly as in the C.
//! - the heredoc delimiter is a C `char` array (signed on the platforms the grammar is checked
//!   on): bytes >= 0x80 never compare equal to a (non-negative) lookahead.
//! - `uint32_t` depth counters decrement through zero and wrap, as in the C.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const HEREDOC_START: usize = 0;
const SIMPLE_HEREDOC_BODY: usize = 1;
const HEREDOC_BODY_BEGINNING: usize = 2;
const HEREDOC_CONTENT: usize = 3;
const HEREDOC_END: usize = 4;
const FILE_DESCRIPTOR: usize = 5;
const EMPTY_VALUE: usize = 6;
const CONCAT: usize = 7;
const VARIABLE_NAME: usize = 8;
const TEST_OPERATOR: usize = 9;
const REGEX: usize = 10;
const REGEX_NO_SLASH: usize = 11;
const REGEX_NO_SPACE: usize = 12;
const EXPANSION_WORD: usize = 13;
const EXTGLOB_PATTERN: usize = 14;
const BARE_DOLLAR: usize = 15;
const BRACE_START: usize = 16;
const IMMEDIATE_DOUBLE_HASH: usize = 17;
const EXTERNAL_EXPANSION_SYM_HASH: usize = 18;
const EXTERNAL_EXPANSION_SYM_BANG: usize = 19;
const EXTERNAL_EXPANSION_SYM_EQUAL: usize = 20;
const CLOSING_BRACE: usize = 21;
const CLOSING_BRACKET: usize = 22;
const HEREDOC_ARROW: usize = 23;
const HEREDOC_ARROW_DASH: usize = 24;
const NEWLINE: usize = 25;
const OPENING_PAREN: usize = 26;
#[allow(dead_code)]
const ESAC: usize = 27;
const ERROR_RECOVERY: usize = 28;

const SERIALIZATION_BUFFER_SIZE: u32 = 1024;

/// `typedef Array(char) String`; `char` is signed.
type CString = Vec<i8>;

#[derive(Default)]
struct Heredoc {
    is_raw: bool,
    started: bool,
    allows_indent: bool,
    delimiter: CString,
    current_leading_word: CString,
}

#[derive(Default)]
struct Scanner {
    last_glob_paren_depth: u8,
    ext_was_in_double_quote: bool,
    ext_saw_outside_quote: bool,
    heredocs: Vec<Heredoc>,
}

// ---- locale-free character classes (C locale) ----

fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}
fn is_wdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}
fn is_walpha(c: i32) -> bool {
    (b'a' as i32..=b'z' as i32).contains(&c) || (b'A' as i32..=b'Z' as i32).contains(&c)
}
fn is_walnum(c: i32) -> bool {
    is_wdigit(c) || is_walpha(c)
}

// ---- lexer helpers ----

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}
unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}
unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}
unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}
unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}
unsafe fn set_sym(lexer: *mut TSLexer, sym: usize) {
    (*lexer).result_symbol = sym as u16;
}

#[inline]
unsafe fn v(valid: *const bool, i: usize) -> bool {
    *valid.add(i)
}
unsafe fn in_error_recovery(valid: *const bool) -> bool {
    v(valid, ERROR_RECOVERY)
}

fn reset_heredoc(h: &mut Heredoc) {
    h.is_raw = false;
    h.started = false;
    h.allows_indent = false;
    h.delimiter.clear();
}

impl Scanner {
    fn reset(&mut self) {
        for h in self.heredocs.iter_mut() {
            reset_heredoc(h);
        }
    }

    unsafe fn serialize(&self, buffer: *mut c_char) -> u32 {
        let mut size: u32 = 0;
        let put = |size: &mut u32, b: u8| {
            *buffer.add(*size as usize) = b as c_char;
            *size += 1;
        };
        put(&mut size, self.last_glob_paren_depth);
        put(&mut size, self.ext_was_in_double_quote as u8);
        put(&mut size, self.ext_saw_outside_quote as u8);
        put(&mut size, self.heredocs.len() as u8);
        for h in self.heredocs.iter() {
            let dsize = h.delimiter.len() as u32;
            if size + 3 + 4 + dsize >= SERIALIZATION_BUFFER_SIZE {
                return 0;
            }
            put(&mut size, h.is_raw as u8);
            put(&mut size, h.started as u8);
            put(&mut size, h.allows_indent as u8);
            for b in dsize.to_ne_bytes() {
                put(&mut size, b);
            }
            for &b in h.delimiter.iter() {
                put(&mut size, b as u8);
            }
        }
        size
    }

    unsafe fn deserialize(&mut self, buffer: *const c_char, length: u32) {
        if length == 0 {
            self.reset();
            return;
        }
        let mut size: usize = 0;
        let rd = |size: &mut usize| -> c_char {
            let b = *buffer.add(*size);
            *size += 1;
            b
        };
        self.last_glob_paren_depth = rd(&mut size) as u8;
        self.ext_was_in_double_quote = rd(&mut size) != 0;
        self.ext_saw_outside_quote = rd(&mut size) != 0;
        let heredoc_count = rd(&mut size) as u8 as u32;
        for i in 0..heredoc_count as usize {
            if i >= self.heredocs.len() {
                self.heredocs.push(Heredoc::default());
            }
            let h = &mut self.heredocs[i];
            h.is_raw = rd(&mut size) != 0;
            h.started = rd(&mut size) != 0;
            h.allows_indent = rd(&mut size) != 0;
            let mut dbytes = [0u8; 4];
            for b in dbytes.iter_mut() {
                *b = rd(&mut size) as u8;
            }
            let dsize = u32::from_ne_bytes(dbytes) as usize;
            h.delimiter.clear();
            for _ in 0..dsize {
                h.delimiter.push(rd(&mut size) as i8);
            }
        }
        assert!(size == length as usize);
    }
}

/// `strcmp(a, b) == 0` over C strings stored in vectors (the end of the vector counts as NUL).
fn c_streq(a: &[i8], b: &[i8]) -> bool {
    let mut i = 0;
    loop {
        let x = a.get(i).copied().unwrap_or(0) as u8;
        let y = b.get(i).copied().unwrap_or(0) as u8;
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}

/// Consume a "word" in POSIX parlance, and returns it unquoted.
unsafe fn advance_word(lexer: *mut TSLexer, unquoted_word: &mut CString) -> bool {
    let mut empty = true;

    let mut quote: i32 = 0;
    if la(lexer) == '\'' as i32 || la(lexer) == '"' as i32 {
        quote = la(lexer);
        advance(lexer);
    }

    while la(lexer) != 0
        && !(if quote != 0 {
            la(lexer) == quote || la(lexer) == '\r' as i32 || la(lexer) == '\n' as i32
        } else {
            is_wspace(la(lexer))
        })
    {
        if la(lexer) == '\\' as i32 {
            advance(lexer);
            if la(lexer) == 0 {
                return false;
            }
        }
        empty = false;
        unquoted_word.push(la(lexer) as i8);
        advance(lexer);
    }
    unquoted_word.push(0);

    if quote != 0 && la(lexer) == quote {
        advance(lexer);
    }

    !empty
}

unsafe fn scan_bare_dollar(lexer: *mut TSLexer) -> bool {
    while is_wspace(la(lexer)) && la(lexer) != '\n' as i32 && !eof(lexer) {
        skip(lexer);
    }

    if la(lexer) == '$' as i32 {
        advance(lexer);
        set_sym(lexer, BARE_DOLLAR);
        mark_end(lexer);
        return is_wspace(la(lexer)) || eof(lexer) || la(lexer) == '"' as i32;
    }

    false
}

unsafe fn scan_heredoc_start(heredoc: &mut Heredoc, lexer: *mut TSLexer) -> bool {
    while is_wspace(la(lexer)) {
        skip(lexer);
    }

    set_sym(lexer, HEREDOC_START);
    heredoc.is_raw =
        la(lexer) == '\'' as i32 || la(lexer) == '"' as i32 || la(lexer) == '\\' as i32;

    let found_delimiter = advance_word(lexer, &mut heredoc.delimiter);
    if !found_delimiter {
        heredoc.delimiter.clear();
        return false;
    }
    found_delimiter
}

unsafe fn scan_heredoc_end_identifier(heredoc: &mut Heredoc, lexer: *mut TSLexer) -> bool {
    heredoc.current_leading_word.clear();
    // Scan the first 'n' characters on this line, to see if they match the heredoc delimiter.
    let mut size: usize = 0;
    if !heredoc.delimiter.is_empty() {
        while la(lexer) != 0
            && la(lexer) != '\n' as i32
            && heredoc.delimiter[size] as i32 == la(lexer)
            && heredoc.current_leading_word.len() < heredoc.delimiter.len()
        {
            heredoc.current_leading_word.push(la(lexer) as i8);
            advance(lexer);
            size += 1;
        }
    }
    heredoc.current_leading_word.push(0);
    if heredoc.delimiter.is_empty() {
        false
    } else {
        c_streq(&heredoc.current_leading_word, &heredoc.delimiter)
    }
}

unsafe fn scan_heredoc_content(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    middle_type: usize,
    end_type: usize,
) -> bool {
    let mut did_advance = false;
    let idx = scanner.heredocs.len() - 1;

    loop {
        let c = la(lexer);
        if c == 0 {
            if eof(lexer) && did_advance {
                reset_heredoc(&mut scanner.heredocs[idx]);
                set_sym(lexer, end_type);
                return true;
            }
            return false;
        } else if c == '\\' as i32 {
            did_advance = true;
            advance(lexer);
            advance(lexer);
        } else if c == '$' as i32 {
            if scanner.heredocs[idx].is_raw {
                did_advance = true;
                advance(lexer);
                continue;
            }
            if did_advance {
                mark_end(lexer);
                set_sym(lexer, middle_type);
                scanner.heredocs[idx].started = true;
                advance(lexer);
                if is_walpha(la(lexer)) || la(lexer) == '{' as i32 || la(lexer) == '(' as i32 {
                    return true;
                }
                continue;
            }
            if middle_type == HEREDOC_BODY_BEGINNING && ((*lexer).get_column)(lexer) == 0 {
                set_sym(lexer, middle_type);
                scanner.heredocs[idx].started = true;
                return true;
            }
            return false;
        } else if c == '\n' as i32 {
            if !did_advance {
                skip(lexer);
            } else {
                advance(lexer);
            }
            did_advance = true;
            if scanner.heredocs[idx].allows_indent {
                while is_wspace(la(lexer)) {
                    advance(lexer);
                }
            }
            set_sym(
                lexer,
                if scanner.heredocs[idx].started {
                    middle_type
                } else {
                    end_type
                },
            );
            mark_end(lexer);
            if scan_heredoc_end_identifier(&mut scanner.heredocs[idx], lexer) {
                if (*lexer).result_symbol as usize == HEREDOC_END {
                    scanner.heredocs.pop();
                }
                return true;
            }
        } else {
            if ((*lexer).get_column)(lexer) == 0 {
                // an alternative is to check the starting column of the heredoc body and track
                // that statefully
                while is_wspace(la(lexer)) {
                    if did_advance {
                        advance(lexer);
                    } else {
                        skip(lexer);
                    }
                }
                if end_type != SIMPLE_HEREDOC_BODY {
                    set_sym(lexer, middle_type);
                    if scan_heredoc_end_identifier(&mut scanner.heredocs[idx], lexer) {
                        return true;
                    }
                }
                if end_type == SIMPLE_HEREDOC_BODY {
                    set_sym(lexer, end_type);
                    mark_end(lexer);
                    if scan_heredoc_end_identifier(&mut scanner.heredocs[idx], lexer) {
                        return true;
                    }
                }
            }
            did_advance = true;
            advance(lexer);
        }
    }
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid: *const bool) -> bool {
    if v(valid, CONCAT) && !in_error_recovery(valid) {
        if !(la(lexer) == 0
            || is_wspace(la(lexer))
            || la(lexer) == '>' as i32
            || la(lexer) == '<' as i32
            || la(lexer) == ')' as i32
            || la(lexer) == '(' as i32
            || la(lexer) == ';' as i32
            || la(lexer) == '&' as i32
            || la(lexer) == '|' as i32
            || (la(lexer) == '}' as i32 && v(valid, CLOSING_BRACE))
            || (la(lexer) == ']' as i32 && v(valid, CLOSING_BRACKET)))
        {
            set_sym(lexer, CONCAT);
            // So for a`b`, we want to return a concat. We check if the 2nd backtick has
            // whitespace after it, and if it does we return concat.
            if la(lexer) == '`' as i32 {
                mark_end(lexer);
                advance(lexer);
                while la(lexer) != '`' as i32 && !eof(lexer) {
                    advance(lexer);
                }
                if eof(lexer) {
                    return false;
                }
                if la(lexer) == '`' as i32 {
                    advance(lexer);
                }
                return is_wspace(la(lexer)) || eof(lexer);
            }
            // strings w/ expansions that contains escaped quotes or backslashes need this to
            // return a concat
            if la(lexer) == '\\' as i32 {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '"' as i32 || la(lexer) == '\'' as i32 || la(lexer) == '\\' as i32 {
                    return true;
                }
                if eof(lexer) {
                    return false;
                }
            } else {
                return true;
            }
        }
        if is_wspace(la(lexer)) && v(valid, CLOSING_BRACE) && !v(valid, EXPANSION_WORD) {
            set_sym(lexer, CONCAT);
            return true;
        }
    }

    if v(valid, IMMEDIATE_DOUBLE_HASH) && !in_error_recovery(valid) {
        // advance two # and ensure not } after
        if la(lexer) == '#' as i32 {
            mark_end(lexer);
            advance(lexer);
            if la(lexer) == '#' as i32 {
                advance(lexer);
                if la(lexer) != '}' as i32 {
                    set_sym(lexer, IMMEDIATE_DOUBLE_HASH);
                    mark_end(lexer);
                    return true;
                }
            }
        }
    }

    if v(valid, EXTERNAL_EXPANSION_SYM_HASH) && !in_error_recovery(valid) {
        if la(lexer) == '#' as i32 || la(lexer) == '=' as i32 || la(lexer) == '!' as i32 {
            set_sym(
                lexer,
                if la(lexer) == '#' as i32 {
                    EXTERNAL_EXPANSION_SYM_HASH
                } else if la(lexer) == '!' as i32 {
                    EXTERNAL_EXPANSION_SYM_BANG
                } else {
                    EXTERNAL_EXPANSION_SYM_EQUAL
                },
            );
            advance(lexer);
            mark_end(lexer);
            while la(lexer) == '#' as i32 || la(lexer) == '=' as i32 || la(lexer) == '!' as i32 {
                advance(lexer);
            }
            while is_wspace(la(lexer)) {
                skip(lexer);
            }
            if la(lexer) == '}' as i32 {
                return true;
            }
            return false;
        }
    }

    if v(valid, EMPTY_VALUE) {
        if is_wspace(la(lexer)) || eof(lexer) || la(lexer) == ';' as i32 || la(lexer) == '&' as i32
        {
            set_sym(lexer, EMPTY_VALUE);
            return true;
        }
    }

    if (v(valid, HEREDOC_BODY_BEGINNING) || v(valid, SIMPLE_HEREDOC_BODY))
        && !scanner.heredocs.is_empty()
        && !scanner.heredocs.last().unwrap().started
        && !in_error_recovery(valid)
    {
        return scan_heredoc_content(scanner, lexer, HEREDOC_BODY_BEGINNING, SIMPLE_HEREDOC_BODY);
    }

    if v(valid, HEREDOC_END) && !scanner.heredocs.is_empty() {
        let heredoc = scanner.heredocs.last_mut().unwrap();
        if scan_heredoc_end_identifier(heredoc, lexer) {
            scanner.heredocs.pop();
            set_sym(lexer, HEREDOC_END);
            return true;
        }
    }

    if v(valid, HEREDOC_CONTENT)
        && !scanner.heredocs.is_empty()
        && scanner.heredocs.last().unwrap().started
        && !in_error_recovery(valid)
    {
        return scan_heredoc_content(scanner, lexer, HEREDOC_CONTENT, HEREDOC_END);
    }

    if v(valid, HEREDOC_START) && !in_error_recovery(valid) && !scanner.heredocs.is_empty() {
        return scan_heredoc_start(scanner.heredocs.last_mut().unwrap(), lexer);
    }

    if v(valid, TEST_OPERATOR) && !v(valid, EXPANSION_WORD) {
        while is_wspace(la(lexer)) && la(lexer) != '\n' as i32 {
            skip(lexer);
        }

        if la(lexer) == '\\' as i32 {
            if v(valid, EXTGLOB_PATTERN) {
                return extglob_pattern(scanner, lexer, valid);
            }
            if v(valid, REGEX_NO_SPACE) {
                return regex(scanner, lexer, valid);
            }
            skip(lexer);

            if eof(lexer) {
                return false;
            }

            if la(lexer) == '\r' as i32 {
                skip(lexer);
                if la(lexer) == '\n' as i32 {
                    skip(lexer);
                }
            } else if la(lexer) == '\n' as i32 {
                skip(lexer);
            } else {
                return false;
            }

            while is_wspace(la(lexer)) {
                skip(lexer);
            }
        }

        if la(lexer) == '\n' as i32 && !v(valid, NEWLINE) {
            skip(lexer);

            while is_wspace(la(lexer)) {
                skip(lexer);
            }
        }

        if la(lexer) == '-' as i32 {
            advance(lexer);

            let mut advanced_once = false;
            while is_walpha(la(lexer)) {
                advanced_once = true;
                advance(lexer);
            }

            if is_wspace(la(lexer)) && advanced_once {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '}' as i32 && v(valid, CLOSING_BRACE) {
                    if v(valid, EXPANSION_WORD) {
                        mark_end(lexer);
                        set_sym(lexer, EXPANSION_WORD);
                        return true;
                    }
                    return false;
                }
                set_sym(lexer, TEST_OPERATOR);
                return true;
            }
            if is_wspace(la(lexer)) && v(valid, EXTGLOB_PATTERN) {
                set_sym(lexer, EXTGLOB_PATTERN);
                return true;
            }
        }

        if v(valid, BARE_DOLLAR) && !in_error_recovery(valid) && scan_bare_dollar(lexer) {
            return true;
        }
    }

    if (v(valid, VARIABLE_NAME) || v(valid, FILE_DESCRIPTOR) || v(valid, HEREDOC_ARROW))
        && !v(valid, REGEX_NO_SLASH)
        && !in_error_recovery(valid)
    {
        loop {
            if (la(lexer) == ' ' as i32
                || la(lexer) == '\t' as i32
                || la(lexer) == '\r' as i32
                || (la(lexer) == '\n' as i32 && !v(valid, NEWLINE)))
                && !v(valid, EXPANSION_WORD)
            {
                skip(lexer);
            } else if la(lexer) == '\\' as i32 {
                skip(lexer);

                if eof(lexer) {
                    mark_end(lexer);
                    set_sym(lexer, VARIABLE_NAME);
                    return true;
                }

                if la(lexer) == '\r' as i32 {
                    skip(lexer);
                }
                if la(lexer) == '\n' as i32 {
                    skip(lexer);
                } else {
                    if la(lexer) == '\\' as i32 && v(valid, EXPANSION_WORD) {
                        return expansion_word(scanner, lexer, valid);
                    }
                    return false;
                }
            } else {
                break;
            }
        }

        // no '*', '@', '?', '-', '$', '0', '_'
        if !v(valid, EXPANSION_WORD)
            && (la(lexer) == '*' as i32
                || la(lexer) == '@' as i32
                || la(lexer) == '?' as i32
                || la(lexer) == '-' as i32
                || la(lexer) == '0' as i32
                || la(lexer) == '_' as i32)
        {
            mark_end(lexer);
            advance(lexer);
            if la(lexer) == '=' as i32
                || la(lexer) == '[' as i32
                || la(lexer) == ':' as i32
                || la(lexer) == '-' as i32
                || la(lexer) == '%' as i32
                || la(lexer) == '#' as i32
                || la(lexer) == '/' as i32
            {
                return false;
            }
            if v(valid, EXTGLOB_PATTERN) && is_wspace(la(lexer)) {
                mark_end(lexer);
                set_sym(lexer, EXTGLOB_PATTERN);
                return true;
            }
        }

        if v(valid, HEREDOC_ARROW) && la(lexer) == '<' as i32 {
            advance(lexer);
            if la(lexer) == '<' as i32 {
                advance(lexer);
                if la(lexer) == '-' as i32 {
                    advance(lexer);
                    let mut heredoc = Heredoc::default();
                    heredoc.allows_indent = true;
                    scanner.heredocs.push(heredoc);
                    set_sym(lexer, HEREDOC_ARROW_DASH);
                } else if la(lexer) == '<' as i32 || la(lexer) == '=' as i32 {
                    return false;
                } else {
                    scanner.heredocs.push(Heredoc::default());
                    set_sym(lexer, HEREDOC_ARROW);
                }
                return true;
            }
            return false;
        }

        let mut is_number = true;
        if is_wdigit(la(lexer)) {
            advance(lexer);
        } else if is_walpha(la(lexer)) || la(lexer) == '_' as i32 {
            is_number = false;
            advance(lexer);
        } else {
            if la(lexer) == '{' as i32 {
                return brace_start(scanner, lexer, valid);
            }
            if v(valid, EXPANSION_WORD) {
                return expansion_word(scanner, lexer, valid);
            }
            if v(valid, EXTGLOB_PATTERN) {
                return extglob_pattern(scanner, lexer, valid);
            }
            return false;
        }

        loop {
            if is_wdigit(la(lexer)) {
                advance(lexer);
            } else if is_walpha(la(lexer)) || la(lexer) == '_' as i32 {
                is_number = false;
                advance(lexer);
            } else {
                break;
            }
        }

        if is_number
            && v(valid, FILE_DESCRIPTOR)
            && (la(lexer) == '>' as i32 || la(lexer) == '<' as i32)
        {
            set_sym(lexer, FILE_DESCRIPTOR);
            return true;
        }

        if v(valid, VARIABLE_NAME) {
            if la(lexer) == '+' as i32 {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '=' as i32 || la(lexer) == ':' as i32 || v(valid, CLOSING_BRACE) {
                    set_sym(lexer, VARIABLE_NAME);
                    return true;
                }
                return false;
            }
            if la(lexer) == '/' as i32 {
                return false;
            }
            if la(lexer) == '=' as i32
                || la(lexer) == '[' as i32
                || (la(lexer) == ':' as i32 && !v(valid, CLOSING_BRACE) && !v(valid, OPENING_PAREN))
                || la(lexer) == '%' as i32
                || (la(lexer) == '#' as i32 && !is_number)
                || la(lexer) == '@' as i32
                || (la(lexer) == '-' as i32 && v(valid, CLOSING_BRACE))
            {
                mark_end(lexer);
                set_sym(lexer, VARIABLE_NAME);
                return true;
            }

            if la(lexer) == '?' as i32 {
                mark_end(lexer);
                advance(lexer);
                set_sym(lexer, VARIABLE_NAME);
                return is_walpha(la(lexer));
            }
        }

        return false;
    }

    if v(valid, BARE_DOLLAR) && !in_error_recovery(valid) && scan_bare_dollar(lexer) {
        return true;
    }

    regex(scanner, lexer, valid)
}

/// The C's `regex:` label; falls through to `extglob_pattern`.
unsafe fn regex(scanner: &mut Scanner, lexer: *mut TSLexer, valid: *const bool) -> bool {
    if (v(valid, REGEX) || v(valid, REGEX_NO_SLASH) || v(valid, REGEX_NO_SPACE))
        && !in_error_recovery(valid)
    {
        if v(valid, REGEX) || v(valid, REGEX_NO_SPACE) {
            while is_wspace(la(lexer)) {
                skip(lexer);
            }
        }

        if (la(lexer) != '"' as i32 && la(lexer) != '\'' as i32)
            || ((la(lexer) == '$' as i32 || la(lexer) == '\'' as i32) && v(valid, REGEX_NO_SLASH))
            || (la(lexer) == '\'' as i32 && v(valid, REGEX_NO_SPACE))
        {
            struct State {
                done: bool,
                advanced_once: bool,
                found_non_alnumdollarunderdash: bool,
                last_was_escape: bool,
                in_single_quote: bool,
                paren_depth: u32,
                bracket_depth: u32,
                brace_depth: u32,
            }

            if la(lexer) == '$' as i32 && v(valid, REGEX_NO_SLASH) {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '(' as i32 {
                    return false;
                }
            }

            mark_end(lexer);

            let mut state = State {
                done: false,
                advanced_once: false,
                found_non_alnumdollarunderdash: false,
                last_was_escape: false,
                in_single_quote: false,
                paren_depth: 0,
                bracket_depth: 0,
                brace_depth: 0,
            };
            while !state.done {
                if state.in_single_quote {
                    if la(lexer) == '\'' as i32 {
                        state.in_single_quote = false;
                        advance(lexer);
                        mark_end(lexer);
                    }
                }
                let c = la(lexer);
                if c == '\\' as i32 {
                    state.last_was_escape = true;
                } else if c == 0 {
                    return false;
                } else if c == '(' as i32 {
                    state.paren_depth = state.paren_depth.wrapping_add(1);
                    state.last_was_escape = false;
                } else if c == '[' as i32 {
                    state.bracket_depth = state.bracket_depth.wrapping_add(1);
                    state.last_was_escape = false;
                } else if c == '{' as i32 {
                    if !state.last_was_escape {
                        state.brace_depth = state.brace_depth.wrapping_add(1);
                    }
                    state.last_was_escape = false;
                } else if c == ')' as i32 {
                    if state.paren_depth == 0 {
                        state.done = true;
                    }
                    state.paren_depth = state.paren_depth.wrapping_sub(1);
                    state.last_was_escape = false;
                } else if c == ']' as i32 {
                    if state.bracket_depth == 0 {
                        state.done = true;
                    }
                    state.bracket_depth = state.bracket_depth.wrapping_sub(1);
                    state.last_was_escape = false;
                } else if c == '}' as i32 {
                    if state.brace_depth == 0 {
                        state.done = true;
                    }
                    state.brace_depth = state.brace_depth.wrapping_sub(1);
                    state.last_was_escape = false;
                } else if c == '\'' as i32 {
                    // Enter or exit a single-quoted string.
                    state.in_single_quote = !state.in_single_quote;
                    advance(lexer);
                    state.advanced_once = true;
                    state.last_was_escape = false;
                    continue;
                } else {
                    state.last_was_escape = false;
                }

                if !state.done {
                    if v(valid, REGEX) {
                        let was_space = !state.in_single_quote && is_wspace(la(lexer));
                        advance(lexer);
                        state.advanced_once = true;
                        if !was_space || state.paren_depth > 0 {
                            mark_end(lexer);
                        }
                    } else if v(valid, REGEX_NO_SLASH) {
                        if la(lexer) == '/' as i32 {
                            mark_end(lexer);
                            set_sym(lexer, REGEX_NO_SLASH);
                            return state.advanced_once;
                        }
                        if la(lexer) == '\\' as i32 {
                            advance(lexer);
                            state.advanced_once = true;
                            if !eof(lexer) && la(lexer) != '[' as i32 && la(lexer) != '/' as i32 {
                                advance(lexer);
                                mark_end(lexer);
                            }
                        } else {
                            let was_space = !state.in_single_quote && is_wspace(la(lexer));
                            advance(lexer);
                            state.advanced_once = true;
                            if !was_space {
                                mark_end(lexer);
                            }
                        }
                    } else if v(valid, REGEX_NO_SPACE) {
                        if la(lexer) == '\\' as i32 {
                            state.found_non_alnumdollarunderdash = true;
                            advance(lexer);
                            if !eof(lexer) {
                                advance(lexer);
                            }
                        } else if la(lexer) == '$' as i32 {
                            mark_end(lexer);
                            advance(lexer);
                            // do not parse a command substitution
                            if la(lexer) == '(' as i32 {
                                return false;
                            }
                            // end $ always means regex, e.g. 99999999$
                            if is_wspace(la(lexer)) {
                                set_sym(lexer, REGEX_NO_SPACE);
                                mark_end(lexer);
                                return true;
                            }
                        } else {
                            let was_space = !state.in_single_quote && is_wspace(la(lexer));
                            if was_space && state.paren_depth == 0 {
                                mark_end(lexer);
                                set_sym(lexer, REGEX_NO_SPACE);
                                return state.found_non_alnumdollarunderdash;
                            }
                            if !is_walnum(la(lexer))
                                && la(lexer) != '$' as i32
                                && la(lexer) != '-' as i32
                                && la(lexer) != '_' as i32
                            {
                                state.found_non_alnumdollarunderdash = true;
                            }
                            advance(lexer);
                        }
                    }
                }
            }

            set_sym(
                lexer,
                if v(valid, REGEX_NO_SLASH) {
                    REGEX_NO_SLASH
                } else if v(valid, REGEX_NO_SPACE) {
                    REGEX_NO_SPACE
                } else {
                    REGEX
                },
            );
            if v(valid, REGEX) && !state.advanced_once {
                return false;
            }
            return true;
        }
    }

    extglob_pattern(scanner, lexer, valid)
}

/// The C's `extglob_pattern:` label; falls through to `expansion_word`.
unsafe fn extglob_pattern(scanner: &mut Scanner, lexer: *mut TSLexer, valid: *const bool) -> bool {
    if v(valid, EXTGLOB_PATTERN) && !in_error_recovery(valid) {
        // first skip ws, then check for ? * + @ !
        while is_wspace(la(lexer)) {
            skip(lexer);
        }

        if la(lexer) == '?' as i32
            || la(lexer) == '*' as i32
            || la(lexer) == '+' as i32
            || la(lexer) == '@' as i32
            || la(lexer) == '!' as i32
            || la(lexer) == '-' as i32
            || la(lexer) == ')' as i32
            || la(lexer) == '\\' as i32
            || la(lexer) == '.' as i32
            || la(lexer) == '[' as i32
            || is_walpha(la(lexer))
        {
            if la(lexer) == '\\' as i32 {
                advance(lexer);
                if (is_wspace(la(lexer)) || la(lexer) == '"' as i32)
                    && la(lexer) != '\r' as i32
                    && la(lexer) != '\n' as i32
                {
                    advance(lexer);
                } else {
                    return false;
                }
            }

            if la(lexer) == ')' as i32 && scanner.last_glob_paren_depth == 0 {
                mark_end(lexer);
                advance(lexer);

                if is_wspace(la(lexer)) {
                    return false;
                }
            }

            mark_end(lexer);
            let was_non_alpha = !is_walpha(la(lexer));
            if la(lexer) != '[' as i32 {
                // no esac
                if la(lexer) == 'e' as i32 {
                    mark_end(lexer);
                    advance(lexer);
                    if la(lexer) == 's' as i32 {
                        advance(lexer);
                        if la(lexer) == 'a' as i32 {
                            advance(lexer);
                            if la(lexer) == 'c' as i32 {
                                advance(lexer);
                                if is_wspace(la(lexer)) {
                                    return false;
                                }
                            }
                        }
                    }
                } else {
                    advance(lexer);
                }
            }

            // -\w is just a word, find something else special
            if la(lexer) == '-' as i32 {
                mark_end(lexer);
                advance(lexer);
                while is_walnum(la(lexer)) {
                    advance(lexer);
                }

                if la(lexer) == ')' as i32 || la(lexer) == '\\' as i32 || la(lexer) == '.' as i32 {
                    return false;
                }
                mark_end(lexer);
            }

            // case item -) or *)
            if la(lexer) == ')' as i32 && scanner.last_glob_paren_depth == 0 {
                mark_end(lexer);
                advance(lexer);
                if is_wspace(la(lexer)) {
                    set_sym(lexer, EXTGLOB_PATTERN);
                    return was_non_alpha;
                }
            }

            if is_wspace(la(lexer)) {
                mark_end(lexer);
                set_sym(lexer, EXTGLOB_PATTERN);
                scanner.last_glob_paren_depth = 0;
                return true;
            }

            if la(lexer) == '$' as i32 {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '{' as i32 || la(lexer) == '(' as i32 {
                    set_sym(lexer, EXTGLOB_PATTERN);
                    return true;
                }
            }

            if la(lexer) == '|' as i32 {
                mark_end(lexer);
                advance(lexer);
                set_sym(lexer, EXTGLOB_PATTERN);
                return true;
            }

            if !is_walnum(la(lexer))
                && la(lexer) != '(' as i32
                && la(lexer) != '"' as i32
                && la(lexer) != '[' as i32
                && la(lexer) != '?' as i32
                && la(lexer) != '/' as i32
                && la(lexer) != '\\' as i32
                && la(lexer) != '_' as i32
                && la(lexer) != '*' as i32
            {
                return false;
            }

            struct State {
                done: bool,
                saw_non_alphadot: bool,
                paren_depth: u32,
                bracket_depth: u32,
                brace_depth: u32,
            }

            let mut state = State {
                done: false,
                saw_non_alphadot: was_non_alpha,
                paren_depth: scanner.last_glob_paren_depth as u32,
                bracket_depth: 0,
                brace_depth: 0,
            };
            while !state.done {
                let c = la(lexer);
                if c == 0 {
                    return false;
                } else if c == '(' as i32 {
                    state.paren_depth = state.paren_depth.wrapping_add(1);
                } else if c == '[' as i32 {
                    state.bracket_depth = state.bracket_depth.wrapping_add(1);
                } else if c == '{' as i32 {
                    state.brace_depth = state.brace_depth.wrapping_add(1);
                } else if c == ')' as i32 {
                    if state.paren_depth == 0 {
                        state.done = true;
                    }
                    state.paren_depth = state.paren_depth.wrapping_sub(1);
                } else if c == ']' as i32 {
                    if state.bracket_depth == 0 {
                        state.done = true;
                    }
                    state.bracket_depth = state.bracket_depth.wrapping_sub(1);
                } else if c == '}' as i32 {
                    if state.brace_depth == 0 {
                        state.done = true;
                    }
                    state.brace_depth = state.brace_depth.wrapping_sub(1);
                }

                if la(lexer) == '|' as i32 {
                    mark_end(lexer);
                    advance(lexer);
                    if state.paren_depth == 0 && state.bracket_depth == 0 && state.brace_depth == 0
                    {
                        set_sym(lexer, EXTGLOB_PATTERN);
                        return true;
                    }
                }

                if !state.done {
                    let was_space = is_wspace(la(lexer));
                    if la(lexer) == '$' as i32 {
                        mark_end(lexer);
                        if !is_walpha(la(lexer))
                            && la(lexer) != '.' as i32
                            && la(lexer) != '\\' as i32
                        {
                            state.saw_non_alphadot = true;
                        }
                        advance(lexer);
                        if la(lexer) == '(' as i32 || la(lexer) == '{' as i32 {
                            set_sym(lexer, EXTGLOB_PATTERN);
                            scanner.last_glob_paren_depth = state.paren_depth as u8;
                            return state.saw_non_alphadot;
                        }
                    }
                    if was_space {
                        mark_end(lexer);
                        set_sym(lexer, EXTGLOB_PATTERN);
                        scanner.last_glob_paren_depth = 0;
                        return state.saw_non_alphadot;
                    }
                    if la(lexer) == '"' as i32 {
                        mark_end(lexer);
                        set_sym(lexer, EXTGLOB_PATTERN);
                        scanner.last_glob_paren_depth = 0;
                        return state.saw_non_alphadot;
                    }
                    if la(lexer) == '\\' as i32 {
                        if !is_walpha(la(lexer))
                            && la(lexer) != '.' as i32
                            && la(lexer) != '\\' as i32
                        {
                            state.saw_non_alphadot = true;
                        }
                        advance(lexer);
                        if is_wspace(la(lexer)) || la(lexer) == '"' as i32 {
                            advance(lexer);
                        }
                    } else {
                        if !is_walpha(la(lexer))
                            && la(lexer) != '.' as i32
                            && la(lexer) != '\\' as i32
                        {
                            state.saw_non_alphadot = true;
                        }
                        advance(lexer);
                    }
                    if !was_space {
                        mark_end(lexer);
                    }
                }
            }

            set_sym(lexer, EXTGLOB_PATTERN);
            scanner.last_glob_paren_depth = 0;
            return state.saw_non_alphadot;
        }
        scanner.last_glob_paren_depth = 0;

        return false;
    }

    expansion_word(scanner, lexer, valid)
}

/// The C's `expansion_word:` label; falls through to `brace_start`.
unsafe fn expansion_word(scanner: &mut Scanner, lexer: *mut TSLexer, valid: *const bool) -> bool {
    if v(valid, EXPANSION_WORD) {
        let mut advanced_once = false;
        let mut advance_once_space = false;
        loop {
            if la(lexer) == '"' as i32 {
                return false;
            }
            if la(lexer) == '$' as i32 {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == '{' as i32
                    || la(lexer) == '(' as i32
                    || la(lexer) == '\'' as i32
                    || is_walnum(la(lexer))
                {
                    set_sym(lexer, EXPANSION_WORD);
                    return advanced_once;
                }
                advanced_once = true;
            }

            if la(lexer) == '}' as i32 {
                mark_end(lexer);
                set_sym(lexer, EXPANSION_WORD);
                return advanced_once || advance_once_space;
            }

            if la(lexer) == '(' as i32 && !(advanced_once || advance_once_space) {
                mark_end(lexer);
                advance(lexer);
                while la(lexer) != ')' as i32 && !eof(lexer) {
                    // if we find a $( or ${ assume this is valid and is a garbage concatenation
                    // of some weird word + an expansion
                    if la(lexer) == '$' as i32 {
                        mark_end(lexer);
                        advance(lexer);
                        if la(lexer) == '{' as i32
                            || la(lexer) == '(' as i32
                            || la(lexer) == '\'' as i32
                            || is_walnum(la(lexer))
                        {
                            set_sym(lexer, EXPANSION_WORD);
                            return advanced_once;
                        }
                        advanced_once = true;
                    } else {
                        advanced_once = advanced_once || !is_wspace(la(lexer));
                        advance_once_space = advance_once_space || is_wspace(la(lexer));
                        advance(lexer);
                    }
                }
                mark_end(lexer);
                if la(lexer) == ')' as i32 {
                    advanced_once = true;
                    advance(lexer);
                    mark_end(lexer);
                    if la(lexer) == '}' as i32 {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            if la(lexer) == '\'' as i32 {
                return false;
            }

            if eof(lexer) {
                return false;
            }
            advanced_once = advanced_once || !is_wspace(la(lexer));
            advance_once_space = advance_once_space || is_wspace(la(lexer));
            advance(lexer);
        }
    }

    brace_start(scanner, lexer, valid)
}

/// The C's `brace_start:` label.
unsafe fn brace_start(_scanner: &mut Scanner, lexer: *mut TSLexer, valid: *const bool) -> bool {
    if v(valid, BRACE_START) && !in_error_recovery(valid) {
        while is_wspace(la(lexer)) {
            skip(lexer);
        }

        if la(lexer) != '{' as i32 {
            return false;
        }

        advance(lexer);
        mark_end(lexer);

        while is_wdigit(la(lexer)) {
            advance(lexer);
        }

        if la(lexer) != '.' as i32 {
            return false;
        }
        advance(lexer);

        if la(lexer) != '.' as i32 {
            return false;
        }
        advance(lexer);

        while is_wdigit(la(lexer)) {
            advance(lexer);
        }

        if la(lexer) != '}' as i32 {
            return false;
        }

        set_sym(lexer, BRACE_START);
        return true;
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_bash_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_bash_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_bash_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    (*(payload as *const Scanner)).serialize(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_bash_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    (*(payload as *mut Scanner)).deserialize(buffer, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_bash_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(&mut *(payload as *mut Scanner), lexer, valid_symbols)
}
