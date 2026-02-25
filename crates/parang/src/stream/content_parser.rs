//! Fast content stream parser for PDF text extraction.
//!
//! Replaces lopdf's nom-based `Content::decode` with a hand-optimized
//! single-pass parser. Key advantages:
//!
//! - **First-byte dispatch**: O(1) token classification vs nom's 9-way `alt()`
//! - **Zero intermediate allocations**: streams operands to callback, reusing buffer
//! - **Native NUL/inline-image handling**: no separate strip pass or multi-attempt decode
//! - **No backtracking**: each byte processed exactly once

use lopdf::{Dictionary, Object, StringFormat};

use crate::Result;

/// Parse a PDF content stream, calling `handler` for each operation.
///
/// The handler receives `(operator, operands)` for each PDF operation.
/// Operands are borrowed from a reusable buffer that is cleared after each call.
pub fn parse_content_stream<F>(data: &[u8], mut handler: F) -> Result<()>
where
    F: FnMut(&str, &[Object]) -> Result<()>,
{
    let mut operands: Vec<Object> = Vec::with_capacity(8);
    let mut pos = 0;
    let len = data.len();

    while pos < len {
        pos = skip_ws(data, pos);
        if pos >= len {
            break;
        }

        let b = data[pos];

        match b {
            // Name: /...
            b'/' => {
                let (name, new_pos) = parse_name(data, pos + 1);
                operands.push(Object::Name(name));
                pos = new_pos;
            }
            // Literal string: (...)
            b'(' => {
                let (s, new_pos) = parse_literal_string(data, pos + 1);
                operands.push(Object::string_literal(s));
                pos = new_pos;
            }
            // Dictionary <<...>> (must check before hex string)
            b'<' if pos + 1 < len && data[pos + 1] == b'<' => {
                let (dict, new_pos) = parse_dictionary(data, pos + 2);
                operands.push(Object::Dictionary(dict));
                pos = new_pos;
            }
            // Hex string: <...>
            b'<' => {
                let (s, new_pos) = parse_hex_string(data, pos + 1);
                operands.push(Object::String(s, StringFormat::Hexadecimal));
                pos = new_pos;
            }
            // Array: [...]
            b'[' => {
                let (arr, new_pos) = parse_array(data, pos + 1);
                operands.push(Object::Array(arr));
                pos = new_pos;
            }
            // Number: starts with digit or sign
            b'0'..=b'9' | b'+' | b'-' => {
                let (num, new_pos) = parse_number(data, pos);
                operands.push(num);
                pos = new_pos;
            }
            // Number: starts with decimal point (.5 etc)
            b'.' if pos + 1 < len && data[pos + 1].is_ascii_digit() => {
                let (num, new_pos) = parse_number(data, pos);
                operands.push(num);
                pos = new_pos;
            }
            // Alphabetic or operator chars: keyword or operator
            _ if b.is_ascii_alphabetic() || b == b'\'' || b == b'"' || b == b'*' => {
                let start = pos;
                pos += 1;
                while pos < len {
                    let c = data[pos];
                    if c.is_ascii_alphabetic() || c == b'*' || c == b'\'' || c == b'"' {
                        pos += 1;
                    } else {
                        break;
                    }
                }
                let word = &data[start..pos];

                match word {
                    b"true" => operands.push(Object::Boolean(true)),
                    b"false" => operands.push(Object::Boolean(false)),
                    b"null" => operands.push(Object::Null),
                    b"BI" => {
                        // Inline image: skip to EI
                        pos = skip_inline_image(data, pos);
                        operands.clear();
                    }
                    _ => {
                        // Operator: emit operation
                        // Safety: word contains only ASCII alphabetic + *'" chars
                        let op_str = unsafe { std::str::from_utf8_unchecked(word) };
                        handler(op_str, &operands)?;
                        operands.clear();
                    }
                }
            }
            // Skip NUL bytes and other unknown characters
            _ => {
                pos += 1;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helper parsers
// ---------------------------------------------------------------------------

/// Skip whitespace (space, tab, CR, LF, form-feed, NUL) and comments.
#[inline]
fn skip_ws(data: &[u8], mut pos: usize) -> usize {
    let len = data.len();
    while pos < len {
        match data[pos] {
            b' ' | b'\t' | b'\r' | b'\n' | 0 | 12 => pos += 1,
            b'%' => {
                // Comment: skip to end of line
                pos += 1;
                while pos < len && data[pos] != b'\r' && data[pos] != b'\n' {
                    pos += 1;
                }
            }
            _ => break,
        }
    }
    pos
}

/// Parse a PDF name after the leading `/`.
fn parse_name(data: &[u8], mut pos: usize) -> (Vec<u8>, usize) {
    let len = data.len();
    let mut name = Vec::with_capacity(16);
    while pos < len {
        let b = data[pos];
        if b == b'#' && pos + 2 < len {
            if let (Some(h), Some(l)) = (hex_val(data[pos + 1]), hex_val(data[pos + 2])) {
                name.push((h << 4) | l);
                pos += 3;
                continue;
            }
        }
        if is_regular(b) {
            name.push(b);
            pos += 1;
        } else {
            break;
        }
    }
    (name, pos)
}

/// Parse a literal string after the opening `(`.
/// Handles balanced parentheses, escape sequences, and octal escapes.
fn parse_literal_string(data: &[u8], mut pos: usize) -> (Vec<u8>, usize) {
    let len = data.len();
    let mut result = Vec::with_capacity(64);
    let mut depth: u32 = 1;

    while pos < len && depth > 0 {
        let b = data[pos];
        match b {
            b'(' => {
                depth += 1;
                result.push(b'(');
                pos += 1;
            }
            b')' => {
                depth -= 1;
                if depth > 0 {
                    result.push(b')');
                }
                pos += 1;
            }
            b'\\' => {
                pos += 1;
                if pos >= len {
                    break;
                }
                match data[pos] {
                    b'n' => {
                        result.push(b'\n');
                        pos += 1;
                    }
                    b'r' => {
                        result.push(b'\r');
                        pos += 1;
                    }
                    b't' => {
                        result.push(b'\t');
                        pos += 1;
                    }
                    b'b' => {
                        result.push(8);
                        pos += 1;
                    }
                    b'f' => {
                        result.push(12);
                        pos += 1;
                    }
                    b'(' => {
                        result.push(b'(');
                        pos += 1;
                    }
                    b')' => {
                        result.push(b')');
                        pos += 1;
                    }
                    b'\\' => {
                        result.push(b'\\');
                        pos += 1;
                    }
                    b'\r' => {
                        pos += 1;
                        if pos < len && data[pos] == b'\n' {
                            pos += 1;
                        }
                    }
                    b'\n' => {
                        pos += 1;
                    }
                    b'0'..=b'7' => {
                        let mut val = (data[pos] - b'0') as u16;
                        pos += 1;
                        if pos < len && (b'0'..=b'7').contains(&data[pos]) {
                            val = val * 8 + (data[pos] - b'0') as u16;
                            pos += 1;
                            if pos < len && (b'0'..=b'7').contains(&data[pos]) {
                                val = val * 8 + (data[pos] - b'0') as u16;
                                pos += 1;
                            }
                        }
                        result.push(val as u8);
                    }
                    other => {
                        result.push(other);
                        pos += 1;
                    }
                }
            }
            b'\r' => {
                // Normalize \r and \r\n to \n
                result.push(b'\n');
                pos += 1;
                if pos < len && data[pos] == b'\n' {
                    pos += 1;
                }
            }
            _ => {
                result.push(b);
                pos += 1;
            }
        }
    }

    (result, pos)
}

/// Parse a hex string after the opening `<`.
fn parse_hex_string(data: &[u8], mut pos: usize) -> (Vec<u8>, usize) {
    let len = data.len();
    let mut result = Vec::with_capacity(64);
    let mut high: Option<u8> = None;

    while pos < len {
        let b = data[pos];
        pos += 1;

        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            b'>' => break,
            _ => continue,
        };

        match high {
            None => high = Some(nibble),
            Some(h) => {
                result.push((h << 4) | nibble);
                high = None;
            }
        }
    }

    if let Some(h) = high {
        result.push(h << 4);
    }

    (result, pos)
}

/// Parse a number (integer or real) using fast hand-written parsers.
/// PDF numbers are simple: [+-]digits[.digits] — no scientific notation.
fn parse_number(data: &[u8], mut pos: usize) -> (Object, usize) {
    let start = pos;
    let len = data.len();
    let mut has_dot = false;

    // Optional sign
    let negative = if pos < len && data[pos] == b'-' {
        pos += 1;
        true
    } else if pos < len && data[pos] == b'+' {
        pos += 1;
        false
    } else {
        false
    };

    // Digits before decimal point
    let int_start = pos;
    let mut int_val: i64 = 0;
    while pos < len && data[pos].is_ascii_digit() {
        int_val = int_val * 10 + (data[pos] - b'0') as i64;
        pos += 1;
    }

    // Decimal point + fractional digits
    if pos < len && data[pos] == b'.' {
        has_dot = true;
        pos += 1;
        let frac_start = pos;
        let mut frac_val: u64 = 0;
        while pos < len && data[pos].is_ascii_digit() {
            frac_val = frac_val * 10 + (data[pos] - b'0') as u64;
            pos += 1;
        }

        // Edge case: sign only or no digits at all
        if pos == int_start && frac_val == 0 && pos == frac_start {
            return (Object::Integer(0), pos.max(start + 1));
        }

        let frac_digits = pos - frac_start;
        // Precomputed powers of 10 for up to 9 fractional digits (covers all PDF use cases)
        static POW10: [f32; 10] = [
            1.0, 10.0, 100.0, 1_000.0, 10_000.0, 100_000.0,
            1_000_000.0, 10_000_000.0, 100_000_000.0, 1_000_000_000.0,
        ];
        let divisor = if frac_digits < 10 {
            POW10[frac_digits]
        } else {
            10.0f32.powi(frac_digits as i32)
        };
        let result = int_val as f32 + frac_val as f32 / divisor;
        (Object::Real(if negative { -result } else { result }), pos)
    } else {
        // Edge case: sign only (e.g. a stray '-' not followed by digits)
        if pos == int_start {
            return (Object::Integer(0), pos.max(start + 1));
        }
        (Object::Integer(if negative { -int_val } else { int_val }), pos)
    }
}

/// Parse an array after the opening `[`.
fn parse_array(data: &[u8], mut pos: usize) -> (Vec<Object>, usize) {
    let len = data.len();
    let mut items = Vec::with_capacity(16);

    loop {
        pos = skip_ws(data, pos);
        if pos >= len || data[pos] == b']' {
            if pos < len {
                pos += 1;
            }
            break;
        }

        let (obj, new_pos) = parse_object(data, pos);
        if new_pos <= pos {
            pos += 1; // prevent infinite loop on unparseable byte
            continue;
        }
        items.push(obj);
        pos = new_pos;
    }

    (items, pos)
}

/// Parse a dictionary after the opening `<<`.
fn parse_dictionary(data: &[u8], mut pos: usize) -> (Dictionary, usize) {
    let len = data.len();
    let mut dict = Dictionary::new();

    loop {
        pos = skip_ws(data, pos);
        if pos >= len {
            break;
        }
        // Check for closing >>
        if data[pos] == b'>' && pos + 1 < len && data[pos + 1] == b'>' {
            pos += 2;
            break;
        }
        // Key must be a name
        if data[pos] != b'/' {
            pos += 1;
            continue;
        }
        let (key, new_pos) = parse_name(data, pos + 1);
        pos = skip_ws(data, new_pos);
        if pos >= len {
            break;
        }
        // Value
        let (value, new_pos) = parse_object(data, pos);
        if new_pos <= pos {
            pos += 1;
            continue;
        }
        dict.set(key, value);
        pos = new_pos;
    }

    (dict, pos)
}

/// Parse a single PDF object (operand) at the given position.
fn parse_object(data: &[u8], pos: usize) -> (Object, usize) {
    let len = data.len();
    if pos >= len {
        return (Object::Null, pos);
    }

    let b = data[pos];
    match b {
        b'/' => {
            let (name, new_pos) = parse_name(data, pos + 1);
            (Object::Name(name), new_pos)
        }
        b'(' => {
            let (s, new_pos) = parse_literal_string(data, pos + 1);
            (Object::string_literal(s), new_pos)
        }
        b'<' if pos + 1 < len && data[pos + 1] == b'<' => {
            let (dict, new_pos) = parse_dictionary(data, pos + 2);
            (Object::Dictionary(dict), new_pos)
        }
        b'<' => {
            let (s, new_pos) = parse_hex_string(data, pos + 1);
            (Object::String(s, StringFormat::Hexadecimal), new_pos)
        }
        b'[' => {
            let (arr, new_pos) = parse_array(data, pos + 1);
            (Object::Array(arr), new_pos)
        }
        b'0'..=b'9' | b'+' | b'-' => parse_number(data, pos),
        b'.' if pos + 1 < len && data[pos + 1].is_ascii_digit() => parse_number(data, pos),
        b't' if data[pos..].starts_with(b"true") => {
            let end = pos + 4;
            if end >= len || !data[end].is_ascii_alphabetic() {
                (Object::Boolean(true), end)
            } else {
                (Object::Null, pos + 1)
            }
        }
        b'f' if data[pos..].starts_with(b"false") => {
            let end = pos + 5;
            if end >= len || !data[end].is_ascii_alphabetic() {
                (Object::Boolean(false), end)
            } else {
                (Object::Null, pos + 1)
            }
        }
        b'n' if data[pos..].starts_with(b"null") => {
            let end = pos + 4;
            if end >= len || !data[end].is_ascii_alphabetic() {
                (Object::Null, end)
            } else {
                (Object::Null, pos + 1)
            }
        }
        _ => (Object::Null, pos + 1),
    }
}

/// Skip an inline image (BI ... ID <data> EI).
fn skip_inline_image(data: &[u8], mut pos: usize) -> usize {
    let len = data.len();

    // Phase 1: skip to ID marker
    while pos + 1 < len {
        if data[pos] == b'I' && data[pos + 1] == b'D' {
            let preceded = pos == 0 || is_ws_byte(data[pos - 1]);
            if preceded {
                pos += 2;
                // Skip one whitespace byte after ID
                if pos < len && is_ws_byte(data[pos]) {
                    pos += 1;
                }
                break;
            }
        }
        pos += 1;
    }

    // Phase 2: skip binary data to EI marker
    while pos + 1 < len {
        if data[pos] == b'E' && data[pos + 1] == b'I' {
            let preceded = pos > 0 && (is_ws_byte(data[pos - 1]) || data[pos - 1] == 0);
            let followed = pos + 2 >= len || is_ws_byte(data[pos + 2]);
            if preceded && followed {
                return pos + 2;
            }
        }
        pos += 1;
    }

    len
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

#[inline]
fn is_ws_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0 | 12)
}

#[inline]
fn is_regular(b: u8) -> bool {
    !matches!(
        b,
        b' ' | b'\t'
            | b'\r'
            | b'\n'
            | 0
            | 12
            | b'('
            | b')'
            | b'<'
            | b'>'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'/'
            | b'%'
    )
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
