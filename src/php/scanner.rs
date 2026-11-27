//! Rust port of tree-sitter-php's external scanner (common/scanner.h, included by php/src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// enum TokenType
const AUTOMATIC_SEMICOLON: usize = 0;
const ENCAPSED_STRING_CHARS: usize = 1;
const ENCAPSED_STRING_CHARS_AFTER_VARIABLE: usize = 2;
const EXECUTION_STRING_CHARS: usize = 3;
const EXECUTION_STRING_CHARS_AFTER_VARIABLE: usize = 4;
const ENCAPSED_STRING_CHARS_HEREDOC: usize = 5;
const ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC: usize = 6;
const EOF_TOKEN: usize = 7;
const HEREDOC_START: usize = 8;
const HEREDOC_END: usize = 9;
const NOWDOC_STRING: usize = 10;
const SENTINEL_ERROR: usize = 11;

const TREE_SITTER_SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// `Array(int32_t)`.
type Str = Vec<i32>;

#[derive(Default, Clone)]
struct Heredoc {
    end_word_indentation_allowed: bool,
    word: Str,
}

#[derive(Default)]
struct Scanner {
    // Written by the C but never read.
    has_leading_whitespace: bool,
    heredocs: Vec<Heredoc>,
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

// The C calls iswspace, iswalnum, iswdigit and iswxdigit. A program that never calls setlocale
// is in the C locale, where these are ASCII only.

/// `iswspace` in the C locale: tab, newline, vertical tab, form feed, carriage return, space.
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswdigit` in the C locale.
fn iswdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}

/// `iswxdigit` in the C locale.
fn iswxdigit(c: i32) -> bool {
    iswdigit(c)
        || (b'a' as i32..=b'f' as i32).contains(&c)
        || (b'A' as i32..=b'F' as i32).contains(&c)
}

/// `iswalnum` in the C locale.
fn iswalnum(c: i32) -> bool {
    iswdigit(c)
        || (b'a' as i32..=b'z' as i32).contains(&c)
        || (b'A' as i32..=b'Z' as i32).contains(&c)
}

fn reset_heredoc(heredoc: &mut Heredoc) {
    // array_delete: frees the contents and sets size and capacity to 0.
    heredoc.word = Vec::new();
    heredoc.end_word_indentation_allowed = false;
}

unsafe fn serialize(scanner: &Scanner, buffer: *mut c_char) -> u32 {
    let buf = buffer as *mut u8;
    let mut size: usize = 0;

    *buf.add(size) = scanner.heredocs.len() as u8;
    size += 1;
    for heredoc in &scanner.heredocs {
        let word_size = heredoc.word.len() * 4;
        if size + 5 + word_size >= TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
            return 0;
        }
        *buf.add(size) = heredoc.end_word_indentation_allowed as u8;
        size += 1;
        core::ptr::copy_nonoverlapping(
            (heredoc.word.len() as u32).to_ne_bytes().as_ptr(),
            buf.add(size),
            4,
        );
        size += 4;
        if !heredoc.word.is_empty() {
            core::ptr::copy_nonoverlapping(
                heredoc.word.as_ptr() as *const u8,
                buf.add(size),
                word_size,
            );
            size += word_size;
        }
    }

    size as u32
}

unsafe fn deserialize(scanner: &mut Scanner, buffer: *const c_char, length: u32) {
    let buf = buffer as *const u8;
    let mut size: usize = 0;
    scanner.has_leading_whitespace = false;

    // The C resets every existing heredoc but never shrinks the array, so stale (now empty)
    // entries stay and are reused below; they are visible to scan() through heredocs.size.
    for h in scanner.heredocs.iter_mut() {
        reset_heredoc(h);
    }

    if length == 0 {
        return;
    }

    let open_heredoc_count = *buf.add(size);
    size += 1;
    for i in 0..open_heredoc_count as usize {
        if i >= scanner.heredocs.len() {
            scanner.heredocs.push(Heredoc::default());
        }
        let heredoc = &mut scanner.heredocs[i];

        heredoc.end_word_indentation_allowed = *buf.add(size) != 0;
        size += 1;
        let mut n = [0u8; 4];
        core::ptr::copy_nonoverlapping(buf.add(size), n.as_mut_ptr(), 4);
        let word_len = u32::from_ne_bytes(n) as usize;
        size += 4;
        let word_size = word_len * 4;
        heredoc.word = vec![0i32; word_len];
        if word_size > 0 {
            core::ptr::copy_nonoverlapping(
                buf.add(size),
                heredoc.word.as_mut_ptr() as *mut u8,
                word_size,
            );
            size += word_size;
        }
    }

    // The C asserts size == length.
    debug_assert_eq!(size, length as usize);
}

unsafe fn scan_whitespace(lexer: *mut TSLexer) -> bool {
    loop {
        while iswspace((*lexer).lookahead) {
            advance(lexer);
        }

        if (*lexer).lookahead == '/' as i32 {
            advance(lexer);

            if (*lexer).lookahead == '/' as i32 {
                advance(lexer);
                while (*lexer).lookahead != 0 && (*lexer).lookahead != '\n' as i32 {
                    advance(lexer);
                }
            } else {
                return false;
            }
        } else {
            return true;
        }
    }
}

unsafe fn is_valid_name_char(lexer: *mut TSLexer) -> bool {
    iswalnum((*lexer).lookahead) || (*lexer).lookahead == '_' as i32 || (*lexer).lookahead >= 0x80
}

unsafe fn is_escapable_sequence(lexer: *mut TSLexer) -> bool {
    let letter = (*lexer).lookahead;

    if letter == 'n' as i32
        || letter == 'r' as i32
        || letter == 't' as i32
        || letter == 'v' as i32
        || letter == 'e' as i32
        || letter == 'f' as i32
        || letter == '\\' as i32
        || letter == '$' as i32
        || letter == '"' as i32
    {
        return true;
    }

    // Hex
    if letter == 'x' as i32 {
        advance(lexer);
        return iswxdigit((*lexer).lookahead);
    }

    // Unicode
    if letter == 'u' as i32 {
        return true;
    }

    // Octal
    iswdigit((*lexer).lookahead)
        && (*lexer).lookahead >= '0' as i32
        && (*lexer).lookahead <= '7' as i32
}

unsafe fn scan_heredoc_word(lexer: *mut TSLexer) -> Str {
    let mut result = Str::new();

    while is_valid_name_char(lexer) {
        result.push((*lexer).lookahead);
        advance(lexer);
    }

    result
}

unsafe fn scan_nowdoc_string(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    let mut has_consumed_content = false;
    if scanner.heredocs.is_empty() {
        return false;
    }

    // While PHP requires the nowdoc end tag to be the very first on a new line,
    // there may be an arbitrary amount of whitespace before the closing token
    while iswspace((*lexer).lookahead) {
        advance(lexer);
        has_consumed_content = true;
    }

    let mut end_tag_matched = false;
    let heredoc_tag: Str = scanner.heredocs.last().unwrap().word.clone();

    for i in 0..heredoc_tag.len() {
        if (*lexer).lookahead != heredoc_tag[i] {
            break;
        }
        advance(lexer);
        has_consumed_content = true;

        end_tag_matched = i == heredoc_tag.len() - 1
            && (iswspace((*lexer).lookahead)
                || (*lexer).lookahead == ';' as i32
                || (*lexer).lookahead == ',' as i32
                || (*lexer).lookahead == ')' as i32);
    }

    if end_tag_matched {
        // There may be an arbitrary amount of white space after the end tag
        while iswspace((*lexer).lookahead)
            && (*lexer).lookahead != '\r' as i32
            && (*lexer).lookahead != '\n' as i32
        {
            advance(lexer);
            has_consumed_content = true;
        }

        // Return to allow the end tag parsing if we've encountered an end tag
        // at a valid position
        if (*lexer).lookahead == ';' as i32
            || (*lexer).lookahead == ',' as i32
            || (*lexer).lookahead == ')' as i32
            || (*lexer).lookahead == '\n' as i32
            || (*lexer).lookahead == '\r' as i32
        {
            return false;
        }
    }

    let mut has_content = has_consumed_content;
    loop {
        ((*lexer).mark_end)(lexer);

        match (*lexer).lookahead {
            10 | 13 => return has_content,
            _ => {
                if ((*lexer).eof)(lexer) {
                    return false;
                }
                advance(lexer);
            }
        }
        has_content = true;
    }
}

unsafe fn scan_encapsed_part_string(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    mut is_after_variable: bool,
    is_heredoc: bool,
    is_execution_string: bool,
) -> bool {
    let mut has_consumed_content = false;

    if is_heredoc && !scanner.heredocs.is_empty() {
        // While PHP requires the heredoc end tag to be the very first on a new
        // line, there may be an arbitrary amount of whitespace before the
        // closing token. However, we should not consume \r or \n
        while iswspace((*lexer).lookahead)
            && (*lexer).lookahead != '\r' as i32
            && (*lexer).lookahead != '\n' as i32
        {
            advance(lexer);
            has_consumed_content = true;
        }

        let heredoc_tag: Str = scanner.heredocs.last().unwrap().word.clone();

        let mut end_tag_matched = false;

        for i in 0..heredoc_tag.len() {
            if (*lexer).lookahead != heredoc_tag[i] {
                break;
            }
            has_consumed_content = true;
            advance(lexer);

            end_tag_matched = i == heredoc_tag.len() - 1
                && (iswspace((*lexer).lookahead)
                    || (*lexer).lookahead == ';' as i32
                    || (*lexer).lookahead == ',' as i32
                    || (*lexer).lookahead == ')' as i32);
        }

        if end_tag_matched {
            // There may be an arbitrary amount of white space after the end tag
            // However, we should not consume \r or \n
            while iswspace((*lexer).lookahead)
                && (*lexer).lookahead != '\r' as i32
                && (*lexer).lookahead != '\n' as i32
            {
                advance(lexer);
                has_consumed_content = true;
            }

            // Return to allow the end tag parsing if we've encountered an end
            // tag at a valid position
            if (*lexer).lookahead == ';' as i32
                || (*lexer).lookahead == ',' as i32
                || (*lexer).lookahead == ')' as i32
                || (*lexer).lookahead == '\n' as i32
                || (*lexer).lookahead == '\r' as i32
            {
                return false;
            }
        }
    }

    let mut has_content = has_consumed_content;
    loop {
        ((*lexer).mark_end)(lexer);

        let la = (*lexer).lookahead;
        if la == '"' as i32 {
            if !is_heredoc && !is_execution_string {
                return has_content;
            }
            advance(lexer);
        } else if la == '`' as i32 {
            if is_execution_string {
                return has_content;
            }
            advance(lexer);
        } else if la == '\n' as i32 || la == '\r' as i32 {
            if is_heredoc {
                return has_content;
            }
            advance(lexer);
        } else if la == '\\' as i32 {
            advance(lexer);

            // \{ should not be interpreted as an escape sequence, but both
            // should be consumed as normal characters
            if (*lexer).lookahead == '{' as i32 {
                advance(lexer);
            } else if is_execution_string && (*lexer).lookahead == '`' as i32 {
                return has_content;
            } else if is_heredoc && (*lexer).lookahead == '\\' as i32 {
                advance(lexer);
            } else if is_escapable_sequence(lexer) {
                return has_content;
            }
        } else if la == '$' as i32 {
            advance(lexer);

            if (is_valid_name_char(lexer) && !iswdigit((*lexer).lookahead))
                || (*lexer).lookahead == '{' as i32
            {
                return has_content;
            }
        } else if la == '-' as i32 && is_after_variable {
            advance(lexer);
            if (*lexer).lookahead == '>' as i32 {
                advance(lexer);
                if is_valid_name_char(lexer) {
                    return has_content;
                }
            }
        } else if la == '-' as i32 || la == '[' as i32 {
            // '-' falls through to '[' in the C when it is not after a variable.
            if la == '[' as i32 && is_after_variable {
                return has_content;
            }
            advance(lexer);
        } else if la == '{' as i32 {
            advance(lexer);
            if (*lexer).lookahead == '$' as i32 {
                return has_content;
            }
        } else {
            if ((*lexer).eof)(lexer) {
                return false;
            }
            advance(lexer);
        }

        is_after_variable = false;
        has_content = true;
    }
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    let is_error_recovery = valid(SENTINEL_ERROR);

    if is_error_recovery {
        return false;
    }

    scanner.has_leading_whitespace = false;

    ((*lexer).mark_end)(lexer);

    if valid(ENCAPSED_STRING_CHARS_AFTER_VARIABLE) {
        (*lexer).result_symbol = ENCAPSED_STRING_CHARS_AFTER_VARIABLE as u16;
        return scan_encapsed_part_string(scanner, lexer, true, false, false);
    }

    if valid(ENCAPSED_STRING_CHARS) {
        (*lexer).result_symbol = ENCAPSED_STRING_CHARS as u16;
        return scan_encapsed_part_string(scanner, lexer, false, false, false);
    }

    if valid(EXECUTION_STRING_CHARS_AFTER_VARIABLE) {
        (*lexer).result_symbol = EXECUTION_STRING_CHARS_AFTER_VARIABLE as u16;
        return scan_encapsed_part_string(scanner, lexer, true, false, true);
    }

    if valid(EXECUTION_STRING_CHARS) {
        (*lexer).result_symbol = EXECUTION_STRING_CHARS as u16;
        return scan_encapsed_part_string(scanner, lexer, false, false, true);
    }

    if valid(ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC) {
        (*lexer).result_symbol = ENCAPSED_STRING_CHARS_AFTER_VARIABLE_HEREDOC as u16;
        return scan_encapsed_part_string(scanner, lexer, true, true, false);
    }

    if valid(ENCAPSED_STRING_CHARS_HEREDOC) {
        (*lexer).result_symbol = ENCAPSED_STRING_CHARS_HEREDOC as u16;
        return scan_encapsed_part_string(scanner, lexer, false, true, false);
    }

    if valid(NOWDOC_STRING) {
        (*lexer).result_symbol = NOWDOC_STRING as u16;
        return scan_nowdoc_string(scanner, lexer);
    }

    if valid(HEREDOC_END) {
        (*lexer).result_symbol = HEREDOC_END as u16;
        if scanner.heredocs.is_empty() {
            return false;
        }

        let heredoc = scanner.heredocs.last().unwrap().clone();

        while iswspace((*lexer).lookahead) {
            skip(lexer);
        }

        let word = scan_heredoc_word(lexer);
        if word != heredoc.word {
            return false;
        }

        ((*lexer).mark_end)(lexer);
        scanner.heredocs.pop();
        return true;
    }

    if !scan_whitespace(lexer) {
        return false;
    }

    if valid(EOF_TOKEN) && ((*lexer).eof)(lexer) {
        (*lexer).result_symbol = EOF_TOKEN as u16;
        return true;
    }

    if valid(HEREDOC_START) {
        (*lexer).result_symbol = HEREDOC_START as u16;
        let mut heredoc = Heredoc::default();

        while iswspace((*lexer).lookahead) {
            skip(lexer);
        }

        heredoc.word = scan_heredoc_word(lexer);
        if heredoc.word.is_empty() {
            return false;
        }
        ((*lexer).mark_end)(lexer);

        scanner.heredocs.push(heredoc);
        return true;
    }

    if valid(AUTOMATIC_SEMICOLON) {
        (*lexer).result_symbol = AUTOMATIC_SEMICOLON as u16;

        if (*lexer).lookahead != '?' as i32 {
            return false;
        }

        advance(lexer);

        return (*lexer).lookahead == '>' as i32;
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    serialize(&*(payload as *const Scanner), buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    deserialize(&mut *(payload as *mut Scanner), buffer, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_php_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(&mut *(payload as *mut Scanner), lexer, valid_symbols)
}
