//! CMap parser: tokenizes and parses CMap streams.
//!
//! Ported from org.apache.fontbox.cmap.CMapParser.

use super::cmap::{CMap, CidRange, CodespaceRange};

/// Parse a CMap from raw bytes (e.g., a ToUnicode stream or embedded CMap).
pub fn parse_cmap(data: &[u8]) -> CMap {
    let mut cmap = CMap::new();
    let mut parser = Parser::new(data);
    parser.parse(&mut cmap);
    // Sort CID ranges by `from` within each code-length bucket to enable binary search.
    cmap.finalize_cid_ranges();
    cmap
}

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Token {
    /// An operator/keyword (e.g., "begincodespacerange", "endbfchar").
    Operator(String),
    /// An integer number.
    Integer(i64),
    /// A hex-encoded byte string: <AABB>.
    HexBytes(Vec<u8>),
    /// A literal name: /Name.
    Name(String),
    /// A parenthesized string: (text).
    LiteralString(Vec<u8>),
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn parse(&mut self, cmap: &mut CMap) {
        let mut prev_tokens: Vec<Token> = Vec::new();

        while let Some(token) = self.next_token() {
            match &token {
                Token::Operator(op) => {
                    match op.as_str() {
                        "begincodespacerange" => {
                            let count = Self::prev_int(&prev_tokens).unwrap_or(0) as usize;
                            self.parse_codespace_range(cmap, count);
                        }
                        "beginbfchar" => {
                            let count = Self::prev_int(&prev_tokens).unwrap_or(0) as usize;
                            self.parse_bf_char(cmap, count);
                        }
                        "beginbfrange" => {
                            let count = Self::prev_int(&prev_tokens).unwrap_or(0) as usize;
                            self.parse_bf_range(cmap, count);
                        }
                        "begincidchar" => {
                            let count = Self::prev_int(&prev_tokens).unwrap_or(0) as usize;
                            self.parse_cid_char(cmap, count);
                        }
                        "begincidrange" => {
                            let count = Self::prev_int(&prev_tokens).unwrap_or(0) as usize;
                            self.parse_cid_range(cmap, count);
                        }
                        "endcmap" => break,
                        _ => {
                            // Handle metadata like /CMapName, /WMode
                            self.handle_metadata(cmap, op, &prev_tokens);
                        }
                    }
                    prev_tokens.clear();
                }
                _ => {
                    prev_tokens.push(token);
                }
            }
        }
    }

    fn handle_metadata(&self, cmap: &mut CMap, op: &str, prev: &[Token]) {
        match op {
            "def" | "defineresource" => {
                // Look for /CMapName (name) def  or  /WMode (int) def
                if prev.len() >= 2 {
                    if let Token::Name(name) = &prev[prev.len() - 2] {
                        match name.as_str() {
                            "CMapName" => {
                                if let Some(val) = Self::token_to_string(&prev[prev.len() - 1])
                                {
                                    cmap.name = val;
                                }
                            }
                            "WMode" => {
                                if let Token::Integer(n) = &prev[prev.len() - 1] {
                                    cmap.wmode = *n as i32;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Section parsers
    // -----------------------------------------------------------------------

    fn parse_codespace_range(&mut self, cmap: &mut CMap, count: usize) {
        for _ in 0..count {
            let start = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let end = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            if start.len() == end.len() && !start.is_empty() {
                cmap.add_codespace_range(CodespaceRange::new(start, end));
            }
        }
        // consume "endcodespacerange"
        self.skip_to_operator("endcodespacerange");
    }

    fn parse_bf_char(&mut self, cmap: &mut CMap, count: usize) {
        for _ in 0..count {
            let input_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let unicode = match self.next_token() {
                Some(Token::HexBytes(b)) => hex_bytes_to_unicode(&b),
                Some(Token::Name(n)) => {
                    // /name → look up in glyph list (simplified: just use name)
                    super::glyph_list::DEFAULT.to_unicode(&n).unwrap_or_default()
                }
                _ => continue,
            };

            if !unicode.is_empty() {
                let code = bytes_to_u32(&input_bytes);
                let length = input_bytes.len();
                cmap.add_char_mapping(code, length, unicode);
            }
        }
        self.skip_to_operator("endbfchar");
    }

    fn parse_bf_range(&mut self, cmap: &mut CMap, count: usize) {
        for _ in 0..count {
            let start_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let end_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };

            let length = start_bytes.len();
            let start = bytes_to_u32(&start_bytes);
            let end = bytes_to_u32(&end_bytes);

            // Third token can be an array [...] or a hex string
            match self.peek_byte() {
                Some(b'[') => {
                    // Array of individual Unicode values
                    self.advance(); // skip '['
                    for code in start..=end {
                        match self.next_token() {
                            Some(Token::HexBytes(b)) => {
                                let unicode = hex_bytes_to_unicode(&b);
                                if !unicode.is_empty() {
                                    cmap.add_char_mapping(code, length, unicode);
                                }
                            }
                            _ => break,
                        }
                    }
                    // skip ']'
                    self.skip_whitespace();
                    if self.peek_byte() == Some(b']') {
                        self.advance();
                    }
                }
                _ => {
                    // Sequential: base Unicode value, incremented for range
                    let base_bytes = match self.next_token() {
                        Some(Token::HexBytes(b)) => b,
                        _ => continue,
                    };

                    let mut unicode_val = bytes_to_u32(&base_bytes);
                    let unicode_byte_len = base_bytes.len();

                    for code in start..=end {
                        let unicode = u32_to_unicode(unicode_val, unicode_byte_len);
                        if !unicode.is_empty() {
                            cmap.add_char_mapping(code, length, unicode);
                        }
                        unicode_val += 1;
                    }
                }
            }
        }
        self.skip_to_operator("endbfrange");
    }

    fn parse_cid_char(&mut self, cmap: &mut CMap, count: usize) {
        for _ in 0..count {
            let input_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let cid = match self.next_token() {
                Some(Token::Integer(n)) => n as u32,
                _ => continue,
            };

            let code = bytes_to_u32(&input_bytes);
            let length = input_bytes.len();
            cmap.add_cid_mapping(code, length, cid);
        }
        self.skip_to_operator("endcidchar");
    }

    fn parse_cid_range(&mut self, cmap: &mut CMap, count: usize) {
        for _ in 0..count {
            let start_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let end_bytes = match self.next_token() {
                Some(Token::HexBytes(b)) => b,
                _ => continue,
            };
            let cid_start = match self.next_token() {
                Some(Token::Integer(n)) => n as u32,
                _ => continue,
            };

            let length = start_bytes.len();
            let from = bytes_to_u32(&start_bytes);
            let to = bytes_to_u32(&end_bytes);

            if from == to {
                // Single mapping
                cmap.add_cid_mapping(from, length, cid_start);
            } else {
                cmap.add_cid_range(CidRange {
                    from,
                    to,
                    cid_start,
                    code_length: length,
                });
            }
        }
        self.skip_to_operator("endcidrange");
    }

    // -----------------------------------------------------------------------
    // Tokenizer
    // -----------------------------------------------------------------------

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.data.len() {
            return None;
        }

        let ch = self.data[self.pos];
        match ch {
            b'<' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                    // << dictionary start — skip it
                    self.pos += 2;
                    Some(Token::Operator("<<".to_string()))
                } else {
                    self.read_hex_string()
                }
            }
            b'>' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                    self.pos += 2;
                    Some(Token::Operator(">>".to_string()))
                } else {
                    self.pos += 1;
                    self.next_token()
                }
            }
            b'(' => self.read_literal_string(),
            b'/' => self.read_name(),
            b'0'..=b'9' | b'-' | b'+' => self.read_number(),
            _ => self.read_keyword(),
        }
    }

    fn read_hex_string(&mut self) -> Option<Token> {
        self.pos += 1; // skip '<'
        let mut bytes = Vec::new();
        let mut hi: Option<u8> = None;

        while self.pos < self.data.len() {
            let ch = self.data[self.pos];
            self.pos += 1;

            if ch == b'>' {
                break;
            }

            let nibble = match ch {
                b'0'..=b'9' => ch - b'0',
                b'a'..=b'f' => ch - b'a' + 10,
                b'A'..=b'F' => ch - b'A' + 10,
                _ => continue, // skip whitespace etc.
            };

            match hi {
                None => hi = Some(nibble),
                Some(h) => {
                    bytes.push((h << 4) | nibble);
                    hi = None;
                }
            }
        }
        // Trailing nibble (pad with 0)
        if let Some(h) = hi {
            bytes.push(h << 4);
        }

        Some(Token::HexBytes(bytes))
    }

    fn read_literal_string(&mut self) -> Option<Token> {
        self.pos += 1; // skip '('
        let mut bytes = Vec::new();
        let mut depth = 1;

        while self.pos < self.data.len() && depth > 0 {
            let ch = self.data[self.pos];
            self.pos += 1;

            match ch {
                b'(' => {
                    depth += 1;
                    bytes.push(ch);
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        bytes.push(ch);
                    }
                }
                b'\\' => {
                    if self.pos < self.data.len() {
                        let esc = self.data[self.pos];
                        self.pos += 1;
                        match esc {
                            b'n' => bytes.push(b'\n'),
                            b'r' => bytes.push(b'\r'),
                            b't' => bytes.push(b'\t'),
                            b'(' => bytes.push(b'('),
                            b')' => bytes.push(b')'),
                            b'\\' => bytes.push(b'\\'),
                            b'0'..=b'7' => {
                                // Octal escape
                                let mut val = (esc - b'0') as u32;
                                for _ in 0..2 {
                                    if self.pos < self.data.len()
                                        && self.data[self.pos] >= b'0'
                                        && self.data[self.pos] <= b'7'
                                    {
                                        val = val * 8 + (self.data[self.pos] - b'0') as u32;
                                        self.pos += 1;
                                    } else {
                                        break;
                                    }
                                }
                                bytes.push(val as u8);
                            }
                            _ => bytes.push(esc),
                        }
                    }
                }
                _ => bytes.push(ch),
            }
        }

        Some(Token::LiteralString(bytes))
    }

    fn read_name(&mut self) -> Option<Token> {
        self.pos += 1; // skip '/'
        let start = self.pos;
        while self.pos < self.data.len() && !is_delimiter(self.data[self.pos]) {
            self.pos += 1;
        }
        let name = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        Some(Token::Name(name))
    }

    fn read_number(&mut self) -> Option<Token> {
        let start = self.pos;
        if self.data[self.pos] == b'-' || self.data[self.pos] == b'+' {
            self.pos += 1;
        }
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.data[start..self.pos]).unwrap_or("0");
        let n = s.parse::<i64>().unwrap_or(0);
        Some(Token::Integer(n))
    }

    fn read_keyword(&mut self) -> Option<Token> {
        let start = self.pos;
        while self.pos < self.data.len()
            && !is_whitespace(self.data[self.pos])
            && !is_delimiter(self.data[self.pos])
        {
            self.pos += 1;
        }
        if self.pos == start {
            // Single delimiter char we don't handle — skip it
            self.pos += 1;
            return self.next_token();
        }
        let kw = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        Some(Token::Operator(kw))
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && is_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.pos < self.data.len() && self.data[self.pos] == b'%' {
                // Skip comment line
                while self.pos < self.data.len() && self.data[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn skip_to_operator(&mut self, target: &str) {
        while let Some(token) = self.next_token() {
            if let Token::Operator(op) = &token {
                if op == target {
                    return;
                }
            }
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        let mut i = self.pos;
        while i < self.data.len() && is_whitespace(self.data[i]) {
            i += 1;
        }
        if i < self.data.len() {
            Some(self.data[i])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.skip_whitespace();
        if self.pos < self.data.len() {
            self.pos += 1;
        }
    }

    fn prev_int(tokens: &[Token]) -> Option<i64> {
        tokens.iter().rev().find_map(|t| {
            if let Token::Integer(n) = t {
                Some(*n)
            } else {
                None
            }
        })
    }

    fn token_to_string(token: &Token) -> Option<String> {
        match token {
            Token::Name(n) => Some(n.clone()),
            Token::LiteralString(b) => Some(String::from_utf8_lossy(b).to_string()),
            Token::Operator(op) => Some(op.clone()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x00 | 0x0C)
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\t' | b'\n' | b'\r' | 0x00 | 0x0C | b'(' | b')' | b'<' | b'>' | b'[' | b']'
            | b'{' | b'}' | b'/' | b'%'
    )
}

/// Convert big-endian bytes to a u32 value.
fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for &b in bytes {
        val = (val << 8) | (b as u32);
    }
    val
}

/// Convert a hex-encoded byte sequence (from a CMap) to a Unicode string.
///
/// The bytes are interpreted as big-endian UTF-16BE codepoints.
fn hex_bytes_to_unicode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // If odd number of bytes, treat as single-byte values
    if bytes.len() % 2 != 0 {
        return bytes
            .iter()
            .filter_map(|&b| char::from_u32(b as u32))
            .collect();
    }

    // Decode as UTF-16BE
    let u16_values: Vec<u16> = bytes
        .chunks(2)
        .map(|chunk| ((chunk[0] as u16) << 8) | (chunk[1] as u16))
        .collect();

    String::from_utf16_lossy(&u16_values)
}

/// Convert a u32 value back to a Unicode string, respecting byte length.
fn u32_to_unicode(val: u32, byte_len: usize) -> String {
    if byte_len <= 2 {
        // Single BMP character
        if let Some(ch) = char::from_u32(val) {
            return ch.to_string();
        }
    }

    // Multi-byte: treat as UTF-16BE
    if byte_len == 4 {
        let hi = ((val >> 16) & 0xFFFF) as u16;
        let lo = (val & 0xFFFF) as u16;
        if hi == 0 {
            // Just a single codepoint
            if let Some(ch) = char::from_u32(lo as u32) {
                return ch.to_string();
            }
        }
        // Could be a surrogate pair
        let u16s = [hi, lo];
        return String::from_utf16_lossy(&u16s);
    }

    // Fallback
    char::from_u32(val)
        .map(|ch| ch.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_bfchar() {
        let cmap_data = br#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /TestCMap def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
3 beginbfchar
<0041> <0041>
<0042> <0042>
<AC00> <AC00>
endbfchar
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.name, "TestCMap");
        assert_eq!(cmap.to_unicode(0x0041, 2), Some("A"));
        assert_eq!(cmap.to_unicode(0x0042, 2), Some("B"));
        assert_eq!(cmap.to_unicode(0xAC00, 2), Some("\u{AC00}")); // Korean 가
    }

    #[test]
    fn test_parse_bfrange_sequential() {
        let cmap_data = br#"
1 begincodespacerange
<00> <FF>
endcodespacerange
1 beginbfrange
<41> <5A> <0041>
endbfrange
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.to_unicode(0x41, 1), Some("A"));
        assert_eq!(cmap.to_unicode(0x42, 1), Some("B"));
        assert_eq!(cmap.to_unicode(0x5A, 1), Some("Z"));
        assert_eq!(cmap.to_unicode(0x5B, 1), None);
    }

    #[test]
    fn test_parse_bfrange_array() {
        let cmap_data = br#"
1 begincodespacerange
<00> <FF>
endcodespacerange
1 beginbfrange
<01> <03> [<0041> <0042> <0043>]
endbfrange
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.to_unicode(0x01, 1), Some("A"));
        assert_eq!(cmap.to_unicode(0x02, 1), Some("B"));
        assert_eq!(cmap.to_unicode(0x03, 1), Some("C"));
    }

    #[test]
    fn test_parse_cidrange() {
        let cmap_data = br#"
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 begincidrange
<0000> <00FF> 0
endcidrange
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.to_cid(0x0000, 2), Some(0));
        assert_eq!(cmap.to_cid(0x0041, 2), Some(0x41));
        assert_eq!(cmap.to_cid(0x00FF, 2), Some(0xFF));
    }

    #[test]
    fn test_parse_cidchar() {
        let cmap_data = br#"
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 begincidchar
<0041> 100
<0042> 200
endcidchar
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.to_cid(0x0041, 2), Some(100));
        assert_eq!(cmap.to_cid(0x0042, 2), Some(200));
    }

    #[test]
    fn test_codespace_range() {
        let cmap_data = br#"
2 begincodespacerange
<00> <80>
<8140> <9FFC>
endcodespacerange
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        let ranges = cmap.codespace_ranges();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].code_length, 1);
        assert_eq!(ranges[1].code_length, 2);
        assert_eq!(cmap.min_code_length(), 1);
        assert_eq!(cmap.max_code_length(), 2);
    }

    #[test]
    fn test_wmode() {
        let cmap_data = br#"
/WMode 1 def
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.wmode, 1);
    }

    #[test]
    fn test_hex_bytes_to_unicode() {
        assert_eq!(hex_bytes_to_unicode(&[0x00, 0x41]), "A");
        assert_eq!(hex_bytes_to_unicode(&[0xAC, 0x00]), "\u{AC00}");
        assert_eq!(hex_bytes_to_unicode(&[0x00, 0x48, 0x00, 0x69]), "Hi");
    }

    #[test]
    fn test_bytes_to_u32() {
        assert_eq!(bytes_to_u32(&[0x41]), 0x41);
        assert_eq!(bytes_to_u32(&[0x00, 0x41]), 0x41);
        assert_eq!(bytes_to_u32(&[0xAC, 0x00]), 0xAC00);
    }

    #[test]
    fn test_comments_are_skipped() {
        let cmap_data = br#"
% This is a comment
1 begincodespacerange
<00> <FF>
endcodespacerange
% Another comment
1 beginbfchar
<41> <0041>
endbfchar
endcmap
"#;
        let cmap = parse_cmap(cmap_data);
        assert_eq!(cmap.to_unicode(0x41, 1), Some("A"));
    }
}
