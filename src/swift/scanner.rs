//! Rust port of tree-sitter-swift's external scanner (src/scanner.c).
//!
//! Locale: the C calls `iswspace` and `iswalnum`; a program that never calls `setlocale` gets the
//! C locale, so both are ASCII only here (`is_wspace`, `is_walnum`). They are used in
//! `should_treat_as_wspace`, `eat_operators` (terminator checks and the `<` custom-operator end)
//! and `eat_whitespace` (comment/operator probing).
#![allow(clippy::all, unsafe_op_in_unsafe_fn)]
use crate::abi::TSLexer;
use core::ffi::{c_char, c_void};

// enum TokenType
const BLOCK_COMMENT: usize = 0;
const RAW_STR_PART: usize = 1;
const RAW_STR_CONTINUING_INDICATOR: usize = 2;
const RAW_STR_END_PART: usize = 3;
const IMPLICIT_SEMI: usize = 4;
const EXPLICIT_SEMI: usize = 5;
const ARROW_OPERATOR: usize = 6;
const DOT_OPERATOR: usize = 7;
const CONJUNCTION_OPERATOR: usize = 8;
const DISJUNCTION_OPERATOR: usize = 9;
const NIL_COALESCING_OPERATOR: usize = 10;
const DOUBLE_OPTIONAL: usize = 11;
const EQUAL_SIGN: usize = 12;
const EQ_EQ: usize = 13;
const PLUS_THEN_WS: usize = 14;
const MINUS_THEN_WS: usize = 15;
const BANG: usize = 16;
const THROWS_KEYWORD: usize = 17;
const RETHROWS_KEYWORD: usize = 18;
const DEFAULT_KEYWORD: usize = 19;
const WHERE_KEYWORD: usize = 20;
const ELSE_KEYWORD: usize = 21;
const CATCH_KEYWORD: usize = 22;
const AS_KEYWORD: usize = 23;
const AS_QUEST: usize = 24;
const AS_BANG: usize = 25;
const ASYNC_KEYWORD: usize = 26;
const CUSTOM_OPERATOR: usize = 27;
const HASH_SYMBOL: usize = 28;
const DIRECTIVE_IF: usize = 29;
const DIRECTIVE_ELSEIF: usize = 30;
const DIRECTIVE_ELSE: usize = 31;
const DIRECTIVE_ENDIF: usize = 32;
const FAKE_TRY_BANG: usize = 33;

const TOKEN_COUNT: u64 = 34;
const OPERATOR_COUNT: usize = 20;

// enum OperatorIndex
const OPERATOR_INDEX_ARROW: u8 = 0;
const OPERATOR_INDEX_DOT: u8 = 1;
const OPERATOR_INDEX_CONJUNCTION: u8 = 2;
const OPERATOR_INDEX_DISJUNCTION: u8 = 3;
const OPERATOR_INDEX_NIL_COALESCING: u8 = 4;
const OPERATOR_INDEX_EQUAL_SIGN: u8 = 5;
const OPERATOR_INDEX_EQUALITY: u8 = 6;
const OPERATOR_INDEX_PLUS_THEN_WHITESPACE: u8 = 7;
const OPERATOR_INDEX_MINUS_THEN_WHITESPACE: u8 = 8;
const OPERATOR_INDEX_BANG: u8 = 9;
const OPERATOR_INDEX_THROWS: u8 = 10;
const OPERATOR_INDEX_RETHROWS: u8 = 11;
const OPERATOR_INDEX_DEFAULT: u8 = 12;
const OPERATOR_INDEX_WHERE: u8 = 13;
const OPERATOR_INDEX_ELSE: u8 = 14;
const OPERATOR_INDEX_CATCH: u8 = 15;
const OPERATOR_INDEX_AS: u8 = 16;
const OPERATOR_INDEX_AS_QUESTION: u8 = 17;
const OPERATOR_INDEX_AS_BANG: u8 = 18;
const OPERATOR_INDEX_ASYNC: u8 = 19;

/// The C strings, each with its terminating NUL so indexing reads `'\0'` exactly as C does.
const OPERATORS: [&[u8]; OPERATOR_COUNT] = [
    b"->\0",
    b".\0",
    b"&&\0",
    b"||\0",
    b"??\0",
    b"=\0",
    b"==\0",
    b"+\0",
    b"-\0",
    b"!\0",
    b"throws\0",
    b"rethrows\0",
    b"default\0",
    b"where\0",
    b"else\0",
    b"catch\0",
    b"as\0",
    b"as?\0",
    b"as!\0",
    b"async\0",
];

#[derive(Clone, Copy, PartialEq)]
enum IllegalTerminatorGroup {
    Alphanumeric,
    OperatorSymbols,
    OperatorOrDot,
    NonWhitespace,
}
use IllegalTerminatorGroup::*;

const OP_ILLEGAL_TERMINATORS: [IllegalTerminatorGroup; OPERATOR_COUNT] = [
    OperatorSymbols, // ->
    OperatorOrDot,   // .
    OperatorSymbols, // &&
    OperatorSymbols, // ||
    OperatorSymbols, // ??
    OperatorSymbols, // =
    OperatorSymbols, // ==
    NonWhitespace,   // +
    NonWhitespace,   // -
    OperatorSymbols, // !
    Alphanumeric,    // throws
    Alphanumeric,    // rethrows
    Alphanumeric,    // default
    Alphanumeric,    // where
    Alphanumeric,    // else
    Alphanumeric,    // catch
    Alphanumeric,    // as
    OperatorSymbols, // as?
    OperatorSymbols, // as!
    Alphanumeric,    // async
];

const OP_SYMBOLS: [usize; OPERATOR_COUNT] = [
    ARROW_OPERATOR,
    DOT_OPERATOR,
    CONJUNCTION_OPERATOR,
    DISJUNCTION_OPERATOR,
    NIL_COALESCING_OPERATOR,
    EQUAL_SIGN,
    EQ_EQ,
    PLUS_THEN_WS,
    MINUS_THEN_WS,
    BANG,
    THROWS_KEYWORD,
    RETHROWS_KEYWORD,
    DEFAULT_KEYWORD,
    WHERE_KEYWORD,
    ELSE_KEYWORD,
    CATCH_KEYWORD,
    AS_KEYWORD,
    AS_QUEST,
    AS_BANG,
    ASYNC_KEYWORD,
];

const OP_SYMBOL_SUPPRESSOR: [u64; OPERATOR_COUNT] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    1u64 << FAKE_TRY_BANG, // BANG
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];

const RESERVED_OP_COUNT: usize = 31;

const RESERVED_OPS: [&[u8]; RESERVED_OP_COUNT] = [
    b"/\0", b"=\0", b"-\0", b"+\0", b"!\0", b"*\0", b"%\0", b"<\0", b">\0", b"&\0", b"|\0", b"^\0",
    b"?\0", b"~\0", b".\0", b"..\0", b"->\0", b"/*\0", b"*/\0", b"+=\0", b"-=\0", b"*=\0", b"/=\0",
    b"%=\0", b">>\0", b"<<\0", b"++\0", b"--\0", b"===\0", b"...\0", b"..<\0",
];

fn is_cross_semi_token(op: usize) -> bool {
    matches!(
        op,
        ARROW_OPERATOR
            | DOT_OPERATOR
            | EQUAL_SIGN
            | EQ_EQ
            | PLUS_THEN_WS
            | MINUS_THEN_WS
            | THROWS_KEYWORD
            | RETHROWS_KEYWORD
            | DEFAULT_KEYWORD
            | WHERE_KEYWORD
            | ELSE_KEYWORD
            | CATCH_KEYWORD
            | AS_KEYWORD
            | AS_QUEST
            | AS_BANG
            | ASYNC_KEYWORD
            | CUSTOM_OPERATOR
    )
}

const NON_CONSUMING_CROSS_SEMI_CHARS: [u32; 8] = [
    '?' as u32, ':' as u32, '{' as u32, '&' as u32, '|' as u32, '^' as u32, '<' as u32, '>' as u32,
];

// enum ParseDirective
#[derive(Clone, Copy, PartialEq)]
enum ParseDirective {
    ContinueParsingNothingFound,
    ContinueParsingTokenFound,
    ContinueParsingSlashConsumed,
    StopParsingNothingFound,
    StopParsingTokenFound,
    StopParsingEndOfFile,
}
use ParseDirective::*;

/// `struct ScannerState { uint32_t ongoing_raw_str_hash_count; }`
#[derive(Default)]
struct ScannerState {
    ongoing_raw_str_hash_count: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_swift_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(ScannerState::default())) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_swift_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        drop(Box::from_raw(payload as *mut ScannerState));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_swift_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> u32 {
    let state = &*(payload as *mut ScannerState);
    let hash_count = state.ongoing_raw_str_hash_count;
    *buffer.add(0) = ((hash_count >> 24) & 0xff) as u8 as c_char;
    *buffer.add(1) = ((hash_count >> 16) & 0xff) as u8 as c_char;
    *buffer.add(2) = ((hash_count >> 8) & 0xff) as u8 as c_char;
    *buffer.add(3) = (hash_count & 0xff) as u8 as c_char;
    4
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_swift_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: u32,
) {
    if length < 4 {
        return;
    }
    // `(uint32_t) buffer[n]` on a signed `char` sign-extends, so bytes >= 0x80 smear into the
    // higher bits exactly as the C does.
    let b = |i: usize| (*buffer.add(i) as i8) as i32 as u32;
    let hash_count = (b(0) << 24) | (b(1) << 16) | (b(2) << 8) | b(3);
    let state = &mut *(payload as *mut ScannerState);
    state.ongoing_raw_str_hash_count = hash_count;
}

unsafe fn advance(lexer: *mut TSLexer) {
    ((*lexer).advance)(lexer, false);
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    ((*lexer).mark_end)(lexer);
}

/// `iswspace` in the C locale: ASCII only.
fn is_wspace(c: i32) -> bool {
    matches!(c, 9..=13 | 32)
}

/// `iswalnum` in the C locale: ASCII only.
fn is_walnum(c: i32) -> bool {
    matches!(c, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a)
}

fn should_treat_as_wspace(character: i32) -> bool {
    is_wspace(character) || ';' as i32 == character
}

fn any_reserved_ops(encountered: &[u8; RESERVED_OP_COUNT]) -> bool {
    encountered.iter().any(|&e| e == 2)
}

fn is_legal_custom_operator(char_idx: i32, first_char: i32, cur_char: i32) -> bool {
    let is_first_char = char_idx == 0;
    match cur_char as u8 as char {
        _ if cur_char > 0xff || cur_char < 0 => {
            is_legal_custom_operator_wide(is_first_char, cur_char)
        }
        '=' | '-' | '+' | '!' | '%' | '<' | '>' | '&' | '|' | '^' | '?' | '~' => true,
        '.' => is_first_char || first_char == '.' as i32,
        '*' | '/' => char_idx != 1 || first_char != '/' as i32,
        _ => is_legal_custom_operator_wide(is_first_char, cur_char),
    }
}

/// The `default:` branch of the C switch (also reached for any non-ASCII-operator character).
fn is_legal_custom_operator_wide(is_first_char: bool, c: i32) -> bool {
    if (0x00A1..=0x00A7).contains(&c)
        || c == 0x00A9
        || c == 0x00AB
        || c == 0x00AC
        || c == 0x00AE
        || (0x00B0..=0x00B1).contains(&c)
        || c == 0x00B6
        || c == 0x00BB
        || c == 0x00BF
        || c == 0x00D7
        || c == 0x00F7
        || (0x2016..=0x2017).contains(&c)
        || (0x2020..=0x2027).contains(&c)
        || (0x2030..=0x203E).contains(&c)
        || (0x2041..=0x2053).contains(&c)
        || (0x2055..=0x205E).contains(&c)
        || (0x2190..=0x23FF).contains(&c)
        || (0x2500..=0x2775).contains(&c)
        || (0x2794..=0x2BFF).contains(&c)
        || (0x2E00..=0x2E7F).contains(&c)
        || (0x3001..=0x3003).contains(&c)
        || (0x3008..=0x3020).contains(&c)
        || c == 0x3030
    {
        true
    } else if (0x0300..=0x036f).contains(&c)
        || (0x1DC0..=0x1DFF).contains(&c)
        || (0x20D0..=0x20FF).contains(&c)
        || (0xFE00..=0xFE0F).contains(&c)
        || (0xFE20..=0xFE2F).contains(&c)
        || (0xE0100..=0xE01EF).contains(&c)
    {
        !is_first_char
    } else {
        false
    }
}

/// Collects valid fixed operators that can begin with the character.
unsafe fn collect_fixed_operator_candidates(
    character: i32,
    valid: &dyn Fn(usize) -> bool,
    candidates: &mut [u8; 4],
) -> u32 {
    let mut count: u32 = 0;
    let mut push = |c: u8| {
        candidates[count as usize] = c;
        count += 1;
    };
    match u8::try_from(character).map(|c| c as char) {
        Ok('-') => {
            if valid(ARROW_OPERATOR) {
                push(OPERATOR_INDEX_ARROW);
            }
            if valid(MINUS_THEN_WS) {
                push(OPERATOR_INDEX_MINUS_THEN_WHITESPACE);
            }
        }
        Ok('.') => {
            if valid(DOT_OPERATOR) {
                push(OPERATOR_INDEX_DOT);
            }
        }
        Ok('&') => {
            if valid(CONJUNCTION_OPERATOR) {
                push(OPERATOR_INDEX_CONJUNCTION);
            }
        }
        Ok('|') => {
            if valid(DISJUNCTION_OPERATOR) {
                push(OPERATOR_INDEX_DISJUNCTION);
            }
        }
        Ok('?') => {
            if valid(NIL_COALESCING_OPERATOR) || valid(DOUBLE_OPTIONAL) {
                push(OPERATOR_INDEX_NIL_COALESCING);
            }
        }
        Ok('=') => {
            if valid(EQUAL_SIGN) {
                push(OPERATOR_INDEX_EQUAL_SIGN);
            }
            if valid(EQ_EQ) {
                push(OPERATOR_INDEX_EQUALITY);
            }
        }
        Ok('+') => {
            if valid(PLUS_THEN_WS) {
                push(OPERATOR_INDEX_PLUS_THEN_WHITESPACE);
            }
        }
        Ok('!') => {
            if valid(BANG) {
                push(OPERATOR_INDEX_BANG);
            }
        }
        Ok('t') => {
            if valid(THROWS_KEYWORD) {
                push(OPERATOR_INDEX_THROWS);
            }
        }
        Ok('r') => {
            if valid(RETHROWS_KEYWORD) {
                push(OPERATOR_INDEX_RETHROWS);
            }
        }
        Ok('d') => {
            if valid(DEFAULT_KEYWORD) {
                push(OPERATOR_INDEX_DEFAULT);
            }
        }
        Ok('w') => {
            if valid(WHERE_KEYWORD) {
                push(OPERATOR_INDEX_WHERE);
            }
        }
        Ok('e') => {
            if valid(ELSE_KEYWORD) {
                push(OPERATOR_INDEX_ELSE);
            }
        }
        Ok('c') => {
            if valid(CATCH_KEYWORD) {
                push(OPERATOR_INDEX_CATCH);
            }
        }
        Ok('a') => {
            if valid(AS_KEYWORD) {
                push(OPERATOR_INDEX_AS);
            }
            if valid(AS_QUEST) {
                push(OPERATOR_INDEX_AS_QUESTION);
            }
            if valid(AS_BANG) {
                push(OPERATOR_INDEX_AS_BANG);
            }
            if valid(ASYNC_KEYWORD) {
                push(OPERATOR_INDEX_ASYNC);
            }
        }
        _ => {}
    }
    count
}

unsafe fn eat_operators(
    lexer: *mut TSLexer,
    valid: &dyn Fn(usize) -> bool,
    mark_end_flag: bool,
    token_is_immediate: bool,
    prior_char: i32,
    symbol_result: &mut usize,
) -> bool {
    let first_char = if prior_char != 0 {
        prior_char
    } else {
        (*lexer).lookahead
    };
    let mut possible_custom_operator =
        valid(CUSTOM_OPERATOR) && is_legal_custom_operator(0, first_char, first_char);
    let mut possible_operators = [0u8; 4];
    let mut possible_operator_count =
        collect_fixed_operator_candidates(first_char, valid, &mut possible_operators);
    if possible_operator_count == 0 && !possible_custom_operator {
        return false;
    }

    // Uninitialised in the C unless a custom operator is possible; only read in that case.
    let mut reserved_operators = [0u8; RESERVED_OP_COUNT];
    if possible_custom_operator {
        for op_idx in 0..RESERVED_OP_COUNT {
            reserved_operators[op_idx] = (RESERVED_OPS[op_idx][0] as i32 == first_char) as u8;
        }
    }

    let mut last_examined_char = first_char;

    let mut str_idx: i32 = if prior_char != 0 { 1 } else { 0 };
    let mut full_match: i32 = -1;
    loop {
        let mut candidate_idx: u32 = 0;
        while candidate_idx < possible_operator_count {
            let op_idx = possible_operators[candidate_idx as usize] as usize;

            if OPERATORS[op_idx][str_idx as usize] == 0 {
                // Make sure that the operator is allowed to have the next character as its lookahead.
                let illegal_terminators = OP_ILLEGAL_TERMINATORS[op_idx];
                let la = (*lexer).lookahead;
                // 0: the '/'..'~' symbol case, 1: the '.' case, 2: default (C switch fall-through)
                let mut stage = if matches!(
                    la as u8 as char,
                    '/' | '='
                        | '-'
                        | '+'
                        | '!'
                        | '*'
                        | '%'
                        | '<'
                        | '>'
                        | '&'
                        | '|'
                        | '^'
                        | '?'
                        | '~'
                ) && (0..=0xff).contains(&la)
                {
                    0
                } else if la == '.' as i32 {
                    1
                } else {
                    2
                };
                let mut broke = false;
                if stage == 0 {
                    if illegal_terminators == OperatorSymbols {
                        broke = true;
                    } else {
                        stage = 1;
                    }
                }
                if !broke && stage == 1 {
                    if illegal_terminators == OperatorOrDot {
                        broke = true;
                    } else {
                        stage = 2;
                    }
                }
                if !broke && stage == 2 {
                    if (is_walnum(la) && illegal_terminators == Alphanumeric)
                        || (!is_wspace(la) && illegal_terminators == NonWhitespace)
                    {
                        // illegal terminator: no match
                    } else {
                        full_match = op_idx as i32;
                        if mark_end_flag {
                            mark_end(lexer);
                        }
                    }
                }

                possible_operator_count -= 1;
                possible_operators[candidate_idx as usize] =
                    possible_operators[possible_operator_count as usize];
                continue;
            }

            if OPERATORS[op_idx][str_idx as usize] as i32 != (*lexer).lookahead {
                possible_operator_count -= 1;
                possible_operators[candidate_idx as usize] =
                    possible_operators[possible_operator_count as usize];
                continue;
            }

            candidate_idx += 1;
        }

        if possible_custom_operator {
            for op_idx in 0..RESERVED_OP_COUNT {
                if reserved_operators[op_idx] == 0 {
                    continue;
                }

                if RESERVED_OPS[op_idx][str_idx as usize] == 0 {
                    reserved_operators[op_idx] = 0;
                    continue;
                }

                if RESERVED_OPS[op_idx][str_idx as usize] as i32 != (*lexer).lookahead {
                    reserved_operators[op_idx] = 0;
                    continue;
                }

                if RESERVED_OPS[op_idx][str_idx as usize + 1] == 0 {
                    reserved_operators[op_idx] = 2;
                    continue;
                }
            }
        }

        possible_custom_operator = possible_custom_operator
            && is_legal_custom_operator(str_idx, first_char, (*lexer).lookahead);

        let encountered_ops = possible_operator_count;
        if encountered_ops == 0 {
            if !possible_custom_operator {
                break;
            } else if mark_end_flag && full_match == -1 {
                mark_end(lexer);
            }
        }

        last_examined_char = (*lexer).lookahead;
        ((*lexer).advance)(lexer, false);
        str_idx += 1;

        if encountered_ops == 0
            && !is_legal_custom_operator(str_idx, first_char, (*lexer).lookahead)
        {
            break;
        }
    }

    if full_match != -1 {
        // A suppressing symbol valid in this position (e.g. `try!`'s fake bang) vetoes the match.
        let suppressing_symbols = OP_SYMBOL_SUPPRESSOR[full_match as usize];
        if suppressing_symbols != 0 {
            let mut suppressor: u64 = 0;
            while suppressor < TOKEN_COUNT {
                if suppressing_symbols & (1u64 << suppressor) == 0 {
                    suppressor += 1;
                    continue;
                }
                if valid(suppressor as usize) {
                    return false;
                }
                suppressor += 1;
            }
        }
        let mut matched_symbol = OP_SYMBOLS[full_match as usize];
        if matched_symbol == NIL_COALESCING_OPERATOR && valid(DOUBLE_OPTIONAL) {
            if !token_is_immediate && !valid(NIL_COALESCING_OPERATOR) {
                return false;
            }
            matched_symbol = if token_is_immediate {
                DOUBLE_OPTIONAL
            } else {
                NIL_COALESCING_OPERATOR
            };
        }
        *symbol_result = matched_symbol;
        return true;
    }

    if possible_custom_operator && !any_reserved_ops(&reserved_operators) {
        if (last_examined_char != '<' as i32 || is_wspace((*lexer).lookahead)) && mark_end_flag {
            mark_end(lexer);
        }
        *symbol_result = CUSTOM_OPERATOR;
        return true;
    }

    false
}

unsafe fn eat_comment(
    lexer: *mut TSLexer,
    mark_end_flag: bool,
    symbol_result: &mut usize,
) -> ParseDirective {
    if (*lexer).lookahead != '/' as i32 {
        return ContinueParsingNothingFound;
    }

    advance(lexer);

    if (*lexer).lookahead != '*' as i32 {
        return ContinueParsingSlashConsumed;
    }

    advance(lexer);

    let mut after_star = false;
    let mut nesting_depth: u32 = 1;
    loop {
        match (*lexer).lookahead {
            0 => return StopParsingEndOfFile,
            42 => {
                // '*'
                advance(lexer);
                after_star = true;
            }
            47 => {
                // '/'
                if after_star {
                    advance(lexer);
                    after_star = false;
                    nesting_depth = nesting_depth.wrapping_sub(1);
                    if nesting_depth == 0 {
                        if mark_end_flag {
                            mark_end(lexer);
                        }
                        *symbol_result = BLOCK_COMMENT;
                        return StopParsingTokenFound;
                    }
                } else {
                    advance(lexer);
                    after_star = false;
                    if (*lexer).lookahead == '*' as i32 {
                        nesting_depth += 1;
                        advance(lexer);
                    }
                }
            }
            _ => {
                advance(lexer);
                after_star = false;
            }
        }
    }
}

unsafe fn eat_whitespace(
    lexer: *mut TSLexer,
    valid: &dyn Fn(usize) -> bool,
    symbol_result: &mut usize,
) -> ParseDirective {
    let mut ws_directive = ContinueParsingNothingFound;
    let semi_is_valid = valid(IMPLICIT_SEMI) && valid(EXPLICIT_SEMI);
    let mut lookahead: u32;
    loop {
        lookahead = (*lexer).lookahead as u32;
        if !should_treat_as_wspace(lookahead as i32) {
            break;
        }
        if lookahead == ';' as u32 {
            if semi_is_valid {
                ws_directive = StopParsingTokenFound;
                ((*lexer).advance)(lexer, false);
            }

            break;
        }

        ((*lexer).advance)(lexer, true);

        mark_end(lexer);

        if ws_directive == ContinueParsingNothingFound
            && (lookahead == '\n' as u32 || lookahead == '\r' as u32)
        {
            ws_directive = ContinueParsingTokenFound;
        }
    }

    let mut any_comment;
    if ws_directive == ContinueParsingTokenFound && lookahead == '/' as u32 {
        let mut has_seen_single_comment = false;
        while (*lexer).lookahead == '/' as i32 {
            // Exploratory comment probe; see the C for the reasoning.
            let mut multiline_comment_result = 0usize;
            any_comment = eat_comment(lexer, false, &mut multiline_comment_result);
            if any_comment == StopParsingTokenFound {
                if !has_seen_single_comment {
                    mark_end(lexer);
                    *symbol_result = multiline_comment_result;
                    return StopParsingTokenFound;
                }
            } else if any_comment == StopParsingEndOfFile {
                return StopParsingEndOfFile;
            } else if any_comment == ContinueParsingSlashConsumed {
                return ContinueParsingSlashConsumed;
            } else if (*lexer).lookahead == '/' as i32 {
                has_seen_single_comment = true;
                while (*lexer).lookahead != '\n' as i32 && (*lexer).lookahead != 0 {
                    ((*lexer).advance)(lexer, true);
                }
            } else if is_wspace((*lexer).lookahead) {
                return StopParsingNothingFound;
            }

            // If we skipped through some comment, we're at whitespace now, so advance.
            while is_wspace((*lexer).lookahead) {
                ((*lexer).advance)(lexer, true);
            }
        }

        let mut operator_result = 0usize;
        let saw_operator = eat_operators(lexer, valid, false, false, 0, &mut operator_result);
        if saw_operator {
            return StopParsingNothingFound;
        } else {
            *symbol_result = IMPLICIT_SEMI;
            ws_directive = StopParsingTokenFound;
        }
    }

    // Characters that are not consumed but still suppress the semi.
    if ws_directive == ContinueParsingTokenFound {
        for i in 0..NON_CONSUMING_CROSS_SEMI_CHARS.len() {
            if NON_CONSUMING_CROSS_SEMI_CHARS[i] == lookahead {
                return ContinueParsingNothingFound;
            }
        }
    }

    if semi_is_valid && ws_directive != ContinueParsingNothingFound {
        let directive_is_valid = valid(DIRECTIVE_IF)
            || valid(DIRECTIVE_ELSEIF)
            || valid(DIRECTIVE_ELSE)
            || valid(DIRECTIVE_ENDIF);
        if lookahead == '#' as u32 && directive_is_valid {
            return ContinueParsingNothingFound;
        }

        *symbol_result = if lookahead == ';' as u32 {
            EXPLICIT_SEMI
        } else {
            IMPLICIT_SEMI
        };
        return ws_directive;
    }

    ContinueParsingNothingFound
}

const DIRECTIVE_COUNT: usize = 4;
const DIRECTIVES: [&[u8]; DIRECTIVE_COUNT] = [b"if\0", b"elseif\0", b"else\0", b"endif\0"];
const DIRECTIVE_SYMBOLS: [usize; DIRECTIVE_COUNT] = [
    DIRECTIVE_IF,
    DIRECTIVE_ELSEIF,
    DIRECTIVE_ELSE,
    DIRECTIVE_ENDIF,
];

unsafe fn find_possible_compiler_directive(lexer: *mut TSLexer) -> usize {
    let mut possible_directives = [true; DIRECTIVE_COUNT];

    let mut str_idx: usize = 0;
    let mut full_match: i32 = -1;
    loop {
        for dir_idx in 0..DIRECTIVE_COUNT {
            if !possible_directives[dir_idx] {
                continue;
            }

            // The C reads past the end of the literal here when the input ends right after a
            // complete directive (the NUL at EOF equals the terminator, so the candidate stays
            // alive and `str_idx` keeps growing into whatever follows in rodata). Reading past the
            // terminator is modelled as a non-matching byte: the candidate dies with `full_match`
            // already recorded, which is the only outcome a stray NUL could not change either.
            let expected_char: u8 = if str_idx < DIRECTIVES[dir_idx].len() {
                DIRECTIVES[dir_idx][str_idx]
            } else {
                0xff
            };
            if expected_char == 0 {
                full_match = dir_idx as i32;
                mark_end(lexer);
            }

            if expected_char as i32 != (*lexer).lookahead {
                possible_directives[dir_idx] = false;
                continue;
            }
        }

        let mut match_count: u8 = 0;
        for dir_idx in 0..DIRECTIVE_COUNT {
            if possible_directives[dir_idx] {
                match_count += 1;
            }
        }

        if match_count == 0 {
            break;
        }

        ((*lexer).advance)(lexer, false);
        str_idx += 1;
    }

    if full_match == -1 {
        // No compiler directive found, so just match the starting symbol
        return HASH_SYMBOL;
    }

    DIRECTIVE_SYMBOLS[full_match as usize]
}

unsafe fn eat_raw_str_part(
    state: &mut ScannerState,
    lexer: *mut TSLexer,
    valid: &dyn Fn(usize) -> bool,
    symbol_result: &mut usize,
) -> bool {
    let mut hash_count = state.ongoing_raw_str_hash_count;
    if !valid(RAW_STR_PART) {
        return false;
    } else if hash_count == 0 {
        // If this is a raw_str_part, it's the first one - look for hashes
        while (*lexer).lookahead == '#' as i32 {
            hash_count = hash_count.wrapping_add(1);
            advance(lexer);
        }

        if hash_count == 0 {
            return false;
        }

        if (*lexer).lookahead == '"' as i32 {
            advance(lexer);
        } else if hash_count == 1 {
            mark_end(lexer);
            *symbol_result = find_possible_compiler_directive(lexer);
            return true;
        } else {
            return false;
        }
    } else if valid(RAW_STR_CONTINUING_INDICATOR) {
        // End of an interpolation: nothing to do except continue.
    } else {
        return false;
    }

    while (*lexer).lookahead != 0 {
        let mut last_char: u8 = 0;
        mark_end(lexer);
        while (*lexer).lookahead != '#' as i32 && (*lexer).lookahead != 0 {
            last_char = (*lexer).lookahead as u8;
            advance(lexer);
            if last_char != b'\\' || (*lexer).lookahead == '\\' as i32 {
                mark_end(lexer);
            }
        }

        // We hit at least one hash - count them and see if they match.
        let mut current_hash_count: u32 = 0;
        while (*lexer).lookahead == '#' as i32 && current_hash_count < hash_count {
            current_hash_count += 1;
            advance(lexer);
        }

        if current_hash_count == hash_count {
            if last_char == b'\\' && (*lexer).lookahead == '(' as i32 {
                *symbol_result = RAW_STR_PART;
                state.ongoing_raw_str_hash_count = hash_count;
                return true;
            } else if last_char == b'"' {
                mark_end(lexer);
                *symbol_result = RAW_STR_END_PART;
                state.ongoing_raw_str_hash_count = 0;
                return true;
            }
        }
    }

    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_swift_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    let state = &mut *(payload as *mut ScannerState);
    let valid = |i: usize| *valid_symbols.add(i);

    // Consume any whitespace at the start.
    let token_is_immediate = !should_treat_as_wspace((*lexer).lookahead);
    let mut ws_result = 0usize;
    let ws_directive = eat_whitespace(lexer, &valid, &mut ws_result);
    if ws_directive == StopParsingTokenFound {
        (*lexer).result_symbol = ws_result as u16;
        return true;
    }

    if ws_directive == StopParsingNothingFound || ws_directive == StopParsingEndOfFile {
        return false;
    }

    let has_ws_result = ws_directive == ContinueParsingTokenFound;

    // Now consume comments (before custom operators so that those aren't treated as comments)
    let mut comment_result = 0usize;
    let comment = if ws_directive == ContinueParsingSlashConsumed {
        ws_directive
    } else {
        eat_comment(lexer, true, &mut comment_result)
    };
    if comment == StopParsingTokenFound {
        mark_end(lexer);
        (*lexer).result_symbol = comment_result as u16;
        return true;
    }

    if comment == StopParsingEndOfFile {
        return false;
    }
    // Now consume any operators that might cause our whitespace to be suppressed.
    let mut operator_result = 0usize;
    let saw_operator = eat_operators(
        lexer,
        &valid,
        !has_ws_result,
        token_is_immediate,
        if comment == ContinueParsingSlashConsumed {
            '/' as i32
        } else {
            0
        },
        &mut operator_result,
    );

    if saw_operator && (!has_ws_result || is_cross_semi_token(operator_result)) {
        (*lexer).result_symbol = operator_result as u16;
        if has_ws_result {
            mark_end(lexer);
        }
        return true;
    }

    if has_ws_result {
        // Don't `mark_end`, since we may have advanced through some operators.
        (*lexer).result_symbol = ws_result as u16;
        return true;
    }

    // Consumes any `#` it sees, even without a result: keep it last.
    let mut raw_str_result = 0usize;
    let saw_raw_str_part = eat_raw_str_part(state, lexer, &valid, &mut raw_str_result);
    if saw_raw_str_part {
        (*lexer).result_symbol = raw_str_result as u16;
        return true;
    }

    false
}
