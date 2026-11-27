//! Rust port of tree-sitter-cpp's external scanner (src/scanner.c): raw string literals.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const RAW_STRING_DELIMITER: usize = 0;
const RAW_STRING_CONTENT: usize = 1;

/// The spec limits delimiters to 16 chars.
const MAX_DELIMITER_LENGTH: usize = 16;

/// `wchar_t` is a 32-bit signed integer on the supported platforms (macOS, Linux).
type WChar = i32;
const WCHAR_SIZE: usize = core::mem::size_of::<WChar>();

/// `{ uint8_t delimiter_length; wchar_t delimiter[16]; }`
#[derive(Default)]
struct Scanner {
    delimiter_length: u8,
    delimiter: [WChar; MAX_DELIMITER_LENGTH],
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

/// `iswspace` in the C locale, which is what a program that never calls `setlocale` gets
/// (ASCII only: tab, newline, vertical tab, form feed, carriage return and space).
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

impl Scanner {
    fn reset(&mut self) {
        self.delimiter_length = 0;
        self.delimiter = [0; MAX_DELIMITER_LENGTH];
    }

    /// Scan the raw string delimiter in `R"delimiter(content)delimiter"`.
    unsafe fn scan_raw_string_delimiter(&mut self, lexer: *mut TSLexer) -> bool {
        if self.delimiter_length > 0 {
            // Closing delimiter: must exactly match the opening delimiter.
            for i in 0..self.delimiter_length as usize {
                if (*lexer).lookahead != self.delimiter[i] {
                    return false;
                }
                advance(lexer);
            }
            self.reset();
            return true;
        }

        // Opening delimiter: record the d-char-sequence up to `(`.
        loop {
            if self.delimiter_length as usize >= MAX_DELIMITER_LENGTH
                || ((*lexer).eof)(lexer)
                || (*lexer).lookahead == '\\' as i32
                || is_wspace((*lexer).lookahead)
            {
                return false;
            }
            if (*lexer).lookahead == '(' as i32 {
                // An empty delimiter fails, so the grammar falls back to a delimiter-less rule.
                return self.delimiter_length > 0;
            }
            self.delimiter[self.delimiter_length as usize] = (*lexer).lookahead;
            self.delimiter_length += 1;
            advance(lexer);
        }
    }

    /// Scan the raw string content in `R"delimiter(content)delimiter"`.
    unsafe fn scan_raw_string_content(&mut self, lexer: *mut TSLexer) -> bool {
        // The progress made through the delimiter since the last ')'.
        let mut delimiter_index: i32 = -1;
        loop {
            // At EOF the content terminates there (an incomplete raw_string_literal).
            if ((*lexer).eof)(lexer) {
                ((*lexer).mark_end)(lexer);
                return true;
            }

            if delimiter_index >= 0 {
                if delimiter_index == self.delimiter_length as i32 {
                    if (*lexer).lookahead == '"' as i32 {
                        return true;
                    }
                    delimiter_index = -1;
                } else if (*lexer).lookahead == self.delimiter[delimiter_index as usize] {
                    delimiter_index += 1;
                } else {
                    delimiter_index = -1;
                }
            }

            if delimiter_index == -1 && (*lexer).lookahead == ')' as i32 {
                // The content doesn't include the `)delimiter"` part.
                ((*lexer).mark_end)(lexer);
                delimiter_index = 0;
            }

            advance(lexer);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_cpp_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_cpp_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_cpp_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    let size = s.delimiter_length as usize * WCHAR_SIZE;
    core::ptr::copy_nonoverlapping(s.delimiter.as_ptr() as *const u8, buffer as *mut u8, size);
    size as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_cpp_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    // The C asserts `length % sizeof(wchar_t) == 0` and would overflow the array for a longer
    // buffer; serialize never produces either, so this clamps instead of aborting.
    let s = &mut *(payload as *mut Scanner);
    s.delimiter_length = (length as usize / WCHAR_SIZE) as u8;
    if length > 0 {
        let n = (length as usize).min(MAX_DELIMITER_LENGTH * WCHAR_SIZE);
        core::ptr::copy_nonoverlapping(buffer as *const u8, s.delimiter.as_mut_ptr() as *mut u8, n);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_cpp_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    let valid = |i: usize| *valid_symbols.add(i);

    if valid(RAW_STRING_DELIMITER) && valid(RAW_STRING_CONTENT) {
        // we're in error recovery
        return false;
    }

    // No skipping leading whitespace: raw-string grammar is space-sensitive.
    if valid(RAW_STRING_DELIMITER) {
        (*lexer).result_symbol = RAW_STRING_DELIMITER as u16;
        return s.scan_raw_string_delimiter(lexer);
    }

    if valid(RAW_STRING_CONTENT) {
        (*lexer).result_symbol = RAW_STRING_CONTENT as u16;
        return s.scan_raw_string_content(lexer);
    }

    false
}
