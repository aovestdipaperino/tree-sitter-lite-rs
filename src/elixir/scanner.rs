//! Rust port of tree-sitter-elixir's external scanner (src/scanner.c). The scanner is stateless:
//! create returns NULL, serialize writes nothing, deserialize ignores its input.
//!
//! Locale functions: the C uses none (every character class is a hand-written comparison), so there
//! is nothing ASCII-only to note here.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// enum TokenType (see grammar.externals)
const QUOTED_CONTENT_I_SINGLE: usize = 0;
const QUOTED_CONTENT_I_DOUBLE: usize = 1;
const NEWLINE_BEFORE_DO: usize = 20;
const NEWLINE_BEFORE_BINARY_OPERATOR: usize = 21;
const NEWLINE_BEFORE_COMMENT: usize = 22;
const BEFORE_UNARY_OPERATOR: usize = 23;
const NOT_IN: usize = 24;
const QUOTED_ATOM_START: usize = 25;

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

fn is_whitespace(c: i32) -> bool {
    c == ' ' as i32 || c == '\t' as i32 || c == '\n' as i32 || c == '\r' as i32
}

fn is_inline_whitespace(c: i32) -> bool {
    c == ' ' as i32 || c == '\t' as i32
}

fn is_newline(c: i32) -> bool {
    c == '\n' as i32 || c == '\r' as i32
}

fn is_digit(c: i32) -> bool {
    '0' as i32 <= c && c <= '9' as i32
}

unsafe fn check_keyword_end(lexer: *mut TSLexer) -> bool {
    if (*lexer).lookahead == ':' as i32 {
        advance(lexer);
        return is_whitespace((*lexer).lookahead);
    }
    false
}

unsafe fn check_operator_end(lexer: *mut TSLexer) -> bool {
    // Keyword
    if (*lexer).lookahead == ':' as i32 {
        return !check_keyword_end(lexer);
    }
    while is_inline_whitespace((*lexer).lookahead) {
        advance(lexer);
    }
    // Operator identifier with arity
    if (*lexer).lookahead == '/' as i32 {
        advance(lexer);
        while is_whitespace((*lexer).lookahead) {
            advance(lexer);
        }
        if is_digit((*lexer).lookahead) {
            return false;
        }
    }
    true
}

const TOKEN_TERMINATORS: &[u8] = b"@.+-^-*/<>|~=&\\%{}[]()\"',;#";

fn is_token_end(c: i32) -> bool {
    for &t in TOKEN_TERMINATORS {
        if c == t as i32 {
            return true;
        }
    }
    is_whitespace(c)
}

struct QuotedContentInfo {
    token_type: usize,
    supports_interpol: bool,
    end_delimiter: i32,
    delimiter_length: u8,
}

const fn q(token_type: usize, supports_interpol: bool, end: u8, len: u8) -> QuotedContentInfo {
    QuotedContentInfo {
        token_type,
        supports_interpol,
        end_delimiter: end as i32,
        delimiter_length: len,
    }
}

const QUOTED_CONTENT_INFOS: [QuotedContentInfo; 20] = [
    q(0, true, b'\'', 1),
    q(1, true, b'"', 1),
    q(2, true, b'\'', 3),
    q(3, true, b'"', 3),
    q(4, true, b')', 1),
    q(5, true, b'}', 1),
    q(6, true, b']', 1),
    q(7, true, b'>', 1),
    q(8, true, b'|', 1),
    q(9, true, b'/', 1),
    q(10, false, b'\'', 1),
    q(11, false, b'"', 1),
    q(12, false, b'\'', 3),
    q(13, false, b'"', 3),
    q(14, false, b')', 1),
    q(15, false, b'}', 1),
    q(16, false, b']', 1),
    q(17, false, b'>', 1),
    q(18, false, b'|', 1),
    q(19, false, b'/', 1),
];

unsafe fn find_quoted_token_info(valid_symbols: *const bool) -> i8 {
    // Quoted tokens are mutually exclusive; if several are valid we are parsing arbitrary code.
    if *valid_symbols.add(QUOTED_CONTENT_I_SINGLE) && *valid_symbols.add(QUOTED_CONTENT_I_DOUBLE) {
        return -1;
    }
    for i in 0..QUOTED_CONTENT_INFOS.len() {
        if *valid_symbols.add(QUOTED_CONTENT_INFOS[i].token_type) {
            return i as i8;
        }
    }
    -1
}

unsafe fn scan_quoted_content(lexer: *mut TSLexer, info: &QuotedContentInfo) -> bool {
    (*lexer).result_symbol = info.token_type as u16;

    let is_heredoc = info.delimiter_length == 3;

    let mut has_content = false;
    loop {
        let mut newline = false;

        if is_newline((*lexer).lookahead) {
            advance(lexer);

            has_content = true;
            newline = true;

            while is_whitespace((*lexer).lookahead) {
                advance(lexer);
            }
        }

        ((*lexer).mark_end)(lexer);

        if (*lexer).lookahead == info.end_delimiter {
            let mut length: u8 = 1;

            while length < info.delimiter_length {
                advance(lexer);
                if (*lexer).lookahead == info.end_delimiter {
                    length += 1;
                } else {
                    break;
                }
            }

            if length == info.delimiter_length && (!is_heredoc || newline) {
                return has_content;
            }
        } else if (*lexer).lookahead == '#' as i32 {
            advance(lexer);
            if info.supports_interpol && (*lexer).lookahead == '{' as i32 {
                return has_content;
            }
        } else if (*lexer).lookahead == '\\' as i32 {
            advance(lexer);
            if is_heredoc && (*lexer).lookahead == '\n' as i32 {
                // Intentionally ignore the escaping: the newline is needed to recognise the
                // heredoc end delimiter.
            } else if info.supports_interpol || (*lexer).lookahead == info.end_delimiter {
                return has_content;
            }
        } else if (*lexer).lookahead == 0 {
            // End of file: no end delimiter, treat everything scanned as content.
            return has_content;
        } else {
            advance(lexer);
        }

        has_content = true;
    }
}

unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}

unsafe fn scan_newline(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    advance(lexer);

    while is_whitespace(la(lexer)) {
        advance(lexer);
    }

    // The whitespace after the newline is included so the parser need not go through it again.
    ((*lexer).mark_end)(lexer);

    if la(lexer) == '#' as i32 {
        (*lexer).result_symbol = NEWLINE_BEFORE_COMMENT as u16;
        return true;
    }

    if la(lexer) == 'd' as i32 && *valid_symbols.add(NEWLINE_BEFORE_DO) {
        (*lexer).result_symbol = NEWLINE_BEFORE_DO as u16;
        advance(lexer);
        if la(lexer) == 'o' as i32 {
            advance(lexer);
            return is_token_end(la(lexer));
        }
        return false;
    }

    if *valid_symbols.add(NEWLINE_BEFORE_BINARY_OPERATOR) {
        (*lexer).result_symbol = NEWLINE_BEFORE_BINARY_OPERATOR as u16;
        let c = la(lexer);
        let is = |ch: char| c == ch as i32;

        if is('&') {
            // &&, &&&
            advance(lexer);
            if is_c(lexer, '&') {
                advance(lexer);
                if is_c(lexer, '&') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            }
        } else if is('=') {
            // =, ==, ===, =~, =>
            advance(lexer);
            if is_c(lexer, '=') {
                advance(lexer);
                if is_c(lexer, '=') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            } else if is_c(lexer, '~') {
                advance(lexer);
                return check_operator_end(lexer);
            } else if is_c(lexer, '>') {
                advance(lexer);
                return check_operator_end(lexer);
            } else {
                return check_operator_end(lexer);
            }
        } else if is(':') {
            // ::
            advance(lexer);
            if is_c(lexer, ':') {
                advance(lexer);
                // Ignore ::: atom
                if is_c(lexer, ':') {
                    return false;
                }
                return check_operator_end(lexer);
            }
        } else if is('+') {
            // ++, +++
            advance(lexer);
            if is_c(lexer, '+') {
                advance(lexer);
                if is_c(lexer, '+') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            }
        } else if is('-') {
            // --, ---, ->
            advance(lexer);
            if is_c(lexer, '-') {
                advance(lexer);
                if is_c(lexer, '-') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            } else if is_c(lexer, '>') {
                advance(lexer);
                return check_operator_end(lexer);
            }
        } else if is('<') {
            // <, <=, <-, <>, <~, <~>, <|>, <<<, <<~
            advance(lexer);
            if is_c(lexer, '=') || is_c(lexer, '-') || is_c(lexer, '>') {
                advance(lexer);
                return check_operator_end(lexer);
            } else if is_c(lexer, '~') {
                advance(lexer);
                if is_c(lexer, '>') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            } else if is_c(lexer, '|') {
                advance(lexer);
                if is_c(lexer, '>') {
                    advance(lexer);
                    return check_operator_end(lexer);
                }
            } else if is_c(lexer, '<') {
                advance(lexer);
                if is_c(lexer, '<') || is_c(lexer, '~') {
                    advance(lexer);
                    return check_operator_end(lexer);
                }
            } else {
                return check_operator_end(lexer);
            }
        } else if is('>') {
            // >, >=, >>>
            advance(lexer);
            if is_c(lexer, '=') {
                advance(lexer);
                return check_operator_end(lexer);
            } else if is_c(lexer, '>') {
                advance(lexer);
                if is_c(lexer, '>') {
                    advance(lexer);
                    return check_operator_end(lexer);
                }
            } else {
                return check_operator_end(lexer);
            }
        } else if is('^') {
            // ^^^
            advance(lexer);
            if is_c(lexer, '^') {
                advance(lexer);
                if is_c(lexer, '^') {
                    advance(lexer);
                    return check_operator_end(lexer);
                }
            }
        } else if is('!') {
            // !=, !==
            advance(lexer);
            if is_c(lexer, '=') {
                advance(lexer);
                if is_c(lexer, '=') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            }
        } else if is('~') {
            // ~>, ~>>
            advance(lexer);
            if is_c(lexer, '>') {
                advance(lexer);
                if is_c(lexer, '>') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            }
        } else if is('|') {
            // |, ||, |||, |>
            advance(lexer);
            if is_c(lexer, '|') {
                advance(lexer);
                if is_c(lexer, '|') {
                    advance(lexer);
                    return check_operator_end(lexer);
                } else {
                    return check_operator_end(lexer);
                }
            } else if is_c(lexer, '>') {
                advance(lexer);
                return check_operator_end(lexer);
            } else {
                return check_operator_end(lexer);
            }
        } else if is('*') {
            // *, **
            advance(lexer);
            if is_c(lexer, '*') {
                advance(lexer);
                return check_operator_end(lexer);
            } else {
                return check_operator_end(lexer);
            }
        } else if is('/') {
            // /, //
            advance(lexer);
            if is_c(lexer, '/') {
                advance(lexer);
                return check_operator_end(lexer);
            } else {
                return check_operator_end(lexer);
            }
        } else if is('.') {
            // ., ..
            advance(lexer);
            if is_c(lexer, '.') {
                advance(lexer);
                // Ignore ... identifier
                if is_c(lexer, '.') {
                    return false;
                }
                return check_operator_end(lexer);
            } else {
                return check_operator_end(lexer);
            }
        } else if is('\\') {
            // double backslash
            advance(lexer);
            if is_c(lexer, '\\') {
                advance(lexer);
                return check_operator_end(lexer);
            }
        } else if is('w') {
            // when
            advance(lexer);
            if is_c(lexer, 'h') {
                advance(lexer);
                if is_c(lexer, 'e') {
                    advance(lexer);
                    if is_c(lexer, 'n') {
                        advance(lexer);
                        return is_token_end(la(lexer)) && check_operator_end(lexer);
                    }
                }
            }
        } else if is('a') {
            // and
            advance(lexer);
            if is_c(lexer, 'n') {
                advance(lexer);
                if is_c(lexer, 'd') {
                    advance(lexer);
                    return is_token_end(la(lexer)) && check_operator_end(lexer);
                }
            }
        } else if is('o') {
            // or
            advance(lexer);
            if is_c(lexer, 'r') {
                advance(lexer);
                return is_token_end(la(lexer)) && check_operator_end(lexer);
            }
        } else if is('i') {
            // in
            advance(lexer);
            if is_c(lexer, 'n') {
                advance(lexer);
                return is_token_end(la(lexer)) && check_operator_end(lexer);
            }
        } else if is('n') {
            // not in
            advance(lexer);
            if is_c(lexer, 'o') {
                advance(lexer);
                if is_c(lexer, 't') {
                    advance(lexer);
                    while is_inline_whitespace(la(lexer)) {
                        advance(lexer);
                    }
                    if is_c(lexer, 'i') {
                        advance(lexer);
                        if is_c(lexer, 'n') {
                            advance(lexer);
                            return is_token_end(la(lexer)) && check_operator_end(lexer);
                        }
                    }
                }
            }
        }
    }

    false
}

unsafe fn is_c(lexer: *mut TSLexer, ch: char) -> bool {
    la(lexer) == ch as i32
}

unsafe fn scan(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let quoted_content_info_idx = find_quoted_token_info(valid_symbols);

    // Quoted content matches any character except close delimiters, escapes and interpolations.
    if quoted_content_info_idx != -1 {
        let info = &QUOTED_CONTENT_INFOS[quoted_content_info_idx as usize];
        return scan_quoted_content(lexer, info);
    }

    let mut skipped_whitespace = false;

    while is_inline_whitespace(la(lexer)) {
        skipped_whitespace = true;
        skip(lexer);
    }

    // Newline: either tokenized as a special newline or ignored.
    if is_newline(la(lexer))
        && (*valid_symbols.add(NEWLINE_BEFORE_DO)
            || *valid_symbols.add(NEWLINE_BEFORE_BINARY_OPERATOR)
            || *valid_symbols.add(NEWLINE_BEFORE_COMMENT))
    {
        return scan_newline(lexer, valid_symbols);
    }

    if is_c(lexer, '+') {
        // before unary +
        if skipped_whitespace && *valid_symbols.add(BEFORE_UNARY_OPERATOR) {
            ((*lexer).mark_end)(lexer);
            advance(lexer);
            if is_c(lexer, '+') || is_c(lexer, ':') || is_c(lexer, '/') {
                return false;
            }
            if is_whitespace(la(lexer)) {
                return false;
            }
            (*lexer).result_symbol = BEFORE_UNARY_OPERATOR as u16;
            return true;
        }
    } else if is_c(lexer, '-') {
        // before unary -
        if skipped_whitespace && *valid_symbols.add(BEFORE_UNARY_OPERATOR) {
            ((*lexer).mark_end)(lexer);
            (*lexer).result_symbol = BEFORE_UNARY_OPERATOR as u16;
            advance(lexer);
            if is_c(lexer, '-') || is_c(lexer, '>') || is_c(lexer, ':') || is_c(lexer, '/') {
                return false;
            }
            if is_whitespace(la(lexer)) {
                return false;
            }
            return true;
        }
    } else if is_c(lexer, 'n') {
        // not in
        if *valid_symbols.add(NOT_IN) {
            (*lexer).result_symbol = NOT_IN as u16;
            advance(lexer);
            if is_c(lexer, 'o') {
                advance(lexer);
                if is_c(lexer, 't') {
                    advance(lexer);
                    while is_inline_whitespace(la(lexer)) {
                        advance(lexer);
                    }
                    if is_c(lexer, 'i') {
                        advance(lexer);
                        if is_c(lexer, 'n') {
                            advance(lexer);
                            return is_token_end(la(lexer));
                        }
                    }
                }
            }
        }
    } else if is_c(lexer, ':') {
        // quoted atom start
        if *valid_symbols.add(QUOTED_ATOM_START) {
            advance(lexer);
            ((*lexer).mark_end)(lexer);
            (*lexer).result_symbol = QUOTED_ATOM_START as u16;
            if is_c(lexer, '"') || is_c(lexer, '\'') {
                return true;
            }
        }
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_elixir_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_elixir_external_scanner_destroy(_payload: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_elixir_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(lexer, valid_symbols)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_elixir_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_elixir_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}
