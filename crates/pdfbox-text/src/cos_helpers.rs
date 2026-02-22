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
}

/// Extension trait for lopdf Object with additional functionality beyond lopdf's built-in methods.
pub trait ObjectExt {
    /// Get as dictionary, also accepting Stream objects (returns the stream's dictionary).
    /// Unlike lopdf's `as_dict()`, this also handles Stream objects.
    fn as_dict_or_stream_dict(&self) -> Result<&lopdf::Dictionary>;

    /// Get as float, accepting both Integer and Real.
    /// lopdf's `as_f32()` already does this, but this returns our PdfError.
    fn to_f32(&self) -> Result<f32>;

    /// Get the object ID if this is a reference.
    fn as_object_id(&self) -> Result<ObjectId>;
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

    fn to_f32(&self) -> Result<f32> {
        match self {
            Object::Integer(n) => Ok(*n as f32),
            Object::Real(n) => Ok(*n as f32),
            _ => Err(PdfError::Parse(format!(
                "expected number, got {:?}",
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
}
