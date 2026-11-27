//! Rust port of tree-sitter-markdown's block external scanner (tree-sitter-markdown/src/scanner.c,
//! tree-sitter-grammars/tree-sitter-markdown v0.5.3).
//!
//! Locale: the C calls `iswalpha`, `iswalnum`, `towlower` (HTML block tag and attribute names) and
//! `isdigit` (ordered list markers). They are ported as the C locale (ASCII only), which is what a
//! program that never calls `setlocale` gets: `is_walpha`, `is_walnum`, `to_lower`, `is_digit`.
//!
//! Integer widths follow the C: the indentation, column, `matched`, `state` and delimiter-length
//! fields are `uint8_t` and wrap; `advance` returns a `size_t`; a `Block` is a C `int`-sized enum,
//! kept here as an `i32` so that a value outside the enum (a corrupt serialized state, or a list
//! item indentation above the maximum) matches no case in `match`, as in the C.
//!
//! Deviations from the C, each only where the C reads or writes out of bounds:
//! - `serialize` copies at most as many open blocks as fit the 1024-byte serialization buffer
//!   (254 blocks, 5 header bytes + 4 bytes per block). The C copies all of them and overruns the
//!   buffer beyond that depth; up to 254 open blocks the output is identical.
//! - `deserialize` copies exactly `blocks_count * 4` bytes of blocks (the C copies
//!   `length - 5` bytes, which is the same for any state `serialize` produced and can overrun the
//!   block array for a length that is not a multiple of four), and a length below 5 (a corrupt
//!   state, where the C reads past the buffer) restores only the fields that are present.
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// TokenType
const LINE_ENDING: usize = 0;
const SOFT_LINE_ENDING: usize = 1;
const BLOCK_CLOSE: usize = 2;
const BLOCK_CONTINUATION: usize = 3;
const BLOCK_QUOTE_START: usize = 4;
const INDENTED_CHUNK_START: usize = 5;
const ATX_H1_MARKER: usize = 6;
const SETEXT_H1_UNDERLINE: usize = 12;
const SETEXT_H2_UNDERLINE: usize = 13;
const THEMATIC_BREAK: usize = 14;
const LIST_MARKER_MINUS: usize = 15;
const LIST_MARKER_PLUS: usize = 16;
const LIST_MARKER_STAR: usize = 17;
const LIST_MARKER_PARENTHESIS: usize = 18;
const LIST_MARKER_DOT: usize = 19;
const LIST_MARKER_MINUS_DONT_INTERRUPT: usize = 20;
const LIST_MARKER_PLUS_DONT_INTERRUPT: usize = 21;
const LIST_MARKER_STAR_DONT_INTERRUPT: usize = 22;
const LIST_MARKER_PARENTHESIS_DONT_INTERRUPT: usize = 23;
const LIST_MARKER_DOT_DONT_INTERRUPT: usize = 24;
const FENCED_CODE_BLOCK_START_BACKTICK: usize = 25;
const FENCED_CODE_BLOCK_START_TILDE: usize = 26;
const BLANK_LINE_START: usize = 27;
const FENCED_CODE_BLOCK_END_BACKTICK: usize = 28;
const FENCED_CODE_BLOCK_END_TILDE: usize = 29;
const HTML_BLOCK_1_START: usize = 30;
const HTML_BLOCK_1_END: usize = 31;
const HTML_BLOCK_2_START: usize = 32;
const HTML_BLOCK_3_START: usize = 33;
const HTML_BLOCK_4_START: usize = 34;
const HTML_BLOCK_5_START: usize = 35;
const HTML_BLOCK_6_START: usize = 36;
const HTML_BLOCK_7_START: usize = 37;
const CLOSE_BLOCK: usize = 38;
const NO_INDENTED_CHUNK: usize = 39;
const ERROR: usize = 40;
const TRIGGER_ERROR: usize = 41;
const TOKEN_EOF: usize = 42;
const MINUS_METADATA: usize = 43;
const PLUS_METADATA: usize = 44;
const PIPE_TABLE_START: usize = 45;
const PIPE_TABLE_LINE_ENDING: usize = 46;

// Block (a C `int` enum)
const BLOCK_QUOTE: i32 = 0;
const INDENTED_CODE_BLOCK: i32 = 1;
const LIST_ITEM: i32 = 2;
const LIST_ITEM_MAX_INDENTATION: i32 = 17;
const FENCED_CODE_BLOCK: i32 = 18;
const ANONYMOUS: i32 = 19;

const BLOCK_BYTES: usize = 4; // sizeof(Block)
const HEADER_BYTES: usize = 5;
const SERIALIZATION_BUFFER_SIZE: usize = 1024;

const HTML_TAG_NAMES_RULE_1: [&[u8]; 3] = [b"pre", b"script", b"style"];

const HTML_TAG_NAMES_RULE_7: [&[u8]; 62] = [
    b"address",
    b"article",
    b"aside",
    b"base",
    b"basefont",
    b"blockquote",
    b"body",
    b"caption",
    b"center",
    b"col",
    b"colgroup",
    b"dd",
    b"details",
    b"dialog",
    b"dir",
    b"div",
    b"dl",
    b"dt",
    b"fieldset",
    b"figcaption",
    b"figure",
    b"footer",
    b"form",
    b"frame",
    b"frameset",
    b"h1",
    b"h2",
    b"h3",
    b"h4",
    b"h5",
    b"h6",
    b"head",
    b"header",
    b"hr",
    b"html",
    b"iframe",
    b"legend",
    b"li",
    b"link",
    b"main",
    b"menu",
    b"menuitem",
    b"nav",
    b"noframes",
    b"ol",
    b"optgroup",
    b"option",
    b"p",
    b"param",
    b"section",
    b"source",
    b"summary",
    b"table",
    b"tbody",
    b"td",
    b"tfoot",
    b"th",
    b"thead",
    b"title",
    b"tr",
    b"track",
    b"ul",
];

static PARAGRAPH_INTERRUPT_SYMBOLS: [bool; 47] = [
    false, // LINE_ENDING,
    false, // SOFT_LINE_ENDING,
    false, // BLOCK_CLOSE,
    false, // BLOCK_CONTINUATION,
    true,  // BLOCK_QUOTE_START,
    false, // INDENTED_CHUNK_START,
    true,  // ATX_H1_MARKER,
    true,  // ATX_H2_MARKER,
    true,  // ATX_H3_MARKER,
    true,  // ATX_H4_MARKER,
    true,  // ATX_H5_MARKER,
    true,  // ATX_H6_MARKER,
    true,  // SETEXT_H1_UNDERLINE,
    true,  // SETEXT_H2_UNDERLINE,
    true,  // THEMATIC_BREAK,
    true,  // LIST_MARKER_MINUS,
    true,  // LIST_MARKER_PLUS,
    true,  // LIST_MARKER_STAR,
    true,  // LIST_MARKER_PARENTHESIS,
    true,  // LIST_MARKER_DOT,
    false, // LIST_MARKER_MINUS_DONT_INTERRUPT,
    false, // LIST_MARKER_PLUS_DONT_INTERRUPT,
    false, // LIST_MARKER_STAR_DONT_INTERRUPT,
    false, // LIST_MARKER_PARENTHESIS_DONT_INTERRUPT,
    false, // LIST_MARKER_DOT_DONT_INTERRUPT,
    true,  // FENCED_CODE_BLOCK_START_BACKTICK,
    true,  // FENCED_CODE_BLOCK_START_TILDE,
    true,  // BLANK_LINE_START,
    false, // FENCED_CODE_BLOCK_END_BACKTICK,
    false, // FENCED_CODE_BLOCK_END_TILDE,
    true,  // HTML_BLOCK_1_START,
    false, // HTML_BLOCK_1_END,
    true,  // HTML_BLOCK_2_START,
    true,  // HTML_BLOCK_3_START,
    true,  // HTML_BLOCK_4_START,
    true,  // HTML_BLOCK_5_START,
    true,  // HTML_BLOCK_6_START,
    false, // HTML_BLOCK_7_START,
    false, // CLOSE_BLOCK,
    false, // NO_INDENTED_CHUNK,
    false, // ERROR,
    false, // TRIGGER_ERROR,
    false, // EOF,
    false, // MINUS_METADATA,
    false, // PLUS_METADATA,
    true,  // PIPE_TABLE_START,
    false, // PIPE_TABLE_LINE_ENDING,
];

// State bitflags used with `Scanner.state`
const STATE_MATCHING: u8 = 1 << 0;
const STATE_WAS_SOFT_LINE_BREAK: u8 = 1 << 1;
const STATE_CLOSE_BLOCK: u8 = 1 << 4;

#[derive(Default)]
struct Scanner {
    open_blocks: Vec<i32>,
    state: u8,
    matched: u8,
    indentation: u8,
    column: u8,
    fenced_code_block_delimiter_length: u8,
    simulate: bool,
}

// ---- locale-free character classes (C locale) ----

fn is_walpha(c: i32) -> bool {
    (b'a' as i32..=b'z' as i32).contains(&c) || (b'A' as i32..=b'Z' as i32).contains(&c)
}
fn is_walnum(c: i32) -> bool {
    is_walpha(c) || is_digit(c)
}
fn is_digit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
}
fn to_lower(c: i32) -> i32 {
    if (b'A' as i32..=b'Z' as i32).contains(&c) {
        c + 32
    } else {
        c
    }
}

/// `is_punctuation(char)`: the C is called with `(char)lexer->lookahead`, a truncation to the low
/// byte (a signed char, so bytes >= 0x80 are negative and never in the ASCII ranges).
fn is_punctuation(c: i32) -> bool {
    let c = c as u8;
    (b'!'..=b'/').contains(&c)
        || (b':'..=b'@').contains(&c)
        || (b'['..=b'`').contains(&c)
        || (b'{'..=b'~').contains(&c)
}

/// `(uint8_t)(block - LIST_ITEM + 2)`
fn list_item_indentation(block: i32) -> u8 {
    (block - LIST_ITEM + 2) as u8
}

/// `uint8_t += size_t` (computed in `size_t`, truncated back)
fn add_u8(a: u8, b: usize) -> u8 {
    (a as usize).wrapping_add(b) as u8
}

// ---- lexer helpers ----

unsafe fn la(lexer: *mut TSLexer) -> i32 {
    (*lexer).lookahead
}
unsafe fn eof(lexer: *mut TSLexer) -> bool {
    ((*lexer).eof)(lexer)
}
unsafe fn set_sym(lexer: *mut TSLexer, sym: usize) {
    (*lexer).result_symbol = sym as u16;
}
fn is_nl(c: i32) -> bool {
    c == '\n' as i32 || c == '\r' as i32
}
fn is_blank(c: i32) -> bool {
    c == ' ' as i32 || c == '\t' as i32
}

impl Scanner {
    fn size(&self) -> usize {
        self.open_blocks.len()
    }

    /// Write the whole state of a Scanner to a byte buffer
    unsafe fn serialize(&self, buffer: *mut c_char) -> u32 {
        let buf = buffer as *mut u8;
        let mut size = 0usize;
        *buf.add(size) = self.state;
        size += 1;
        *buf.add(size) = self.matched;
        size += 1;
        *buf.add(size) = self.indentation;
        size += 1;
        *buf.add(size) = self.column;
        size += 1;
        *buf.add(size) = self.fenced_code_block_delimiter_length;
        size += 1;
        // The C copies every open block; the bound keeps the write inside the buffer.
        let max_blocks = (SERIALIZATION_BUFFER_SIZE - HEADER_BYTES) / BLOCK_BYTES;
        let blocks_count = self.open_blocks.len().min(max_blocks);
        for b in &self.open_blocks[..blocks_count] {
            let bytes = b.to_ne_bytes();
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(size), BLOCK_BYTES);
            size += BLOCK_BYTES;
        }
        size as u32
    }

    /// Read the whole state of a Scanner from a byte buffer
    unsafe fn deserialize(&mut self, buffer: *const c_char, length: u32) {
        self.open_blocks.clear();
        self.state = 0;
        self.matched = 0;
        self.indentation = 0;
        self.column = 0;
        self.fenced_code_block_delimiter_length = 0;
        if length > 0 {
            let buf = buffer as *const u8;
            let length = length as usize;
            if length < HEADER_BYTES {
                // Corrupt state (the C reads past the buffer): restore the fields that exist.
                debug_assert!(false, "markdown scanner state shorter than its header");
                let get = |i: usize| if i < length { *buf.add(i) } else { 0 };
                self.state = get(0);
                self.matched = get(1);
                self.indentation = get(2);
                self.column = get(3);
                self.fenced_code_block_delimiter_length = get(4);
                return;
            }
            self.state = *buf;
            self.matched = *buf.add(1);
            self.indentation = *buf.add(2);
            self.column = *buf.add(3);
            self.fenced_code_block_delimiter_length = *buf.add(4);
            let blocks_size = length - HEADER_BYTES;
            if blocks_size > 0 {
                let blocks_count = blocks_size / BLOCK_BYTES;
                for i in 0..blocks_count {
                    let mut bytes = [0u8; BLOCK_BYTES];
                    core::ptr::copy_nonoverlapping(
                        buf.add(HEADER_BYTES + i * BLOCK_BYTES),
                        bytes.as_mut_ptr(),
                        BLOCK_BYTES,
                    );
                    self.open_blocks.push(i32::from_ne_bytes(bytes));
                }
            }
        }
    }

    unsafe fn mark_end(&self, lexer: *mut TSLexer) {
        if !self.simulate {
            ((*lexer).mark_end)(lexer);
        }
    }

    /// Advance the lexer one character. Also keeps track of the current column, counting tabs as
    /// spaces with tab stop 4.
    unsafe fn advance(&mut self, lexer: *mut TSLexer) -> usize {
        let mut size = 1usize;
        if la(lexer) == '\t' as i32 {
            size = (4i32 - self.column as i32) as usize;
            self.column = 0;
        } else {
            self.column = ((self.column as u32 + 1) % 4) as u8;
        }
        ((*lexer).advance)(lexer, false);
        size
    }

    /// Try to match the given block, i.e. consume all tokens that belong to the block.
    unsafe fn match_block(&mut self, lexer: *mut TSLexer, block: i32) -> bool {
        match block {
            INDENTED_CODE_BLOCK => {
                while self.indentation < 4 {
                    if is_blank(la(lexer)) {
                        self.indentation = add_u8(self.indentation, self.advance(lexer));
                    } else {
                        break;
                    }
                }
                if self.indentation >= 4 && !is_nl(la(lexer)) {
                    self.indentation = self.indentation.wrapping_sub(4);
                    return true;
                }
            }
            LIST_ITEM..=LIST_ITEM_MAX_INDENTATION => {
                while self.indentation < list_item_indentation(block) {
                    if is_blank(la(lexer)) {
                        self.indentation = add_u8(self.indentation, self.advance(lexer));
                    } else {
                        break;
                    }
                }
                if self.indentation >= list_item_indentation(block) {
                    self.indentation = self.indentation.wrapping_sub(list_item_indentation(block));
                    return true;
                }
                if is_nl(la(lexer)) {
                    self.indentation = 0;
                    return true;
                }
            }
            BLOCK_QUOTE => {
                while is_blank(la(lexer)) {
                    self.indentation = add_u8(self.indentation, self.advance(lexer));
                }
                if la(lexer) == '>' as i32 {
                    self.advance(lexer);
                    self.indentation = 0;
                    if is_blank(la(lexer)) {
                        let a = self.advance(lexer);
                        self.indentation = add_u8(self.indentation, a.wrapping_sub(1));
                    }
                    return true;
                }
            }
            FENCED_CODE_BLOCK | ANONYMOUS => return true,
            _ => {}
        }
        false
    }

    unsafe fn parse_fenced_code_block(
        &mut self,
        delimiter: char,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        let d = delimiter as i32;
        // count the number of backticks
        let mut level: u8 = 0;
        while la(lexer) == d {
            self.advance(lexer);
            level = level.wrapping_add(1);
        }
        self.mark_end(lexer);
        // If this is able to close a fenced code block then that is the only valid interpretation.
        if (if delimiter == '`' {
            v(FENCED_CODE_BLOCK_END_BACKTICK)
        } else {
            v(FENCED_CODE_BLOCK_END_TILDE)
        }) && self.indentation < 4
            && level >= self.fenced_code_block_delimiter_length
        {
            while is_blank(la(lexer)) {
                self.advance(lexer);
            }
            if is_nl(la(lexer)) {
                self.fenced_code_block_delimiter_length = 0;
                set_sym(
                    lexer,
                    if delimiter == '`' {
                        FENCED_CODE_BLOCK_END_BACKTICK
                    } else {
                        FENCED_CODE_BLOCK_END_TILDE
                    },
                );
                return true;
            }
        }
        // If this could be the start of a fenced code block, check if the info string contains
        // any backticks.
        if (if delimiter == '`' {
            v(FENCED_CODE_BLOCK_START_BACKTICK)
        } else {
            v(FENCED_CODE_BLOCK_START_TILDE)
        }) && level >= 3
        {
            let mut info_string_has_backtick = false;
            if delimiter == '`' {
                while !is_nl(la(lexer)) && !eof(lexer) {
                    if la(lexer) == '`' as i32 {
                        info_string_has_backtick = true;
                        break;
                    }
                    self.advance(lexer);
                }
            }
            if !info_string_has_backtick {
                set_sym(
                    lexer,
                    if delimiter == '`' {
                        FENCED_CODE_BLOCK_START_BACKTICK
                    } else {
                        FENCED_CODE_BLOCK_START_TILDE
                    },
                );
                if !self.simulate {
                    self.open_blocks.push(FENCED_CODE_BLOCK);
                }
                self.fenced_code_block_delimiter_length = level;
                self.indentation = 0;
                return true;
            }
        }
        false
    }

    unsafe fn parse_star(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        self.advance(lexer);
        self.mark_end(lexer);
        let mut star_count: usize = 1;
        let mut extra_indentation: u8 = 0;
        loop {
            if la(lexer) == '*' as i32 {
                if star_count == 1 && extra_indentation >= 1 && v(LIST_MARKER_STAR) {
                    self.mark_end(lexer);
                }
                star_count += 1;
                self.advance(lexer);
            } else if is_blank(la(lexer)) {
                if star_count == 1 {
                    let a = self.advance(lexer);
                    extra_indentation = add_u8(extra_indentation, a);
                } else {
                    self.advance(lexer);
                }
            } else {
                break;
            }
        }
        let line_end = is_nl(la(lexer));
        let mut dont_interrupt = false;
        if star_count == 1 && line_end {
            extra_indentation = 1;
            dont_interrupt = self.matched as usize == self.size();
        }
        let thematic_break = star_count >= 3 && line_end;
        let list_marker_star = star_count >= 1 && extra_indentation >= 1;
        if v(THEMATIC_BREAK) && thematic_break && self.indentation < 4 {
            set_sym(lexer, THEMATIC_BREAK);
            self.mark_end(lexer);
            self.indentation = 0;
            return true;
        }
        if (if dont_interrupt {
            v(LIST_MARKER_STAR_DONT_INTERRUPT)
        } else {
            v(LIST_MARKER_STAR)
        }) && list_marker_star
        {
            if star_count == 1 {
                self.mark_end(lexer);
            }
            extra_indentation = extra_indentation.wrapping_sub(1);
            if extra_indentation <= 3 {
                extra_indentation = extra_indentation.wrapping_add(self.indentation);
                self.indentation = 0;
            } else {
                let temp = self.indentation;
                self.indentation = extra_indentation;
                extra_indentation = temp;
            }
            if !self.simulate {
                self.open_blocks.push(LIST_ITEM + extra_indentation as i32);
            }
            set_sym(
                lexer,
                if dont_interrupt {
                    LIST_MARKER_STAR_DONT_INTERRUPT
                } else {
                    LIST_MARKER_STAR
                },
            );
            return true;
        }
        false
    }

    unsafe fn parse_thematic_break_underscore(
        &mut self,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        self.advance(lexer);
        self.mark_end(lexer);
        let mut underscore_count: usize = 1;
        loop {
            if la(lexer) == '_' as i32 {
                underscore_count += 1;
                self.advance(lexer);
            } else if is_blank(la(lexer)) {
                self.advance(lexer);
            } else {
                break;
            }
        }
        let line_end = is_nl(la(lexer));
        if underscore_count >= 3 && line_end && *valid_symbols.add(THEMATIC_BREAK) {
            set_sym(lexer, THEMATIC_BREAK);
            self.mark_end(lexer);
            self.indentation = 0;
            return true;
        }
        false
    }

    unsafe fn parse_block_quote(
        &mut self,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        if *valid_symbols.add(BLOCK_QUOTE_START) {
            self.advance(lexer);
            self.indentation = 0;
            if is_blank(la(lexer)) {
                let a = self.advance(lexer);
                self.indentation = add_u8(self.indentation, a.wrapping_sub(1));
            }
            set_sym(lexer, BLOCK_QUOTE_START);
            if !self.simulate {
                self.open_blocks.push(BLOCK_QUOTE);
            }
            return true;
        }
        false
    }

    unsafe fn parse_atx_heading(
        &mut self,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        if *valid_symbols.add(ATX_H1_MARKER) && self.indentation <= 3 {
            self.mark_end(lexer);
            let mut level: u16 = 0;
            while la(lexer) == '#' as i32 && level <= 6 {
                self.advance(lexer);
                level += 1;
            }
            if level <= 6 && (is_blank(la(lexer)) || is_nl(la(lexer))) {
                set_sym(lexer, ATX_H1_MARKER + (level as usize - 1));
                self.indentation = 0;
                self.mark_end(lexer);
                return true;
            }
        }
        false
    }

    unsafe fn parse_setext_underline(
        &mut self,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        if *valid_symbols.add(SETEXT_H1_UNDERLINE) && self.matched as usize == self.size() {
            self.mark_end(lexer);
            while la(lexer) == '=' as i32 {
                self.advance(lexer);
            }
            while is_blank(la(lexer)) {
                self.advance(lexer);
            }
            if is_nl(la(lexer)) {
                set_sym(lexer, SETEXT_H1_UNDERLINE);
                self.mark_end(lexer);
                return true;
            }
        }
        false
    }

    /// Advance over one line ending (`\r`, `\r\n` or `\n`).
    unsafe fn advance_newline(&mut self, lexer: *mut TSLexer) {
        if la(lexer) == '\r' as i32 {
            self.advance(lexer);
            if la(lexer) == '\n' as i32 {
                self.advance(lexer);
            }
        } else {
            self.advance(lexer);
        }
    }

    unsafe fn parse_plus(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        if self.indentation <= 3
            && (v(LIST_MARKER_PLUS) || v(LIST_MARKER_PLUS_DONT_INTERRUPT) || v(PLUS_METADATA))
        {
            self.advance(lexer);
            if v(PLUS_METADATA) && la(lexer) == '+' as i32 {
                self.advance(lexer);
                if la(lexer) != '+' as i32 {
                    return false;
                }
                self.advance(lexer);
                while is_blank(la(lexer)) {
                    self.advance(lexer);
                }
                if !is_nl(la(lexer)) {
                    return false;
                }
                loop {
                    // advance over newline
                    self.advance_newline(lexer);
                    // check for pluses
                    let mut plus_count: usize = 0;
                    while la(lexer) == '+' as i32 {
                        plus_count += 1;
                        self.advance(lexer);
                    }
                    if plus_count == 3 {
                        // if exactly 3 check if next symbol (after eventual whitespace) is newline
                        while is_blank(la(lexer)) {
                            self.advance(lexer);
                        }
                        if is_nl(la(lexer)) {
                            // if so also consume newline
                            self.advance_newline(lexer);
                            self.mark_end(lexer);
                            set_sym(lexer, PLUS_METADATA);
                            return true;
                        }
                    }
                    // otherwise consume rest of line
                    while !is_nl(la(lexer)) && !eof(lexer) {
                        self.advance(lexer);
                    }
                    // if end of file is reached, then this is not metadata
                    if eof(lexer) {
                        break;
                    }
                }
            } else {
                let mut extra_indentation: u8 = 0;
                while is_blank(la(lexer)) {
                    let a = self.advance(lexer);
                    extra_indentation = add_u8(extra_indentation, a);
                }
                let mut dont_interrupt = false;
                if is_nl(la(lexer)) {
                    extra_indentation = 1;
                    dont_interrupt = true;
                }
                dont_interrupt = dont_interrupt && self.matched as usize == self.size();
                if extra_indentation >= 1
                    && (if dont_interrupt {
                        v(LIST_MARKER_PLUS_DONT_INTERRUPT)
                    } else {
                        v(LIST_MARKER_PLUS)
                    })
                {
                    set_sym(
                        lexer,
                        if dont_interrupt {
                            LIST_MARKER_PLUS_DONT_INTERRUPT
                        } else {
                            LIST_MARKER_PLUS
                        },
                    );
                    extra_indentation = extra_indentation.wrapping_sub(1);
                    if extra_indentation <= 3 {
                        extra_indentation = extra_indentation.wrapping_add(self.indentation);
                        self.indentation = 0;
                    } else {
                        let temp = self.indentation;
                        self.indentation = extra_indentation;
                        extra_indentation = temp;
                    }
                    if !self.simulate {
                        self.open_blocks.push(LIST_ITEM + extra_indentation as i32);
                    }
                    return true;
                }
            }
        }
        false
    }

    unsafe fn parse_ordered_list_marker(
        &mut self,
        lexer: *mut TSLexer,
        valid_symbols: *const bool,
    ) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        if self.indentation <= 3
            && (v(LIST_MARKER_PARENTHESIS)
                || v(LIST_MARKER_DOT)
                || v(LIST_MARKER_PARENTHESIS_DONT_INTERRUPT)
                || v(LIST_MARKER_DOT_DONT_INTERRUPT))
        {
            let mut digits: usize = 1;
            let mut dont_interrupt = !is_digit(la(lexer));
            self.advance(lexer);
            while is_digit(la(lexer)) {
                dont_interrupt = true;
                digits += 1;
                self.advance(lexer);
            }
            if digits >= 1 && digits <= 9 {
                let mut dot = false;
                let mut parenthesis = false;
                if la(lexer) == '.' as i32 {
                    self.advance(lexer);
                    dot = true;
                } else if la(lexer) == ')' as i32 {
                    self.advance(lexer);
                    parenthesis = true;
                }
                if dot || parenthesis {
                    let mut extra_indentation: u8 = 0;
                    while is_blank(la(lexer)) {
                        let a = self.advance(lexer);
                        extra_indentation = add_u8(extra_indentation, a);
                    }
                    let line_end = is_nl(la(lexer));
                    if line_end {
                        extra_indentation = 1;
                        dont_interrupt = true;
                    }
                    dont_interrupt = dont_interrupt && self.matched as usize == self.size();
                    if extra_indentation >= 1
                        && (if dot {
                            if dont_interrupt {
                                v(LIST_MARKER_DOT_DONT_INTERRUPT)
                            } else {
                                v(LIST_MARKER_DOT)
                            }
                        } else if dont_interrupt {
                            v(LIST_MARKER_PARENTHESIS_DONT_INTERRUPT)
                        } else {
                            v(LIST_MARKER_PARENTHESIS)
                        })
                    {
                        set_sym(
                            lexer,
                            if dot {
                                LIST_MARKER_DOT
                            } else {
                                LIST_MARKER_PARENTHESIS
                            },
                        );
                        extra_indentation = extra_indentation.wrapping_sub(1);
                        if extra_indentation <= 3 {
                            extra_indentation = extra_indentation.wrapping_add(self.indentation);
                            self.indentation = 0;
                        } else {
                            let temp = self.indentation;
                            self.indentation = extra_indentation;
                            extra_indentation = temp;
                        }
                        if !self.simulate {
                            self.open_blocks.push(
                                (LIST_ITEM as usize + extra_indentation as usize + digits) as i32,
                            );
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    unsafe fn parse_minus(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        if self.indentation <= 3
            && (v(LIST_MARKER_MINUS)
                || v(LIST_MARKER_MINUS_DONT_INTERRUPT)
                || v(SETEXT_H2_UNDERLINE)
                || v(THEMATIC_BREAK)
                || v(MINUS_METADATA))
        {
            self.mark_end(lexer);
            let mut whitespace_after_minus = false;
            let mut minus_after_whitespace = false;
            let mut minus_count: usize = 0;
            let mut extra_indentation: u8 = 0;

            loop {
                if la(lexer) == '-' as i32 {
                    if minus_count == 1 && extra_indentation >= 1 {
                        self.mark_end(lexer);
                    }
                    minus_count += 1;
                    self.advance(lexer);
                    minus_after_whitespace = whitespace_after_minus;
                } else if is_blank(la(lexer)) {
                    if minus_count == 1 {
                        let a = self.advance(lexer);
                        extra_indentation = add_u8(extra_indentation, a);
                    } else {
                        self.advance(lexer);
                    }
                    whitespace_after_minus = true;
                } else {
                    break;
                }
            }
            let line_end = is_nl(la(lexer));
            let mut dont_interrupt = false;
            if minus_count == 1 && line_end {
                extra_indentation = 1;
                dont_interrupt = true;
            }
            dont_interrupt = dont_interrupt && self.matched as usize == self.size();
            let thematic_break = minus_count >= 3 && line_end;
            let underline = minus_count >= 1
                && !minus_after_whitespace
                && line_end
                && self.matched as usize == self.size();
            let list_marker_minus = minus_count >= 1 && extra_indentation >= 1;
            let mut success = false;
            if v(SETEXT_H2_UNDERLINE) && underline {
                set_sym(lexer, SETEXT_H2_UNDERLINE);
                self.mark_end(lexer);
                self.indentation = 0;
                success = true;
            } else if v(THEMATIC_BREAK) && thematic_break {
                set_sym(lexer, THEMATIC_BREAK);
                self.mark_end(lexer);
                self.indentation = 0;
                success = true;
            } else if (if dont_interrupt {
                v(LIST_MARKER_MINUS_DONT_INTERRUPT)
            } else {
                v(LIST_MARKER_MINUS)
            }) && list_marker_minus
            {
                if minus_count == 1 {
                    self.mark_end(lexer);
                }
                extra_indentation = extra_indentation.wrapping_sub(1);
                if extra_indentation <= 3 {
                    extra_indentation = extra_indentation.wrapping_add(self.indentation);
                    self.indentation = 0;
                } else {
                    let temp = self.indentation;
                    self.indentation = extra_indentation;
                    extra_indentation = temp;
                }
                if !self.simulate {
                    self.open_blocks.push(LIST_ITEM + extra_indentation as i32);
                }
                set_sym(
                    lexer,
                    if dont_interrupt {
                        LIST_MARKER_MINUS_DONT_INTERRUPT
                    } else {
                        LIST_MARKER_MINUS
                    },
                );
                return true;
            }
            if minus_count == 3 && !minus_after_whitespace && line_end && v(MINUS_METADATA) {
                loop {
                    // advance over newline
                    self.advance_newline(lexer);
                    // check for minuses
                    minus_count = 0;
                    while la(lexer) == '-' as i32 {
                        minus_count += 1;
                        self.advance(lexer);
                    }
                    if minus_count == 3 {
                        // if exactly 3 check if next symbol (after eventual whitespace) is newline
                        while is_blank(la(lexer)) {
                            self.advance(lexer);
                        }
                        if is_nl(la(lexer)) {
                            // if so also consume newline
                            self.advance_newline(lexer);
                            self.mark_end(lexer);
                            set_sym(lexer, MINUS_METADATA);
                            return true;
                        }
                    }
                    // otherwise consume rest of line
                    while !is_nl(la(lexer)) && !eof(lexer) {
                        self.advance(lexer);
                    }
                    // if end of file is reached, then this is not metadata
                    if eof(lexer) {
                        break;
                    }
                }
            }
            if success {
                return true;
            }
        }
        false
    }

    unsafe fn parse_html_block(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        if !(v(HTML_BLOCK_1_START)
            || v(HTML_BLOCK_1_END)
            || v(HTML_BLOCK_2_START)
            || v(HTML_BLOCK_3_START)
            || v(HTML_BLOCK_4_START)
            || v(HTML_BLOCK_5_START)
            || v(HTML_BLOCK_6_START)
            || v(HTML_BLOCK_7_START))
        {
            return false;
        }
        self.advance(lexer);
        if la(lexer) == '?' as i32 && v(HTML_BLOCK_3_START) {
            self.advance(lexer);
            set_sym(lexer, HTML_BLOCK_3_START);
            if !self.simulate {
                self.open_blocks.push(ANONYMOUS);
            }
            return true;
        }
        if la(lexer) == '!' as i32 {
            // could be block 2
            self.advance(lexer);
            if la(lexer) == '-' as i32 {
                self.advance(lexer);
                if la(lexer) == '-' as i32 && v(HTML_BLOCK_2_START) {
                    self.advance(lexer);
                    set_sym(lexer, HTML_BLOCK_2_START);
                    if !self.simulate {
                        self.open_blocks.push(ANONYMOUS);
                    }
                    return true;
                }
            } else if ('A' as i32 <= la(lexer)) && la(lexer) <= 'Z' as i32 && v(HTML_BLOCK_4_START)
            {
                self.advance(lexer);
                set_sym(lexer, HTML_BLOCK_4_START);
                if !self.simulate {
                    self.open_blocks.push(ANONYMOUS);
                }
                return true;
            } else if la(lexer) == '[' as i32 {
                self.advance(lexer);
                if la(lexer) == 'C' as i32 {
                    self.advance(lexer);
                    if la(lexer) == 'D' as i32 {
                        self.advance(lexer);
                        if la(lexer) == 'A' as i32 {
                            self.advance(lexer);
                            if la(lexer) == 'T' as i32 {
                                self.advance(lexer);
                                if la(lexer) == 'A' as i32 {
                                    self.advance(lexer);
                                    if la(lexer) == '[' as i32 && v(HTML_BLOCK_5_START) {
                                        self.advance(lexer);
                                        set_sym(lexer, HTML_BLOCK_5_START);
                                        if !self.simulate {
                                            self.open_blocks.push(ANONYMOUS);
                                        }
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let starting_slash = la(lexer) == '/' as i32;
        if starting_slash {
            self.advance(lexer);
        }
        let mut name = [0u8; 11];
        let mut name_length: usize = 0;
        while is_walpha(la(lexer)) {
            if name_length < 10 {
                name[name_length] = to_lower(la(lexer)) as u8;
                name_length += 1;
            } else {
                name_length = 12;
            }
            self.advance(lexer);
        }
        if name_length == 0 {
            return false;
        }
        let mut tag_closed = false;
        if name_length < 11 {
            name[name_length] = 0;
            let name = &name[..name_length];
            let next_symbol_valid =
                is_blank(la(lexer)) || is_nl(la(lexer)) || la(lexer) == '>' as i32;
            if next_symbol_valid {
                // try block 1 names
                for n in HTML_TAG_NAMES_RULE_1.iter() {
                    if name == *n {
                        if starting_slash {
                            if v(HTML_BLOCK_1_END) {
                                set_sym(lexer, HTML_BLOCK_1_END);
                                return true;
                            }
                        } else if v(HTML_BLOCK_1_START) {
                            set_sym(lexer, HTML_BLOCK_1_START);
                            if !self.simulate {
                                self.open_blocks.push(ANONYMOUS);
                            }
                            return true;
                        }
                    }
                }
            }
            if !next_symbol_valid && la(lexer) == '/' as i32 {
                self.advance(lexer);
                if la(lexer) == '>' as i32 {
                    self.advance(lexer);
                    tag_closed = true;
                }
            }
            if next_symbol_valid || tag_closed {
                // try block 2 names
                for n in HTML_TAG_NAMES_RULE_7.iter() {
                    if name == *n && v(HTML_BLOCK_6_START) {
                        set_sym(lexer, HTML_BLOCK_6_START);
                        if !self.simulate {
                            self.open_blocks.push(ANONYMOUS);
                        }
                        return true;
                    }
                }
            }
        }

        if !v(HTML_BLOCK_7_START) {
            return false;
        }

        if !tag_closed {
            // tag name (continued)
            while is_walnum(la(lexer)) || la(lexer) == '-' as i32 {
                self.advance(lexer);
            }
            if !starting_slash {
                // attributes
                let mut had_whitespace = false;
                loop {
                    // whitespace
                    while is_blank(la(lexer)) {
                        had_whitespace = true;
                        self.advance(lexer);
                    }
                    if la(lexer) == '/' as i32 {
                        self.advance(lexer);
                        break;
                    }
                    if la(lexer) == '>' as i32 {
                        break;
                    }
                    // attribute name
                    if !had_whitespace {
                        return false;
                    }
                    if !is_walpha(la(lexer)) && la(lexer) != '_' as i32 && la(lexer) != ':' as i32 {
                        return false;
                    }
                    had_whitespace = false;
                    self.advance(lexer);
                    while is_walnum(la(lexer))
                        || la(lexer) == '_' as i32
                        || la(lexer) == '.' as i32
                        || la(lexer) == ':' as i32
                        || la(lexer) == '-' as i32
                    {
                        self.advance(lexer);
                    }
                    // attribute value specification
                    // optional whitespace
                    while is_blank(la(lexer)) {
                        had_whitespace = true;
                        self.advance(lexer);
                    }
                    // =
                    if la(lexer) == '=' as i32 {
                        self.advance(lexer);
                        had_whitespace = false;
                        // optional whitespace
                        while is_blank(la(lexer)) {
                            self.advance(lexer);
                        }
                        // attribute value
                        if la(lexer) == '\'' as i32 || la(lexer) == '"' as i32 {
                            let delimiter = la(lexer);
                            self.advance(lexer);
                            while la(lexer) != delimiter && !is_nl(la(lexer)) && !eof(lexer) {
                                self.advance(lexer);
                            }
                            if la(lexer) != delimiter {
                                return false;
                            }
                            self.advance(lexer);
                        } else {
                            // unquoted attribute value
                            let mut had_one = false;
                            while !is_blank(la(lexer))
                                && la(lexer) != '"' as i32
                                && la(lexer) != '\'' as i32
                                && la(lexer) != '=' as i32
                                && la(lexer) != '<' as i32
                                && la(lexer) != '>' as i32
                                && la(lexer) != '`' as i32
                                && !is_nl(la(lexer))
                                && !eof(lexer)
                            {
                                self.advance(lexer);
                                had_one = true;
                            }
                            if !had_one {
                                return false;
                            }
                        }
                    }
                }
            } else {
                while is_blank(la(lexer)) {
                    self.advance(lexer);
                }
            }
            if la(lexer) != '>' as i32 {
                return false;
            }
            self.advance(lexer);
        }
        while is_blank(la(lexer)) {
            self.advance(lexer);
        }
        if is_nl(la(lexer)) {
            set_sym(lexer, HTML_BLOCK_7_START);
            if !self.simulate {
                self.open_blocks.push(ANONYMOUS);
            }
            return true;
        }
        false
    }

    unsafe fn parse_pipe_table(
        &mut self,
        lexer: *mut TSLexer,
        _valid_symbols: *const bool,
    ) -> bool {
        // PIPE_TABLE_START is zero width
        self.mark_end(lexer);
        // count number of cells
        let mut cell_count: usize = 0;
        // also remember if we see starting and ending pipes, as empty headers have to have both
        let mut starting_pipe = false;
        let mut ending_pipe = false;
        let empty = true;
        if la(lexer) == '|' as i32 {
            starting_pipe = true;
            self.advance(lexer);
        }
        while !is_nl(la(lexer)) && !eof(lexer) {
            if la(lexer) == '|' as i32 {
                cell_count += 1;
                ending_pipe = true;
                self.advance(lexer);
            } else {
                if !is_blank(la(lexer)) {
                    ending_pipe = false;
                }
                if la(lexer) == '\\' as i32 {
                    self.advance(lexer);
                    if is_punctuation(la(lexer)) {
                        self.advance(lexer);
                    }
                } else {
                    self.advance(lexer);
                }
            }
        }
        if empty && cell_count == 0 && !(starting_pipe && ending_pipe) {
            return false;
        }
        if !ending_pipe {
            cell_count += 1;
        }

        // check the following line for a delimiter row
        // parse a newline
        if la(lexer) == '\n' as i32 {
            self.advance(lexer);
        } else if la(lexer) == '\r' as i32 {
            self.advance(lexer);
            if la(lexer) == '\n' as i32 {
                self.advance(lexer);
            }
        } else {
            return false;
        }
        self.indentation = 0;
        self.column = 0;
        loop {
            if is_blank(la(lexer)) {
                let a = self.advance(lexer);
                self.indentation = add_u8(self.indentation, a);
            } else {
                break;
            }
        }
        self.simulate = true;
        let mut matched_temp: u8 = 0;
        while matched_temp < self.size() as u8 {
            let b = self.open_blocks[matched_temp as usize];
            if self.match_block(lexer, b) {
                matched_temp = matched_temp.wrapping_add(1);
            } else {
                return false;
            }
        }

        // check if delimiter row has the same number of cells and at least one pipe
        let mut delimiter_cell_count: usize = 0;
        if la(lexer) == '|' as i32 {
            self.advance(lexer);
        }
        loop {
            while is_blank(la(lexer)) {
                self.advance(lexer);
            }
            if la(lexer) == '|' as i32 {
                delimiter_cell_count += 1;
                self.advance(lexer);
                continue;
            }
            if la(lexer) == ':' as i32 {
                self.advance(lexer);
                if la(lexer) != '-' as i32 {
                    return false;
                }
            }
            let mut had_one_minus = false;
            while la(lexer) == '-' as i32 {
                had_one_minus = true;
                self.advance(lexer);
            }
            if had_one_minus {
                delimiter_cell_count += 1;
            }
            if la(lexer) == ':' as i32 {
                if !had_one_minus {
                    return false;
                }
                self.advance(lexer);
            }
            while is_blank(la(lexer)) {
                self.advance(lexer);
            }
            if la(lexer) == '|' as i32 {
                if !had_one_minus {
                    delimiter_cell_count += 1;
                }
                self.advance(lexer);
                continue;
            }
            if !is_nl(la(lexer)) {
                return false;
            } else {
                break;
            }
        }
        // if the cell counts are not equal then this is not a table
        if cell_count != delimiter_cell_count {
            return false;
        }

        set_sym(lexer, PIPE_TABLE_START);
        true
    }

    unsafe fn scan(&mut self, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
        let v = |i: usize| *valid_symbols.add(i);
        // A normal tree-sitter rule decided that the current branch is invalid and now
        // "requests" an error to stop the branch
        if v(TRIGGER_ERROR) {
            set_sym(lexer, ERROR);
            return true;
        }

        // Close the inner most block after the next line break as requested.
        if v(CLOSE_BLOCK) {
            self.state |= STATE_CLOSE_BLOCK;
            set_sym(lexer, CLOSE_BLOCK);
            return true;
        }

        // if we are at the end of the file and there are still open blocks close them all
        if eof(lexer) {
            if v(TOKEN_EOF) {
                set_sym(lexer, TOKEN_EOF);
                return true;
            }
            if self.size() > 0 {
                set_sym(lexer, BLOCK_CLOSE);
                if !self.simulate {
                    self.open_blocks.pop();
                }
                return true;
            }
            return false;
        }

        if self.state & STATE_MATCHING == 0 {
            // Parse any preceeding whitespace and remember its length.
            loop {
                if is_blank(la(lexer)) {
                    let a = self.advance(lexer);
                    self.indentation = add_u8(self.indentation, a);
                } else {
                    break;
                }
            }
            // We are not matching. This is where the parsing logic for most "normal" token is.
            if v(INDENTED_CHUNK_START) && !v(NO_INDENTED_CHUNK) {
                if self.indentation >= 4 && !is_nl(la(lexer)) {
                    set_sym(lexer, INDENTED_CHUNK_START);
                    if !self.simulate {
                        self.open_blocks.push(INDENTED_CODE_BLOCK);
                    }
                    self.indentation = self.indentation.wrapping_sub(4);
                    return true;
                }
            }
            // Decide which tokens to consider based on the first non-whitespace character
            let c = la(lexer);
            if c == '\r' as i32 || c == '\n' as i32 {
                if v(BLANK_LINE_START) {
                    // A blank line token is actually just 0 width, so do not consume the characters
                    set_sym(lexer, BLANK_LINE_START);
                    return true;
                }
            } else if c == '`' as i32 {
                return self.parse_fenced_code_block('`', lexer, valid_symbols);
            } else if c == '~' as i32 {
                return self.parse_fenced_code_block('~', lexer, valid_symbols);
            } else if c == '*' as i32 {
                return self.parse_star(lexer, valid_symbols);
            } else if c == '_' as i32 {
                return self.parse_thematic_break_underscore(lexer, valid_symbols);
            } else if c == '>' as i32 {
                return self.parse_block_quote(lexer, valid_symbols);
            } else if c == '#' as i32 {
                return self.parse_atx_heading(lexer, valid_symbols);
            } else if c == '=' as i32 {
                return self.parse_setext_underline(lexer, valid_symbols);
            } else if c == '+' as i32 {
                return self.parse_plus(lexer, valid_symbols);
            } else if is_digit(c) {
                return self.parse_ordered_list_marker(lexer, valid_symbols);
            } else if c == '-' as i32 {
                return self.parse_minus(lexer, valid_symbols);
            } else if c == '<' as i32 {
                return self.parse_html_block(lexer, valid_symbols);
            }
            if !is_nl(la(lexer)) && v(PIPE_TABLE_START) {
                return self.parse_pipe_table(lexer, valid_symbols);
            }
        } else {
            // we are in the state of trying to match all currently open blocks
            let mut partial_success = false;
            while self.matched < self.size() as u8 {
                if self.matched == (self.size() as u8).wrapping_sub(1)
                    && self.state & STATE_CLOSE_BLOCK != 0
                {
                    if !partial_success {
                        self.state &= !STATE_CLOSE_BLOCK;
                    }
                    break;
                }
                let b = self.open_blocks[self.matched as usize];
                if self.match_block(lexer, b) {
                    partial_success = true;
                    self.matched = self.matched.wrapping_add(1);
                } else {
                    if self.state & STATE_WAS_SOFT_LINE_BREAK != 0 {
                        self.state &= !STATE_MATCHING;
                    }
                    break;
                }
            }
            if partial_success {
                if self.matched as usize == self.size() {
                    self.state &= !STATE_MATCHING;
                }
                set_sym(lexer, BLOCK_CONTINUATION);
                return true;
            }

            if self.state & STATE_WAS_SOFT_LINE_BREAK == 0 {
                set_sym(lexer, BLOCK_CLOSE);
                self.open_blocks.pop();
                if self.matched as usize == self.size() {
                    self.state &= !STATE_MATCHING;
                }
                return true;
            }
        }

        // The parser just encountered a line break. Setup the state correspondingly
        if (v(LINE_ENDING) || v(SOFT_LINE_ENDING) || v(PIPE_TABLE_LINE_ENDING)) && is_nl(la(lexer))
        {
            self.advance_newline(lexer);
            self.indentation = 0;
            self.column = 0;
            if self.state & STATE_CLOSE_BLOCK == 0
                && (v(SOFT_LINE_ENDING) || v(PIPE_TABLE_LINE_ENDING))
            {
                ((*lexer).mark_end)(lexer);
                loop {
                    if is_blank(la(lexer)) {
                        let a = self.advance(lexer);
                        self.indentation = add_u8(self.indentation, a);
                    } else {
                        break;
                    }
                }
                self.simulate = true;
                let matched_temp = self.matched;
                self.matched = 0;
                let mut one_will_be_matched = false;
                while self.matched < self.size() as u8 {
                    let b = self.open_blocks[self.matched as usize];
                    if self.match_block(lexer, b) {
                        self.matched = self.matched.wrapping_add(1);
                        one_will_be_matched = true;
                    } else {
                        break;
                    }
                }
                let all_will_be_matched = self.matched as usize == self.size();
                if !eof(lexer) && !self.scan(lexer, PARAGRAPH_INTERRUPT_SYMBOLS.as_ptr()) {
                    self.matched = matched_temp;
                    // If the last line break ended a paragraph and no new block opened, the last
                    // line break should have been a soft line break. Reset the counter for
                    // matched blocks
                    self.matched = 0;
                    self.indentation = 0;
                    self.column = 0;
                    // If there is at least one open block, we should be in the matching state.
                    if one_will_be_matched {
                        self.state |= STATE_MATCHING;
                    } else {
                        self.state &= !STATE_MATCHING;
                    }
                    if v(PIPE_TABLE_LINE_ENDING) {
                        if all_will_be_matched {
                            set_sym(lexer, PIPE_TABLE_LINE_ENDING);
                            return true;
                        }
                    } else {
                        set_sym(lexer, SOFT_LINE_ENDING);
                        // reset some state variables
                        self.state |= STATE_WAS_SOFT_LINE_BREAK;
                        return true;
                    }
                } else {
                    self.matched = matched_temp;
                }
                self.indentation = 0;
                self.column = 0;
            }
            if v(LINE_ENDING) {
                self.matched = 0;
                if self.size() > 0 {
                    self.state |= STATE_MATCHING;
                } else {
                    self.state &= !STATE_MATCHING;
                }
                // reset some state variables
                self.state &= !STATE_WAS_SOFT_LINE_BREAK;
                set_sym(lexer, LINE_ENDING);
                return true;
            }
        }
        false
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut Scanner));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    (*(payload as *const Scanner)).serialize(buffer)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    (*(payload as *mut Scanner)).deserialize(buffer, length);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_markdown_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let s = &mut *(payload as *mut Scanner);
    s.simulate = false;
    s.scan(lexer, valid_symbols)
}
