//! Type0 (composite) PDF fonts for CJK text.
//!
//! Type0 fonts wrap a CIDFont descendant and use CMaps for
//! character code → CID → Unicode conversion.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.PDType0Font.

use lopdf::{Document, Object};

use super::char_to_compact;
use crate::cos_helpers::{name_to_string, obj_to_f32, DocumentExt};
use crate::encoding::cmap::CMap;
use crate::encoding::cmap_manager;
use crate::encoding::cmap_parser;
use crate::Result;

/// A width range entry: cid_start..=cid_end all have the same width.
#[derive(Clone)]
struct WidthRange {
    start: u32,
    end: u32,
    width: f32,
}

/// Per-CID width storage with range compression.
/// Uses individual HashMap entries for small ranges and range entries for large ones.
#[derive(Clone)]
struct CidWidths {
    /// Individual CID → width mappings (for small ranges / W arrays).
    individual: std::collections::HashMap<u32, f32>,
    /// Range-based widths (for large cid_start..cid_end ranges).
    /// Sorted by start CID for binary search.
    ranges: Vec<WidthRange>,
}

impl CidWidths {
    fn new() -> Self {
        Self {
            individual: std::collections::HashMap::new(),
            ranges: Vec::new(),
        }
    }

    fn insert_individual(&mut self, cid: u32, width: f32) {
        self.individual.insert(cid, width);
    }

    fn insert_range(&mut self, start: u32, end: u32, width: f32) {
        // For small ranges (<=32 entries), expand individually for faster lookup
        if end - start <= 32 {
            for cid in start..=end {
                self.individual.insert(cid, width);
            }
        } else {
            self.ranges.push(WidthRange { start, end, width });
        }
    }

    fn finalize(&mut self) {
        self.ranges.sort_unstable_by_key(|r| r.start);
    }

    fn get(&self, cid: u32) -> Option<f32> {
        // Check individual first (most common for real fonts)
        if let Some(&w) = self.individual.get(&cid) {
            return Some(w);
        }
        // Binary search through ranges
        self.ranges
            .binary_search_by(|r| {
                if cid < r.start {
                    std::cmp::Ordering::Greater
                } else if cid > r.end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()
            .map(|i| self.ranges[i].width)
    }
}

/// A Type0 (composite) font.
#[derive(Clone)]
pub struct Type0Font {
    /// Base font name.
    base_font: String,
    /// Encoding CMap: character code → CID.
    encoding_cmap: CMap,
    /// Whether the encoding CMap is a predefined one.
    is_cmap_predefined: bool,
    /// ToUnicode CMap: character code → Unicode (highest priority).
    to_unicode_cmap: Option<CMap>,
    /// UCS2 CMap: CID → Unicode (for CJK fonts).
    ucs2_cmap: Option<CMap>,
    /// Whether the descendant font is CJK (Adobe-Korea1, etc.).
    is_descendant_cjk: bool,
    /// Default width (DW) from the CIDFont descendant.
    default_width: f32,
    /// Per-CID widths with range compression.
    widths: CidWidths,
}

impl Type0Font {
    /// Create a Type0Font from a font dictionary.
    pub fn from_dict(doc: &Document, font_dict: &lopdf::Dictionary) -> Result<Self> {
        let base_font = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|obj| match obj {
                Object::Name(n) => Some(name_to_string(n)),
                _ => None,
            })
            .unwrap_or_default();

        // Read encoding CMap
        let (encoding_cmap, is_cmap_predefined) =
            Self::read_encoding_cmap(doc, font_dict);

        // Read ToUnicode CMap
        let to_unicode_cmap = Self::read_to_unicode(doc, font_dict);

        // Read descendant font
        let descendant_dict = Self::get_descendant_dict(doc, font_dict);

        // Determine CJK status from CIDSystemInfo
        let (is_descendant_cjk, registry, ordering) =
            Self::read_cid_system_info(doc, descendant_dict);

        // Fetch UCS2 CMap for CJK fonts
        let ucs2_cmap = if is_cmap_predefined || is_descendant_cjk {
            Self::fetch_ucs2_cmap(&registry, &ordering)
        } else {
            None
        };

        // Read widths from CIDFont descendant
        let default_width = Self::read_default_width(doc, descendant_dict);
        let widths = Self::read_widths(doc, descendant_dict);

        Ok(Type0Font {
            base_font,
            encoding_cmap,
            is_cmap_predefined,
            to_unicode_cmap,
            ucs2_cmap,
            is_descendant_cjk,
            default_width,
            widths,
        })
    }

    /// Decode a character code to Unicode.
    ///
    /// Fallback chain:
    /// 1. ToUnicode CMap (code → Unicode)
    /// 2. Encoding CMap + UCS2 CMap (code → CID → Unicode) for CJK
    /// 3. Identity mapping fallback
    pub fn to_unicode(&self, code: u32) -> Option<compact_str::CompactString> {
        // Tier 1: ToUnicode CMap
        if let Some(ref cmap) = self.to_unicode_cmap {
            // Try 2-byte first (most common for CJK)
            if let Some(unicode) = cmap.to_unicode(code, 2) {
                return Some(unicode.into());
            }
            // Try 1-byte
            if let Some(unicode) = cmap.to_unicode(code, 1) {
                return Some(unicode.into());
            }
            // Try 3 and 4 byte
            if let Some(unicode) = cmap.to_unicode(code, 3) {
                return Some(unicode.into());
            }
            if let Some(unicode) = cmap.to_unicode(code, 4) {
                return Some(unicode.into());
            }
        }

        // Tier 2: Encoding CMap → CID, UCS2 CMap → Unicode
        if self.is_cmap_predefined || self.is_descendant_cjk {
            if let Some(ref ucs2) = self.ucs2_cmap {
                let cid = self.code_to_cid(code);
                // UCS2 CMap maps CID → Unicode using 2-byte codes
                if let Some(unicode) = ucs2.to_unicode(cid, 2) {
                    return Some(unicode.into());
                }
                if let Some(unicode) = ucs2.to_unicode(cid, 1) {
                    return Some(unicode.into());
                }
            }
        }

        // Tier 3: Identity fallback (if ToUnicode exists but is non-predefined)
        if self.to_unicode_cmap.is_some() && !self.is_cmap_predefined {
            return char_to_compact(code);
        }

        // Tier 4: Identity-H/V encoding without ToUnicode or UCS2 → CID is Unicode
        if self.encoding_cmap.name.starts_with("Identity") {
            let cid = self.code_to_cid(code);
            return char_to_compact(cid);
        }

        None
    }

    /// Convert character code to CID using the encoding CMap.
    pub fn code_to_cid(&self, code: u32) -> u32 {
        // Try different code lengths
        for len in [2, 1, 3, 4] {
            if let Some(cid) = self.encoding_cmap.to_cid(code, len) {
                return cid;
            }
        }
        // Identity fallback: code = CID
        code
    }

    /// Get the width for a character code.
    pub fn get_width(&self, code: u32) -> f32 {
        let cid = self.code_to_cid(code);
        self.widths.get(cid).unwrap_or(self.default_width)
    }

    /// Read a character code from the byte stream using the encoding CMap's
    /// codespace ranges. Returns (code, bytes_consumed).
    pub fn read_code(&self, data: &[u8], offset: usize) -> (u32, usize) {
        let remaining = data.len() - offset;
        if remaining == 0 {
            return (0, 0);
        }

        let min_len = self.encoding_cmap.min_code_length().max(1);
        let max_len = self.encoding_cmap.max_code_length().min(remaining);

        for len in min_len..=max_len {
            let bytes = &data[offset..offset + len];
            if self.encoding_cmap.matches_codespace(bytes) {
                let mut code = 0u32;
                for &b in bytes {
                    code = (code << 8) | b as u32;
                }
                return (code, len);
            }
        }

        // Fallback: single byte
        (data[offset] as u32, 1)
    }

    /// Get the base font name.
    pub fn base_font(&self) -> &str {
        &self.base_font
    }

    // -----------------------------------------------------------------------
    // Initialization helpers
    // -----------------------------------------------------------------------

    fn read_encoding_cmap(
        doc: &Document,
        font_dict: &lopdf::Dictionary,
    ) -> (CMap, bool) {
        let enc_obj = match font_dict.get(b"Encoding") {
            Ok(obj) => obj,
            Err(_) => return (CMap::new(), false),
        };

        match enc_obj {
            Object::Name(name_bytes) => {
                let name = name_to_string(name_bytes);
                if let Some(cmap) = cmap_manager::get_predefined_cmap(&name) {
                    (cmap, true)
                } else {
                    (CMap::new(), true)
                }
            }
            Object::Reference(id) => {
                match doc.get_object(*id) {
                    Ok(Object::Name(name_bytes)) => {
                        let name = name_to_string(name_bytes);
                        if let Some(cmap) = cmap_manager::get_predefined_cmap(&name) {
                            (cmap, true)
                        } else {
                            (CMap::new(), true)
                        }
                    }
                    Ok(obj) => {
                        // Embedded CMap stream
                        if let Ok(data) = doc.get_stream_data(obj) {
                            (cmap_parser::parse_cmap(&data), false)
                        } else {
                            (CMap::new(), false)
                        }
                    }
                    Err(_) => (CMap::new(), false),
                }
            }
            Object::Stream(_) => {
                if let Ok(data) = doc.get_stream_data(enc_obj) {
                    (cmap_parser::parse_cmap(&data), false)
                } else {
                    (CMap::new(), false)
                }
            }
            _ => (CMap::new(), false),
        }
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

    fn get_descendant_dict<'a>(
        doc: &'a Document,
        font_dict: &'a lopdf::Dictionary,
    ) -> Option<&'a lopdf::Dictionary> {
        let desc_arr = font_dict.get(b"DescendantFonts").ok()?;
        let arr = match desc_arr {
            Object::Array(arr) => arr,
            Object::Reference(id) => {
                match doc.get_object(*id).ok()? {
                    Object::Array(arr) => arr,
                    _ => return None,
                }
            }
            _ => return None,
        };

        if arr.is_empty() {
            return None;
        }

        match &arr[0] {
            Object::Reference(id) => {
                doc.get_object(*id)
                    .ok()
                    .and_then(|obj| obj.as_dict().ok())
            }
            Object::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    fn read_cid_system_info(
        doc: &Document,
        descendant: Option<&lopdf::Dictionary>,
    ) -> (bool, String, String) {
        let desc = match descendant {
            Some(d) => d,
            None => return (false, String::new(), String::new()),
        };

        let csi_obj = match desc.get(b"CIDSystemInfo") {
            Ok(obj) => obj,
            Err(_) => return (false, String::new(), String::new()),
        };

        let csi_dict = match csi_obj {
            Object::Dictionary(d) => d,
            Object::Reference(id) => {
                match doc.get_object(*id).ok().and_then(|o| o.as_dict().ok()) {
                    Some(d) => d,
                    None => return (false, String::new(), String::new()),
                }
            }
            _ => return (false, String::new(), String::new()),
        };

        let registry = csi_dict
            .get(b"Registry")
            .ok()
            .and_then(|obj| match obj {
                Object::String(bytes, _) => {
                    Some(String::from_utf8_lossy(bytes).to_string())
                }
                _ => None,
            })
            .unwrap_or_default();

        let ordering = csi_dict
            .get(b"Ordering")
            .ok()
            .and_then(|obj| match obj {
                Object::String(bytes, _) => {
                    Some(String::from_utf8_lossy(bytes).to_string())
                }
                _ => None,
            })
            .unwrap_or_default();

        let is_cjk = registry == "Adobe"
            && matches!(
                ordering.as_str(),
                "Korea1" | "GB1" | "CNS1" | "Japan1"
            );

        (is_cjk, registry, ordering)
    }

    fn fetch_ucs2_cmap(registry: &str, ordering: &str) -> Option<CMap> {
        if registry.is_empty() || ordering.is_empty() {
            return None;
        }
        let ucs2_name = format!("{}-{}-UCS2", registry, ordering);
        cmap_manager::get_predefined_cmap(&ucs2_name)
    }

    fn read_default_width(doc: &Document, descendant: Option<&lopdf::Dictionary>) -> f32 {
        let desc = match descendant {
            Some(d) => d,
            None => return 1000.0,
        };
        match desc.get(b"DW") {
            Ok(obj) => match obj {
                Object::Integer(n) => *n as f32,
                Object::Real(f) => *f,
                Object::Reference(id) => {
                    doc.get_object(*id)
                        .ok()
                        .and_then(|o| obj_to_f32(o).ok())
                        .unwrap_or(1000.0)
                }
                _ => 1000.0,
            },
            Err(_) => 1000.0,
        }
    }

    fn read_widths(
        doc: &Document,
        descendant: Option<&lopdf::Dictionary>,
    ) -> CidWidths {
        let mut result = CidWidths::new();
        let desc = match descendant {
            Some(d) => d,
            None => return result,
        };

        let w_obj = match desc.get(b"W") {
            Ok(obj) => obj,
            Err(_) => return result,
        };

        let arr = match w_obj {
            Object::Array(arr) => arr,
            Object::Reference(id) => {
                match doc.get_object(*id) {
                    Ok(Object::Array(arr)) => arr,
                    _ => return result,
                }
            }
            _ => return result,
        };

        Self::parse_w_array(doc, arr, &mut result);
        result.finalize();
        result
    }

    /// Parse the CIDFont W (Widths) array.
    ///
    /// Format: [ cid_start [w1 w2 ...] | cid_start cid_end width ... ]
    fn parse_w_array(
        doc: &Document,
        arr: &[Object],
        widths: &mut CidWidths,
    ) {
        let mut i = 0;
        while i < arr.len() {
            let cid_start = match Self::obj_to_u32(doc, &arr[i]) {
                Some(v) => v,
                None => {
                    i += 1;
                    continue;
                }
            };
            i += 1;

            if i >= arr.len() {
                break;
            }

            // Next element determines format
            let next = Self::resolve_obj(doc, &arr[i]);
            match next {
                Some(Object::Array(width_arr)) => {
                    // Format: cid [w1 w2 w3 ...]
                    for (j, w_obj) in width_arr.iter().enumerate() {
                        let w = Self::obj_to_f32_val(doc, w_obj);
                        widths.insert_individual(cid_start + j as u32, w);
                    }
                    i += 1;
                }
                Some(Object::Integer(_)) | Some(Object::Real(_)) => {
                    // Format: cid_start cid_end width
                    let cid_end = Self::obj_to_u32(doc, &arr[i]).unwrap_or(cid_start);
                    i += 1;
                    let width = if i < arr.len() {
                        let w = Self::obj_to_f32_val(doc, &arr[i]);
                        i += 1;
                        w
                    } else {
                        1000.0
                    };
                    widths.insert_range(cid_start, cid_end, width);
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    fn resolve_obj<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
        match obj {
            Object::Reference(id) => doc.get_object(*id).ok(),
            other => Some(other),
        }
    }

    fn obj_to_u32(doc: &Document, obj: &Object) -> Option<u32> {
        match obj {
            Object::Integer(n) => Some(*n as u32),
            Object::Real(f) => Some(*f as u32),
            Object::Reference(id) => {
                doc.get_object(*id)
                    .ok()
                    .and_then(|o| Self::obj_to_u32(doc, o))
            }
            _ => None,
        }
    }

    fn obj_to_f32_val(doc: &Document, obj: &Object) -> f32 {
        match obj {
            Object::Integer(n) => *n as f32,
            Object::Real(f) => *f,
            Object::Reference(id) => {
                doc.get_object(*id)
                    .ok()
                    .and_then(|o| obj_to_f32(o).ok())
                    .unwrap_or(0.0)
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_to_cid_identity() {
        // With an empty encoding CMap, code_to_cid should return the code itself
        let font = Type0Font {
            base_font: "TestFont".to_string(),
            encoding_cmap: CMap::new(),
            is_cmap_predefined: false,
            to_unicode_cmap: None,
            ucs2_cmap: None,
            is_descendant_cjk: false,
            default_width: 1000.0,
            widths: CidWidths::new(),
        };
        assert_eq!(font.code_to_cid(0x41), 0x41);
    }

    #[test]
    fn test_to_unicode_via_tounicode_cmap() {
        let mut tounicode = CMap::new();
        tounicode.add_char_mapping(0x0041, 2, "A".to_string());
        tounicode.add_char_mapping(0xAC00, 2, "\u{AC00}".to_string()); // Korean 가

        let font = Type0Font {
            base_font: "TestFont".to_string(),
            encoding_cmap: CMap::new(),
            is_cmap_predefined: false,
            to_unicode_cmap: Some(tounicode),
            ucs2_cmap: None,
            is_descendant_cjk: false,
            default_width: 1000.0,
            widths: CidWidths::new(),
        };

        assert_eq!(font.to_unicode(0x0041), Some(compact_str::CompactString::from("A")));
        assert_eq!(font.to_unicode(0xAC00), Some(compact_str::CompactString::from("\u{AC00}")));
    }

    #[test]
    fn test_width_lookup() {
        let mut widths = CidWidths::new();
        widths.insert_individual(100, 500.0);
        widths.insert_individual(101, 600.0);

        let font = Type0Font {
            base_font: "TestFont".to_string(),
            encoding_cmap: CMap::new(),
            is_cmap_predefined: false,
            to_unicode_cmap: None,
            ucs2_cmap: None,
            is_descendant_cjk: false,
            default_width: 1000.0,
            widths,
        };

        // Identity CMap: code = CID
        assert_eq!(font.get_width(100), 500.0);
        assert_eq!(font.get_width(101), 600.0);
        assert_eq!(font.get_width(200), 1000.0); // default
    }
}
