//! CMap (Character Map) data structure for PDF font encoding.
//!
//! Ported from org.apache.fontbox.cmap.CMap.

use std::collections::HashMap;

/// A CMap defines mappings from character codes to Unicode strings and/or CIDs.
///
/// Character codes can be 1-4 bytes. Mappings are organized by code byte length
/// for efficient lookup.
#[derive(Debug, Clone)]
pub struct CMap {
    /// CMap name.
    pub name: String,
    /// Writing mode: 0=horizontal, 1=vertical.
    pub wmode: i32,

    /// Codespace ranges defining valid code boundaries.
    codespace_ranges: Vec<CodespaceRange>,

    /// Code → Unicode mappings, keyed by byte length (1-4).
    char_to_unicode: [HashMap<u32, String>; 4],

    /// Code → CID direct mappings, keyed by byte length (1-4).
    code_to_cid: [HashMap<u32, u32>; 4],
    /// Code → CID range mappings.
    cid_ranges: Vec<CidRange>,

    /// Reverse mapping: Unicode string → code bytes.
    unicode_to_code: HashMap<String, Vec<u8>>,

    /// Minimum/maximum code lengths.
    min_code_length: usize,
    max_code_length: usize,
}

impl CMap {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            wmode: 0,
            codespace_ranges: Vec::new(),
            char_to_unicode: [
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ],
            code_to_cid: [
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ],
            cid_ranges: Vec::new(),
            unicode_to_code: HashMap::new(),
            min_code_length: 4,
            max_code_length: 0,
        }
    }

    /// Look up a character code and return its Unicode string.
    pub fn to_unicode(&self, code: u32, length: usize) -> Option<&str> {
        if length == 0 || length > 4 {
            return None;
        }
        self.char_to_unicode[length - 1]
            .get(&code)
            .map(|s| s.as_str())
    }

    /// Convenience: try all code lengths (1, 2, 3, 4) to find a Unicode mapping.
    pub fn to_unicode_any(&self, code: u32) -> Option<&str> {
        for len in 0..4 {
            if let Some(s) = self.char_to_unicode[len].get(&code) {
                return Some(s.as_str());
            }
        }
        None
    }

    /// Look up a character code and return its CID.
    pub fn to_cid(&self, code: u32, length: usize) -> Option<u32> {
        if length == 0 || length > 4 {
            return None;
        }
        // Direct lookup first
        if let Some(&cid) = self.code_to_cid[length - 1].get(&code) {
            return Some(cid);
        }
        // Range lookup
        for range in &self.cid_ranges {
            if range.code_length == length && code >= range.from && code <= range.to {
                return Some(range.cid_start + (code - range.from));
            }
        }
        None
    }

    /// Get code bytes for a Unicode string (reverse lookup).
    pub fn get_codes_from_unicode(&self, unicode: &str) -> Option<&[u8]> {
        self.unicode_to_code.get(unicode).map(|v| v.as_slice())
    }

    /// Add a codespace range.
    pub fn add_codespace_range(&mut self, range: CodespaceRange) {
        let len = range.code_length;
        if len < self.min_code_length {
            self.min_code_length = len;
        }
        if len > self.max_code_length {
            self.max_code_length = len;
        }
        self.codespace_ranges.push(range);
    }

    /// Add a character code → Unicode mapping.
    pub fn add_char_mapping(&mut self, code: u32, length: usize, unicode: String) {
        if length == 0 || length > 4 {
            return;
        }
        // Reverse mapping
        let code_bytes = &code.to_be_bytes()[4 - length..];
        self.unicode_to_code
            .entry(unicode.clone())
            .or_insert_with(|| code_bytes.to_vec());
        self.char_to_unicode[length - 1].insert(code, unicode);
    }

    /// Add a character code → CID direct mapping.
    pub fn add_cid_mapping(&mut self, code: u32, length: usize, cid: u32) {
        if length == 0 || length > 4 {
            return;
        }
        self.code_to_cid[length - 1].insert(code, cid);
    }

    /// Add a CID range mapping.
    pub fn add_cid_range(&mut self, range: CidRange) {
        self.cid_ranges.push(range);
    }

    /// Get codespace ranges.
    pub fn codespace_ranges(&self) -> &[CodespaceRange] {
        &self.codespace_ranges
    }

    /// Check if a byte sequence matches any codespace range.
    pub fn matches_codespace(&self, code: &[u8]) -> bool {
        self.codespace_ranges
            .iter()
            .any(|r| r.matches(code))
    }

    /// Get the minimum code length.
    pub fn min_code_length(&self) -> usize {
        self.min_code_length
    }

    /// Get the maximum code length.
    pub fn max_code_length(&self) -> usize {
        self.max_code_length
    }

    /// Total number of Unicode mappings.
    pub fn unicode_mapping_count(&self) -> usize {
        self.char_to_unicode.iter().map(|m| m.len()).sum()
    }

    /// Total number of CID mappings (direct).
    pub fn cid_mapping_count(&self) -> usize {
        self.code_to_cid.iter().map(|m| m.len()).sum()
    }

    /// Merge another CMap's mappings into this one.
    pub fn merge(&mut self, other: &CMap) {
        for range in &other.codespace_ranges {
            self.add_codespace_range(range.clone());
        }
        for (i, map) in other.char_to_unicode.iter().enumerate() {
            for (&code, unicode) in map {
                self.char_to_unicode[i]
                    .entry(code)
                    .or_insert_with(|| unicode.clone());
            }
        }
        for (i, map) in other.code_to_cid.iter().enumerate() {
            for (&code, &cid) in map {
                self.code_to_cid[i].entry(code).or_insert(cid);
            }
        }
        for range in &other.cid_ranges {
            self.cid_ranges.push(range.clone());
        }
        for (unicode, code_bytes) in &other.unicode_to_code {
            self.unicode_to_code
                .entry(unicode.clone())
                .or_insert_with(|| code_bytes.clone());
        }
    }
}

impl Default for CMap {
    fn default() -> Self {
        Self::new()
    }
}

/// A codespace range defines the valid boundaries for character codes.
#[derive(Debug, Clone)]
pub struct CodespaceRange {
    /// Start byte values for each position.
    pub start: Vec<u8>,
    /// End byte values for each position.
    pub end: Vec<u8>,
    /// Number of bytes in this codespace.
    pub code_length: usize,
}

impl CodespaceRange {
    pub fn new(start: Vec<u8>, end: Vec<u8>) -> Self {
        let code_length = start.len();
        Self {
            start,
            end,
            code_length,
        }
    }

    /// Check if a byte sequence falls within this codespace range.
    pub fn matches(&self, code: &[u8]) -> bool {
        if code.len() != self.code_length {
            return false;
        }
        for i in 0..self.code_length {
            if code[i] < self.start[i] || code[i] > self.end[i] {
                return false;
            }
        }
        true
    }
}

/// A range of character codes that map to consecutive CID values.
#[derive(Debug, Clone)]
pub struct CidRange {
    /// Start code value.
    pub from: u32,
    /// End code value (inclusive).
    pub to: u32,
    /// CID value for the start code.
    pub cid_start: u32,
    /// Byte length of codes in this range.
    pub code_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cmap() {
        let cmap = CMap::new();
        assert_eq!(cmap.wmode, 0);
        assert_eq!(cmap.unicode_mapping_count(), 0);
        assert_eq!(cmap.cid_mapping_count(), 0);
    }

    #[test]
    fn test_unicode_mapping() {
        let mut cmap = CMap::new();
        cmap.add_char_mapping(0x0041, 2, "A".to_string());
        cmap.add_char_mapping(0x0042, 2, "B".to_string());

        assert_eq!(cmap.to_unicode(0x0041, 2), Some("A"));
        assert_eq!(cmap.to_unicode(0x0042, 2), Some("B"));
        assert_eq!(cmap.to_unicode(0x0043, 2), None);
    }

    #[test]
    fn test_cid_direct_mapping() {
        let mut cmap = CMap::new();
        cmap.add_cid_mapping(0x0041, 2, 100);
        assert_eq!(cmap.to_cid(0x0041, 2), Some(100));
        assert_eq!(cmap.to_cid(0x0042, 2), None);
    }

    #[test]
    fn test_cid_range_mapping() {
        let mut cmap = CMap::new();
        cmap.add_cid_range(CidRange {
            from: 0x0041,
            to: 0x005A,
            cid_start: 100,
            code_length: 2,
        });

        assert_eq!(cmap.to_cid(0x0041, 2), Some(100)); // A
        assert_eq!(cmap.to_cid(0x0042, 2), Some(101)); // B
        assert_eq!(cmap.to_cid(0x005A, 2), Some(125)); // Z
        assert_eq!(cmap.to_cid(0x005B, 2), None); // out of range
    }

    #[test]
    fn test_codespace_range() {
        let range = CodespaceRange::new(vec![0x00, 0x00], vec![0xFF, 0xFF]);
        assert!(range.matches(&[0x00, 0x41]));
        assert!(range.matches(&[0xFF, 0xFF]));
        assert!(!range.matches(&[0x00])); // wrong length
    }

    #[test]
    fn test_reverse_lookup() {
        let mut cmap = CMap::new();
        cmap.add_char_mapping(0x0041, 2, "A".to_string());

        let code = cmap.get_codes_from_unicode("A");
        assert_eq!(code, Some([0x00, 0x41].as_slice()));
    }

    #[test]
    fn test_merge() {
        let mut cmap1 = CMap::new();
        cmap1.add_char_mapping(0x41, 1, "A".to_string());

        let mut cmap2 = CMap::new();
        cmap2.add_char_mapping(0x42, 1, "B".to_string());

        cmap1.merge(&cmap2);
        assert_eq!(cmap1.to_unicode(0x41, 1), Some("A"));
        assert_eq!(cmap1.to_unicode(0x42, 1), Some("B"));
    }
}
