//! Rust port of tree-sitter-c-sharp's external scanner (src/scanner.c).
//!
//! `iswspace` from the C is ported as the C locale: ASCII only (9..=13 and 32).
#![allow(clippy::all, unsafe_op_in_unsafe_fn, unused_assignments)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

const OPT_SEMI: usize = 0;
const INTERPOLATION_REGULAR_START: usize = 1;
const INTERPOLATION_VERBATIM_START: usize = 2;
const INTERPOLATION_RAW_START: usize = 3;
const INTERPOLATION_START_QUOTE: usize = 4;
const INTERPOLATION_END_QUOTE: usize = 5;
const INTERPOLATION_OPEN_BRACE: usize = 6;
const INTERPOLATION_CLOSE_BRACE: usize = 7;
const INTERPOLATION_STRING_CONTENT: usize = 8;
const RAW_STRING_START: usize = 9;
const RAW_STRING_END: usize = 10;
const RAW_STRING_CONTENT: usize = 11;

const REGULAR: u8 = 1 << 0;
const VERBATIM: u8 = 1 << 1;
const RAW: u8 = 1 << 2;

const SERIALIZATION_BUFFER_SIZE: usize = 1024;

#[derive(Clone, Copy, Default)]
struct Interpolation {
    dollar_count: u8,
    open_brace_count: u8,
    quote_count: u8,
    string_type: u8,
}

impl Interpolation {
    fn is_regular(&self) -> bool {
        self.string_type & REGULAR != 0
    }
    fn is_verbatim(&self) -> bool {
        self.string_type & VERBATIM != 0
    }
    fn is_raw(&self) -> bool {
        self.string_type & RAW != 0
    }
}

struct Scanner {
    quote_count: u8,
    interpolation_stack: Vec<Interpolation>,
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
fn ch(c: u8) -> i32 {
    c as i32
}
/// `iswspace` in the C locale.
#[inline]
fn iswspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_c_sharp_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner {
        quote_count: 0,
        interpolation_stack: Vec::new(),
    })) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_c_sharp_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_c_sharp_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let s = &*(payload as *const Scanner);
    if s.interpolation_stack.len() * 4 + 2 > SERIALIZATION_BUFFER_SIZE {
        return 0;
    }
    let buf = core::slice::from_raw_parts_mut(buffer as *mut u8, SERIALIZATION_BUFFER_SIZE);
    let mut size = 0usize;
    buf[size] = s.quote_count;
    size += 1;
    // `(char)stack.size`: the check above bounds the size to 255, so this never truncates.
    buf[size] = s.interpolation_stack.len() as u8;
    size += 1;
    for i in &s.interpolation_stack {
        buf[size] = i.dollar_count;
        buf[size + 1] = i.open_brace_count;
        buf[size + 2] = i.quote_count;
        buf[size + 3] = i.string_type;
        size += 4;
    }
    size as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_c_sharp_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    let s = &mut *(payload as *mut Scanner);
    s.quote_count = 0;
    s.interpolation_stack.clear();
    if length > 0 {
        let buf = core::slice::from_raw_parts(buffer as *const u8, length as usize);
        // The C reads the buffer unchecked and ends with `assert(size == length)`; buffers come
        // from `serialize` so they are always well-formed. Bytes past the end read as 0 here
        // instead of out of bounds.
        let at = |i: usize| if i < buf.len() { buf[i] } else { 0 };
        let mut size = 0usize;
        s.quote_count = at(size);
        size += 1;
        let n = at(size) as usize;
        size += 1;
        for _ in 0..n {
            s.interpolation_stack.push(Interpolation {
                dollar_count: at(size),
                open_brace_count: at(size + 1),
                quote_count: at(size + 2),
                string_type: at(size + 3),
            });
            size += 4;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_c_sharp_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let scanner = &mut *(payload as *mut Scanner);
    let valid = |i: usize| *valid_symbols.add(i);
    let l = lexer;

    let mut brace_advanced: u8 = 0;
    let mut quote_count: u8 = 0;
    let mut did_advance = false;

    // error recovery, gives better trees this way
    if valid(OPT_SEMI) && valid(INTERPOLATION_REGULAR_START) {
        return false;
    }

    if valid(OPT_SEMI) {
        (*l).result_symbol = OPT_SEMI as u16;
        if la(l) == ch(b';') {
            advance(l);
        }
        return true;
    }

    if valid(RAW_STRING_START) {
        while iswspace(la(l)) {
            skip(l);
        }

        if la(l) == ch(b'"') {
            while la(l) == ch(b'"') {
                advance(l);
                quote_count = quote_count.wrapping_add(1);
            }

            if quote_count >= 3 {
                (*l).result_symbol = RAW_STRING_START as u16;
                scanner.quote_count = quote_count;
                return true;
            }
        }
    }

    if valid(RAW_STRING_END) && la(l) == ch(b'"') {
        while la(l) == ch(b'"') {
            advance(l);
            quote_count = quote_count.wrapping_add(1);
        }

        if quote_count == scanner.quote_count {
            (*l).result_symbol = RAW_STRING_END as u16;
            scanner.quote_count = 0;
            return true;
        }

        did_advance = quote_count > 0;
    }

    if valid(RAW_STRING_CONTENT) {
        while la(l) != 0 {
            if la(l) == ch(b'"') {
                mark_end(l);
                quote_count = 0;

                while la(l) == ch(b'"') {
                    advance(l);
                    quote_count = quote_count.wrapping_add(1);
                }

                if quote_count == scanner.quote_count {
                    (*l).result_symbol = RAW_STRING_CONTENT as u16;
                    return true;
                }
            }
            advance(l);
            did_advance = true;
        }
        mark_end(l);
        (*l).result_symbol = RAW_STRING_CONTENT as u16;
        return true;
    }

    if valid(INTERPOLATION_REGULAR_START)
        || valid(INTERPOLATION_VERBATIM_START)
        || valid(INTERPOLATION_RAW_START)
    {
        while iswspace(la(l)) {
            skip(l);
        }

        let mut dollar_advanced: u8 = 0;

        let mut is_verbatim = false;

        if la(l) == ch(b'@') {
            is_verbatim = true;
            advance(l);
        }

        while la(l) == ch(b'$') && quote_count == 0 {
            advance(l);
            dollar_advanced = dollar_advanced.wrapping_add(1);
        }

        if dollar_advanced > 0 && (la(l) == ch(b'"') || la(l) == ch(b'@')) {
            (*l).result_symbol = INTERPOLATION_REGULAR_START as u16;
            let mut interpolation = Interpolation {
                dollar_count: dollar_advanced,
                open_brace_count: 0,
                quote_count: 0,
                string_type: 0,
            };

            if is_verbatim || la(l) == ch(b'@') {
                if la(l) == ch(b'@') {
                    advance(l);
                    is_verbatim = true;
                }
                (*l).result_symbol = INTERPOLATION_VERBATIM_START as u16;
                interpolation.string_type = VERBATIM;
            }

            mark_end(l);
            advance(l);

            if la(l) == ch(b'"') && !is_verbatim {
                advance(l);
                if la(l) == ch(b'"') {
                    (*l).result_symbol = INTERPOLATION_RAW_START as u16;
                    interpolation.string_type |= RAW;
                    scanner.interpolation_stack.push(interpolation);
                }
                // If we find 1 or 3 quotes, we push an interpolation.
                // If there's only two quotes, that's just an empty string
            } else {
                interpolation.string_type |= REGULAR;
                scanner.interpolation_stack.push(interpolation);
            }

            return true;
        }
    }

    if valid(INTERPOLATION_START_QUOTE) && !scanner.interpolation_stack.is_empty() {
        let cur = scanner.interpolation_stack.last_mut().unwrap();

        if cur.is_verbatim() || cur.is_regular() {
            if la(l) == ch(b'"') {
                advance(l);
                cur.quote_count = cur.quote_count.wrapping_add(1);
            }
        } else {
            while la(l) == ch(b'"') {
                advance(l);
                cur.quote_count = cur.quote_count.wrapping_add(1);
            }
        }

        (*l).result_symbol = INTERPOLATION_START_QUOTE as u16;
        return cur.quote_count > 0;
    }

    if valid(INTERPOLATION_END_QUOTE) && !scanner.interpolation_stack.is_empty() {
        let cur = *scanner.interpolation_stack.last().unwrap();

        while la(l) == ch(b'"') {
            advance(l);
            quote_count = quote_count.wrapping_add(1);
        }

        if quote_count == cur.quote_count {
            (*l).result_symbol = INTERPOLATION_END_QUOTE as u16;
            scanner.interpolation_stack.pop();
            return true;
        }

        did_advance = quote_count > 0;
    }

    if valid(INTERPOLATION_OPEN_BRACE) && !scanner.interpolation_stack.is_empty() {
        let cur = scanner.interpolation_stack.last_mut().unwrap();

        while la(l) == ch(b'{') && brace_advanced < cur.dollar_count {
            advance(l);
            brace_advanced = brace_advanced.wrapping_add(1);
        }

        if brace_advanced > 0
            && brace_advanced == cur.dollar_count
            && (brace_advanced == 0 || la(l) != ch(b'{'))
        {
            cur.open_brace_count = brace_advanced;
            (*l).result_symbol = INTERPOLATION_OPEN_BRACE as u16;
            return true;
        }
    }

    if valid(INTERPOLATION_CLOSE_BRACE) && !scanner.interpolation_stack.is_empty() {
        let mut brace_advanced: u8 = 0;
        let cur = scanner.interpolation_stack.last_mut().unwrap();

        while iswspace(la(l)) {
            advance(l);
        }

        while la(l) == ch(b'}') {
            advance(l);
            brace_advanced = brace_advanced.wrapping_add(1);

            if brace_advanced == cur.open_brace_count {
                cur.open_brace_count = 0;
                (*l).result_symbol = INTERPOLATION_CLOSE_BRACE as u16;
                return true;
            }
        }

        return false;
    }

    if valid(INTERPOLATION_STRING_CONTENT) && !scanner.interpolation_stack.is_empty() {
        (*l).result_symbol = INTERPOLATION_STRING_CONTENT as u16;
        let cur = *scanner.interpolation_stack.last().unwrap();

        while la(l) != 0 {
            // top-down approach, first see if it's raw
            if cur.is_raw() {
                if la(l) == ch(b'"') {
                    mark_end(l);
                    advance(l);
                    if la(l) == ch(b'"') {
                        advance(l);
                        let mut quote_advanced: u8 = 2;
                        while la(l) == ch(b'"') {
                            quote_advanced = quote_advanced.wrapping_add(1);
                            advance(l);
                        }
                        if quote_advanced == cur.quote_count {
                            return did_advance;
                        }
                    }
                }

                if la(l) == ch(b'{') {
                    mark_end(l);

                    while la(l) == ch(b'{') && brace_advanced < cur.open_brace_count {
                        advance(l);
                        brace_advanced = brace_advanced.wrapping_add(1);
                    }

                    if brace_advanced == cur.open_brace_count
                        && (brace_advanced == 0 || la(l) != ch(b'{'))
                    {
                        return did_advance;
                    }
                }
            }
            // then verbatim, since it could be verbatim + raw, but run the raw branch first
            else if cur.is_verbatim() {
                if la(l) == ch(b'"') {
                    mark_end(l);
                    advance(l);
                    if la(l) == ch(b'"') {
                        advance(l);
                        continue;
                    }
                    return did_advance;
                }

                if la(l) == ch(b'{') {
                    mark_end(l);

                    while la(l) == ch(b'{') && brace_advanced < cur.open_brace_count {
                        advance(l);
                        brace_advanced = brace_advanced.wrapping_add(1);
                    }

                    if brace_advanced == cur.open_brace_count
                        && (brace_advanced == 0 || la(l) != ch(b'{'))
                    {
                        return did_advance;
                    }
                }
            }
            // finally regular
            else if cur.is_regular() {
                if la(l) == ch(b'\\') || la(l) == ch(b'\n') || la(l) == ch(b'"') {
                    mark_end(l);
                    return did_advance;
                }

                if la(l) == ch(b'{') {
                    mark_end(l);

                    while la(l) == ch(b'{') && brace_advanced < cur.open_brace_count {
                        advance(l);
                        brace_advanced = brace_advanced.wrapping_add(1);
                    }

                    if brace_advanced == cur.open_brace_count
                        && (brace_advanced == 0 || la(l) != ch(b'{'))
                    {
                        // if we're in a brace we're not allowed to collect more than the
                        // open_brace_count
                        return did_advance;
                    }
                }
            }

            if la(l) != ch(b'{') {
                brace_advanced = 0;
            }
            advance(l);
            did_advance = true;
        }

        mark_end(l);
        return did_advance;
    }

    false
}
