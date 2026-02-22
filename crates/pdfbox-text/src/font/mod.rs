//! PDF font handling: decode character codes to Unicode text.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.

pub mod simple_font;
pub mod type0_font;

use lopdf::{Document, ObjectId};

use crate::cos_helpers::name_to_string;
use crate::Result;
use simple_font::SimpleFont;
use type0_font::Type0Font;

/// A PDF font that can decode character codes to Unicode text.
pub enum PdfFont {
    /// Simple fonts: Type1, TrueType, MMType1.
    Simple(SimpleFont),
    /// Type0 (composite) fonts for CJK text.
    Type0(Type0Font),
    /// Type3 fonts — to be implemented in pdf09.
    Type3Stub,
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
                Ok(PdfFont::Type3Stub)
            }
            _ => Ok(PdfFont::Type3Stub), // Unknown → stub
        }
    }

    /// Decode a single character code to a Unicode string.
    ///
    /// Returns None if no mapping can be determined.
    pub fn to_unicode(&self, code: u32) -> Option<String> {
        match self {
            PdfFont::Simple(f) => f.to_unicode(code),
            PdfFont::Type0(f) => f.to_unicode(code),
            PdfFont::Type3Stub => None,
        }
    }

    /// Get the width of a character code in 1/1000 text space units.
    pub fn get_width(&self, code: u32) -> f32 {
        match self {
            PdfFont::Simple(f) => f.get_width(code),
            PdfFont::Type0(f) => f.get_width(code),
            PdfFont::Type3Stub => 0.0,
        }
    }

    /// Whether this is a stub (unimplemented font type).
    pub fn is_stub(&self) -> bool {
        matches!(self, PdfFont::Type3Stub)
    }
}
