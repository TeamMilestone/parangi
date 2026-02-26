//! PDF font handling: decode character codes to Unicode text.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.

pub mod simple_font;
pub mod type0_font;
pub mod type3_font;

use compact_str::CompactString;
use lopdf::{Document, ObjectId};

use crate::cos_helpers::name_to_string;
use crate::Result;
use simple_font::SimpleFont;
use type0_font::Type0Font;
use type3_font::Type3Font;

/// A PDF font that can decode character codes to Unicode text.
#[derive(Clone)]
pub enum PdfFont {
    /// Simple fonts: Type1, TrueType, MMType1.
    Simple(SimpleFont),
    /// Type0 (composite) fonts for CJK text.
    Type0(Type0Font),
    /// Type3 user-defined fonts.
    Type3(Type3Font),
}

impl PdfFont {
    /// Create a PdfFont from a font dictionary.
    pub fn from_dict(
        doc: &Document,
        font_dict: &lopdf::Dictionary,
        _oid: ObjectId,
    ) -> Result<Self> {
        let subtype = font_dict
            .get(b"Subtype")
            .ok()
            .and_then(|obj| match obj {
                lopdf::Object::Name(n) => Some(name_to_string(n)),
                _ => None,
            })
            .unwrap_or_default();

        match subtype.as_str() {
            "Type1" | "TrueType" | "MMType1" => {
                let font = SimpleFont::from_dict(doc, font_dict, &subtype)?;
                Ok(PdfFont::Simple(font))
            }
            "Type0" => {
                let font = Type0Font::from_dict(doc, font_dict)?;
                Ok(PdfFont::Type0(font))
            }
            "Type3" => {
                let font = Type3Font::from_dict(doc, font_dict)?;
                Ok(PdfFont::Type3(font))
            }
            _ => {
                // Unknown subtype — try as simple font
                let font = SimpleFont::from_dict(doc, font_dict, &subtype)?;
                Ok(PdfFont::Simple(font))
            }
        }
    }

    /// Decode a single character code to a Unicode string.
    /// Returns CompactString to avoid per-glyph heap allocation (most glyphs ≤24 bytes).
    pub fn to_unicode(&self, code: u32) -> Option<CompactString> {
        match self {
            PdfFont::Simple(f) => f.to_unicode(code),
            PdfFont::Type0(f) => f.to_unicode(code),
            PdfFont::Type3(f) => f.to_unicode(code),
        }
    }

    /// Get the width of a character code in 1/1000 text space units.
    pub fn get_width(&self, code: u32) -> f32 {
        match self {
            PdfFont::Simple(f) => f.get_width(code),
            PdfFont::Type0(f) => f.get_width(code),
            PdfFont::Type3(f) => f.get_width(code),
        }
    }

    /// Read a character code from the byte stream at the given offset.
    /// Returns (code, bytes_consumed). For simple/Type3 fonts this is always
    /// 1 byte; for Type0 fonts it depends on the encoding CMap codespace ranges.
    pub fn read_code(&self, data: &[u8], offset: usize) -> (u32, usize) {
        match self {
            PdfFont::Simple(_) | PdfFont::Type3(_) => {
                if offset < data.len() {
                    (data[offset] as u32, 1)
                } else {
                    (0, 0)
                }
            }
            PdfFont::Type0(f) => f.read_code(data, offset),
        }
    }

    /// Whether this font can decode character codes.
    pub fn is_stub(&self) -> bool {
        false
    }

    /// Get the base font name.
    pub fn base_font_name(&self) -> &str {
        match self {
            PdfFont::Simple(f) => f.base_font(),
            PdfFont::Type0(f) => f.base_font(),
            PdfFont::Type3(f) => f.base_font(),
        }
    }
}

/// Convert a Unicode codepoint (u32) to CompactString without heap allocation.
pub(crate) fn char_to_compact(code: u32) -> Option<CompactString> {
    let ch = char::from_u32(code)?;
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    Some(CompactString::from(s as &str))
}
