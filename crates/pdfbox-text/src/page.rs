//! PDF page access and content stream extraction.

use std::sync::Arc;

use lopdf::{Document, Object, ObjectId};

use crate::cos_helpers::DocumentExt;
use crate::resources::PdfResources;
use crate::{PdfError, Result};

/// A single page of a PDF document.
pub struct PdfPage {
    doc: Arc<Document>,
    object_id: ObjectId,
}

impl PdfPage {
    /// Create a new PdfPage.
    pub(crate) fn new(doc: Arc<Document>, object_id: ObjectId) -> Result<Self> {
        // Verify the object exists and is a page dictionary
        let obj = doc
            .get_object(object_id)
            .map_err(|e| PdfError::Parse(format!("page object {:?}: {}", object_id, e)))?;
        let _dict = obj.as_dict()?;
        Ok(Self { doc, object_id })
    }

    /// Get the page dictionary.
    pub fn dictionary(&self) -> Result<&lopdf::Dictionary> {
        Ok(self.doc
            .get_object(self.object_id)
            .map_err(|e| PdfError::Parse(e.to_string()))?
            .as_dict()?)
    }

    /// Get page resources (fonts, xobjects, etc.).
    pub fn resources(&self) -> Result<Option<PdfResources<'_>>> {
        let dict = self.dictionary()?;

        // Resources may be on the page itself or inherited from parent
        if let Some(res_obj) = self.doc.dict_get_opt(dict, b"Resources")? {
            let res_dict = res_obj.as_dict()?;
            return Ok(Some(PdfResources::new(Arc::clone(&self.doc), res_dict)));
        }

        // Try parent page tree node
        if let Some(parent_obj) = self.doc.dict_get_opt(dict, b"Parent")? {
            let parent_dict = parent_obj.as_dict()?;
            if let Some(res_obj) = self.doc.dict_get_opt(parent_dict, b"Resources")? {
                let res_dict = res_obj.as_dict()?;
                return Ok(Some(PdfResources::new(Arc::clone(&self.doc), res_dict)));
            }
        }

        Ok(None)
    }

    /// Get the raw content stream bytes for this page.
    ///
    /// A page may have a single stream or an array of streams.
    /// Multiple streams are concatenated with a newline separator.
    pub fn content_bytes(&self) -> Result<Vec<u8>> {
        let dict = self.dictionary()?;
        let contents = match self.doc.dict_get_opt(dict, b"Contents")? {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };

        match contents {
            Object::Array(arr) => {
                let mut bytes = Vec::new();
                for item in arr {
                    let item = self.doc.deref(item)?;
                    if let Object::Stream(stream) = item {
                        let data = stream.decompressed_content().map_err(|e| {
                            PdfError::Parse(format!("stream decompression failed: {}", e))
                        })?;
                        if !bytes.is_empty() {
                            bytes.push(b'\n');
                        }
                        bytes.extend_from_slice(&data);
                    }
                }
                Ok(bytes)
            }
            Object::Stream(stream) => {
                let data = stream.decompressed_content().map_err(|e| {
                    PdfError::Parse(format!("stream decompression failed: {}", e))
                })?;
                Ok(data)
            }
            Object::Reference(id) => {
                let obj = self.doc.get_object(*id).map_err(|e| {
                    PdfError::Parse(format!("content ref {:?}: {}", id, e))
                })?;
                if let Object::Stream(stream) = obj {
                    let data = stream.decompressed_content().map_err(|e| {
                        PdfError::Parse(format!("stream decompression failed: {}", e))
                    })?;
                    Ok(data)
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Get the media box (page dimensions) as [x0, y0, x1, y1].
    pub fn media_box(&self) -> Result<[f32; 4]> {
        let dict = self.dictionary()?;
        self.get_box(dict, b"MediaBox")?
            .ok_or_else(|| PdfError::MissingEntry("MediaBox".into()))
    }

    /// Get the crop box, falling back to media box.
    pub fn crop_box(&self) -> Result<[f32; 4]> {
        let dict = self.dictionary()?;
        if let Some(crop) = self.get_box(dict, b"CropBox")? {
            return Ok(crop);
        }
        self.media_box()
    }

    /// Get page rotation in degrees (0, 90, 180, 270).
    pub fn rotation(&self) -> Result<i64> {
        let dict = self.dictionary()?;
        Ok(self.doc.dict_get_i64(dict, b"Rotate")?.unwrap_or(0))
    }

    /// Get the page object ID.
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Get a reference to the underlying document.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Basic text extraction: just extract raw string operands from content stream.
    /// This is a placeholder until the full stream engine is implemented.
    pub fn extract_text_basic(&self) -> Result<String> {
        let content_bytes = self.content_bytes()?;
        if content_bytes.is_empty() {
            return Ok(String::new());
        }

        let content = lopdf::content::Content::decode(&content_bytes)
            .map_err(|e| PdfError::Parse(format!("content decode: {}", e)))?;

        let mut text = String::new();
        for op in &content.operations {
            match op.operator.as_str() {
                "Tj" | "TJ" => {
                    for operand in &op.operands {
                        Self::collect_text_from_operand(operand, &mut text);
                    }
                }
                "'" | "\"" => {
                    // These operators also show text (with line spacing)
                    for operand in &op.operands {
                        Self::collect_text_from_operand(operand, &mut text);
                    }
                }
                _ => {}
            }
        }

        Ok(text)
    }

    fn collect_text_from_operand(obj: &Object, out: &mut String) {
        match obj {
            Object::String(bytes, _) => {
                // Basic: assume Latin-1 for now; proper decoding comes in later iterations
                for &b in bytes.iter() {
                    out.push(b as char);
                }
            }
            Object::Array(arr) => {
                for item in arr {
                    Self::collect_text_from_operand(item, out);
                }
            }
            _ => {}
        }
    }

    fn get_box(&self, dict: &lopdf::Dictionary, key: &[u8]) -> Result<Option<[f32; 4]>> {
        match self.doc.dict_get_opt(dict, key)? {
            Some(Object::Array(arr)) if arr.len() >= 4 => {
                let x0 = arr[0].as_f32()?;
                let y0 = arr[1].as_f32()?;
                let x1 = arr[2].as_f32()?;
                let y1 = arr[3].as_f32()?;
                Ok(Some([x0, y0, x1, y1]))
            }
            _ => Ok(None),
        }
    }
}

impl std::fmt::Debug for PdfPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfPage")
            .field("object_id", &self.object_id)
            .finish()
    }
}
