//! Rust port of tree-sitter-erlang's external scanner (src/scanner.c).
//!
//! Stateless scanner: no payload, nothing serialized. No locale functions are used: the C's
//! `is_whitespace` tests code point ranges directly (0x01..=0x20 and 0x80..=0xA0).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const TQ_STRING: usize = 0;
const TQ_SIGIL_STRING: usize = 1;

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn skip(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, true);
}

unsafe fn is_whitespace(lexer: *mut TSLexer) -> bool {
    let c = (*lexer).lookahead;
    (0x01..=0x20).contains(&c) || (0x80..=0xA0).contains(&c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_erlang_external_scanner_scan(
    _payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    if *valid_symbols.add(TQ_STRING) || *valid_symbols.add(TQ_SIGIL_STRING) {
        while is_whitespace(lexer) {
            skip(lexer);
        }
        let mut is_sigil_string = false;
        if *valid_symbols.add(TQ_SIGIL_STRING) {
            if (*lexer).lookahead == '~' as i32 {
                is_sigil_string = true;
                advance(lexer);
                match (*lexer).lookahead {
                    // 's' | 'S' | 'b' | 'B'
                    0x73 | 0x53 | 0x62 | 0x42 => advance(lexer),
                    0x22 => {}
                    _ => return false,
                }
            }
        }

        let quote = '"' as i32;
        let nl = '\n' as i32;
        if (*lexer).lookahead == quote {
            advance(lexer);
            if (*lexer).lookahead == quote {
                advance(lexer);
                if (*lexer).lookahead == quote {
                    advance(lexer);
                    // uint16_t in the C: wraps around after 65535 quotes.
                    let mut delimiter_count: u16 = 3;
                    while (*lexer).lookahead == quote {
                        delimiter_count = delimiter_count.wrapping_add(1);
                        advance(lexer);
                    }
                    while (*lexer).lookahead != nl && is_whitespace(lexer) {
                        advance(lexer);
                    }
                    if (*lexer).lookahead == nl {
                        advance(lexer);
                    } else {
                        return false;
                    }
                    loop {
                        if (*lexer).lookahead == nl {
                            advance(lexer);
                            while (*lexer).lookahead != nl && is_whitespace(lexer) {
                                advance(lexer);
                            }
                            let mut check: u16 = delimiter_count;
                            while check > 0 {
                                if (*lexer).lookahead != quote {
                                    break;
                                }
                                advance(lexer);
                                check -= 1;
                            }
                            if check == 0 {
                                ((*lexer).mark_end)(lexer);
                                (*lexer).result_symbol = if is_sigil_string {
                                    TQ_SIGIL_STRING as _
                                } else {
                                    TQ_STRING as _
                                };
                                return true;
                            }
                        } else if ((*lexer).eof)(lexer) {
                            return false;
                        } else {
                            advance(lexer);
                        }
                    }
                }
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_erlang_external_scanner_serialize(
    _payload: *mut c_void,
    _buffer: *mut c_char,
) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_erlang_external_scanner_deserialize(
    _payload: *mut c_void,
    _buffer: *const c_char,
    _length: u32,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_erlang_external_scanner_create() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_erlang_external_scanner_destroy(_payload: *mut c_void) {}
