//! Type3 PDF fonts.
//!
//! Type3 fonts define each glyph as a sequence of PDF content stream operators.
//! For text extraction, we primarily need the ToUnicode CMap (if any) and encoding.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.PDType3Font.

use lopdf::{Document, Object};

use crate::cos_helpers::{name_to_string, obj_to_f32, DocumentExt};
use crate::encoding::cmap::CMap;
use crate::encoding::cmap_parser;
use crate::encoding::dictionary::build_encoding;
use crate::encoding::glyph_list;
use crate::encoding::predefined;
use crate::encoding::Encoding;
use crate::Result;

/// A Type3 font.
///
/// Type3 fonts are user-defined fonts where each glyph is a small PDF content stream.
/// For text extraction, we rely on ToUnicode CMap or encoding + glyph list.
pub struct Type3Font {
    /// Base font name.
    base_font: String,
    /// The character encoding.
    encoding: Encoding,
    /// Optional ToUnicode CMap.
    to_unicode_cmap: Option<CMap>,
    /// Widths array.
    widths: Vec<f32>,
    /// First character code.
    first_char: u16,
}

impl Type3Font {
    /// Create a Type3Font from a font dictionary.
    pub fn from_dict(doc: &Document, font_dict: &lopdf::Dictionary) -> Result<Self> {
        let base_font = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|obj| match obj {
                Object::Name(n) => Some(name_to_string(n)),
                _ => None,
            })
            .unwrap_or_default();

        // Read encoding
        let encoding = if let Ok(enc_obj) = font_dict.get(b"Encoding") {
            build_encoding(doc, enc_obj, false)
        } else {
            // Type3 fonts may use a custom encoding in the Encoding dict
            predefined::STANDARD.clone()
        };

        // Read ToUnicode CMap
        let to_unicode_cmap = Self::read_to_unicode(doc, font_dict);

        // Read widths
        let first_char = doc
            .dict_get_i64(font_dict, b"FirstChar")
            .ok()
            .flatten()
            .unwrap_or(0) as u16;
        let widths = Self::read_widths(doc, font_dict);

        Ok(Type3Font {
            base_font,
            encoding,
            to_unicode_cmap,
            widths,
            first_char,
        })
    }

    /// Decode a character code to Unicode.
    pub fn to_unicode(&self, code: u32) -> Option<String> {
        // Tier 1: ToUnicode CMap
        if let Some(ref cmap) = self.to_unicode_cmap {
            if let Some(unicode) = cmap.to_unicode(code, 1) {
                return Some(unicode.to_string());
            }
            if let Some(unicode) = cmap.to_unicode(code, 2) {
                return Some(unicode.to_string());
            }
        }

        // Tier 2: Encoding → glyph name → GlyphList
        if let Some(name) = self.encoding.get_name(code as u16) {
            if name == ".notdef" {
                return None;
            }
            let gl = &*glyph_list::DEFAULT;
            if let Some(unicode) = gl.to_unicode(name) {
                return Some(unicode);
            }
        }

        None
    }

    /// Get the width of a character code.
    pub fn get_width(&self, code: u32) -> f32 {
        if code >= self.first_char as u32 {
            let idx = (code - self.first_char as u32) as usize;
            if idx < self.widths.len() {
                return self.widths[idx];
            }
        }
        0.0
    }

    /// Get the base font name.
    pub fn base_font(&self) -> &str {
        &self.base_font
    }

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

    fn read_widths(doc: &Document, font_dict: &lopdf::Dictionary) -> Vec<f32> {
        let widths_obj = match font_dict.get(b"Widths") {
            Ok(obj) => obj,
            Err(_) => return Vec::new(),
        };

        let arr = match widths_obj {
            Object::Array(arr) => arr,
            Object::Reference(id) => {
                match doc.get_object(*id) {
                    Ok(Object::Array(arr)) => arr,
                    _ => return Vec::new(),
                }
            }
            _ => return Vec::new(),
        };

        arr.iter()
            .map(|obj| match obj {
                Object::Integer(n) => *n as f32,
                Object::Real(f) => *f,
                Object::Reference(id) => {
                    doc.get_object(*id)
                        .ok()
                        .and_then(|o| obj_to_f32(o).ok())
                        .unwrap_or(0.0)
                }
                _ => 0.0,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};

    #[test]
    fn test_type3_font_creation() {
        let doc = Document::new();
        let dict = dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type3".to_vec()),
            "BaseFont" => Object::Name(b"CustomType3".to_vec()),
            "Encoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "FirstChar" => Object::Integer(32),
            "Widths" => Object::Array(
                (0..96).map(|i| Object::Integer(500 + i)).collect()
            )
        };
        let font = Type3Font::from_dict(&doc, &dict).unwrap();

        assert_eq!(font.base_font(), "CustomType3");
        assert_eq!(font.to_unicode(65), Some("A".to_string()));
        assert_eq!(font.get_width(32), 500.0);
        assert_eq!(font.get_width(65), 533.0);
    }
}
