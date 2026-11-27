//! Rust port of tree-sitter-kotlin's external scanner (src/scanner.c).
//!
//! `iswspace`, `iswalpha` and `iswalnum` are the C locale (ASCII only), which is what a program
//! that never calls `setlocale` gets.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const AUTOMATIC_SEMICOLON: usize = 0;
const MULTILINE_COMMENT: usize = 1;
const STRING_START: usize = 2;
const STRING_END: usize = 3;
const STRING_CONTENT: usize = 4;
const PRIMARY_CONSTRUCTOR_KEYWORD: usize = 5;
const IMPORT_DOT: usize = 6;
const INTERPOLATION_EXPRESSION_START: usize = 7;
const INTERPOLATION_IDENTIFIER_START: usize = 8;
const BY_DELEGATION_HINT: usize = 9;

const DELIMITER_LENGTH: u32 = 3;
const TREE_SITTER_SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// `typedef char Delimiter;` (signedness is the platform's, like the C).
type Delimiter = c_char;

/// `Array(Delimiter) Stack`: entries are two bytes, `[delimiter, prefix_len]`.
type Stack = Vec<Delimiter>;

fn stack_push(stack: &mut Stack, chr: c_char, triple: bool, prefix_len: u8) {
    if stack.len() + 1 >= TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        std::process::abort();
    }
    stack.push(if triple { chr.wrapping_add(1) } else { chr });
    stack.push(prefix_len as Delimiter);
}

fn stack_pop(stack: &mut Stack) {
    if stack.len() < 2 {
        std::process::abort();
    }
    stack.truncate(stack.len() - 2);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
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

unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}

unsafe fn set_result(lexer: *mut TSLexer, sym: usize) {
    (*lexer).result_symbol = sym as u16;
}

/// `iswspace` in the C locale.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswalpha` in the C locale.
fn is_walpha(c: i32) -> bool {
    matches!(c, 65..=90 | 97..=122)
}

/// `iswalnum` in the C locale.
fn is_walnum(c: i32) -> bool {
    is_walpha(c) || matches!(c, 48..=57)
}

unsafe fn scan_string_start(lexer: *mut TSLexer, stack: &mut Stack) -> bool {
    let mut prefix_len: u8 = 0;
    while la(lexer) == b'$' as i32 {
        advance(lexer);
        if prefix_len < 255 {
            prefix_len += 1;
        }
    }
    if prefix_len == 0 {
        prefix_len = 1;
    }

    if la(lexer) != b'"' as i32 {
        return false;
    }
    advance(lexer);
    mark_end(lexer);
    let mut count = 1;
    while count < DELIMITER_LENGTH {
        if la(lexer) != b'"' as i32 {
            stack_push(stack, b'"' as c_char, false, prefix_len);
            return true;
        }
        advance(lexer);
        count += 1;
    }
    mark_end(lexer);
    stack_push(stack, b'"' as c_char, true, prefix_len);
    true
}

unsafe fn scan_string_content(
    lexer: *mut TSLexer,
    stack: &mut Stack,
    valid_symbols: *const bool,
) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    if stack.len() < 2 {
        return false;
    }
    let prefix_len = stack[stack.len() - 1] as u8;
    let raw_delim = stack[stack.len() - 2];
    let is_triple = (raw_delim & 1) != 0;
    let end_char: c_char = if is_triple {
        raw_delim.wrapping_sub(1)
    } else {
        raw_delim
    };
    // `lexer->lookahead == end_char` compares an int with a promoted (possibly negative) char.
    let end = end_char as i32;
    let mut has_content = false;
    while la(lexer) != 0 {
        if la(lexer) == b'$' as i32 {
            if has_content {
                set_result(lexer, STRING_CONTENT);
                return true;
            }
            advance(lexer);
            mark_end(lexer);
            let mut additional_dollars: u16 = 0;
            while la(lexer) == b'$' as i32 {
                advance(lexer);
                additional_dollars = additional_dollars.wrapping_add(1);
            }
            let total_dollars: u16 = 1u16.wrapping_add(additional_dollars);
            if total_dollars >= prefix_len as u16
                && (is_walpha(la(lexer)) || la(lexer) == b'_' as i32 || la(lexer) == b'{' as i32)
            {
                if total_dollars > prefix_len as u16 {
                    set_result(lexer, STRING_CONTENT);
                    return true;
                }
                if additional_dollars > 0 {
                    mark_end(lexer);
                }
                if valid(INTERPOLATION_EXPRESSION_START) && la(lexer) == b'{' as i32 {
                    advance(lexer);
                    if la(lexer) == b'}' as i32 {
                        return false;
                    }
                    mark_end(lexer);
                    set_result(lexer, INTERPOLATION_EXPRESSION_START);
                    return true;
                }
                if valid(INTERPOLATION_IDENTIFIER_START)
                    && (is_walpha(la(lexer)) || la(lexer) == b'_' as i32)
                {
                    set_result(lexer, INTERPOLATION_IDENTIFIER_START);
                    return true;
                }
                return false;
            }
            if additional_dollars > 0 {
                mark_end(lexer);
            }
            set_result(lexer, STRING_CONTENT);
            return true;
        }
        if la(lexer) == b'\\' as i32 {
            advance(lexer);
            if la(lexer) == b'$' as i32 {
                advance(lexer);
                if la(lexer) == end {
                    stack_pop(stack);
                    advance(lexer);
                    mark_end(lexer);
                    set_result(lexer, STRING_END);
                    return true;
                }
            } else if is_triple && la(lexer) == end {
                has_content = true;
                continue;
            }
        } else if la(lexer) == end {
            if is_triple {
                mark_end(lexer);
                let mut count = 1;
                while count < DELIMITER_LENGTH {
                    advance(lexer);
                    if la(lexer) != end {
                        mark_end(lexer);
                        set_result(lexer, STRING_CONTENT);
                        return true;
                    }
                    count += 1;
                }
                if has_content && la(lexer) == end {
                    set_result(lexer, STRING_CONTENT);
                    return true;
                }
                set_result(lexer, STRING_END);
                mark_end(lexer);
                while la(lexer) == end {
                    advance(lexer);
                    mark_end(lexer);
                }
                stack_pop(stack);
                return true;
            }
            if has_content {
                mark_end(lexer);
                set_result(lexer, STRING_CONTENT);
                return true;
            }
            stack_pop(stack);
            advance(lexer);
            mark_end(lexer);
            set_result(lexer, STRING_END);
            return true;
        }
        advance(lexer);
        has_content = true;
    }
    false
}

unsafe fn scan_multiline_comment(lexer: *mut TSLexer) -> bool {
    if la(lexer) != b'/' as i32 {
        return false;
    }
    advance(lexer);
    if la(lexer) != b'*' as i32 {
        return false;
    }
    advance(lexer);

    let mut after_star = false;
    let mut nesting_depth: u32 = 1;
    loop {
        match la(lexer) {
            c if c == b'*' as i32 => {
                advance(lexer);
                after_star = true;
            }
            c if c == b'/' as i32 => {
                advance(lexer);
                if after_star {
                    after_star = false;
                    nesting_depth = nesting_depth.wrapping_sub(1);
                    if nesting_depth == 0 {
                        set_result(lexer, MULTILINE_COMMENT);
                        mark_end(lexer);
                        return true;
                    }
                } else {
                    after_star = false;
                    if la(lexer) == b'*' as i32 {
                        nesting_depth = nesting_depth.wrapping_add(1);
                        advance(lexer);
                    }
                }
            }
            0 => {
                if eof(lexer) {
                    set_result(lexer, MULTILINE_COMMENT);
                    mark_end(lexer);
                    return true;
                }
                return false;
            }
            _ => {
                advance(lexer);
                after_star = false;
            }
        }
    }
}

unsafe fn scan_whitespace_and_comments(lexer: *mut TSLexer) -> bool {
    while is_wspace(la(lexer)) {
        skip(lexer);
    }
    true
}

fn is_word_char(c: i32) -> bool {
    is_walnum(c) || c == b'_' as i32
}

unsafe fn scan_for_word(lexer: *mut TSLexer, word: &[u8]) -> bool {
    skip(lexer);
    for &w in word {
        if la(lexer) != w as c_char as i32 {
            return false;
        }
        skip(lexer);
    }
    if is_word_char(la(lexer)) {
        return false;
    }
    true
}

unsafe fn check_word(lexer: *mut TSLexer, word: &[u8]) -> bool {
    for &w in word {
        if la(lexer) != w as c_char as i32 {
            return false;
        }
        skip(lexer);
    }
    !is_word_char(la(lexer))
}

unsafe fn skip_whitespace_and_comments(lexer: *mut TSLexer) -> bool {
    loop {
        while is_wspace(la(lexer)) {
            skip(lexer);
        }
        if la(lexer) != b'/' as i32 {
            return true;
        }
        skip(lexer);
        if la(lexer) == b'/' as i32 {
            skip(lexer);
            while la(lexer) != b'\n' as i32 && la(lexer) != b'\r' as i32 && !eof(lexer) {
                skip(lexer);
            }
        } else if la(lexer) == b'*' as i32 {
            skip(lexer);
            let mut depth: u32 = 1;
            while depth > 0 && !eof(lexer) {
                if la(lexer) == b'*' as i32 {
                    skip(lexer);
                    if la(lexer) == b'/' as i32 {
                        skip(lexer);
                        depth -= 1;
                    }
                } else if la(lexer) == b'/' as i32 {
                    skip(lexer);
                    if la(lexer) == b'*' as i32 {
                        skip(lexer);
                        depth += 1;
                    }
                } else {
                    skip(lexer);
                }
            }
        } else {
            return false;
        }
    }
}

unsafe fn followed_by_arrow(lexer: *mut TSLexer) -> bool {
    if !skip_whitespace_and_comments(lexer) {
        return false;
    }
    if la(lexer) != b'-' as i32 {
        return false;
    }
    skip(lexer);
    la(lexer) == b'>' as i32
}

unsafe fn check_modifier_then_constructor(lexer: *mut TSLexer) -> bool {
    let mut word = [0u8; 20];
    let mut len = 0usize;
    while is_word_char(la(lexer)) && len < 19 {
        word[len] = la(lexer) as u8;
        len += 1;
        skip(lexer);
    }
    let w = &word[..len];
    if w != b"public" && w != b"private" && w != b"protected" && w != b"internal" {
        return false;
    }

    while la(lexer) == b' ' as i32 || la(lexer) == b'\t' as i32 {
        skip(lexer);
    }

    check_word(lexer, b"constructor")
}

unsafe fn check_annotation_then_constructor(lexer: *mut TSLexer) -> bool {
    while la(lexer) == b'@' as i32 {
        skip(lexer);
        if !is_word_char(la(lexer)) {
            return false;
        }
        while is_word_char(la(lexer)) {
            skip(lexer);
        }
        while la(lexer) == b'.' as i32 {
            skip(lexer);
            if !is_word_char(la(lexer)) {
                break;
            }
            while is_word_char(la(lexer)) {
                skip(lexer);
            }
        }
        if la(lexer) == b'(' as i32 {
            let mut depth: u32 = 1;
            skip(lexer);
            while depth > 0 && la(lexer) != 0 && !eof(lexer) {
                if la(lexer) == b'"' as i32 {
                    skip(lexer);
                    while la(lexer) != b'"' as i32 && la(lexer) != 0 && !eof(lexer) {
                        if la(lexer) == b'\\' as i32 {
                            skip(lexer);
                        }
                        skip(lexer);
                    }
                    if la(lexer) == b'"' as i32 {
                        skip(lexer);
                    }
                } else {
                    if la(lexer) == b'(' as i32 {
                        depth += 1;
                    } else if la(lexer) == b')' as i32 {
                        depth -= 1;
                    }
                    skip(lexer);
                }
            }
        }
        while is_wspace(la(lexer)) {
            skip(lexer);
        }
    }
    if is_word_char(la(lexer)) && la(lexer) != b'c' as i32 {
        return check_modifier_then_constructor(lexer);
    }
    check_word(lexer, b"constructor")
}

/// The `constructor` keyword scan shared by the `case 'c'` arm and the main scan: advances while
/// matching, returns whether all 11 characters matched.
unsafe fn advance_over_constructor(lexer: *mut TSLexer) -> bool {
    let kw = b"constructor";
    let mut matched = true;
    for &k in kw.iter() {
        if la(lexer) != k as i32 {
            matched = false;
            break;
        }
        advance(lexer);
    }
    matched
}

unsafe fn scan_automatic_semicolon(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    let valid = |i: usize| *valid_symbols.add(i);
    set_result(lexer, AUTOMATIC_SEMICOLON);
    mark_end(lexer);

    let mut sameline = true;
    loop {
        if eof(lexer) {
            return true;
        }

        if la(lexer) == b';' as i32 {
            advance(lexer);
            mark_end(lexer);
            return true;
        }

        if !is_wspace(la(lexer)) {
            break;
        }

        if la(lexer) == b'\n' as i32 {
            skip(lexer);
            sameline = false;
            break;
        }

        if la(lexer) == b'\r' as i32 {
            skip(lexer);

            if la(lexer) == b'\n' as i32 {
                skip(lexer);
            }

            sameline = false;
            break;
        }

        skip(lexer);
    }

    if !scan_whitespace_and_comments(lexer) {
        return false;
    }

    if sameline {
        return match la(lexer) {
            c if c == b'i' as i32 => scan_for_word(lexer, b"mport"),
            c if c == b';' as i32 => {
                advance(lexer);
                mark_end(lexer);
                true
            }
            _ => false,
        };
    }

    let c = la(lexer);
    let ch = if (0..128).contains(&c) { c as u8 } else { 0xff };
    match ch {
        b',' | b'.' | b':' | b'*' | b'%' | b'>' | b'<' | b'=' | b'{' | b'[' | b'(' | b'?'
        | b'|' | b'&' => false,

        b'/' => {
            advance(lexer);
            if la(lexer) == b'/' as i32 {
                skip(lexer);
                while la(lexer) != b'\n' as i32
                    && la(lexer) != b'\r' as i32
                    && la(lexer) != 0
                    && !eof(lexer)
                {
                    skip(lexer);
                }
                if !skip_whitespace_and_comments(lexer) {
                    return false;
                }
                let c2 = la(lexer);
                let ch2 = if (0..128).contains(&c2) {
                    c2 as u8
                } else {
                    0xff
                };
                match ch2 {
                    b'.' | b',' | b':' | b'*' | b'%' | b'>' | b'<' | b'=' | b'{' | b'[' | b'('
                    | b'?' | b'|' | b'&' | b'/' => false,
                    b'!' => {
                        skip(lexer);
                        if la(lexer) == b'=' as i32 {
                            return false;
                        }
                        true
                    }
                    b'e' => {
                        if scan_for_word(lexer, b"lse") {
                            if followed_by_arrow(lexer) {
                                return true;
                            }
                            return false;
                        }
                        true
                    }
                    b'a' => {
                        if scan_for_word(lexer, b"s") {
                            return false;
                        }
                        true
                    }
                    b'w' => {
                        if scan_for_word(lexer, b"here") {
                            return false;
                        }
                        true
                    }
                    b'c' => {
                        if scan_for_word(lexer, b"atch") {
                            return false;
                        }
                        true
                    }
                    b'b' => {
                        if valid(BY_DELEGATION_HINT) && scan_for_word(lexer, b"y") {
                            return false;
                        }
                        true
                    }
                    b'f' => {
                        if scan_for_word(lexer, b"inally") {
                            return false;
                        }
                        true
                    }
                    _ => true,
                }
            } else if la(lexer) == b'*' as i32 {
                advance(lexer);
                let mut nesting_depth: u32 = 1;
                let mut after_star = false;
                while nesting_depth > 0 && !eof(lexer) {
                    match la(lexer) {
                        c if c == b'*' as i32 => {
                            advance(lexer);
                            after_star = true;
                        }
                        c if c == b'/' as i32 => {
                            advance(lexer);
                            if after_star {
                                after_star = false;
                                nesting_depth -= 1;
                            } else {
                                if la(lexer) == b'*' as i32 {
                                    nesting_depth += 1;
                                    advance(lexer);
                                }
                                after_star = false;
                            }
                        }
                        0 => {
                            if eof(lexer) {
                                set_result(lexer, MULTILINE_COMMENT);
                                mark_end(lexer);
                                return true;
                            }
                            // fallthrough
                            advance(lexer);
                            after_star = false;
                        }
                        _ => {
                            advance(lexer);
                            after_star = false;
                        }
                    }
                }
                while is_wspace(la(lexer)) {
                    skip(lexer);
                }
                let c3 = la(lexer);
                let ch3 = if (0..128).contains(&c3) {
                    c3 as u8
                } else {
                    0xff
                };
                match ch3 {
                    b'.' | b',' | b':' | b'%' | b'>' | b'<' | b'=' | b'{' | b'[' | b'(' | b'?'
                    | b'|' | b'&' | b'/' | b'*' => {
                        mark_end(lexer);
                        set_result(lexer, MULTILINE_COMMENT);
                        true
                    }
                    b'!' => {
                        mark_end(lexer);
                        skip(lexer);
                        if la(lexer) == b'=' as i32 {
                            set_result(lexer, MULTILINE_COMMENT);
                            return true;
                        }
                        true
                    }
                    b'e' => {
                        mark_end(lexer);
                        if scan_for_word(lexer, b"lse") {
                            if followed_by_arrow(lexer) {
                                return true;
                            }
                            set_result(lexer, MULTILINE_COMMENT);
                            return true;
                        }
                        true
                    }
                    b'a' => {
                        mark_end(lexer);
                        if scan_for_word(lexer, b"s") {
                            set_result(lexer, MULTILINE_COMMENT);
                            return true;
                        }
                        true
                    }
                    b'w' => {
                        mark_end(lexer);
                        if scan_for_word(lexer, b"here") {
                            set_result(lexer, MULTILINE_COMMENT);
                            return true;
                        }
                        true
                    }
                    b'b' => {
                        if valid(BY_DELEGATION_HINT) {
                            mark_end(lexer);
                            if scan_for_word(lexer, b"y") {
                                set_result(lexer, MULTILINE_COMMENT);
                                return true;
                            }
                        }
                        true
                    }
                    _ => true,
                }
            } else {
                false
            }
        }

        b'+' | b'-' => true,

        b'!' => {
            skip(lexer);
            la(lexer) != b'=' as i32
        }

        b'b' => !(valid(BY_DELEGATION_HINT) && scan_for_word(lexer, b"y")),

        b'e' => {
            if !scan_for_word(lexer, b"lse") {
                return true;
            }
            followed_by_arrow(lexer)
        }

        b'a' => !scan_for_word(lexer, b"s"),

        b'w' => !scan_for_word(lexer, b"here"),

        b'i' => {
            if valid(PRIMARY_CONSTRUCTOR_KEYWORD)
                && !valid(STRING_CONTENT)
                && check_modifier_then_constructor(lexer)
            {
                return false;
            }
            true
        }

        b'p' => {
            if valid(PRIMARY_CONSTRUCTOR_KEYWORD)
                && !valid(STRING_CONTENT)
                && check_modifier_then_constructor(lexer)
            {
                return false;
            }
            true
        }

        b'c' => {
            if valid(PRIMARY_CONSTRUCTOR_KEYWORD) && !valid(STRING_CONTENT) {
                let matched = advance_over_constructor(lexer);
                if matched && !is_word_char(la(lexer)) {
                    set_result(lexer, PRIMARY_CONSTRUCTOR_KEYWORD);
                    mark_end(lexer);
                    return true;
                }
                return true;
            }
            !scan_for_word(lexer, b"atch")
        }

        b'f' => !scan_for_word(lexer, b"inally"),

        b'@' => {
            if valid(PRIMARY_CONSTRUCTOR_KEYWORD)
                && !valid(STRING_CONTENT)
                && check_annotation_then_constructor(lexer)
            {
                return false;
            }
            true
        }

        b';' => {
            advance(lexer);
            mark_end(lexer);
            true
        }

        _ => true,
    }
}

unsafe fn scan_import_dot(lexer: *mut TSLexer) -> bool {
    if la(lexer) != b'.' as i32 {
        return false;
    }

    mark_end(lexer);

    advance(lexer);

    let mut found_newline = false;
    while is_wspace(la(lexer)) {
        if la(lexer) == b'\n' as i32 || la(lexer) == b'\r' as i32 {
            found_newline = true;
        }
        skip(lexer);
    }

    if found_newline && la(lexer) == b'i' as i32 && scan_for_word(lexer, b"mport") {
        set_result(lexer, AUTOMATIC_SEMICOLON);
        return true;
    }

    set_result(lexer, IMPORT_DOT);
    mark_end(lexer);
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_kotlin_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let stack = &mut *(payload as *mut Stack);
    let valid = |i: usize| *valid_symbols.add(i);

    if valid(AUTOMATIC_SEMICOLON) {
        let ret = scan_automatic_semicolon(lexer, valid_symbols);
        if ret {
            return ret;
        }
    }

    if valid(IMPORT_DOT) {
        if scan_import_dot(lexer) {
            return true;
        }
    }

    if valid(PRIMARY_CONSTRUCTOR_KEYWORD) && !valid(STRING_CONTENT) {
        while is_wspace(la(lexer)) {
            skip(lexer);
        }
        if la(lexer) == b'c' as i32 {
            let matched = advance_over_constructor(lexer);
            if matched && !is_word_char(la(lexer)) {
                set_result(lexer, PRIMARY_CONSTRUCTOR_KEYWORD);
                mark_end(lexer);
                return true;
            }
        }
    }

    if valid(STRING_CONTENT)
        || valid(INTERPOLATION_EXPRESSION_START)
        || valid(INTERPOLATION_IDENTIFIER_START)
    {
        if scan_string_content(lexer, stack, valid_symbols) {
            return true;
        }
    }

    while is_wspace(la(lexer)) {
        skip(lexer);
    }

    if valid(STRING_START) && scan_string_start(lexer, stack) {
        set_result(lexer, STRING_START);
        return true;
    }

    if valid(MULTILINE_COMMENT) && scan_multiline_comment(lexer) {
        return true;
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_kotlin_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Stack::new())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_kotlin_external_scanner_destroy(payload: *mut c_void) {
    drop(Box::from_raw(payload as *mut Stack));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_kotlin_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let stack = &*(payload as *const Stack);
    let mut n = stack.len();
    if n > TREE_SITTER_SERIALIZATION_BUFFER_SIZE {
        n = TREE_SITTER_SERIALIZATION_BUFFER_SIZE;
    }
    if n > 0 {
        core::ptr::copy_nonoverlapping(stack.as_ptr(), buffer, n);
    }
    n as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_kotlin_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let stack = &mut *(payload as *mut Stack);
    if length > 0 && length % 2 == 0 {
        stack.clear();
        stack.extend_from_slice(core::slice::from_raw_parts(buffer, length as usize));
    } else {
        stack.clear();
    }
}
