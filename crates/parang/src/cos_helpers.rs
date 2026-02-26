//! Helper traits and functions for working with lopdf objects.

use lopdf::{Document, Object, ObjectId};

use crate::{PdfError, Result};

/// Extension trait for lopdf Document to simplify common operations.
pub trait DocumentExt {
    /// Dereference an object, following indirect references.
    fn deref<'a>(&'a self, obj: &'a Object) -> Result<&'a Object>;

    /// Get a dictionary entry by key, dereferencing indirect references.
    fn dict_get<'a>(&'a self, dict: &'a lopdf::Dictionary, key: &[u8]) -> Result<&'a Object>;

    /// Try to get a dictionary entry, returning None if not found.
    fn dict_get_opt<'a>(
        &'a self,
        dict: &'a lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<&'a Object>>;

    /// Get a name value from a dictionary entry.
    fn dict_get_name<'a>(
        &'a self,
        dict: &'a lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<&'a [u8]>>;

    /// Get an integer value from a dictionary entry.
    fn dict_get_i64(&self, dict: &lopdf::Dictionary, key: &[u8]) -> Result<Option<i64>>;

    /// Get a float value from a dictionary entry (accepts both integer and real).
    fn dict_get_f32(&self, dict: &lopdf::Dictionary, key: &[u8]) -> Result<Option<f32>>;

    /// Get a numeric array from a dictionary entry as Vec<f32>.
    fn dict_get_number_array(
        &self,
        dict: &lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<Vec<f32>>>;

    /// Get a stream object and decompress its content.
    fn get_stream_data(&self, obj: &Object) -> Result<Vec<u8>>;

    /// Get raw bytes of a lazy-loaded stream from the backing buffer.
    fn get_stream_backing_bytes<'a>(&'a self, stream: &lopdf::Stream) -> Option<&'a [u8]>;
}

impl DocumentExt for Document {
    fn deref<'a>(&'a self, obj: &'a Object) -> Result<&'a Object> {
        match obj {
            Object::Reference(id) => self
                .get_object(*id)
                .map_err(|e| PdfError::Parse(format!("failed to dereference {:?}: {}", id, e))),
            other => Ok(other),
        }
    }

    fn dict_get<'a>(&'a self, dict: &'a lopdf::Dictionary, key: &[u8]) -> Result<&'a Object> {
        let obj = dict
            .get(key)
            .map_err(|_| PdfError::MissingEntry(String::from_utf8_lossy(key).into_owned()))?;
        self.deref(obj)
    }

    fn dict_get_opt<'a>(
        &'a self,
        dict: &'a lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<&'a Object>> {
        match dict.get(key) {
            Ok(obj) => Ok(Some(self.deref(obj)?)),
            Err(_) => Ok(None),
        }
    }

    fn dict_get_name<'a>(
        &'a self,
        dict: &'a lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<&'a [u8]>> {
        match self.dict_get_opt(dict, key)? {
            Some(Object::Name(name)) => Ok(Some(name.as_slice())),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }

    fn dict_get_i64(&self, dict: &lopdf::Dictionary, key: &[u8]) -> Result<Option<i64>> {
        match self.dict_get_opt(dict, key)? {
            Some(Object::Integer(n)) => Ok(Some(*n)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }

    fn dict_get_f32(&self, dict: &lopdf::Dictionary, key: &[u8]) -> Result<Option<f32>> {
        match self.dict_get_opt(dict, key)? {
            Some(Object::Integer(n)) => Ok(Some(*n as f32)),
            Some(Object::Real(n)) => Ok(Some(*n as f32)),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }

    fn dict_get_number_array(
        &self,
        dict: &lopdf::Dictionary,
        key: &[u8],
    ) -> Result<Option<Vec<f32>>> {
        match self.dict_get_opt(dict, key)? {
            Some(Object::Array(arr)) => {
                let mut result = Vec::with_capacity(arr.len());
                for item in arr {
                    let item = self.deref(item)?;
                    result.push(obj_to_f32(item)?);
                }
                Ok(Some(result))
            }
            None => Ok(None),
            _ => Ok(None),
        }
    }

    fn get_stream_data(&self, obj: &Object) -> Result<Vec<u8>> {
        let obj = match obj {
            Object::Reference(id) => self
                .get_object(*id)
                .map_err(|e| PdfError::Parse(format!("stream ref: {}", e)))?,
            other => other,
        };
        match obj {
            Object::Stream(stream) => {
                // If content is empty (lazy-loaded stream), try the backing buffer first.
                if stream.content.is_empty() {
                    if let Some(raw) = self.get_stream_backing_bytes(stream) {
                        return match stream.decompressed_content_from(raw) {
                            Ok(data) => Ok(data),
                            Err(_) => Ok(raw.to_vec()),
                        };
                    }
                }
                // Fall back to stored content (or empty if no backing buffer).
                match stream.decompressed_content() {
                    Ok(data) => Ok(data),
                    Err(_) => Ok(stream.content.clone()),
                }
            }
            _ => Err(PdfError::Parse(format!(
                "expected stream, got {:?}",
                obj.type_name()
            ))),
        }
    }

    fn get_stream_backing_bytes<'a>(&'a self, stream: &lopdf::Stream) -> Option<&'a [u8]> {
        Document::get_stream_backing_bytes(self, stream)
    }
}

/// Extension trait for lopdf Object with additional functionality beyond lopdf's built-in methods.
pub trait ObjectExt {
    /// Get as dictionary, also accepting Stream objects (returns the stream's dictionary).
    /// Unlike lopdf's `as_dict()`, this also handles Stream objects.
    fn as_dict_or_stream_dict(&self) -> Result<&lopdf::Dictionary>;

    /// Get the object ID if this is a reference.
    fn as_object_id(&self) -> Result<ObjectId>;

    /// Check if this object is null.
    fn is_null(&self) -> bool;
}

impl ObjectExt for Object {
    fn as_dict_or_stream_dict(&self) -> Result<&lopdf::Dictionary> {
        match self {
            Object::Dictionary(d) => Ok(d),
            Object::Stream(s) => Ok(&s.dict),
            _ => Err(PdfError::Parse(format!(
                "expected dictionary or stream, got {:?}",
                self.type_name()
            ))),
        }
    }

    fn as_object_id(&self) -> Result<ObjectId> {
        match self {
            Object::Reference(id) => Ok(*id),
            _ => Err(PdfError::Parse(format!(
                "expected reference, got {:?}",
                self.type_name()
            ))),
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Object::Null)
    }
}

/// Convert a PDF object to f32 (accepts Integer or Real).
pub fn obj_to_f32(obj: &Object) -> Result<f32> {
    match obj {
        Object::Integer(n) => Ok(*n as f32),
        Object::Real(n) => Ok(*n as f32),
        _ => Err(PdfError::Parse(format!(
            "expected number, got {:?}",
            obj.type_name()
        ))),
    }
}

/// Convert a PDF name object (byte slice) to a UTF-8 string.
/// PDF names can contain #XX hex-encoded bytes.
pub fn name_to_string(name: &[u8]) -> String {
    let mut result = Vec::with_capacity(name.len());
    let mut i = 0;
    while i < name.len() {
        if name[i] == b'#' && i + 2 < name.len() {
            if let (Some(hi), Some(lo)) = (
                hex_digit(name[i + 1]),
                hex_digit(name[i + 2]),
            ) {
                result.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        result.push(name[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decode a PDF text string (byte string or hex string) to a Rust String.
/// Handles both UTF-16BE (BOM: FE FF) and PDFDocEncoding (Latin-1 superset).
pub fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM
        let mut chars = Vec::new();
        let mut i = 2;
        while i + 1 < bytes.len() {
            let code_unit = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            chars.push(code_unit);
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 with BOM
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else {
        // PDFDocEncoding (superset of Latin-1)
        bytes.iter().map(|&b| b as char).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_to_string_simple() {
        assert_eq!(name_to_string(b"Helvetica"), "Helvetica");
    }

    #[test]
    fn test_name_to_string_hex_encoded() {
        // #20 is space
        assert_eq!(name_to_string(b"My#20Font"), "My Font");
    }

    #[test]
    fn test_name_to_string_hex_encoded_sharp() {
        // #23 is '#'
        assert_eq!(name_to_string(b"A#23B"), "A#B");
    }

    #[test]
    fn test_decode_text_string_latin1() {
        assert_eq!(decode_text_string(b"Hello"), "Hello");
    }

    #[test]
    fn test_decode_text_string_utf16be() {
        // FE FF = BOM, then 'H' 'i' in UTF-16BE
        let bytes = [0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
        assert_eq!(decode_text_string(&bytes), "Hi");
    }

    #[test]
    fn test_decode_text_string_utf8_bom() {
        let bytes = [0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
        assert_eq!(decode_text_string(&bytes), "Hello");
    }

    #[test]
    fn test_obj_to_f32() {
        assert_eq!(obj_to_f32(&Object::Integer(42)).unwrap(), 42.0);
        assert!((obj_to_f32(&Object::Real(3.14)).unwrap() - 3.14).abs() < 0.01);
        assert!(obj_to_f32(&Object::Boolean(true)).is_err());
    }

    #[test]
    fn test_object_ext_is_null() {
        assert!(Object::Null.is_null());
        assert!(!Object::Integer(0).is_null());
    }

    #[test]
    fn test_object_ext_as_dict_or_stream_dict() {
        let dict = lopdf::Dictionary::new();
        let obj = Object::Dictionary(dict);
        assert!(obj.as_dict_or_stream_dict().is_ok());

        let stream = lopdf::Stream::new(lopdf::Dictionary::new(), vec![]);
        let obj = Object::Stream(stream);
        assert!(obj.as_dict_or_stream_dict().is_ok());

        assert!(Object::Integer(0).as_dict_or_stream_dict().is_err());
    }
}
