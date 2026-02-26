//! Adobe Glyph List: maps glyph names to Unicode strings.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.encoding.GlyphList.

use hashbrown::HashMap;
use std::sync::LazyLock;

/// The default Adobe Glyph List (AGL) + additional PDFBox entries.
pub static DEFAULT: LazyLock<GlyphList> = LazyLock::new(|| {
    let mut gl = GlyphList::new();
    gl.load_from_str(include_str!("../data/glyphlist.txt"));
    gl.load_from_str(include_str!("../data/additional.txt"));
    gl
});

/// Zapf Dingbats glyph list.
pub static ZAPF_DINGBATS: LazyLock<GlyphList> = LazyLock::new(|| {
    let mut gl = GlyphList::new();
    gl.load_from_str(include_str!("../data/zapfdingbats.txt"));
    gl
});

/// Bidirectional mapping between glyph names and Unicode strings.
pub struct GlyphList {
    name_to_unicode: HashMap<String, String>,
    unicode_to_name: HashMap<String, String>,
}

impl GlyphList {
    pub fn new() -> Self {
        Self {
            name_to_unicode: HashMap::new(),
            unicode_to_name: HashMap::new(),
        }
    }

    /// Load entries from a text file content (format: "name;XXXX YYYY").
    fn load_from_str(&mut self, data: &str) {
        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, hex_codes)) = line.split_once(';') {
                let unicode = Self::hex_to_string(hex_codes);
                if !unicode.is_empty() {
                    self.name_to_unicode
                        .entry(name.to_string())
                        .or_insert_with(|| unicode.clone());
                    self.unicode_to_name
                        .entry(unicode)
                        .or_insert_with(|| name.to_string());
                }
            }
        }
    }

    /// Convert space-separated hex codepoints to a Unicode string.
    fn hex_to_string(hex_codes: &str) -> String {
        let mut result = String::new();
        for hex in hex_codes.split_whitespace() {
            if let Ok(cp) = u32::from_str_radix(hex, 16) {
                if let Some(ch) = char::from_u32(cp) {
                    result.push(ch);
                }
            }
        }
        result
    }

    /// Look up a glyph name and return its Unicode string.
    ///
    /// Handles special cases:
    /// - `.notdef` → None
    /// - `uniXXXX` format (exactly 7 chars starting with "uni")
    /// - `uXXXX`/`uXXXXX` format (Unicode scalar value)
    /// - Names with dot suffixes (e.g., "A.swash" → look up "A")
    pub fn to_unicode(&self, name: &str) -> Option<String> {
        if name.is_empty() || name == ".notdef" {
            return None;
        }

        // Direct lookup
        if let Some(unicode) = self.name_to_unicode.get(name) {
            return Some(unicode.clone());
        }

        // Try uniXXXX format (exactly 7 chars)
        if name.len() == 7 && name.starts_with("uni") {
            if let Ok(cp) = u32::from_str_radix(&name[3..], 16) {
                if let Some(ch) = char::from_u32(cp) {
                    return Some(ch.to_string());
                }
            }
        }

        // Try longer uniXXXXXXXX format (multiples of 4 hex digits)
        if name.starts_with("uni") && name.len() > 7 && (name.len() - 3) % 4 == 0 {
            let hex_part = &name[3..];
            let mut result = String::new();
            let mut valid = true;
            for chunk in hex_part.as_bytes().chunks(4) {
                if let Ok(cp) = u32::from_str_radix(
                    std::str::from_utf8(chunk).unwrap_or(""),
                    16,
                ) {
                    if let Some(ch) = char::from_u32(cp) {
                        result.push(ch);
                    } else {
                        valid = false;
                        break;
                    }
                } else {
                    valid = false;
                    break;
                }
            }
            if valid && !result.is_empty() {
                return Some(result);
            }
        }

        // Try uXXXX-uXXXXX format (single codepoint, 4-6 hex digits)
        if name.starts_with('u') && name.len() >= 5 && name.len() <= 7 {
            let hex_part = &name[1..];
            if hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(cp) = u32::from_str_radix(hex_part, 16) {
                    if let Some(ch) = char::from_u32(cp) {
                        return Some(ch.to_string());
                    }
                }
            }
        }

        // Try removing suffix after dot (e.g., "A.swash" → "A")
        if let Some(dot_pos) = name.find('.') {
            let base = &name[..dot_pos];
            if let Some(unicode) = self.name_to_unicode.get(base) {
                return Some(unicode.clone());
            }
        }

        None
    }

    /// Look up a Unicode codepoint and return its glyph name.
    pub fn code_point_to_name(&self, cp: u32) -> Option<&str> {
        if let Some(ch) = char::from_u32(cp) {
            let s = ch.to_string();
            self.unicode_to_name.get(&s).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Look up a Unicode string and return its glyph name.
    pub fn sequence_to_name(&self, unicode: &str) -> Option<&str> {
        self.unicode_to_name.get(unicode).map(|s| s.as_str())
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.name_to_unicode.len()
    }

    pub fn is_empty(&self) -> bool {
        self.name_to_unicode.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_glyph_list_loaded() {
        let gl = &*DEFAULT;
        assert!(gl.len() > 4000, "should have 4000+ entries, got {}", gl.len());
    }

    #[test]
    fn test_basic_lookup() {
        let gl = &*DEFAULT;
        assert_eq!(gl.to_unicode("A"), Some("A".to_string()));
        assert_eq!(gl.to_unicode("space"), Some(" ".to_string()));
        assert_eq!(gl.to_unicode("fi"), Some("\u{FB01}".to_string()));
    }

    #[test]
    fn test_notdef() {
        let gl = &*DEFAULT;
        assert_eq!(gl.to_unicode(".notdef"), None);
        assert_eq!(gl.to_unicode(""), None);
    }

    #[test]
    fn test_uni_format() {
        let gl = &*DEFAULT;
        assert_eq!(gl.to_unicode("uni0041"), Some("A".to_string()));
        assert_eq!(gl.to_unicode("uniAC00"), Some("\u{AC00}".to_string())); // Korean '가'
    }

    #[test]
    fn test_u_format() {
        let gl = &*DEFAULT;
        assert_eq!(gl.to_unicode("u0041"), Some("A".to_string()));
        assert_eq!(gl.to_unicode("uAC00"), Some("\u{AC00}".to_string()));
    }

    #[test]
    fn test_dot_suffix() {
        let gl = &*DEFAULT;
        // "A.swash" should resolve via base name "A"
        assert_eq!(gl.to_unicode("A.swash"), Some("A".to_string()));
    }

    #[test]
    fn test_reverse_lookup() {
        let gl = &*DEFAULT;
        assert_eq!(gl.code_point_to_name(0x0041), Some("A"));
        assert_eq!(gl.code_point_to_name(0x0020), Some("space"));
    }

    #[test]
    fn test_zapf_dingbats() {
        let gl = &*ZAPF_DINGBATS;
        assert!(gl.len() > 100);
    }
}
