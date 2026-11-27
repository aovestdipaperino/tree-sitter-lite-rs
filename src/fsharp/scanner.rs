//! Rust port of tree-sitter-fsharp's external scanner (common/scanner.h, included by
//! fsharp/src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn, unused_assignments)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// enum TokenType
const NEWLINE: usize = 0;
const INDENT: usize = 1;
const DEDENT: usize = 2;
const THEN: usize = 3;
const ELSE: usize = 4;
const ELIF: usize = 5;
const PREPROC_IF: usize = 6;
const PREPROC_ELSE: usize = 7;
const PREPROC_END: usize = 8;
const CLASS: usize = 9;
const BEGIN: usize = 10;
const STRUCT: usize = 11;
const INTERFACE: usize = 12;
const END: usize = 13;
const AND: usize = 14;
const WITH: usize = 15;
const TRIPLE_QUOTE_CONTENT: usize = 16;
const FORMAT_TRIPLE_QUOTE_CONTENT: usize = 17;
const BLOCK_COMMENT_CONTENT: usize = 18;
const INSIDE_STRING: usize = 19;
const NEWLINE_NO_ALIGNED: usize = 20;
const TUPLE_MARKER: usize = 21;
const QUOTED_CLOSE: usize = 22;
const UNTYPED_QUOTED_CLOSE: usize = 23;
const MULTI_DOLLAR_TRIPLE_QUOTE_START: usize = 24;
const MULTI_DOLLAR_TRIPLE_QUOTED_CONTENT: usize = 25;
const MULTI_DOLLAR_INTERP_START: usize = 26;
const MULTI_DOLLAR_INTERP_END: usize = 27;
const MULTI_DOLLAR_TRIPLE_QUOTE_END: usize = 28;
const TYAPP_OPEN: usize = 29;
const PAREN_INDENT: usize = 30;
const TYPE_DECL_NEWLINE: usize = 31;
const IN: usize = 32;
const ERROR_SENTINEL: usize = 33;

const TREE_SITTER_SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// The three `Array(...)` fields of the C `Scanner` are `Vec`s here. `indents` and `paren_indents`
/// always have the same length (`push_indent`/`pop_indent`/`deserialize` keep them in step).
#[derive(Default)]
struct Scanner {
    indents: Vec<u16>,
    paren_indents: Vec<bool>,
    preprocessor_indents: Vec<u16>,
    multi_dollar_count: u8,
}

/// A character constant as the lexer's `int32_t` lookahead.
const fn c(b: u8) -> i32 {
    b as i32
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

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}

fn is_word_char(ch: i32) -> bool {
    (ch >= c(b'a') && ch <= c(b'z'))
        || (ch >= c(b'A') && ch <= c(b'Z'))
        || (ch >= c(b'0') && ch <= c(b'9'))
        || ch == c(b'_')
        || ch == c(b'\'')
}

fn push_indent(scanner: &mut Scanner, indent_length: u16, is_paren_indent: bool) {
    scanner.indents.push(indent_length);
    scanner.paren_indents.push(is_paren_indent);
}

fn pop_indent(scanner: &mut Scanner) {
    if !scanner.indents.is_empty() {
        scanner.indents.pop();
    }
    if !scanner.paren_indents.is_empty() {
        scanner.paren_indents.pop();
    }
}

/// `*array_back(&indents)`; every caller checks `indents.size > 0` first (the C reads out of
/// bounds otherwise), so an empty array cannot reach here.
fn peek_indent_length(scanner: &Scanner) -> u16 {
    *scanner
        .indents
        .last()
        .expect("peek_indent_length on empty stack")
}

fn peek_is_paren_indent(scanner: &Scanner) -> bool {
    !scanner.paren_indents.is_empty() && *scanner.paren_indents.last().unwrap()
}

unsafe fn is_type_application_open(lexer: *mut TSLexer) -> bool {
    let mut angle_depth: i32 = 1;
    let mut paren_depth: i32 = 0;

    while !eof(lexer) && angle_depth > 0 {
        let ch = la(lexer);

        if ch == c(b'\n') || ch == c(b'\r') {
            return false;
        }

        if is_word_char(ch)
            || ch == c(b' ')
            || ch == c(b'\t')
            || ch == c(b',')
            || ch == c(b'*')
            || ch == c(b'.')
            || ch == c(b':')
            || ch == c(b'#')
            || ch == c(b'^')
            || ch == c(b'/')
            || ch == c(b'|')
            || ch == c(b'{')
            || ch == c(b'}')
            || ch == c(b'[')
            || ch == c(b']')
        {
            advance(lexer);
            continue;
        }

        if ch == c(b'(') {
            paren_depth += 1;
            advance(lexer);
            continue;
        }

        if ch == c(b')') {
            if paren_depth <= 0 {
                return false;
            }
            paren_depth -= 1;
            advance(lexer);
            continue;
        }

        if ch == c(b'<') {
            angle_depth += 1;
            advance(lexer);
            continue;
        }

        if ch == c(b'>') {
            angle_depth -= 1;
            if angle_depth == 0 {
                return true;
            }
            advance(lexer);
            continue;
        }

        if ch == c(b'-') {
            advance(lexer);
            if la(lexer) == c(b'>') {
                advance(lexer);
                continue;
            }
            return false;
        }

        return false;
    }

    false
}

unsafe fn scan_n_chars(lexer: *mut TSLexer, ch: u8, count: u8) -> bool {
    mark_end(lexer);
    for _ in 0..count {
        if la(lexer) != c(ch) {
            return false;
        }
        advance(lexer);
    }
    mark_end(lexer);
    true
}

unsafe fn scan_block_comment(lexer: *mut TSLexer) -> bool {
    mark_end(lexer);
    if la(lexer) != c(b'(') {
        return false;
    }

    advance(lexer);
    if la(lexer) != c(b'*') {
        return false;
    }

    advance(lexer);

    loop {
        match la(lexer) {
            x if x == c(b'(') => {
                scan_block_comment(lexer);
            }
            x if x == c(b'*') => {
                advance(lexer);
                if la(lexer) == c(b')') {
                    advance(lexer);
                    return true;
                }
            }
            0 => return true,
            _ => advance(lexer),
        }
    }
}

unsafe fn is_infix_op_start(lexer: *mut TSLexer) -> bool {
    let x = la(lexer);
    if x == c(b'+') || x == c(b'-') {
        skip(lexer);
        !(la(lexer) >= c(b'0') && la(lexer) <= c(b'9'))
    } else if x == c(b'*')
        || x == c(b'%')
        || x == c(b'&')
        || x == c(b'=')
        || x == c(b'?')
        || x == c(b'<')
        || x == c(b'>')
        || x == c(b'^')
    {
        true
    } else if x == c(b'/') {
        skip(lexer);
        la(lexer) != c(b'/')
    } else if x == c(b'.') {
        skip(lexer);
        la(lexer) != c(b'.')
    } else if x == c(b'!') {
        skip(lexer);
        la(lexer) == c(b'=')
    } else if x == c(b':') {
        skip(lexer);
        la(lexer) == c(b'=')
            || la(lexer) == c(b':')
            || la(lexer) == c(b'?')
            || la(lexer) == c(b' ')
            || la(lexer) == c(b'>')
    } else if x == c(b'o') {
        skip(lexer);
        la(lexer) == c(b'r')
    } else if x == c(b'@') || x == c(b'$') {
        skip(lexer);
        la(lexer) != c(b'"')
    } else {
        false
    }
}

unsafe fn is_bracket_end(lexer: *mut TSLexer) -> bool {
    let x = la(lexer);
    x == c(b')') || x == c(b']') || x == c(b'}')
}

/// Match the remaining characters of a keyword after its first character; true if they match and
/// a non-word character follows.
unsafe fn match_keyword_rest(lexer: *mut TSLexer, rest: &[u8]) -> bool {
    for &r in rest {
        if la(lexer) != c(r) {
            return false;
        }
        advance(lexer);
    }
    !is_word_char(la(lexer))
}

struct BlockOpener {
    first_char: u8,
    rest: &'static [u8],
    token: usize,
}

const BLOCK_OPENERS: [BlockOpener; 4] = [
    BlockOpener {
        first_char: b'c',
        rest: b"lass",
        token: CLASS,
    },
    BlockOpener {
        first_char: b'b',
        rest: b"egin",
        token: BEGIN,
    },
    BlockOpener {
        first_char: b's',
        rest: b"truct",
        token: STRUCT,
    },
    BlockOpener {
        first_char: b'i',
        rest: b"nterface",
        token: INTERFACE,
    },
];

unsafe fn try_dedent_for_preproc(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    if !scanner.indents.is_empty() && !scanner.preprocessor_indents.is_empty() {
        let current_indent_length = peek_indent_length(scanner);
        let current_preproc_length = *scanner.preprocessor_indents.last().unwrap();
        if current_preproc_length < current_indent_length {
            pop_indent(scanner);
            (*lexer).result_symbol = DEDENT as u16;
            return true;
        }
    }
    false
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);

    if valid(ERROR_SENTINEL) {
        if eof(lexer) && scanner.indents.len() > 1 {
            pop_indent(scanner);
            (*lexer).result_symbol = DEDENT as u16;
            return true;
        }
    }

    if valid(INSIDE_STRING) && !valid(ERROR_SENTINEL) {
        return false;
    }

    if valid(TYAPP_OPEN) && !valid(ERROR_SENTINEL) && la(lexer) == c(b'<') {
        mark_end(lexer);
        advance(lexer);
        mark_end(lexer);
        if is_type_application_open(lexer) {
            (*lexer).result_symbol = TYAPP_OPEN as u16;
            return true;
        }
        return false;
    }

    if !valid(ERROR_SENTINEL) && scanner.multi_dollar_count > 1 {
        if valid(MULTI_DOLLAR_INTERP_START) && la(lexer) == c(b'{') {
            if scan_n_chars(lexer, b'{', scanner.multi_dollar_count) {
                (*lexer).result_symbol = MULTI_DOLLAR_INTERP_START as u16;
                return true;
            }
        }

        if valid(MULTI_DOLLAR_INTERP_END) && la(lexer) == c(b'}') {
            if scan_n_chars(lexer, b'}', scanner.multi_dollar_count) {
                (*lexer).result_symbol = MULTI_DOLLAR_INTERP_END as u16;
                return true;
            }
        }

        if valid(MULTI_DOLLAR_TRIPLE_QUOTE_END) && la(lexer) == c(b'"') {
            if scan_n_chars(lexer, b'"', 3) {
                scanner.multi_dollar_count = 0;
                (*lexer).result_symbol = MULTI_DOLLAR_TRIPLE_QUOTE_END as u16;
                return true;
            }
        }
    }

    if !valid(ERROR_SENTINEL)
        && (valid(TRIPLE_QUOTE_CONTENT)
            || valid(FORMAT_TRIPLE_QUOTE_CONTENT)
            || valid(MULTI_DOLLAR_TRIPLE_QUOTED_CONTENT))
    {
        let is_format = valid(FORMAT_TRIPLE_QUOTE_CONTENT);
        let is_multi_dollar = valid(MULTI_DOLLAR_TRIPLE_QUOTED_CONTENT);
        let mut has_content = false;
        mark_end(lexer);
        loop {
            if la(lexer) == 0 {
                break;
            }
            if (is_format || is_multi_dollar) && la(lexer) == c(b'{') {
                let brace_count: u8 = if is_multi_dollar {
                    scanner.multi_dollar_count
                } else {
                    1
                };
                mark_end(lexer);

                if !is_multi_dollar {
                    advance(lexer);
                    if la(lexer) == c(b'{') {
                        advance(lexer);
                        mark_end(lexer);
                        has_content = true;
                        continue;
                    }
                    if !has_content {
                        return false;
                    }
                    break;
                }

                let mut matches_interp_start = true;
                for _ in 0..brace_count {
                    if la(lexer) != c(b'{') {
                        matches_interp_start = false;
                        break;
                    }
                    advance(lexer);
                }
                if matches_interp_start {
                    if !has_content {
                        return false;
                    }
                    break;
                }
                has_content = true;
                mark_end(lexer);
                continue;
            }
            if la(lexer) != c(b'"') {
                advance(lexer);
                has_content = true;
            } else {
                if is_multi_dollar {
                    advance(lexer);
                    if la(lexer) == c(b'"') {
                        advance(lexer);
                        if la(lexer) == c(b'"') {
                            break;
                        }
                    }
                    has_content = true;
                    mark_end(lexer);
                } else {
                    mark_end(lexer);
                    skip(lexer);
                    if la(lexer) == c(b'"') {
                        skip(lexer);
                        if la(lexer) == c(b'"') {
                            skip(lexer);
                            break;
                        }
                    }
                }
                has_content = true;
                mark_end(lexer);
            }
        }
        if is_multi_dollar {
            (*lexer).result_symbol = MULTI_DOLLAR_TRIPLE_QUOTED_CONTENT as u16;
        } else {
            (*lexer).result_symbol = if is_format {
                FORMAT_TRIPLE_QUOTE_CONTENT
            } else {
                TRIPLE_QUOTE_CONTENT
            } as u16;
        }
        return true;
    }

    if valid(TYPE_DECL_NEWLINE) {
        if eof(lexer) {
            (*lexer).result_symbol = TYPE_DECL_NEWLINE as u16;
            return true;
        }
        if la(lexer) == c(b'\n') || la(lexer) == c(b'\r') {
            let mut next_indent: u32 = 0;
            loop {
                if la(lexer) == c(b'\n') || la(lexer) == c(b'\r') {
                    next_indent = 0;
                    skip(lexer);
                } else if la(lexer) == c(b' ') {
                    next_indent = next_indent.wrapping_add(1);
                    skip(lexer);
                } else if la(lexer) == c(b'\t') {
                    next_indent = next_indent.wrapping_add(8);
                    skip(lexer);
                } else {
                    break;
                }
            }
            if eof(lexer) {
                (*lexer).result_symbol = TYPE_DECL_NEWLINE as u16;
                return true;
            }
            let scope_indent: u32 = if !scanner.indents.is_empty() {
                *scanner.indents.last().unwrap() as u32
            } else {
                0
            };
            if next_indent <= scope_indent {
                (*lexer).result_symbol = TYPE_DECL_NEWLINE as u16;
                return true;
            }
            return false;
        }
    }

    mark_end(lexer);

    let mut found_end_of_line = false;
    let mut found_end_of_line_semi_colon = false;
    let mut found_start_of_infix_op = false;
    let mut found_same_line_pipe_infix = false;
    let mut found_bracket_end = false;
    let mut found_preprocessor_end = false;
    let mut found_preproc_if = false;
    let mut found_comment_start = false;
    let mut indent_length: u32 = ((*lexer).get_column)(lexer);

    loop {
        if la(lexer) == c(b'\n') {
            found_end_of_line = true;
            indent_length = 0;
            skip(lexer);
        } else if la(lexer) == c(b' ') {
            indent_length = indent_length.wrapping_add(1);
            skip(lexer);
        } else if la(lexer) == c(b'\r') || la(lexer) == 0x0c {
            indent_length = 0;
            skip(lexer);
        } else if la(lexer) == c(b'\t') {
            indent_length = indent_length.wrapping_add(8);
            skip(lexer);
        } else if eof(lexer) {
            found_end_of_line = true;
            indent_length = 0;
            break;
        } else if la(lexer) == c(b'/') {
            skip(lexer);
            if !valid(INSIDE_STRING) && la(lexer) == c(b'/') {
                if !found_preproc_if {
                    return false;
                }
                while la(lexer) != c(b'\n') && !eof(lexer) {
                    skip(lexer);
                }
            } else {
                return false;
            }
        } else if la(lexer) == c(b'#') {
            advance(lexer);
            if la(lexer) == c(b'e') {
                advance(lexer);
                if la(lexer) == c(b'n') {
                    advance(lexer);
                    if la(lexer) == c(b'd') {
                        advance(lexer);
                        if la(lexer) == c(b'i') {
                            advance(lexer);
                            if la(lexer) == c(b'f') {
                                advance(lexer);
                                found_preprocessor_end = true;
                                if try_dedent_for_preproc(scanner, lexer) {
                                    return true;
                                }
                                if valid(PREPROC_END) {
                                    if !scanner.preprocessor_indents.is_empty() {
                                        scanner.preprocessor_indents.pop();
                                    }
                                    mark_end(lexer);
                                    (*lexer).result_symbol = PREPROC_END as u16;
                                    return true;
                                }
                            }
                        }
                    }
                } else if la(lexer) == c(b'l') {
                    advance(lexer);
                    if la(lexer) == c(b's') {
                        advance(lexer);
                        if la(lexer) == c(b'e') {
                            advance(lexer);
                            if try_dedent_for_preproc(scanner, lexer) {
                                return true;
                            }
                            if valid(PREPROC_ELSE) {
                                mark_end(lexer);
                                (*lexer).result_symbol = PREPROC_ELSE as u16;
                                return true;
                            }
                        }
                    }
                }
            } else if la(lexer) == c(b'i') {
                advance(lexer);
                if la(lexer) == c(b'f') {
                    advance(lexer);
                    found_preproc_if = true;
                    if valid(NEWLINE) || valid(INDENT) {
                        while la(lexer) != c(b'\n') && !eof(lexer) {
                            skip(lexer);
                        }
                    } else {
                        if !scanner.indents.is_empty() {
                            if valid(PREPROC_IF) {
                                let current_indent_length = peek_indent_length(scanner);
                                scanner.preprocessor_indents.push(current_indent_length);
                            } else {
                                pop_indent(scanner);
                                (*lexer).result_symbol = DEDENT as u16;
                                return true;
                            }
                        } else {
                            mark_end(lexer);
                            (*lexer).result_symbol = PREPROC_IF as u16;
                            return true;
                        }
                    }
                }
            } else {
                if found_end_of_line && valid(NEWLINE_NO_ALIGNED) {
                    (*lexer).result_symbol = NEWLINE_NO_ALIGNED as u16;
                    return true;
                }
                return false;
            }
        } else {
            break;
        }
    }

    let mut failed_at_sign_match = false;
    if !valid(ERROR_SENTINEL) && la(lexer) == c(b'@') {
        mark_end(lexer);
        advance(lexer);
        if la(lexer) == c(b'@') {
            advance(lexer);
            if la(lexer) == c(b'>') {
                if valid(DEDENT) && scanner.indents.len() > 1 {
                    pop_indent(scanner);
                    (*lexer).result_symbol = DEDENT as u16;
                    return true;
                }
                advance(lexer);
                mark_end(lexer);
                (*lexer).result_symbol = UNTYPED_QUOTED_CLOSE as u16;
                return true;
            }
            failed_at_sign_match = true;
            found_start_of_infix_op = true;
        } else if la(lexer) == c(b'>') {
            if valid(DEDENT) && scanner.indents.len() > 1 {
                pop_indent(scanner);
                (*lexer).result_symbol = DEDENT as u16;
                return true;
            }
            advance(lexer);
            mark_end(lexer);
            (*lexer).result_symbol = QUOTED_CLOSE as u16;
            return true;
        } else {
            failed_at_sign_match = true;
            found_start_of_infix_op = true;
        }
    }

    if !valid(ERROR_SENTINEL) && la(lexer) == c(b'$') {
        mark_end(lexer);

        if valid(MULTI_DOLLAR_TRIPLE_QUOTE_START) {
            let mut dollar_count: u8 = 0;
            while la(lexer) == c(b'$') && dollar_count < u8::MAX {
                advance(lexer);
                dollar_count += 1;
            }
            if dollar_count > 1 && scan_n_chars(lexer, b'"', 3) {
                scanner.multi_dollar_count = dollar_count;
                (*lexer).result_symbol = MULTI_DOLLAR_TRIPLE_QUOTE_START as u16;
                return true;
            }
            if found_end_of_line && !scanner.indents.is_empty() {
                let current_indent_length = peek_indent_length(scanner);
                if valid(DEDENT) && indent_length < current_indent_length as u32 {
                    let can_dedent_paren_indent =
                        !peek_is_paren_indent(scanner) || indent_length == 0 || eof(lexer);
                    if can_dedent_paren_indent {
                        pop_indent(scanner);
                        (*lexer).result_symbol = DEDENT as u16;
                        return true;
                    }
                }
                if valid(NEWLINE)
                    && indent_length == current_indent_length as u32
                    && indent_length > 0
                {
                    (*lexer).result_symbol = NEWLINE as u16;
                    return true;
                }
            }
            return false;
        }
    }

    let mut failed_block_opener = false;
    {
        for op in BLOCK_OPENERS.iter() {
            if valid(op.token) && la(lexer) == c(op.first_char) {
                mark_end(lexer);
                indent_length = ((*lexer).get_column)(lexer);
                advance(lexer);
                if match_keyword_rest(lexer, op.rest) {
                    mark_end(lexer);
                    (*lexer).result_symbol = op.token as u16;
                    return true;
                }
                failed_block_opener = true;
                break;
            }
        }
    }

    if found_end_of_line
        && valid(NEWLINE_NO_ALIGNED)
        && !found_start_of_infix_op
        && !found_preprocessor_end
    {
        (*lexer).result_symbol = NEWLINE_NO_ALIGNED as u16;
        return true;
    }

    if !failed_block_opener && !failed_at_sign_match {
        if valid(NEWLINE) && la(lexer) == c(b';') {
            advance(lexer);
            while la(lexer) == c(b' ') || la(lexer) == c(b'\n') {
                advance(lexer);
            }
            found_end_of_line = true;
            found_end_of_line_semi_colon = true;
            mark_end(lexer);
        }

        if la(lexer) == c(b't') && (valid(THEN) || valid(DEDENT)) {
            advance(lexer);
            if la(lexer) == c(b'h') {
                advance(lexer);
                if la(lexer) == c(b'e') {
                    advance(lexer);
                    if la(lexer) == c(b'n') {
                        advance(lexer);
                        if !is_word_char(la(lexer)) {
                            if valid(THEN) {
                                mark_end(lexer);
                                (*lexer).result_symbol = THEN as u16;
                                return true;
                            } else {
                                pop_indent(scanner);
                                (*lexer).result_symbol = DEDENT as u16;
                                return true;
                            }
                        }
                    }
                }
            }
        } else if la(lexer) == c(b'a') && (valid(AND) || valid(DEDENT)) {
            advance(lexer);
            if la(lexer) == c(b'n') {
                advance(lexer);
                if la(lexer) == c(b'd') {
                    advance(lexer);
                    if la(lexer) == c(b' ') {
                        if valid(AND) {
                            mark_end(lexer);
                            (*lexer).result_symbol = AND as u16;
                            return true;
                        } else {
                            pop_indent(scanner);
                            (*lexer).result_symbol = DEDENT as u16;
                            return true;
                        }
                    }
                }
            }
        } else if la(lexer) == c(b'w') && (valid(WITH) || valid(DEDENT)) {
            advance(lexer);
            if la(lexer) == c(b'i') {
                advance(lexer);
                if la(lexer) == c(b't') {
                    advance(lexer);
                    if la(lexer) == c(b'h') {
                        advance(lexer);
                        if la(lexer) == c(b' ') {
                            if valid(WITH) {
                                mark_end(lexer);
                                (*lexer).result_symbol = WITH as u16;
                                return true;
                            } else {
                                pop_indent(scanner);
                                (*lexer).result_symbol = DEDENT as u16;
                                return true;
                            }
                        }
                    }
                }
            }
        } else if la(lexer) == c(b'i') && valid(IN) {
            advance(lexer);
            if la(lexer) == c(b'n') {
                advance(lexer);
                if !is_word_char(la(lexer)) {
                    pop_indent(scanner);
                    mark_end(lexer);
                    (*lexer).result_symbol = IN as u16;
                    return true;
                }
            }
        } else if la(lexer) == c(b'e')
            && (valid(ELSE) || valid(ELIF) || valid(END) || valid(DEDENT))
        {
            advance(lexer);
            // int16_t token_indent_level = lexer->get_column(lexer);
            let token_indent_level: i16 = ((*lexer).get_column)(lexer) as i16;
            if la(lexer) == c(b'l') {
                advance(lexer);
                if la(lexer) == c(b's') && (valid(ELSE) || valid(DEDENT)) {
                    advance(lexer);
                    if la(lexer) == c(b'e') {
                        advance(lexer);
                        if !is_word_char(la(lexer)) {
                            if valid(ELSE) {
                                if !scanner.indents.is_empty()
                                    && (token_indent_level as i32)
                                        < peek_indent_length(scanner) as i32
                                {
                                    pop_indent(scanner);
                                    (*lexer).result_symbol = DEDENT as u16;
                                    return true;
                                } else {
                                    mark_end(lexer);
                                    loop {
                                        if la(lexer) == c(b' ')
                                            || la(lexer) == c(b'\n')
                                            || la(lexer) == c(b'\r')
                                            || la(lexer) == c(b'\t')
                                        {
                                            advance(lexer);
                                        } else {
                                            break;
                                        }
                                    }
                                    if la(lexer) == c(b'i') {
                                        advance(lexer);
                                        if la(lexer) == c(b'f') {
                                            advance(lexer);
                                            if !is_word_char(la(lexer)) {
                                                mark_end(lexer);
                                                (*lexer).result_symbol = ELIF as u16;
                                                return true;
                                            }
                                        }
                                    }
                                    (*lexer).result_symbol = ELSE as u16;
                                    return true;
                                }
                            } else {
                                pop_indent(scanner);
                                (*lexer).result_symbol = DEDENT as u16;
                                return true;
                            }
                        }
                    }
                } else if la(lexer) == c(b'i') && (valid(ELIF) || valid(DEDENT)) {
                    advance(lexer);
                    if la(lexer) == c(b'f') {
                        advance(lexer);
                        if !is_word_char(la(lexer)) {
                            if valid(ELIF) {
                                if !scanner.indents.is_empty()
                                    && (token_indent_level as i32)
                                        < peek_indent_length(scanner) as i32
                                {
                                    pop_indent(scanner);
                                    (*lexer).result_symbol = DEDENT as u16;
                                    return true;
                                } else {
                                    mark_end(lexer);
                                    (*lexer).result_symbol = ELIF as u16;
                                    return true;
                                }
                            } else {
                                pop_indent(scanner);
                                (*lexer).result_symbol = DEDENT as u16;
                                return true;
                            }
                        }
                    }
                }
            } else if la(lexer) == c(b'n') && (valid(END) || valid(DEDENT)) {
                advance(lexer);
                if la(lexer) == c(b'd') {
                    advance(lexer);
                    if !is_word_char(la(lexer)) {
                        if valid(END) {
                            mark_end(lexer);
                            (*lexer).result_symbol = END as u16;
                            return true;
                        } else if valid(DEDENT) && !scanner.indents.is_empty() {
                            pop_indent(scanner);
                            (*lexer).result_symbol = DEDENT as u16;
                            return true;
                        }
                    }
                }
            }
        } else if is_bracket_end(lexer) {
            found_bracket_end = true;
        } else if is_infix_op_start(lexer) {
            found_start_of_infix_op = true;
        } else if la(lexer) == c(b'|') {
            skip(lexer);
            let x = la(lexer);
            if x == c(b']') || x == c(b'}') {
                found_bracket_end = true;
            } else if x == c(b'>') {
                found_start_of_infix_op = true;
            } else if x == c(b' ') {
                if !found_end_of_line {
                    found_start_of_infix_op = true;
                    found_same_line_pipe_infix = true;
                } else {
                    if indent_length == 0 {
                        indent_length = 1;
                    }
                    if !scanner.indents.is_empty() {
                        let current_indent_length = peek_indent_length(scanner);
                        if found_end_of_line
                            && indent_length == current_indent_length as u32
                            && indent_length > 0
                            && !found_start_of_infix_op
                            && !found_bracket_end
                        {
                            if valid(NEWLINE) && !found_preprocessor_end {
                                (*lexer).result_symbol = NEWLINE as u16;
                                return true;
                            }
                        }
                    }
                }
            } else {
                found_start_of_infix_op = true;
            }
        } else if la(lexer) == c(b'(') {
            skip(lexer);
            if la(lexer) == c(b'*') {
                found_comment_start = true;
            }
        }
    }

    if valid(NEWLINE) && found_end_of_line_semi_colon && !found_comment_start && !found_bracket_end
    {
        (*lexer).result_symbol = NEWLINE as u16;
        return true;
    }

    if valid(INDENT)
        && !valid(ERROR_SENTINEL)
        && !found_bracket_end
        && !found_preprocessor_end
        && !found_same_line_pipe_infix
    {
        // push_indent(uint16_t indent_length): truncating conversion
        push_indent(scanner, indent_length as u16, false);
        (*lexer).result_symbol = INDENT as u16;
        return true;
    }

    if valid(PAREN_INDENT)
        && !valid(ERROR_SENTINEL)
        && !found_bracket_end
        && !found_preprocessor_end
        && !found_same_line_pipe_infix
    {
        push_indent(scanner, indent_length as u16, true);
        (*lexer).result_symbol = PAREN_INDENT as u16;
        return true;
    }

    if !scanner.indents.is_empty() {
        let is_paren_indent = peek_is_paren_indent(scanner);
        let current_indent_length = peek_indent_length(scanner) as u32;

        if found_bracket_end && valid(DEDENT) {
            pop_indent(scanner);
            (*lexer).result_symbol = DEDENT as u16;
            return true;
        }

        if found_end_of_line {
            if indent_length == current_indent_length
                && indent_length > 0
                && !found_start_of_infix_op
                && !found_bracket_end
            {
                if valid(NEWLINE) && !found_preprocessor_end && !found_comment_start {
                    (*lexer).result_symbol = NEWLINE as u16;
                    return true;
                }
            }

            let can_dedent_preproc: bool;

            if !scanner.preprocessor_indents.is_empty() {
                let current_preproc_length = *scanner.preprocessor_indents.last().unwrap() as u32;
                can_dedent_preproc = current_preproc_length < indent_length;
            } else {
                can_dedent_preproc = true;
            }

            let can_dedent_infix_op: bool;

            if found_start_of_infix_op {
                can_dedent_infix_op = indent_length.wrapping_add(2) < current_indent_length;
            } else {
                can_dedent_infix_op = true;
            }

            let can_dedent_paren_indent = !is_paren_indent || indent_length == 0 || eof(lexer);

            if indent_length < current_indent_length
                && !found_bracket_end
                && can_dedent_preproc
                && can_dedent_infix_op
                && (!valid(TUPLE_MARKER) || valid(ERROR_SENTINEL))
                && can_dedent_paren_indent
            {
                pop_indent(scanner);
                (*lexer).result_symbol = DEDENT as u16;
                return true;
            }
        }
    }

    if valid(BLOCK_COMMENT_CONTENT) && !valid(ERROR_SENTINEL) {
        mark_end(lexer);
        loop {
            if la(lexer) == 0 {
                break;
            }
            if la(lexer) != c(b'(') && la(lexer) != c(b'*') {
                advance(lexer);
            } else if la(lexer) == c(b'*') {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == c(b')') {
                    break;
                }
            } else if scan_block_comment(lexer) {
                mark_end(lexer);
                advance(lexer);
                if la(lexer) == c(b'*') {
                    break;
                }
            }
        }
        (*lexer).result_symbol = BLOCK_COMMENT_CONTENT as u16;
        return true;
    }

    false
}

/// The C writes `size` bytes with guards `size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE` on every
/// loop but not on the unconditional writes (the two header bytes and the indent count). The
/// largest possible size is 2 + 255 + 1 + 255 + 32 = 545, below the 1024 buffer, so the guards
/// never fire and no write goes out of bounds.
unsafe fn serialize(scanner: &Scanner, buffer: *mut c_char) -> u32 {
    let mut size: usize = 0;
    let put = |size: &mut usize, v: u8| {
        *buffer.add(*size) = v as c_char;
        *size += 1;
    };

    put(&mut size, scanner.multi_dollar_count);

    let mut preprocessor_count = scanner.preprocessor_indents.len();
    if preprocessor_count > u8::MAX as usize {
        preprocessor_count = u8::MAX as usize;
    }

    put(&mut size, preprocessor_count as u8);

    let mut iter = 0;
    while iter < preprocessor_count && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        let e = scanner.preprocessor_indents[iter] as u8;
        put(&mut size, e);
        iter += 1;
    }

    let mut indent_count = if !scanner.indents.is_empty() {
        scanner.indents.len() - 1
    } else {
        0
    };
    if indent_count > u8::MAX as usize {
        indent_count = u8::MAX as usize;
    }
    put(&mut size, indent_count as u8);

    let mut iter: usize = 1;
    while iter <= indent_count && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        put(&mut size, scanner.indents[iter] as u8);
        iter += 1;
    }

    let paren_bytes = (indent_count + 7) / 8;
    let mut byte_index = 0;
    while byte_index < paren_bytes && size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        let mut bits: u8 = 0;
        for bit in 0..8usize {
            let indent_index = byte_index * 8 + bit + 1;
            if indent_index > indent_count {
                break;
            }
            if scanner.paren_indents[indent_index] {
                bits |= 1u8 << bit;
            }
        }
        put(&mut size, bits);
        byte_index += 1;
    }

    size as u32
}

unsafe fn deserialize(scanner: &mut Scanner, buffer: *const c_char, length: u32) {
    scanner.indents.clear();
    scanner.indents.push(0);
    scanner.paren_indents.clear();
    scanner.paren_indents.push(false);

    scanner.preprocessor_indents.clear();
    scanner.multi_dollar_count = 0;
    let length = length as usize;
    if length > 0 {
        let byte = |i: usize| *(buffer as *const u8).add(i);
        let mut size: usize = 0;
        scanner.multi_dollar_count = byte(size);
        size += 1;

        if size >= length {
            return;
        }
        let preprocessor_count = byte(size) as usize;
        size += 1;

        let mut preproc_end = size + preprocessor_count;
        if preproc_end > length {
            preproc_end = length;
        }
        while size < preproc_end {
            scanner.preprocessor_indents.push(byte(size) as u16);
            size += 1;
        }

        if size >= length {
            return;
        }
        let indent_count = byte(size) as usize;
        size += 1;
        let mut indent_bytes_end = size + indent_count;
        if indent_bytes_end > length {
            indent_bytes_end = length;
        }
        while size < indent_bytes_end {
            scanner.indents.push(byte(size) as u16);
            size += 1;
        }

        let actual_indent_count = if !scanner.indents.is_empty() {
            scanner.indents.len() - 1
        } else {
            0
        };
        let paren_bytes = (actual_indent_count + 7) / 8;
        for indent_index in 0..actual_indent_count {
            let byte_offset = indent_index / 8;
            let mut is_paren_indent = false;
            if size + byte_offset < length {
                let bits = byte(size + byte_offset);
                is_paren_indent = ((bits >> (indent_index % 8)) & 1) != 0;
            }
            scanner.paren_indents.push(is_paren_indent);
        }
        size += paren_bytes;
        let _ = size;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fsharp_external_scanner_create() -> *mut c_void {
    let mut scanner = Box::new(Scanner::default());
    deserialize(&mut scanner, core::ptr::null(), 0);
    Box::into_raw(scanner) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fsharp_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fsharp_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    serialize(&*(payload as *const Scanner), buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fsharp_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    deserialize(&mut *(payload as *mut Scanner), buffer, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fsharp_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    scan(&mut *(payload as *mut Scanner), lexer, valid_symbols)
}
