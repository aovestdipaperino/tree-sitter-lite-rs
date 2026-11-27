//! Rust port of tree-sitter-perl's external scanner (src/scanner.c, crate version 1.1.2).
//!
//! Notes on faithfulness:
//! - `serialize` writes nothing and `deserialize` ignores its input, exactly like the C: the
//!   scanner state (delimiters, heredoc queues) lives only in the payload and is never restored.
//! - `iswspace`, `iswalnum`, `iswupper`, `iswalpha` and `isdigit` are ported as the C locale (ASCII
//!   only).
//! - The C `malloc`s the payload and leaves `start_delimiter_char`, `end_delimiter_char`,
//!   `is_separator_delimiter_parsed` and `is_delimiter_enclosing` uninitialised until the first
//!   start delimiter is parsed; here they start as zero / false.
//! - The C queues are linked lists of 1000-byte unions; they are `VecDeque`s here. The C copies the
//!   words it builds with `strncat` into `char[1000]` buffers without any bound check (a stack
//!   overrun for an identifier or a heredoc body word of 1000 or more bytes); here the words are
//!   growable vectors, which gives the C's result wherever the C stays in bounds.
//! - `strncat(word, (char *)&lookahead, 1)` appends the first byte of the native-endian `int32_t`
//!   and nothing when that byte is 0; `to_ne_bytes()[0]` reproduces it.
//! - `scan_nested_delimiters` is recursive in the C (one frame per nested start delimiter, so a
//!   long run of start delimiters overflows the C stack). It is an iteration with a depth counter
//!   here: the same lexer calls in the same order, without the stack growth.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};
use std::collections::VecDeque;

const SCALAR_VARIABLE_EXTERNAL: usize = 0;
const START_DELIMITER: usize = 1;
const END_DELIMITER: usize = 2;
const STRING_CONTENT: usize = 3;
const STRING_SINGLE_QUOTED_CONTENT: usize = 4;
const STRING_SINGLE_Q_QUOTED_CONTENT: usize = 5;
const STRING_QQ_QUOTED_CONTENT: usize = 6;
const STRING_DOUBLE_QUOTED_CONTENT: usize = 7;
const START_DELIMITER_QW: usize = 8;
const ELEMENT_IN_QW: usize = 9;
const END_DELIMITER_QW: usize = 10;
const START_DELIMITER_REGEX: usize = 11;
const START_DELIMITER_SLASH: usize = 12;
const REGEX_PATTERN: usize = 13;
const END_DELIMITER_REGEX: usize = 14;
const START_DELIMITER_SEARCH_REPLACE: usize = 15;
const SEARCH_REPLACE_CONTENT: usize = 16;
const SEPARATOR_DELIMITER_SEARCH_REPLACE: usize = 17;
const END_DELIMITER_SEARCH_REPLACE: usize = 18;
const START_DELIMITER_TRANSLITERATION: usize = 19;
const TRANSLITERATION_CONTENT: usize = 20;
const SEPARATOR_DELIMITER_TRANSLITERATION: usize = 21;
const END_DELIMITER_TRANSLITERATION: usize = 22;
const IMAGINARY_HEREDOC_START: usize = 23;
const HEREDOC_START_IDENTIFIER: usize = 24;
const HEREDOC_CONTENT: usize = 25;
const HEREDOC_END_IDENTIFIER: usize = 26;
const POD_CONTENT: usize = 27;
const DATA_NOT_FOR_COMPILER: usize = 28;
const AUTOMATIC_SEMICOLON: usize = 29;

#[derive(Default)]
struct Scanner {
    start_delimiter_char: i32,
    end_delimiter_char: i32,
    is_separator_delimiter_parsed: bool,
    is_delimiter_enclosing: bool,
    started_heredoc: bool,
    started_heredoc_body: bool,
    heredoc_identifier_queue: VecDeque<Vec<u8>>,
    heredoc_allows_interpolation: VecDeque<bool>,
    heredoc_allows_indent: VecDeque<bool>,
}

// C locale ctype functions.
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}
fn isdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}
fn iswalpha(c: i32) -> bool {
    (b'a' as i32..=b'z' as i32).contains(&c) || (b'A' as i32..=b'Z' as i32).contains(&c)
}
fn iswupper(c: i32) -> bool {
    (b'A' as i32..=b'Z' as i32).contains(&c)
}
fn iswalnum(c: i32) -> bool {
    isdigit(c) || iswalpha(c)
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

unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}

unsafe fn set_result(lexer: *mut TSLexer, sym: usize) {
    (*lexer).result_symbol = sym as u16;
}

unsafe fn run_over_spaces(lexer: *mut TSLexer) {
    while iswspace(la(lexer)) {
        skip(lexer);
    }
}

unsafe fn run_with_spaces(lexer: *mut TSLexer) {
    while iswspace(la(lexer)) {
        advance(lexer);
    }
}

unsafe fn is_identifier(lexer: *mut TSLexer) -> bool {
    iswalnum(la(lexer)) || la(lexer) == b'_' as i32
}

fn is_char(c: i32, ch: u8) -> bool {
    c == ch as i32
}

unsafe fn handle_interpolation(scanner: &Scanner, lexer: *mut TSLexer, surrounding: usize) -> bool {
    if is_char(la(lexer), b'$') {
        // allow $ to be last character in a regex
        if surrounding == SEARCH_REPLACE_CONTENT || surrounding == REGEX_PATTERN {
            advance(lexer);
            run_with_spaces(lexer);
            if la(lexer) == scanner.end_delimiter_char {
                set_result(lexer, surrounding);
                mark_end(lexer);
                return true;
            }
        }
        return false;
    }
    false
}

unsafe fn handle_escape_sequence(lexer: *mut TSLexer, surrounding: usize) -> bool {
    if is_char(la(lexer), b'\\') {
        advance(lexer);
        let c = la(lexer);
        if [b't', b'n', b'r', b'f', b'b', b'a', b'e']
            .iter()
            .any(|&x| is_char(c, x))
        {
            mark_end(lexer);
            return false;
        } else {
            set_result(lexer, surrounding);
            advance(lexer);
            mark_end(lexer);
            return true;
        }
    }
    false
}

/// Iterative form of the C's recursive `scan_nested_delimiters` (see the module notes).
unsafe fn scan_nested_delimiters(
    scanner: &Scanner,
    lexer: *mut TSLexer,
    token_type: usize,
) -> bool {
    let mut depth: usize = 0;
    while la(lexer) != 0 {
        if la(lexer) == scanner.end_delimiter_char {
            set_result(lexer, token_type);
            advance(lexer);
            mark_end(lexer);
            if depth == 0 {
                return true;
            }
            depth -= 1;
        } else if la(lexer) == scanner.start_delimiter_char {
            set_result(lexer, token_type);
            advance(lexer);
            depth += 1;
        } else if is_char(la(lexer), b'\\') {
            advance(lexer);
            advance(lexer);
        } else {
            advance(lexer);
        }
    }
    mark_end(lexer);
    false
}

unsafe fn parse_delimited_and_interpolated_content(
    scanner: &Scanner,
    lexer: *mut TSLexer,
    token_type: usize,
    ending_delimiter: usize,
    can_escape: bool,
    can_interpolate: bool,
) -> bool {
    if la(lexer) == scanner.end_delimiter_char {
        set_result(lexer, ending_delimiter);
        advance(lexer);
        mark_end(lexer);
        true
    } else {
        if can_interpolate && is_char(la(lexer), b'$') {
            return handle_interpolation(scanner, lexer, token_type);
        }
        if can_escape && is_char(la(lexer), b'\\') {
            return handle_escape_sequence(lexer, token_type);
        }
        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }
        if la(lexer) == scanner.start_delimiter_char {
            set_result(lexer, token_type);
            advance(lexer);
            return scan_nested_delimiters(scanner, lexer, token_type);
        }
        set_result(lexer, token_type);
        advance(lexer);
        mark_end(lexer);
        true
    }
}

fn set_end_delimiter(scanner: &mut Scanner, start_delimiter: i32) {
    scanner.is_delimiter_enclosing = true;
    if start_delimiter == b'(' as i32 {
        scanner.end_delimiter_char = b')' as i32;
    } else if start_delimiter == b'<' as i32 {
        scanner.end_delimiter_char = b'>' as i32;
    } else if start_delimiter == b'[' as i32 {
        scanner.end_delimiter_char = b']' as i32;
    } else if start_delimiter == b'{' as i32 {
        scanner.end_delimiter_char = b'}' as i32;
    } else {
        scanner.is_delimiter_enclosing = false;
        scanner.end_delimiter_char = start_delimiter;
    }
}

unsafe fn process_separator_delimiter(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    separator_token: usize,
    end_token: usize,
) -> bool {
    if scanner.is_separator_delimiter_parsed {
        set_result(lexer, end_token);
        advance(lexer);
        mark_end(lexer);
        true
    } else {
        set_result(lexer, separator_token);
        advance(lexer);
        mark_end(lexer);

        if scanner.is_delimiter_enclosing {
            run_over_spaces(lexer);

            if la(lexer) == scanner.start_delimiter_char {
                set_result(lexer, separator_token);
                advance(lexer);
                mark_end(lexer);

                scanner.is_separator_delimiter_parsed = true;

                return true;
            }

            false
        } else {
            scanner.is_separator_delimiter_parsed = true;

            true
        }
    }
}

unsafe fn parse_start_delimiter(
    scanner: &mut Scanner,
    lexer: *mut TSLexer,
    token_type: usize,
) -> bool {
    run_over_spaces(lexer);

    scanner.start_delimiter_char = la(lexer);
    let s = scanner.start_delimiter_char;
    set_end_delimiter(scanner, s);

    scanner.is_separator_delimiter_parsed = false;

    set_result(lexer, token_type);
    advance(lexer);
    mark_end(lexer);

    true
}

/// `strncat(word, (char *)&lookahead, 1)`: the first byte of the native-endian `int32_t`, nothing
/// when it is 0.
fn push_low_byte(word: &mut Vec<u8>, c: i32) {
    let b = c.to_ne_bytes()[0];
    if b != 0 {
        word.push(b);
    }
}

unsafe fn advance_word(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    let mut empty = true;
    let mut has_space_before = false;
    let mut allows_interpolation = true;
    let mut unquoted_word: Vec<u8> = Vec::new();

    // <<~EOF
    if is_char(la(lexer), b'~') {
        scanner.heredoc_allows_indent.push_back(true);
        advance(lexer);
    } else {
        scanner.heredoc_allows_indent.push_back(false);
    }

    // <<\EOF, <<~\EOF
    if is_char(la(lexer), b'\\') {
        allows_interpolation = false;
        advance(lexer);
    }

    if iswspace(la(lexer)) {
        run_over_spaces(lexer);
        has_space_before = true;
    }

    let mut quote: i32 = 0;
    if is_char(la(lexer), b'\'') || is_char(la(lexer), b'"') || is_char(la(lexer), b'`') {
        allows_interpolation = !is_char(la(lexer), b'\'');
        quote = la(lexer);
        advance(lexer);
    } else if has_space_before {
        return false;
    }

    while la(lexer) != 0
        && is_identifier(lexer)
        && !(if quote != 0 {
            la(lexer) == quote
        } else {
            iswspace(la(lexer))
        })
    {
        if is_char(la(lexer), b'\\') {
            advance(lexer);
            if la(lexer) == 0 {
                return false;
            }
        }
        empty = false;

        push_low_byte(&mut unquoted_word, la(lexer));
        advance(lexer);
    }

    if quote != 0 && la(lexer) == quote {
        advance(lexer);
    }

    if !empty {
        scanner.heredoc_identifier_queue.push_back(unquoted_word);
        scanner
            .heredoc_allows_interpolation
            .push_back(allows_interpolation);
        scanner.started_heredoc = true;
    }

    !empty
}

unsafe fn exit_if_heredoc_end_delimiter(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    let mut word: Vec<u8> = Vec::new();
    set_result(lexer, HEREDOC_END_IDENTIFIER);
    while !iswspace(la(lexer)) {
        push_low_byte(&mut word, la(lexer));
        advance(lexer);

        if la(lexer) == 0 {
            break;
        }
    }

    // The caller guarantees the identifier queue is not empty.
    if scanner.heredoc_identifier_queue.front() == Some(&word) {
        set_result(lexer, HEREDOC_END_IDENTIFIER);
        mark_end(lexer);

        scanner.started_heredoc = false;
        scanner.started_heredoc_body = false;
        scanner.heredoc_identifier_queue.pop_front();
        scanner.heredoc_allows_interpolation.pop_front();
        true
    } else {
        set_result(lexer, HEREDOC_CONTENT);
        true
    }
}

unsafe fn is_special_variable_identifier(lexer: *mut TSLexer) -> bool {
    let c = la(lexer);
    isdigit(c)
        || [
            b'!', b'_', b'"', b'$', b'%', b'&', b'\'', b'(', b')', b'+', b'-', b'.', b'/', b':',
            b';', b'<', b'=', b'>', b'?', b'@', b']', b'[', b'\\', b'`', b'|', b'~',
        ]
        .iter()
        .any(|&x| is_char(c, x))
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);

    // On ERROR, the external scanner is called with all valid_symbols true.
    if valid(SCALAR_VARIABLE_EXTERNAL)
        && valid(START_DELIMITER)
        && valid(END_DELIMITER)
        && valid(STRING_CONTENT)
        && valid(STRING_SINGLE_QUOTED_CONTENT)
        && valid(STRING_SINGLE_Q_QUOTED_CONTENT)
        && valid(STRING_QQ_QUOTED_CONTENT)
        && valid(STRING_DOUBLE_QUOTED_CONTENT)
        && valid(START_DELIMITER_QW)
        && valid(END_DELIMITER_QW)
        && valid(START_DELIMITER_REGEX)
        && valid(REGEX_PATTERN)
        && valid(END_DELIMITER_REGEX)
        && valid(START_DELIMITER_SEARCH_REPLACE)
        && valid(SEARCH_REPLACE_CONTENT)
        && valid(SEPARATOR_DELIMITER_SEARCH_REPLACE)
        && valid(END_DELIMITER_SEARCH_REPLACE)
        && valid(START_DELIMITER_TRANSLITERATION)
        && valid(TRANSLITERATION_CONTENT)
        && valid(SEPARATOR_DELIMITER_TRANSLITERATION)
        && valid(END_DELIMITER_TRANSLITERATION)
        && valid(IMAGINARY_HEREDOC_START)
        && valid(HEREDOC_START_IDENTIFIER)
        && valid(HEREDOC_CONTENT)
        && valid(HEREDOC_END_IDENTIFIER)
        && valid(POD_CONTENT)
        && valid(DATA_NOT_FOR_COMPILER)
        && valid(START_DELIMITER_SLASH)
        && valid(AUTOMATIC_SEMICOLON)
    {
        return false;
    }

    if valid(SCALAR_VARIABLE_EXTERNAL) {
        run_over_spaces(lexer);

        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }

        let mut met_identifier = false;
        if is_special_variable_identifier(lexer) {
            if isdigit(la(lexer)) {
                advance(lexer);
                if iswalnum(la(lexer)) {
                    set_result(lexer, SCALAR_VARIABLE_EXTERNAL);
                    advance(lexer);
                    met_identifier = true;
                } else {
                    mark_end(lexer);
                    return false;
                }
            } else {
                mark_end(lexer);
                return false;
            }
        } else if is_char(la(lexer), b'#') || is_char(la(lexer), b'*') {
            set_result(lexer, SCALAR_VARIABLE_EXTERNAL);
            advance(lexer);
        } else if is_char(la(lexer), b'^') {
            set_result(lexer, SCALAR_VARIABLE_EXTERNAL);
            advance(lexer);
            if iswupper(la(lexer)) && iswalpha(la(lexer)) {
                advance(lexer);
                mark_end(lexer);
                return true;
            } else {
                mark_end(lexer);
                return false;
            }
        }

        while is_identifier(lexer) {
            set_result(lexer, SCALAR_VARIABLE_EXTERNAL);
            advance(lexer);
            met_identifier = true;
        }

        mark_end(lexer);
        return met_identifier;
    }

    if valid(STRING_SINGLE_QUOTED_CONTENT) {
        if is_char(la(lexer), b'\'') {
            mark_end(lexer);
            advance(lexer);
            return false;
        } else if is_char(la(lexer), b'\\') {
            set_result(lexer, STRING_SINGLE_QUOTED_CONTENT);
            advance(lexer);

            if is_char(la(lexer), b'\'') {
                advance(lexer);
            }
            mark_end(lexer);
            return true;
        }

        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }

        set_result(lexer, STRING_SINGLE_QUOTED_CONTENT);
        advance(lexer);
        mark_end(lexer);

        return true;
    }

    if valid(START_DELIMITER) {
        return parse_start_delimiter(scanner, lexer, START_DELIMITER);
    }

    if valid(STRING_SINGLE_Q_QUOTED_CONTENT) {
        return parse_delimited_and_interpolated_content(
            scanner,
            lexer,
            STRING_SINGLE_Q_QUOTED_CONTENT,
            END_DELIMITER,
            false,
            false,
        );
    }

    if valid(STRING_QQ_QUOTED_CONTENT) {
        return parse_delimited_and_interpolated_content(
            scanner,
            lexer,
            STRING_QQ_QUOTED_CONTENT,
            END_DELIMITER,
            true,
            true,
        );
    }

    if valid(STRING_DOUBLE_QUOTED_CONTENT) {
        if is_char(la(lexer), b'"') {
            mark_end(lexer);
            advance(lexer);
            return false;
        }

        if is_char(la(lexer), b'$') {
            return handle_interpolation(scanner, lexer, STRING_DOUBLE_QUOTED_CONTENT);
        }
        if is_char(la(lexer), b'\\') {
            return handle_escape_sequence(lexer, STRING_DOUBLE_QUOTED_CONTENT);
        }

        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }

        set_result(lexer, STRING_DOUBLE_QUOTED_CONTENT);
        advance(lexer);
        mark_end(lexer);
        return true;
    }

    if valid(START_DELIMITER_QW) {
        return parse_start_delimiter(scanner, lexer, START_DELIMITER_QW);
    }

    if valid(ELEMENT_IN_QW) {
        run_over_spaces(lexer);

        if la(lexer) == scanner.end_delimiter_char {
            set_result(lexer, END_DELIMITER_QW);
            advance(lexer);
            mark_end(lexer);
            return true;
        }

        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }

        while la(lexer) != 0
            && !is_char(la(lexer), b' ')
            && !is_char(la(lexer), b'\t')
            && !is_char(la(lexer), b'\r')
            && !is_char(la(lexer), b'\n')
            && la(lexer) != scanner.end_delimiter_char
        {
            set_result(lexer, ELEMENT_IN_QW);
            advance(lexer);
        }

        mark_end(lexer);
        return true;
    }

    if valid(START_DELIMITER_SLASH) {
        if is_char(la(lexer), b'/') {
            return parse_start_delimiter(scanner, lexer, START_DELIMITER_SLASH);
        } else {
            mark_end(lexer);
            return false;
        }
    }

    if valid(START_DELIMITER_REGEX) {
        return parse_start_delimiter(scanner, lexer, START_DELIMITER_REGEX);
    }
    if valid(REGEX_PATTERN) {
        return parse_delimited_and_interpolated_content(
            scanner,
            lexer,
            REGEX_PATTERN,
            END_DELIMITER_REGEX,
            true,
            true,
        );
    }

    if valid(START_DELIMITER_SEARCH_REPLACE) {
        return parse_start_delimiter(scanner, lexer, START_DELIMITER_SEARCH_REPLACE);
    }

    if valid(SEARCH_REPLACE_CONTENT) {
        if la(lexer) == scanner.end_delimiter_char {
            return process_separator_delimiter(
                scanner,
                lexer,
                SEPARATOR_DELIMITER_SEARCH_REPLACE,
                END_DELIMITER_SEARCH_REPLACE,
            );
        } else {
            if is_char(la(lexer), b'$') {
                return handle_interpolation(scanner, lexer, SEARCH_REPLACE_CONTENT);
            }
            if is_char(la(lexer), b'\\') {
                return handle_escape_sequence(lexer, SEARCH_REPLACE_CONTENT);
            }

            if la(lexer) == 0 {
                mark_end(lexer);
                return false;
            }

            if la(lexer) == scanner.start_delimiter_char {
                set_result(lexer, SEARCH_REPLACE_CONTENT);
                advance(lexer);
                return scan_nested_delimiters(scanner, lexer, SEARCH_REPLACE_CONTENT);
            }

            set_result(lexer, SEARCH_REPLACE_CONTENT);
            advance(lexer);
            return true;
        }
    }

    if valid(START_DELIMITER_TRANSLITERATION) {
        return parse_start_delimiter(scanner, lexer, START_DELIMITER_TRANSLITERATION);
    }
    if valid(TRANSLITERATION_CONTENT) {
        if la(lexer) == scanner.end_delimiter_char {
            return process_separator_delimiter(
                scanner,
                lexer,
                SEPARATOR_DELIMITER_TRANSLITERATION,
                END_DELIMITER_TRANSLITERATION,
            );
        }

        if la(lexer) == 0 {
            mark_end(lexer);
            return false;
        }

        if is_char(la(lexer), b'\\') {
            set_result(lexer, TRANSLITERATION_CONTENT);
            advance(lexer);
            if la(lexer) == scanner.end_delimiter_char {
                advance(lexer);
            }

            mark_end(lexer);
            return true;
        }

        if la(lexer) == scanner.start_delimiter_char {
            set_result(lexer, TRANSLITERATION_CONTENT);
            advance(lexer);
            return scan_nested_delimiters(scanner, lexer, TRANSLITERATION_CONTENT);
        }

        set_result(lexer, TRANSLITERATION_CONTENT);
        advance(lexer);
        mark_end(lexer);
        return true;
    }

    if valid(HEREDOC_START_IDENTIFIER) {
        set_result(lexer, HEREDOC_START_IDENTIFIER);
        return advance_word(scanner, lexer);
    }

    if (valid(HEREDOC_CONTENT) || valid(IMAGINARY_HEREDOC_START))
        && !scanner.heredoc_identifier_queue.is_empty()
    {
        if la(lexer) == 0 && !scanner.started_heredoc_body {
            return false;
        }

        if is_char(la(lexer), b'\n') && !scanner.started_heredoc_body {
            scanner.started_heredoc_body = true;

            set_result(lexer, IMAGINARY_HEREDOC_START);
            mark_end(lexer);
            return true;
        }
        if scanner.started_heredoc_body {
            // The queues hold matching entries (the identifier queue is non-empty here).
            let interp = scanner
                .heredoc_allows_interpolation
                .front()
                .copied()
                .unwrap_or(false);
            let c = la(lexer);
            // The C switch falls through from '\\' to '$' to '\n'.
            if is_char(c, b'\\') && interp {
                return handle_escape_sequence(lexer, HEREDOC_CONTENT);
            }
            if is_char(c, b'$') && interp {
                return false;
            }
            if is_char(c, b'\\') || is_char(c, b'$') || is_char(c, b'\n') {
                skip(lexer);
                mark_end(lexer);
                if scanner
                    .heredoc_allows_indent
                    .front()
                    .copied()
                    .unwrap_or(false)
                {
                    while iswspace(la(lexer)) {
                        advance(lexer);
                    }
                }
                return exit_if_heredoc_end_delimiter(scanner, lexer);
            }
            if la(lexer) == 0 {
                scanner.started_heredoc_body = false;
                mark_end(lexer);
                return false;
            }
            set_result(lexer, HEREDOC_CONTENT);
            advance(lexer);
            return true;
        } else {
            return false;
        }
    }

    if valid(POD_CONTENT) {
        while la(lexer) != 0 {
            set_result(lexer, POD_CONTENT);

            if is_char(la(lexer), b'\n') {
                advance(lexer);
                if is_char(la(lexer), b'=') {
                    advance(lexer);
                    if is_char(la(lexer), b'c') {
                        advance(lexer);
                        if is_char(la(lexer), b'u') {
                            advance(lexer);
                            if is_char(la(lexer), b't') {
                                advance(lexer);
                                while la(lexer) != 0 && !is_char(la(lexer), b'\n') {
                                    advance(lexer);
                                }
                                mark_end(lexer);
                                return true;
                            }
                        }
                    }
                }
            } else {
                advance(lexer);
            }
        }

        mark_end(lexer);
        return true;
    }

    if valid(DATA_NOT_FOR_COMPILER) {
        set_result(lexer, DATA_NOT_FOR_COMPILER);
        while la(lexer) != 0 {
            advance(lexer);
        }

        mark_end(lexer);
        return true;
    }
    if valid(AUTOMATIC_SEMICOLON) {
        loop {
            if is_char(la(lexer), b'}') || la(lexer) == 0 {
                set_result(lexer, AUTOMATIC_SEMICOLON);
                mark_end(lexer);
                return true;
            } else if is_char(la(lexer), b';') || !iswspace(la(lexer)) {
                return false;
            } else if iswspace(la(lexer)) {
                skip(lexer);
            }
        }
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_perl_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_perl_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

/// The C serializes nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_perl_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

/// The C ignores the buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_perl_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_perl_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    scan(s, lexer, valid_symbols)
}
