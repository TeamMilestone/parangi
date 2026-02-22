//! Character encoding and CMap support.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.encoding.

pub mod cmap;
pub mod cmap_parser;
pub mod dictionary;
pub mod glyph_list;
pub mod predefined;

use std::collections::HashMap;

/// A character encoding that maps character codes to glyph names.
///
/// This is the base encoding type used by simple fonts (Type1, TrueType).
/// Ported from PDFBox's Encoding.java.
#[derive(Debug, Clone)]
pub struct Encoding {
    name: String,
    code_to_name: HashMap<u16, String>,
    name_to_code: HashMap<String, u16>,
}

impl Encoding {
    /// Create a new empty encoding with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            code_to_name: HashMap::new(),
            name_to_code: HashMap::new(),
        }
    }

    /// Add a mapping from character code to glyph name.
    pub fn add(&mut self, code: u16, name: &str) {
        self.code_to_name.insert(code, name.to_string());
        self.name_to_code.insert(name.to_string(), code);
    }

    /// Check if a character code has a mapping.
    pub fn has_code(&self, code: u16) -> bool {
        self.code_to_name.contains_key(&code)
    }

    /// Get the glyph name for a character code.
    pub fn get_name(&self, code: u16) -> Option<&str> {
        self.code_to_name.get(&code).map(|s| s.as_str())
    }

    /// Get the character code for a glyph name.
    pub fn get_code(&self, name: &str) -> Option<u16> {
        self.name_to_code.get(name).copied()
    }

    /// Get the encoding name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of code-to-name mappings.
    pub fn len(&self) -> usize {
        self.code_to_name.len()
    }

    /// Whether the encoding has no mappings.
    pub fn is_empty(&self) -> bool {
        self.code_to_name.is_empty()
    }

    /// Iterate over all (code, name) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &str)> {
        self.code_to_name.iter().map(|(&k, v)| (k, v.as_str()))
    }
}
