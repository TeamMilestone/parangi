//! PDF page resources (fonts, xobjects, extended graphics states, etc.).

use std::sync::Arc;

use lopdf::{Document, Object, ObjectId};

use crate::cos_helpers::DocumentExt;
use crate::Result;

/// Page-level resources dictionary wrapper.
///
/// Provides access to fonts, XObjects, and other named resources
/// referenced by content stream operators.
pub struct PdfResources<'a> {
    doc: Arc<Document>,
    dict: &'a lopdf::Dictionary,
}

impl<'a> PdfResources<'a> {
    /// Create a new PdfResources wrapper.
    pub fn new(doc: Arc<Document>, dict: &'a lopdf::Dictionary) -> Self {
        Self { doc, dict }
    }

    /// Get the Font sub-dictionary.
    pub fn font_dict(&self) -> Result<Option<&lopdf::Dictionary>> {
        match self.doc.dict_get_opt(self.dict, b"Font")? {
            Some(obj) => Ok(Some(obj.as_dict()?)),
            None => Ok(None),
        }
    }

    /// Get a font dictionary by its resource name (e.g., "F1").
    pub fn get_font_dict(&self, name: &[u8]) -> Result<Option<(&lopdf::Dictionary, ObjectId)>> {
        let font_dict = match self.font_dict()? {
            Some(d) => d,
            None => return Ok(None),
        };

        match font_dict.get(name) {
            Ok(obj) => {
                // We need the object ID for caching
                let (resolved, id) = match obj {
                    Object::Reference(id) => {
                        let resolved = self
                            .doc
                            .get_object(*id)
                            .map_err(|e| crate::PdfError::Parse(e.to_string()))?;
                        (resolved, Some(*id))
                    }
                    _ => (obj, None),
                };
                let dict = resolved.as_dict()?;
                // Use the object ID if available, otherwise create a synthetic one
                let oid = id.unwrap_or((0, 0));
                Ok(Some((dict, oid)))
            }
            Err(_) => Ok(None),
        }
    }

    /// Get all font names defined in this resource dictionary.
    pub fn font_names(&self) -> Result<Vec<Vec<u8>>> {
        let font_dict = match self.font_dict()? {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        Ok(font_dict.iter().map(|(k, _)| k.clone()).collect())
    }

    /// Get the ExtGState sub-dictionary.
    pub fn ext_g_state_dict(&self) -> Result<Option<&lopdf::Dictionary>> {
        match self.doc.dict_get_opt(self.dict, b"ExtGState")? {
            Some(obj) => Ok(Some(obj.as_dict()?)),
            None => Ok(None),
        }
    }

    /// Get an extended graphics state by name.
    pub fn get_ext_g_state(&self, name: &[u8]) -> Result<Option<&lopdf::Dictionary>> {
        let gs_dict = match self.ext_g_state_dict()? {
            Some(d) => d,
            None => return Ok(None),
        };

        match gs_dict.get(name) {
            Ok(obj) => {
                let resolved = self.doc.deref(obj)?;
                Ok(Some(resolved.as_dict()?))
            }
            Err(_) => Ok(None),
        }
    }

    /// Get the XObject sub-dictionary.
    pub fn xobject_dict(&self) -> Result<Option<&lopdf::Dictionary>> {
        match self.doc.dict_get_opt(self.dict, b"XObject")? {
            Some(obj) => Ok(Some(obj.as_dict()?)),
            None => Ok(None),
        }
    }

    /// Get an XObject by name, returning its dictionary and stream.
    pub fn get_xobject(&self, name: &[u8]) -> Result<Option<&Object>> {
        let xobj_dict = match self.xobject_dict()? {
            Some(d) => d,
            None => return Ok(None),
        };

        match xobj_dict.get(name) {
            Ok(obj) => Ok(Some(self.doc.deref(obj)?)),
            Err(_) => Ok(None),
        }
    }

    /// Get the ColorSpace sub-dictionary.
    pub fn color_space_dict(&self) -> Result<Option<&lopdf::Dictionary>> {
        match self.doc.dict_get_opt(self.dict, b"ColorSpace")? {
            Some(obj) => Ok(Some(obj.as_dict()?)),
            None => Ok(None),
        }
    }

    /// Check if a given resource name exists in the Font sub-dictionary.
    pub fn has_font(&self, name: &[u8]) -> Result<bool> {
        let font_dict = match self.font_dict()? {
            Some(d) => d,
            None => return Ok(false),
        };
        Ok(font_dict.has(name))
    }

    /// Get the underlying dictionary.
    pub fn dictionary(&self) -> &lopdf::Dictionary {
        self.dict
    }

    /// Get a reference to the document.
    pub fn document(&self) -> &Document {
        &self.doc
    }
}

impl<'a> std::fmt::Debug for PdfResources<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfResources").finish()
    }
}
