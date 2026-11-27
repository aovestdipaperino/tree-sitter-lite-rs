//! Rust port of tree-sitter-dockerfile's external scanner (src/scanner.c, tag v0.2.0).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const MAX_HEREDOCS: usize = 10;
const DEL_SPACE: usize = 512;
/// `TREE_SITTER_SERIALIZATION_BUFFER_SIZE`
const SERIALIZATION_BUFFER_SIZE: usize = 1024;

const HEREDOC_MARKER: u16 = 0;
const HEREDOC_LINE: u16 = 1;
const HEREDOC_END: u16 = 2;
const HEREDOC_NL: u16 = 3;
const ERROR_SENTINEL: usize = 4;

/// `{ bool in_heredoc; bool stripping_heredoc; unsigned heredoc_count; char *heredocs[10]; }`
///
/// Each heredoc is the C string (without its terminating NUL, which is implicit): byte 0 is the
/// strip indicator (`-` or space), the delimiter follows. The C builds it from `char`s, so a
/// codepoint whose low byte is 0 ends the string early (`strlen`); `scan_marker` truncates the same
/// way, which makes every later use (strlen, the comparison loop) behave identically.
#[derive(Default)]
struct Scanner {
    in_heredoc: bool,
    stripping_heredoc: bool,
    heredoc_count: u32,
    heredocs: [Option<Vec<u8>>; MAX_HEREDOCS],
}

/// `iswspace` in the C locale, which is what a program that never calls `setlocale` gets.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

unsafe fn advance(lexer: *mut TSLexer, skip: bool) {
    ((*lexer).advance)(lexer, skip);
}

unsafe fn skip_whitespace(lexer: *mut TSLexer) {
    while (*lexer).lookahead != 0
        && (*lexer).lookahead != '\n' as i32
        && is_wspace((*lexer).lookahead)
    {
        advance(lexer, true);
    }
}

impl Scanner {
    unsafe fn scan_marker(&mut self, lexer: *mut TSLexer) -> bool {
        skip_whitespace(lexer);

        if (*lexer).lookahead != '<' as i32 {
            return false;
        }
        advance(lexer, false);

        if (*lexer).lookahead != '<' as i32 {
            return false;
        }
        advance(lexer, false);

        let mut stripping = false;
        if (*lexer).lookahead == '-' as i32 {
            stripping = true;
            advance(lexer, false);
        }

        let mut quote: i32 = 0;
        if (*lexer).lookahead == '"' as i32 || (*lexer).lookahead == '\'' as i32 {
            quote = (*lexer).lookahead;
            advance(lexer, false);
        }

        // char delimiter[DEL_SPACE]; index 0 is reserved for the strip indicator.
        let mut delimiter = [0u8; DEL_SPACE];
        let mut del_idx: usize = 1;

        while (*lexer).lookahead != 0
            && (if quote != 0 {
                (*lexer).lookahead != quote
            } else {
                !is_wspace((*lexer).lookahead)
            })
        {
            if (*lexer).lookahead == '\\' as i32 {
                advance(lexer, false);
                if (*lexer).lookahead == 0 {
                    return false;
                }
            }

            if del_idx > 0 {
                // int32_t -> char truncates to the low byte.
                delimiter[del_idx] = (*lexer).lookahead as u8;
                del_idx += 1;
            }
            advance(lexer, false);

            // Out of space: stop recording but keep consuming the marker. del_idx stays <= 509 so
            // every write above is in bounds (as in the C).
            if del_idx >= DEL_SPACE - 2 {
                del_idx = 0;
            }
        }

        if quote != 0 {
            if (*lexer).lookahead != quote {
                return false;
            }
            advance(lexer, false);
        }

        if del_idx == 0 {
            (*lexer).result_symbol = HEREDOC_MARKER;
            return true;
        }

        delimiter[0] = if stripping { b'-' } else { b' ' };
        // The C string ends at the first NUL byte (a truncated codepoint can be one).
        let end = delimiter[..del_idx]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(del_idx);
        let del_copy = delimiter[..end].to_vec();

        if self.heredoc_count == 0 {
            self.heredoc_count = 1;
            self.heredocs[0] = Some(del_copy);
            self.stripping_heredoc = stripping;
        } else if self.heredoc_count as usize >= MAX_HEREDOCS {
            // dropped
        } else {
            self.heredocs[self.heredoc_count as usize] = Some(del_copy);
            self.heredoc_count += 1;
        }

        (*lexer).result_symbol = HEREDOC_MARKER;
        true
    }

    unsafe fn scan_content(&mut self, lexer: *mut TSLexer, valid: &dyn Fn(usize) -> bool) -> bool {
        if self.heredoc_count == 0 {
            self.in_heredoc = false;
            return false;
        }

        self.in_heredoc = true;

        if self.stripping_heredoc {
            skip_whitespace(lexer);
        }

        if valid(HEREDOC_END as usize) {
            let mut delim_idx = 1usize;
            // Look for the current heredoc delimiter. `char` is signed (x86-64 and Apple arm64):
            // a byte >= 0x80 compares as negative and never equals a lookahead codepoint.
            {
                let h = self.heredocs[0].as_ref().unwrap();
                while delim_idx < h.len()
                    && (*lexer).lookahead != 0
                    && (*lexer).lookahead == h[delim_idx] as i8 as i32
                {
                    advance(lexer, false);
                    delim_idx += 1;
                }
            }

            // Check if the entire string matched.
            if delim_idx >= self.heredocs[0].as_ref().unwrap().len() {
                (*lexer).result_symbol = HEREDOC_END;

                // Shift the first heredoc off the list.
                self.heredocs[0] = None;
                for i in 1..self.heredoc_count as usize {
                    self.heredocs[i - 1] = self.heredocs[i].take();
                }
                self.heredocs[self.heredoc_count as usize - 1] = None;
                self.heredoc_count -= 1;

                if self.heredoc_count > 0 {
                    self.stripping_heredoc = self.heredocs[0].as_ref().unwrap()[0] == b'-';
                } else {
                    self.in_heredoc = false;
                }

                return true;
            }
        }

        if !valid(HEREDOC_LINE as usize) {
            return false;
        }

        (*lexer).result_symbol = HEREDOC_LINE;

        loop {
            match (*lexer).lookahead {
                0 => {
                    if ((*lexer).eof)(lexer) {
                        self.in_heredoc = false;
                        return true;
                    }
                    advance(lexer, false);
                }
                10 => return true,
                _ => advance(lexer, false),
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dockerfile_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dockerfile_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dockerfile_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    let buf = core::slice::from_raw_parts_mut(buffer as *mut u8, SERIALIZATION_BUFFER_SIZE);

    let mut pos = 0usize;
    buf[pos] = s.in_heredoc as u8;
    pos += 1;
    buf[pos] = s.stripping_heredoc as u8;
    pos += 1;

    for i in 0..s.heredoc_count as usize {
        let h = s.heredocs[i].as_ref().unwrap();
        // strlen + 1: the terminating NUL is copied too.
        let len = h.len() + 1;

        // If we run out of space, drop the heredocs that don't fit (same bound as the C, which
        // leaves room for the final NUL, so no write is out of bounds).
        if pos + len + 1 > SERIALIZATION_BUFFER_SIZE {
            break;
        }

        buf[pos..pos + h.len()].copy_from_slice(h);
        buf[pos + h.len()] = 0;
        pos += len;
    }

    // A NUL at the end makes it easy to detect.
    buf[pos] = 0;
    pos += 1;
    pos as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dockerfile_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    // Free all current heredocs.
    for i in 0..s.heredoc_count as usize {
        s.heredocs[i] = None;
    }

    if length == 0 {
        s.in_heredoc = false;
        s.stripping_heredoc = false;
        s.heredoc_count = 0;
    } else {
        // The C reads `strlen` past `length` only on a buffer without its final NUL, which the
        // serializer never produces; here a read past `length` counts as a NUL.
        let buf = core::slice::from_raw_parts(buffer as *const u8, length as usize);
        let at = |i: usize| -> u8 { buf.get(i).copied().unwrap_or(0) };
        let mut pos = 0usize;
        s.in_heredoc = at(pos) != 0;
        pos += 1;
        s.stripping_heredoc = at(pos) != 0;
        pos += 1;

        let mut heredoc_count = 0u32;
        for i in 0..MAX_HEREDOCS {
            let mut len = 0usize;
            while at(pos + len) != 0 {
                len += 1;
            }

            // The ending NUL means we are done.
            if len == 0 {
                break;
            }

            s.heredocs[i] = Some(buf[pos..pos + len].to_vec());
            heredoc_count += 1;

            // Skip the string and its NUL.
            pos += len + 1;
        }

        s.heredoc_count = heredoc_count;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_dockerfile_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    let valid = |i: usize| *valid_symbols.add(i);

    if valid(ERROR_SENTINEL) {
        return if s.in_heredoc {
            s.scan_content(lexer, &valid)
        } else {
            s.scan_marker(lexer)
        };
    }

    // HEREDOC_NL only matches a linebreak if there are open heredocs.
    if valid(HEREDOC_NL as usize) && s.heredoc_count > 0 && (*lexer).lookahead == '\n' as i32 {
        (*lexer).result_symbol = HEREDOC_NL;
        advance(lexer, false);
        return true;
    }

    if valid(HEREDOC_MARKER as usize) {
        return s.scan_marker(lexer);
    }

    if valid(HEREDOC_LINE as usize) || valid(HEREDOC_END as usize) {
        return s.scan_content(lexer, &valid);
    }

    false
}
