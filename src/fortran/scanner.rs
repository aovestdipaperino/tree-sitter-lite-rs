//! Rust port of tree-sitter-fortran's external scanner (src/scanner.c, 0.5.1).
//!
//! Locale functions (`iswspace`, `iswblank`, `iswalpha`, `iswalnum`, `iswdigit`, `iswxdigit`) are
//! ported as the C locale: ASCII only. Where the C passes the 32-bit lookahead to a `char`
//! parameter (`is_identifier_char`, `is_boz_sentinel`, `is_exp_sentinel`, `quote`), the value is
//! truncated to a signed 8-bit `char` exactly like the C on the platforms that matter.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const LINE_CONTINUATION: u16 = 0;
const INTEGER_LITERAL: u16 = 1;
const FLOAT_LITERAL: u16 = 2;
const BOZ_LITERAL: u16 = 3;
const STRING_LITERAL: u16 = 4;
const STRING_LITERAL_KIND: u16 = 5;
const END_OF_STATEMENT: u16 = 6;
const PREPROC_UNARY_OPERATOR: u16 = 7;
const HOLLERITH_CONSTANT: u16 = 8;

/// `typedef struct { bool in_line_continuation; } Scanner;`
#[derive(Default)]
struct Scanner {
    in_line_continuation: bool,
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

fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

fn is_wblank(c: i32) -> bool {
    matches!(c, 9 | 32)
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

fn is_wxdigit(c: i32) -> bool {
    is_wdigit(c)
        || (b'a' as i32..=b'f' as i32).contains(&c)
        || (b'A' as i32..=b'F' as i32).contains(&c)
}

/// The C passes the lookahead to a `char` parameter: truncate to a signed byte. A negative value
/// is outside the C locale's ASCII range, so every ASCII test below sees it as "no".
fn as_char(c: i32) -> i32 {
    (c as i8) as i32
}

fn is_identifier_char(chr: i32) -> bool {
    is_walnum(chr) || chr == b'_' as i32
}

fn is_boz_sentinel(chr: i32) -> bool {
    matches!(chr as u8 as char, 'B' | 'b' | 'O' | 'o' | 'Z' | 'z') && chr >= 0
}

fn is_exp_sentinel(chr: i32) -> bool {
    matches!(chr as u8 as char, 'D' | 'd' | 'E' | 'e') && chr >= 0
}

/// The C version recurses after a `&&` continuation (`scan_int(lexer);` with the result discarded);
/// the recursion depth is the number of `&&` continuations, so a long enough input overflows the C
/// stack. This is the same computation as a loop: every `mark_end` of the nested calls marks the
/// position where the innermost successful call stopped, and the outermost one marks it last.
unsafe fn scan_int(lexer: *mut TSLexer) -> bool {
    if !is_wdigit((*lexer).lookahead) {
        return false;
    }
    loop {
        while is_wdigit((*lexer).lookahead) {
            advance(lexer);
        }
        // handle line continuations
        if (*lexer).lookahead == b'&' as i32 {
            skip(lexer);
            while is_wspace((*lexer).lookahead) {
                skip(lexer);
            }
            // second '&' required to continue the literal
            if (*lexer).lookahead == b'&' as i32 {
                skip(lexer);
                // nested scan_int: fails without side effects when no digit follows
                if is_wdigit((*lexer).lookahead) {
                    continue;
                }
            }
        }
        break;
    }
    mark_end(lexer);
    true
}

unsafe fn scan_number(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = INTEGER_LITERAL;
    let mut digits = scan_int(lexer);
    if (*lexer).lookahead == b'.' as i32 {
        advance(lexer);
        if digits && !is_walnum((*lexer).lookahead) {
            mark_end(lexer);
        }
        (*lexer).result_symbol = FLOAT_LITERAL;
    }
    digits = scan_int(lexer) || digits;
    if digits {
        if is_exp_sentinel(as_char((*lexer).lookahead)) {
            advance(lexer);
            if (*lexer).lookahead == b'+' as i32 || (*lexer).lookahead == b'-' as i32 {
                advance(lexer);
            }
            if !scan_int(lexer) {
                return true; // valid number token with junk after it
            }
            mark_end(lexer);
            (*lexer).result_symbol = FLOAT_LITERAL;
        }
    }
    digits
}

unsafe fn scan_boz(lexer: *mut TSLexer) -> bool {
    (*lexer).result_symbol = BOZ_LITERAL;
    let mut boz_prefix = false;
    if is_boz_sentinel(as_char((*lexer).lookahead)) {
        advance(lexer);
        boz_prefix = true;
    }
    if (*lexer).lookahead == b'\'' as i32 || (*lexer).lookahead == b'"' as i32 {
        let quote = (*lexer).lookahead;
        advance(lexer);
        if !is_wxdigit((*lexer).lookahead) {
            return false;
        }
        while is_wxdigit((*lexer).lookahead) {
            advance(lexer);
        }
        if (*lexer).lookahead != quote {
            return false;
        }
        advance(lexer);
        if !boz_prefix && !is_boz_sentinel(as_char((*lexer).lookahead)) {
            return false;
        }
        mark_end(lexer);
        return true;
    }
    false
}

unsafe fn skip_literal_continuation_sequence(lexer: *mut TSLexer) -> bool {
    if (*lexer).lookahead != b'&' as i32 {
        return true;
    }
    skip(lexer);
    while is_wspace((*lexer).lookahead) {
        skip(lexer);
    }
    if (*lexer).lookahead == b'&' as i32 {
        skip(lexer);
        return true;
    }
    false
}

unsafe fn scan_hollerith_constant(lexer: *mut TSLexer) -> bool {
    let mut length: u32 = 0;
    while is_wdigit((*lexer).lookahead) {
        // unsigned arithmetic wraps in the C
        let new_length = length
            .wrapping_mul(10)
            .wrapping_add(((*lexer).lookahead - b'0' as i32) as u32);
        if new_length < length {
            return false;
        }
        length = new_length;
        advance(lexer);
        if !skip_literal_continuation_sequence(lexer) {
            return false;
        }
    }
    if length == 0 {
        return false;
    }
    if (*lexer).lookahead != b'H' as i32 && (*lexer).lookahead != b'h' as i32 {
        return false;
    }
    advance(lexer);
    // C: `for (int i = 0; i < length; i++)` compares `i` converted to unsigned. The loop returns at
    // end of input, so a u32 counter gives the same result without the C's signed overflow.
    let mut i: u32 = 0;
    while i < length {
        if (*lexer).lookahead == 0 || eof(lexer) {
            return false;
        }
        if !skip_literal_continuation_sequence(lexer) {
            return false;
        }
        advance(lexer);
        i += 1;
    }
    (*lexer).result_symbol = HOLLERITH_CONSTANT;
    mark_end(lexer);
    true
}

unsafe fn scan_end_of_statement(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    if eof(lexer) {
        skip(lexer);
        (*lexer).result_symbol = END_OF_STATEMENT;
        return true;
    }
    if scanner.in_line_continuation {
        return false;
    }
    if (*lexer).lookahead == b'\r' as i32 {
        skip(lexer);
        if (*lexer).lookahead == b'\n' as i32 {
            skip(lexer);
        }
    } else if (*lexer).lookahead == b'\n' as i32 {
        skip(lexer);
    } else if (*lexer).lookahead != b'!' as i32 {
        return false;
    }
    (*lexer).result_symbol = END_OF_STATEMENT;
    true
}

unsafe fn scan_start_line_continuation(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    scanner.in_line_continuation = (*lexer).lookahead == b'&' as i32;
    if !scanner.in_line_continuation {
        return false;
    }
    advance(lexer);
    (*lexer).result_symbol = LINE_CONTINUATION;
    true
}

unsafe fn scan_end_line_continuation(scanner: &mut Scanner, lexer: *mut TSLexer) -> bool {
    if !scanner.in_line_continuation {
        return false;
    }
    if (*lexer).lookahead == b'!' as i32 {
        return false;
    }
    scanner.in_line_continuation = false;
    if (*lexer).lookahead == b'&' as i32 {
        advance(lexer);
    }
    (*lexer).result_symbol = LINE_CONTINUATION;
    true
}

unsafe fn scan_string_literal_kind(lexer: *mut TSLexer) -> bool {
    if !is_walpha((*lexer).lookahead) {
        return false;
    }
    (*lexer).result_symbol = STRING_LITERAL_KIND;
    let mut current_char: i32 = 0;
    while is_identifier_char(as_char((*lexer).lookahead)) && !eof(lexer) {
        current_char = as_char((*lexer).lookahead);
        if (*lexer).lookahead == b'_' as i32 {
            mark_end(lexer);
        }
        advance(lexer);
    }
    if current_char != b'_' as i32
        || ((*lexer).lookahead != b'"' as i32 && (*lexer).lookahead != b'\'' as i32)
    {
        return false;
    }
    true
}

unsafe fn scan_string_literal(lexer: *mut TSLexer) -> bool {
    // `const char opening_quote`: truncated to a signed byte (a lookahead such as U+4E22 becomes '"')
    let opening_quote = as_char((*lexer).lookahead);
    if opening_quote != b'"' as i32 && opening_quote != b'\'' as i32 {
        return false;
    }
    advance(lexer);
    (*lexer).result_symbol = STRING_LITERAL;
    while (*lexer).lookahead != b'\n' as i32 && !eof(lexer) {
        if (*lexer).lookahead == b'&' as i32 {
            advance(lexer);
            while is_wblank((*lexer).lookahead) {
                advance(lexer);
            }
            if (*lexer).lookahead == b'\n' as i32 || (*lexer).lookahead == b'\r' as i32 {
                while is_wspace((*lexer).lookahead) {
                    advance(lexer);
                }
            }
        }
        if (*lexer).lookahead == opening_quote {
            advance(lexer);
            skip_literal_continuation_sequence(lexer);
            if (*lexer).lookahead != opening_quote {
                return true;
            }
        }
        advance(lexer);
    }
    false
}

unsafe fn scan_preproc_unary_operator(lexer: *mut TSLexer) -> bool {
    // `const char next_char`: truncated to a signed byte (U+4E2D becomes '-')
    let next_char = as_char((*lexer).lookahead);
    if next_char == b'!' as i32
        || next_char == b'~' as i32
        || next_char == b'-' as i32
        || next_char == b'+' as i32
    {
        advance(lexer);
        (*lexer).result_symbol = PREPROC_UNARY_OPERATOR;
        return true;
    }
    false
}

unsafe fn scan(scanner: &mut Scanner, lexer: *mut TSLexer, valid: &dyn Fn(u16) -> bool) -> bool {
    while is_wblank((*lexer).lookahead) {
        skip(lexer);
    }
    if valid(END_OF_STATEMENT) {
        if scan_end_of_statement(scanner, lexer) {
            return true;
        }
    }
    while is_wspace((*lexer).lookahead) {
        skip(lexer);
    }
    if scan_end_line_continuation(scanner, lexer) {
        return true;
    }
    if valid(STRING_LITERAL) {
        if scan_string_literal(lexer) {
            return true;
        }
    }
    if valid(HOLLERITH_CONSTANT) {
        if scan_hollerith_constant(lexer) {
            return true;
        }
    }
    if valid(INTEGER_LITERAL) || valid(FLOAT_LITERAL) || valid(BOZ_LITERAL) {
        if scan_number(lexer) {
            return true;
        }
        if scan_boz(lexer) {
            return true;
        }
    }
    if valid(PREPROC_UNARY_OPERATOR) {
        if scan_preproc_unary_operator(lexer) {
            return true;
        }
    }
    if scan_start_line_continuation(scanner, lexer) {
        return true;
    }
    if valid(STRING_LITERAL_KIND) {
        if scan_string_literal_kind(lexer) {
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fortran_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fortran_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fortran_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    *buffer = s.in_line_continuation as c_char;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fortran_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    if length > 0 {
        s.in_line_continuation = *buffer != 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_fortran_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    scan(s, lexer, &|i| *valid_symbols.add(i as usize))
}
