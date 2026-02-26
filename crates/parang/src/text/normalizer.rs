//! Unicode normalization and diacritics merging for extracted text.
//!
//! Ported from PDFTextStripper.normalize() and related logic.
//! Handles:
//! - Unicode NFC normalization
//! - Combining diacritical mark merging with preceding characters
//! - Ligature decomposition (fi → fi, fl → fl, etc.)

use unicode_normalization::UnicodeNormalization;

/// Normalize a string of extracted text.
///
/// Applies:
/// 1. Ligature decomposition (optional, common PDF ligatures)
/// 2. Unicode NFC normalization
pub fn normalize_text(text: &str) -> String {
    let decomposed = decompose_ligatures(text);
    decomposed.nfc().collect()
}

/// Decompose common ligatures into their constituent characters.
///
/// PDF fonts often use ligature glyphs (fi, fl, ffi, ffl, etc.) that map
/// to single Unicode codepoints. For text search and comparison, these
/// should be decomposed into individual characters.
fn decompose_ligatures(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\u{FB00}' => result.push_str("ff"),
            '\u{FB01}' => result.push_str("fi"),
            '\u{FB02}' => result.push_str("fl"),
            '\u{FB03}' => result.push_str("ffi"),
            '\u{FB04}' => result.push_str("ffl"),
            '\u{FB05}' => result.push_str("st"), // long s + t
            '\u{FB06}' => result.push_str("st"),
            _ => result.push(ch),
        }
    }
    result
}

/// Merge combining diacritical marks with preceding TextPosition unicode values.
///
/// In some PDFs, combining marks (U+0300-U+036F) are emitted as separate glyphs.
/// This function merges them with the preceding base character.
///
/// Works on the raw TextPosition unicode strings before text assembly.
/// Returns true if any merging occurred.
pub fn merge_diacritics(unicode_values: &mut [String]) -> bool {
    if unicode_values.len() < 2 {
        return false;
    }

    let mut merged = false;
    let mut i = 1;

    while i < unicode_values.len() {
        if is_combining_mark(&unicode_values[i]) && !unicode_values[i - 1].is_empty() {
            // Merge combining mark into previous character
            let mark = unicode_values[i].clone();
            unicode_values[i - 1].push_str(&mark);

            // NFC normalize the merged result
            let normalized: String = unicode_values[i - 1].nfc().collect();
            unicode_values[i - 1] = normalized;

            // Mark current position as empty (to be removed later)
            unicode_values[i].clear();
            merged = true;
        }
        i += 1;
    }

    merged
}

/// Merge combining diacritical marks directly on TextPositions.
///
/// Avoids cloning all unicode strings — only processes positions with
/// combining marks, using std::mem::take for zero-copy transfers.
/// Includes a quick early-return if no combining marks exist.
pub fn merge_diacritics_in_place(positions: &mut Vec<super::TextPosition>) {
    if positions.len() < 2 {
        return;
    }

    // Quick check: any combining marks at all?
    let has_combining = positions.iter().any(|p| {
        !p.unicode.is_empty() && p.unicode.chars().all(|ch| is_combining_char(ch))
    });
    if !has_combining {
        return;
    }

    let mut i = 1;
    while i < positions.len() {
        if !positions[i].unicode.is_empty()
            && positions[i].unicode.chars().all(|ch| is_combining_char(ch))
            && !positions[i - 1].unicode.is_empty()
        {
            let mark = std::mem::take(&mut positions[i].unicode);
            positions[i - 1].unicode.push_str(&mark);
            let normalized: String = positions[i - 1].unicode.nfc().collect();
            positions[i - 1].unicode = normalized.into();
        }
        i += 1;
    }
    positions.retain(|p| !p.unicode.is_empty());
}

/// Check if a string consists entirely of combining diacritical marks.
fn is_combining_mark(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().all(|ch| is_combining_char(ch))
}

/// Check if a character is a Unicode combining character.
fn is_combining_char(ch: char) -> bool {
    let cp = ch as u32;
    // Combining Diacritical Marks
    (0x0300..=0x036F).contains(&cp)
        // Combining Diacritical Marks Extended
        || (0x1AB0..=0x1AFF).contains(&cp)
        // Combining Diacritical Marks Supplement
        || (0x1DC0..=0x1DFF).contains(&cp)
        // Combining Half Marks
        || (0xFE20..=0xFE2F).contains(&cp)
        // Combining Diacritical Marks for Symbols
        || (0x20D0..=0x20FF).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        assert_eq!(normalize_text("Hello"), "Hello");
        assert_eq!(normalize_text(""), "");
    }

    #[test]
    fn test_ligature_decomposition() {
        // fi ligature → "fi"
        assert_eq!(normalize_text("\u{FB01}nd"), "find");
        // fl ligature → "fl"
        assert_eq!(normalize_text("\u{FB02}ow"), "flow");
        // ff ligature → "ff"
        assert_eq!(normalize_text("\u{FB00}ice"), "ffice");
        // ffi ligature → "ffi"
        assert_eq!(normalize_text("o\u{FB03}ce"), "office");
        // ffl ligature → "ffl"
        assert_eq!(normalize_text("ba\u{FB04}e"), "baffle");
    }

    #[test]
    fn test_nfc_normalization() {
        // e + combining acute accent → é
        let input = "e\u{0301}";
        let result = normalize_text(input);
        assert_eq!(result, "é");
    }

    #[test]
    fn test_merge_diacritics() {
        let mut values = vec![
            "e".to_string(),
            "\u{0301}".to_string(), // combining acute accent
            "s".to_string(),
        ];
        let merged = merge_diacritics(&mut values);
        assert!(merged);
        assert_eq!(values[0], "é");
        assert_eq!(values[1], ""); // Cleared
        assert_eq!(values[2], "s");
    }

    #[test]
    fn test_merge_diacritics_no_combining() {
        let mut values = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let merged = merge_diacritics(&mut values);
        assert!(!merged);
        assert_eq!(values, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_is_combining_mark() {
        assert!(is_combining_mark("\u{0300}")); // Combining grave accent
        assert!(is_combining_mark("\u{0301}")); // Combining acute accent
        assert!(is_combining_mark("\u{0308}")); // Combining diaeresis
        assert!(!is_combining_mark("a"));
        assert!(!is_combining_mark(""));
        assert!(!is_combining_mark("ab"));
    }

    #[test]
    fn test_korean_text_unchanged() {
        // Korean text should pass through unchanged
        assert_eq!(normalize_text("안녕하세요"), "안녕하세요");
    }

    #[test]
    fn test_merge_multiple_diacritics() {
        let mut values = vec![
            "o".to_string(),
            "\u{0308}".to_string(), // combining diaeresis → ö
            "u".to_string(),
            "\u{0308}".to_string(), // combining diaeresis → ü
        ];
        let merged = merge_diacritics(&mut values);
        assert!(merged);
        assert_eq!(values[0], "ö");
        assert_eq!(values[2], "ü");
    }
}
