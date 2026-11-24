//! Rust port of tree-sitter-python's external scanner (src/scanner.c).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const NEWLINE: usize = 0;
const INDENT: usize = 1;
const DEDENT: usize = 2;
const STRING_START: usize = 3;
const STRING_CONTENT: usize = 4;
const ESCAPE_INTERPOLATION: usize = 5;
const STRING_END: usize = 6;
#[allow(dead_code)]
const COMMENT: usize = 7;
const CLOSE_PAREN: usize = 8;
const CLOSE_BRACKET: usize = 9;
const CLOSE_BRACE: usize = 10;
const EXCEPT: usize = 11;

const SINGLE_QUOTE: u8 = 1 << 0;
const DOUBLE_QUOTE: u8 = 1 << 1;
const BACK_QUOTE: u8 = 1 << 2;
const RAW: u8 = 1 << 3;
const FORMAT: u8 = 1 << 4;
const TRIPLE: u8 = 1 << 5;
const BYTES: u8 = 1 << 6;

const SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// One string delimiter: a flag byte, exactly like the C `Delimiter { char flags; }`.
#[derive(Clone, Copy, Default)]
struct Delimiter(u8);

impl Delimiter {
    fn is_format(self) -> bool {
        self.0 & FORMAT != 0
    }
    fn is_raw(self) -> bool {
        self.0 & RAW != 0
    }
    fn is_triple(self) -> bool {
        self.0 & TRIPLE != 0
    }
    fn is_bytes(self) -> bool {
        self.0 & BYTES != 0
    }
    fn end_character(self) -> i32 {
        if self.0 & SINGLE_QUOTE != 0 {
            b'\'' as i32
        } else if self.0 & DOUBLE_QUOTE != 0 {
            b'"' as i32
        } else if self.0 & BACK_QUOTE != 0 {
            b'`' as i32
        } else {
            0
        }
    }
    fn set_end_character(&mut self, c: u8) {
        self.0 |= match c {
            b'\'' => SINGLE_QUOTE,
            b'"' => DOUBLE_QUOTE,
            b'`' => BACK_QUOTE,
            _ => unreachable!("set_end_character"),
        };
    }
}

struct Scanner {
    indents: Vec<u16>,
    delimiters: Vec<Delimiter>,
    inside_interpolated_string: bool,
}

impl Scanner {
    fn deserialize(&mut self, buffer: &[u8]) {
        self.delimiters.clear();
        self.indents.clear();
        self.indents.push(0);
        if !buffer.is_empty() {
            let length = buffer.len();
            let mut size = 0usize;
            self.inside_interpolated_string = buffer[size] != 0;
            size += 1;
            let delimiter_count = buffer[size] as usize;
            size += 1;
            if delimiter_count > 0 {
                self.delimiters.extend(
                    buffer[size..size + delimiter_count]
                        .iter()
                        .map(|&b| Delimiter(b)),
                );
                size += delimiter_count;
            }
            while size + 1 < length {
                self.indents
                    .push(buffer[size] as u16 | ((buffer[size + 1] as u16) << 8));
                size += 2;
            }
        }
    }
}

#[inline]
unsafe fn la(l: *mut TSLexer) -> i32 {
    (*l).lookahead
}
#[inline]
unsafe fn advance(l: *mut TSLexer) {
    ((*l).advance)(l, false)
}
#[inline]
unsafe fn skip(l: *mut TSLexer) {
    ((*l).advance)(l, true)
}
#[inline]
unsafe fn mark_end(l: *mut TSLexer) {
    ((*l).mark_end)(l)
}
#[inline]
unsafe fn eof(l: *mut TSLexer) -> bool {
    ((*l).eof)(l)
}
#[inline]
fn ch(c: u8) -> i32 {
    c as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_python_external_scanner_create() -> *mut c_void {
    let mut s = Box::new(Scanner {
        indents: Vec::new(),
        delimiters: Vec::new(),
        inside_interpolated_string: false,
    });
    s.deserialize(&[]);
    Box::into_raw(s) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_python_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_python_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    let buf = core::slice::from_raw_parts_mut(buffer as *mut u8, SERIALIZATION_BUFFER_SIZE);
    let mut size = 0usize;
    buf[size] = s.inside_interpolated_string as u8;
    size += 1;
    let delimiter_count = s.delimiters.len().min(u8::MAX as usize);
    buf[size] = delimiter_count as u8;
    size += 1;
    for (k, d) in s.delimiters.iter().take(delimiter_count).enumerate() {
        buf[size + k] = d.0;
    }
    size += delimiter_count;
    // The C loop checks `size < 1024` before writing two bytes, which can overrun the buffer by
    // one byte when `size` is odd. Stop one write earlier instead of overrunning.
    for &indent in s.indents.iter().skip(1) {
        if size + 2 > SERIALIZATION_BUFFER_SIZE {
            break;
        }
        buf[size] = (indent & 0xFF) as u8;
        buf[size + 1] = (indent >> 8) as u8;
        size += 2;
    }
    size as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_python_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    let buf: &[u8] = if length == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(buffer as *const u8, length as usize)
    };
    s.deserialize(buf);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_python_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let scanner = &mut *(payload as *mut Scanner);
    let valid = |i: usize| *valid_symbols.add(i);
    let l = lexer;

    let error_recovery_mode = valid(STRING_CONTENT) && valid(INDENT);
    let within_brackets = valid(CLOSE_BRACE) || valid(CLOSE_PAREN) || valid(CLOSE_BRACKET);

    if valid(ESCAPE_INTERPOLATION)
        && !scanner.delimiters.is_empty()
        && (la(l) == ch(b'{') || la(l) == ch(b'}'))
        && !error_recovery_mode
    {
        let delimiter = *scanner.delimiters.last().unwrap();
        if delimiter.is_format() {
            mark_end(l);
            let is_left_brace = la(l) == ch(b'{');
            advance(l);
            if (la(l) == ch(b'{') && is_left_brace) || (la(l) == ch(b'}') && !is_left_brace) {
                advance(l);
                mark_end(l);
                (*l).result_symbol = ESCAPE_INTERPOLATION as u16;
                return true;
            }
            return false;
        }
    }

    if valid(STRING_CONTENT) && !scanner.delimiters.is_empty() && !error_recovery_mode {
        let delimiter = *scanner.delimiters.last().unwrap();
        let end_char = delimiter.end_character();
        let mut has_content = false;
        while la(l) != 0 {
            if (la(l) == ch(b'{') || la(l) == ch(b'}')) && delimiter.is_format() {
                mark_end(l);
                (*l).result_symbol = STRING_CONTENT as u16;
                return has_content;
            }
            if la(l) == ch(b'\\') {
                if delimiter.is_raw() {
                    advance(l);
                    if la(l) == delimiter.end_character() || la(l) == ch(b'\\') {
                        advance(l);
                    }
                    if la(l) == ch(b'\r') {
                        advance(l);
                        if la(l) == ch(b'\n') {
                            advance(l);
                        }
                    } else if la(l) == ch(b'\n') {
                        advance(l);
                    }
                    continue;
                }
                if delimiter.is_bytes() {
                    mark_end(l);
                    advance(l);
                    if la(l) == ch(b'N') || la(l) == ch(b'u') || la(l) == ch(b'U') {
                        advance(l);
                    } else {
                        (*l).result_symbol = STRING_CONTENT as u16;
                        return has_content;
                    }
                } else {
                    mark_end(l);
                    (*l).result_symbol = STRING_CONTENT as u16;
                    return has_content;
                }
            } else if la(l) == end_char {
                if delimiter.is_triple() {
                    mark_end(l);
                    advance(l);
                    if la(l) == end_char {
                        advance(l);
                        if la(l) == end_char {
                            if has_content {
                                (*l).result_symbol = STRING_CONTENT as u16;
                            } else {
                                advance(l);
                                mark_end(l);
                                scanner.delimiters.pop();
                                (*l).result_symbol = STRING_END as u16;
                                scanner.inside_interpolated_string = false;
                            }
                            return true;
                        }
                        mark_end(l);
                        (*l).result_symbol = STRING_CONTENT as u16;
                        return true;
                    }
                    mark_end(l);
                    (*l).result_symbol = STRING_CONTENT as u16;
                    return true;
                }
                if has_content {
                    (*l).result_symbol = STRING_CONTENT as u16;
                } else {
                    advance(l);
                    scanner.delimiters.pop();
                    (*l).result_symbol = STRING_END as u16;
                    scanner.inside_interpolated_string = false;
                }
                mark_end(l);
                return true;
            } else if la(l) == ch(b'\n') && has_content && !delimiter.is_triple() {
                return false;
            }
            advance(l);
            has_content = true;
        }
    }

    mark_end(l);

    let mut found_end_of_line = false;
    let mut indent_length: u16 = 0;
    let mut first_comment_indent_length: i32 = -1;
    loop {
        if la(l) == ch(b'\n') {
            found_end_of_line = true;
            indent_length = 0;
            skip(l);
        } else if la(l) == ch(b' ') {
            indent_length = indent_length.wrapping_add(1);
            skip(l);
        } else if la(l) == ch(b'\r') || la(l) == 0x0c {
            indent_length = 0;
            skip(l);
        } else if la(l) == ch(b'\t') {
            indent_length = indent_length.wrapping_add(8);
            skip(l);
        } else if la(l) == ch(b'#')
            && (valid(INDENT) || valid(DEDENT) || valid(NEWLINE) || valid(EXCEPT))
        {
            if !found_end_of_line {
                return false;
            }
            if first_comment_indent_length == -1 {
                first_comment_indent_length = indent_length as i32;
            }
            while la(l) != 0 && la(l) != ch(b'\n') {
                skip(l);
            }
            skip(l);
            indent_length = 0;
        } else if la(l) == ch(b'\\') {
            skip(l);
            if la(l) == ch(b'\r') {
                skip(l);
            }
            if la(l) == ch(b'\n') || eof(l) {
                skip(l);
            } else {
                return false;
            }
        } else if eof(l) {
            indent_length = 0;
            found_end_of_line = true;
            break;
        } else {
            break;
        }
    }

    if found_end_of_line {
        if !scanner.indents.is_empty() {
            let current_indent_length = *scanner.indents.last().unwrap();

            if valid(INDENT) && indent_length > current_indent_length {
                scanner.indents.push(indent_length);
                (*l).result_symbol = INDENT as u16;
                return true;
            }

            let next_tok_is_string_start =
                la(l) == ch(b'"') || la(l) == ch(b'\'') || la(l) == ch(b'`');

            if (valid(DEDENT)
                || (!valid(NEWLINE)
                    && !(valid(STRING_START) && next_tok_is_string_start)
                    && !within_brackets))
                && indent_length < current_indent_length
                && !scanner.inside_interpolated_string
                && first_comment_indent_length < current_indent_length as i32
            {
                scanner.indents.pop();
                (*l).result_symbol = DEDENT as u16;
                return true;
            }
        }

        if valid(NEWLINE) && !error_recovery_mode {
            (*l).result_symbol = NEWLINE as u16;
            return true;
        }
    }

    if first_comment_indent_length == -1 && valid(STRING_START) {
        let mut delimiter = Delimiter::default();
        let mut has_flags = false;
        while la(l) != 0 {
            if la(l) == ch(b'f') || la(l) == ch(b'F') || la(l) == ch(b't') || la(l) == ch(b'T') {
                delimiter.0 |= FORMAT;
            } else if la(l) == ch(b'r') || la(l) == ch(b'R') {
                delimiter.0 |= RAW;
            } else if la(l) == ch(b'b') || la(l) == ch(b'B') {
                delimiter.0 |= BYTES;
            } else if la(l) != ch(b'u') && la(l) != ch(b'U') {
                break;
            }
            has_flags = true;
            advance(l);
        }

        if la(l) == ch(b'`') {
            delimiter.set_end_character(b'`');
            advance(l);
            mark_end(l);
        } else if la(l) == ch(b'\'') {
            delimiter.set_end_character(b'\'');
            advance(l);
            mark_end(l);
            if la(l) == ch(b'\'') {
                advance(l);
                if la(l) == ch(b'\'') {
                    advance(l);
                    mark_end(l);
                    delimiter.0 |= TRIPLE;
                }
            }
        } else if la(l) == ch(b'"') {
            delimiter.set_end_character(b'"');
            advance(l);
            mark_end(l);
            if la(l) == ch(b'"') {
                advance(l);
                if la(l) == ch(b'"') {
                    advance(l);
                    mark_end(l);
                    delimiter.0 |= TRIPLE;
                }
            }
        }

        if delimiter.end_character() != 0 {
            scanner.delimiters.push(delimiter);
            (*l).result_symbol = STRING_START as u16;
            scanner.inside_interpolated_string = delimiter.is_format();
            return true;
        }
        if has_flags {
            return false;
        }
    }
    false
}
