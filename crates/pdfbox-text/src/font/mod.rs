//! PDF font handling: decode character codes to Unicode text.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.

pub mod simple_font;

use lopdf::{Document, ObjectId};

use crate::cos_helpers::name_to_string;
use crate::Result;
use simple_font::SimpleFont;

/// A PDF font that can decode character codes to Unicode text.
pub enum PdfFont {
    /// Simple fonts: Type1, TrueType, MMType1.
    Simple(SimpleFont),
    /// Type0 (composite) fonts — to be implemented in pdf08.
    Type0Stub,
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
                // Stub — will be implemented in pdf08
                Ok(PdfFont::Type0Stub)
            }
            "Type3" => {
                // Stub — will be implemented in pdf09
                Ok(PdfFont::Type3Stub)
            }
            _ => Ok(PdfFont::Type0Stub), // Unknown → stub
        }
    }

    /// Decode a single character code to a Unicode string.
    ///
    /// Returns None if no mapping can be determined.
    pub fn to_unicode(&self, code: u32) -> Option<String> {
        match self {
            PdfFont::Simple(f) => f.to_unicode(code),
            PdfFont::Type0Stub | PdfFont::Type3Stub => None,
        }
    }

    /// Get the width of a character code in 1/1000 text space units.
    pub fn get_width(&self, code: u32) -> f32 {
        match self {
            PdfFont::Simple(f) => f.get_width(code),
            PdfFont::Type0Stub | PdfFont::Type3Stub => 0.0,
        }
    }

    /// Whether this is a stub (unimplemented font type).
    pub fn is_stub(&self) -> bool {
        matches!(self, PdfFont::Type0Stub | PdfFont::Type3Stub)
    }
}
