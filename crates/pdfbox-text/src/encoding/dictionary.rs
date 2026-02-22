//! DictionaryEncoding: PDF encoding defined by an /Encoding dictionary.
//!
//! Handles /BaseEncoding + /Differences array.
//! Ported from org.apache.pdfbox.pdmodel.font.encoding.DictionaryEncoding.

use lopdf::{Document, Object};

use super::predefined;
use super::Encoding;
use crate::cos_helpers::name_to_string;

/// Build an Encoding from a PDF /Encoding entry.
///
/// The /Encoding value can be:
/// - A name (e.g., /WinAnsiEncoding) → use predefined encoding directly
/// - A dictionary with optional /BaseEncoding name + /Differences array
///
/// `is_symbolic`: if true and no /BaseEncoding is specified, use StandardEncoding
/// as the base (PDFBox behavior for symbolic fonts differs, but StandardEncoding
/// is the PDF spec default for non-symbolic).
pub fn build_encoding(
    doc: &Document,
    encoding_obj: &Object,
    is_symbolic: bool,
) -> Encoding {
    match encoding_obj {
        // Direct name reference: /Encoding /WinAnsiEncoding
        Object::Name(name_bytes) => {
            let name = name_to_string(name_bytes);
            if let Some(enc) = predefined::get_encoding(&name) {
                enc.clone()
            } else {
                // Unknown encoding name — return empty
                Encoding::new(&name)
            }
        }
        // Reference — dereference and recurse
        Object::Reference(id) => {
            if let Ok(resolved) = doc.get_object(*id) {
                build_encoding(doc, resolved, is_symbolic)
            } else {
                Encoding::new("Unknown")
            }
        }
        // Dictionary with /BaseEncoding and /Differences
        Object::Dictionary(dict) => {
            build_from_dict(doc, dict, is_symbolic)
        }
        // Stream can also have a dict
        Object::Stream(stream) => {
            build_from_dict(doc, &stream.dict, is_symbolic)
        }
        _ => Encoding::new("Unknown"),
    }
}

fn build_from_dict(
    doc: &Document,
    dict: &lopdf::Dictionary,
    is_symbolic: bool,
) -> Encoding {
    // Determine base encoding
    let base = if let Ok(base_obj) = dict.get(b"BaseEncoding") {
        match base_obj {
            Object::Name(name_bytes) => {
                let name = name_to_string(name_bytes);
                predefined::get_encoding(&name)
                    .cloned()
                    .unwrap_or_else(|| Encoding::new(&name))
            }
            _ => default_base_encoding(is_symbolic),
        }
    } else {
        default_base_encoding(is_symbolic)
    };

    // Start with base encoding entries
    let mut enc = Encoding::new("DictionaryEncoding");
    for (code, name) in base.iter() {
        enc.add(code, name);
    }

    // Apply /Differences array
    if let Ok(diffs) = dict.get(b"Differences") {
        apply_differences(doc, &mut enc, diffs);
    }

    enc
}

/// Get the default base encoding when none is specified.
fn default_base_encoding(is_symbolic: bool) -> Encoding {
    if is_symbolic {
        // For symbolic fonts, start with empty encoding
        Encoding::new("BuiltIn")
    } else {
        // PDF spec: default base encoding is StandardEncoding
        predefined::STANDARD.clone()
    }
}

/// Apply a /Differences array to an encoding.
///
/// Format: [ code1 /name1 /name2 ... codeN /nameN ... ]
/// Each integer sets the current code, each name maps to the current code
/// and increments it.
fn apply_differences(doc: &Document, enc: &mut Encoding, diffs_obj: &Object) {
    let arr = match diffs_obj {
        Object::Array(arr) => arr,
        Object::Reference(id) => {
            if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                arr
            } else {
                return;
            }
        }
        _ => return,
    };

    let mut current_code: u16 = 0;

    for item in arr {
        let item = match item {
            Object::Reference(id) => {
                if let Ok(obj) = doc.get_object(*id) {
                    obj
                } else {
                    continue;
                }
            }
            other => other,
        };

        match item {
            Object::Integer(n) => {
                current_code = *n as u16;
            }
            Object::Name(name_bytes) => {
                let name = name_to_string(name_bytes);
                enc.add(current_code, &name);
                current_code = current_code.wrapping_add(1);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};

    #[test]
    fn test_name_encoding() {
        let doc = Document::new();
        let obj = Object::Name(b"WinAnsiEncoding".to_vec());
        let enc = build_encoding(&doc, &obj, false);
        assert_eq!(enc.get_name(65), Some("A"));
        assert_eq!(enc.get_name(32), Some("space"));
    }

    #[test]
    fn test_dictionary_encoding_with_differences() {
        let doc = Document::new();
        let dict = dictionary! {
            "BaseEncoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "Differences" => Object::Array(vec![
                Object::Integer(65),
                Object::Name(b"Aspecial".to_vec()),
                Object::Name(b"Bspecial".to_vec()),
                Object::Integer(200),
                Object::Name(b"myGlyph".to_vec()),
            ])
        };
        let obj = Object::Dictionary(dict);
        let enc = build_encoding(&doc, &obj, false);

        // Differences override
        assert_eq!(enc.get_name(65), Some("Aspecial"));
        assert_eq!(enc.get_name(66), Some("Bspecial")); // auto-incremented
        assert_eq!(enc.get_name(200), Some("myGlyph"));

        // Non-overridden entries from base
        assert_eq!(enc.get_name(32), Some("space"));
        assert_eq!(enc.get_name(67), Some("C")); // from WinAnsi
    }

    #[test]
    fn test_empty_differences() {
        let doc = Document::new();
        let dict = dictionary! {
            "BaseEncoding" => Object::Name(b"StandardEncoding".to_vec())
        };
        let obj = Object::Dictionary(dict);
        let enc = build_encoding(&doc, &obj, false);
        assert_eq!(enc.get_name(65), Some("A"));
    }

    #[test]
    fn test_no_base_encoding_nonsymbolic() {
        let doc = Document::new();
        let dict = dictionary! {
            "Differences" => Object::Array(vec![
                Object::Integer(65),
                Object::Name(b"CustomA".to_vec()),
            ])
        };
        let obj = Object::Dictionary(dict);
        let enc = build_encoding(&doc, &obj, false);

        // Should default to StandardEncoding, then apply differences
        assert_eq!(enc.get_name(65), Some("CustomA"));
        assert_eq!(enc.get_name(32), Some("space")); // from StandardEncoding
    }

    #[test]
    fn test_symbolic_no_base() {
        let doc = Document::new();
        let dict = dictionary! {
            "Differences" => Object::Array(vec![
                Object::Integer(1),
                Object::Name(b"sym1".to_vec()),
                Object::Name(b"sym2".to_vec()),
            ])
        };
        let obj = Object::Dictionary(dict);
        let enc = build_encoding(&doc, &obj, true);

        // Symbolic fonts start with empty base
        assert_eq!(enc.get_name(1), Some("sym1"));
        assert_eq!(enc.get_name(2), Some("sym2"));
        assert_eq!(enc.get_name(32), None); // no base encoding
    }
}
