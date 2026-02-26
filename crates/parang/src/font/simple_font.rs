//! Simple PDF fonts: Type1, TrueType, MMType1.
//!
//! Handles the toUnicode fallback chain:
//! 1. ToUnicode CMap (highest priority)
//! 2. Encoding → glyph name → GlyphList → Unicode
//!
//! Ported from PDFBox PDSimpleFont / PDType1Font / PDTrueTypeFont.

use lopdf::Document;

use crate::cos_helpers::{name_to_string, obj_to_f32, DocumentExt};
use crate::encoding::cmap::CMap;
use crate::encoding::cmap_parser;
use crate::encoding::dictionary::build_encoding;
use crate::encoding::glyph_list::{self, GlyphList};
use crate::encoding::predefined;
use crate::encoding::Encoding;
use crate::Result;

/// A simple (single-byte) PDF font.
#[derive(Clone)]
pub struct SimpleFont {
    /// Font subtype: "Type1", "TrueType", "MMType1".
    subtype: String,
    /// Base font name.
    base_font: String,
    /// The character encoding (code → glyph name).
    encoding: Encoding,
    /// Optional ToUnicode CMap (code → Unicode, highest priority).
    to_unicode_cmap: Option<CMap>,
    /// Glyph widths array (indexed by code - first_char).
    widths: Vec<f32>,
    /// First character code in the Widths array.
    first_char: u16,
    /// Width for characters not in the Widths array.
    missing_width: f32,
    /// Whether this is a symbolic font.
    is_symbolic: bool,
    /// Which GlyphList to use (default or ZapfDingbats).
    use_zapf_dingbats_glyph_list: bool,
}

impl SimpleFont {
    /// Create a SimpleFont from a PDF font dictionary.
    pub fn from_dict(
        doc: &Document,
        font_dict: &lopdf::Dictionary,
        subtype: &str,
    ) -> Result<Self> {
        let base_font = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|obj| match obj {
                lopdf::Object::Name(n) => Some(name_to_string(n)),
                _ => None,
            })
            .unwrap_or_default();

        // Determine if symbolic
        let is_symbolic = Self::read_is_symbolic(doc, font_dict);

        // Determine if ZapfDingbats
        let use_zapf = base_font.contains("ZapfDingbats") || base_font.contains("Dingbats");

        // Read encoding
        let encoding = Self::read_encoding(doc, font_dict, is_symbolic, &base_font);

        // Read ToUnicode CMap
        let to_unicode_cmap = Self::read_to_unicode(doc, font_dict);

        // Read widths
        let first_char = doc
            .dict_get_i64(font_dict, b"FirstChar")
            .ok()
            .flatten()
            .unwrap_or(0) as u16;
        let widths = Self::read_widths(doc, font_dict);
        let missing_width = Self::read_missing_width(doc, font_dict);

        Ok(SimpleFont {
            subtype: subtype.to_string(),
            base_font,
            encoding,
            to_unicode_cmap,
            widths,
            first_char,
            missing_width,
            is_symbolic,
            use_zapf_dingbats_glyph_list: use_zapf,
        })
    }

    /// Decode a single character code to Unicode.
    ///
    /// Fallback chain:
    /// 1. ToUnicode CMap
    /// 2. Encoding → glyph name → GlyphList → Unicode
    pub fn to_unicode(&self, code: u32) -> Option<compact_str::CompactString> {
        // Tier 1: ToUnicode CMap (highest priority)
        if let Some(ref cmap) = self.to_unicode_cmap {
            // Simple fonts use 1-byte codes
            if let Some(unicode) = cmap.to_unicode(code, 1) {
                return Some(unicode.into());
            }
            // Also try 2-byte (some CMaps use 2-byte codes even for simple fonts)
            if let Some(unicode) = cmap.to_unicode(code, 2) {
                return Some(unicode.into());
            }
        }

        // Tier 2: Encoding → glyph name → GlyphList → Unicode
        if let Some(name) = self.encoding.get_name(code as u16) {
            if name == ".notdef" {
                return None;
            }
            let gl = self.glyph_list();
            if let Some(unicode) = gl.to_unicode(name) {
                return Some(unicode.into());
            }
        }

        None
    }

    /// Get the width of a character code in 1/1000 text space units.
    pub fn get_width(&self, code: u32) -> f32 {
        if code >= self.first_char as u32 {
            let idx = (code - self.first_char as u32) as usize;
            if idx < self.widths.len() {
                return self.widths[idx];
            }
        }
        self.missing_width
    }

    /// Get the appropriate GlyphList for this font.
    fn glyph_list(&self) -> &'static GlyphList {
        if self.use_zapf_dingbats_glyph_list {
            &glyph_list::ZAPF_DINGBATS
        } else {
            &glyph_list::DEFAULT
        }
    }

    /// Read whether the font is symbolic from the FontDescriptor flags.
    fn read_is_symbolic(doc: &Document, font_dict: &lopdf::Dictionary) -> bool {
        if let Ok(desc_obj) = font_dict.get(b"FontDescriptor") {
            let desc = match desc_obj {
                lopdf::Object::Reference(id) => {
                    doc.get_object(*id)
                        .ok()
                        .and_then(|o| o.as_dict().ok())
                }
                lopdf::Object::Dictionary(d) => Some(d),
                _ => None,
            };
            if let Some(desc_dict) = desc {
                if let Ok(flags_obj) = desc_dict.get(b"Flags") {
                    let flags = match flags_obj {
                        lopdf::Object::Integer(n) => *n,
                        lopdf::Object::Reference(id) => {
                            doc.get_object(*id)
                                .ok()
                                .and_then(|o| o.as_i64().ok())
                                .unwrap_or(0)
                        }
                        _ => 0,
                    };
                    // Bit 3 (0-indexed bit 2) = Symbolic
                    return (flags & 4) != 0;
                }
            }
        }
        false
    }

    /// Read the encoding from the font dictionary.
    fn read_encoding(
        doc: &Document,
        font_dict: &lopdf::Dictionary,
        is_symbolic: bool,
        base_font: &str,
    ) -> Encoding {
        // Special handling for ZapfDingbats and Symbol standard fonts
        if base_font.contains("ZapfDingbats") || base_font.contains("Dingbats") {
            if let Ok(enc_obj) = font_dict.get(b"Encoding") {
                return build_encoding(doc, enc_obj, true);
            }
            return predefined::ZAPF_DINGBATS.clone();
        }
        if base_font == "Symbol" {
            if let Ok(enc_obj) = font_dict.get(b"Encoding") {
                return build_encoding(doc, enc_obj, true);
            }
            return predefined::SYMBOL.clone();
        }

        // Normal encoding resolution
        if let Ok(enc_obj) = font_dict.get(b"Encoding") {
            build_encoding(doc, enc_obj, is_symbolic)
        } else {
            // No explicit encoding — use StandardEncoding as default
            predefined::STANDARD.clone()
        }
    }

    /// Read the ToUnicode CMap from the font dictionary.
    fn read_to_unicode(doc: &Document, font_dict: &lopdf::Dictionary) -> Option<CMap> {
        let to_unicode_obj = font_dict.get(b"ToUnicode").ok()?;
        let stream_data = doc.get_stream_data(to_unicode_obj).ok()?;
        let cmap = cmap_parser::parse_cmap(&stream_data);
        if cmap.unicode_mapping_count() > 0 {
            Some(cmap)
        } else {
            None
        }
    }

    /// Read the Widths array from the font dictionary.
    fn read_widths(doc: &Document, font_dict: &lopdf::Dictionary) -> Vec<f32> {
        let widths_obj = match font_dict.get(b"Widths") {
            Ok(obj) => obj,
            Err(_) => return Vec::new(),
        };

        let arr = match widths_obj {
            lopdf::Object::Array(arr) => arr,
            lopdf::Object::Reference(id) => {
                match doc.get_object(*id) {
                    Ok(lopdf::Object::Array(arr)) => arr,
                    _ => return Vec::new(),
                }
            }
            _ => return Vec::new(),
        };

        arr.iter()
            .map(|obj| {
                match obj {
                    lopdf::Object::Integer(n) => *n as f32,
                    lopdf::Object::Real(f) => *f,
                    lopdf::Object::Reference(id) => {
                        doc.get_object(*id)
                            .ok()
                            .and_then(|o| obj_to_f32(o).ok())
                            .unwrap_or(0.0)
                    }
                    _ => 0.0,
                }
            })
            .collect()
    }

    /// Read the MissingWidth from the FontDescriptor.
    fn read_missing_width(doc: &Document, font_dict: &lopdf::Dictionary) -> f32 {
        if let Ok(desc_obj) = font_dict.get(b"FontDescriptor") {
            let desc = match desc_obj {
                lopdf::Object::Reference(id) => {
                    doc.get_object(*id)
                        .ok()
                        .and_then(|o| o.as_dict().ok())
                }
                lopdf::Object::Dictionary(d) => Some(d),
                _ => None,
            };
            if let Some(desc_dict) = desc {
                if let Ok(mw_obj) = desc_dict.get(b"MissingWidth") {
                    return match mw_obj {
                        lopdf::Object::Integer(n) => *n as f32,
                        lopdf::Object::Real(f) => *f,
                        lopdf::Object::Reference(id) => {
                            doc.get_object(*id)
                                .ok()
                                .and_then(|o| obj_to_f32(o).ok())
                                .unwrap_or(0.0)
                        }
                        _ => 0.0,
                    };
                }
            }
        }
        0.0
    }

    /// Get the base font name.
    pub fn base_font(&self) -> &str {
        &self.base_font
    }

    /// Get the subtype.
    pub fn subtype(&self) -> &str {
        &self.subtype
    }

    /// Whether this is a symbolic font.
    pub fn is_symbolic(&self) -> bool {
        self.is_symbolic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};

    fn make_simple_font_dict() -> lopdf::Dictionary {
        dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type1".to_vec()),
            "BaseFont" => Object::Name(b"Helvetica".to_vec()),
            "Encoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "FirstChar" => Object::Integer(32),
            "Widths" => Object::Array(
                (0..224).map(|i| Object::Integer(500 + i)).collect()
            )
        }
    }

    #[test]
    fn test_simple_font_creation() {
        let doc = Document::new();
        let dict = make_simple_font_dict();
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        assert_eq!(font.base_font(), "Helvetica");
        assert_eq!(font.subtype(), "Type1");
    }

    #[test]
    fn test_to_unicode_via_encoding() {
        let doc = Document::new();
        let dict = make_simple_font_dict();
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        // 65 = 'A' in WinAnsiEncoding
        assert_eq!(font.to_unicode(65), Some(compact_str::CompactString::from("A")));
        // 32 = 'space' in WinAnsiEncoding
        assert_eq!(font.to_unicode(32), Some(compact_str::CompactString::from(" ")));
    }

    #[test]
    fn test_get_width() {
        let doc = Document::new();
        let dict = make_simple_font_dict();
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        // FirstChar=32, so code 32 → index 0 → width 500
        assert_eq!(font.get_width(32), 500.0);
        // code 65 → index 33 → width 533
        assert_eq!(font.get_width(65), 533.0);
    }

    #[test]
    fn test_missing_width() {
        let doc = Document::new();
        let mut dict = make_simple_font_dict();
        // Only 10 widths instead of full range
        dict.set("Widths", Object::Array(
            (0..10).map(|i| Object::Integer(600 + i)).collect()
        ));
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        // Code 32 (first_char) → index 0 → 600
        assert_eq!(font.get_width(32), 600.0);
        // Code 100 → index 68, out of widths range → missing_width (0.0 by default)
        assert_eq!(font.get_width(100), 0.0);
    }

    #[test]
    fn test_notdef_returns_none() {
        let doc = Document::new();
        let dict = make_simple_font_dict();
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        // Code 0 is not mapped in WinAnsiEncoding
        assert_eq!(font.to_unicode(0), None);
    }

    #[test]
    fn test_standard_encoding_default() {
        let doc = Document::new();
        let dict = dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type1".to_vec()),
            "BaseFont" => Object::Name(b"SomeFont".to_vec())
        };
        let font = SimpleFont::from_dict(&doc, &dict, "Type1").unwrap();

        // Without explicit encoding, should default to StandardEncoding
        // code 65 = 'A' in StandardEncoding
        assert_eq!(font.to_unicode(65), Some(compact_str::CompactString::from("A")));
    }
}
