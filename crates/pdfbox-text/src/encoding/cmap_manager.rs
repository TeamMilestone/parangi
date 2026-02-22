//! CMap manager: loads and caches predefined CMaps.
//!
//! Ported from org.apache.pdfbox.pdmodel.font.CMapManager.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::cmap::CMap;
use super::cmap_parser;

// Include the generated CMap data table.
include!(concat!(env!("OUT_DIR"), "/cmap_data.rs"));

/// Global cache of predefined CMaps.
static CMAP_CACHE: LazyLock<Mutex<HashMap<String, CMap>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get a predefined CMap by name.
///
/// Returns a cloned CMap from the cache, parsing it on first access.
/// Returns None if the CMap name is not found in the predefined data.
pub fn get_predefined_cmap(name: &str) -> Option<CMap> {
    // Check cache first
    {
        let cache = CMAP_CACHE.lock().unwrap();
        if let Some(cmap) = cache.get(name) {
            return Some(cmap.clone());
        }
    }

    // Parse from embedded data
    let data = PREDEFINED_CMAPS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, data)| *data)?;

    let cmap = cmap_parser::parse_cmap(data.as_bytes());

    // Cache it
    {
        let mut cache = CMAP_CACHE.lock().unwrap();
        cache.insert(name.to_string(), cmap.clone());
    }

    Some(cmap)
}

/// Check if a predefined CMap exists by name.
pub fn has_predefined_cmap(name: &str) -> bool {
    PREDEFINED_CMAPS.iter().any(|(n, _)| *n == name)
}

/// Get the Identity-H CMap (code = CID).
pub fn identity_h() -> CMap {
    get_predefined_cmap("Identity-H").unwrap_or_else(|| {
        // Fallback: create a simple identity CMap
        let mut cmap = CMap::new();
        cmap.name = "Identity-H".to_string();
        cmap
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_h() {
        let cmap = get_predefined_cmap("Identity-H");
        assert!(cmap.is_some(), "Identity-H should be available");
        let cmap = cmap.unwrap();
        assert!(
            cmap.codespace_ranges().len() > 0,
            "Identity-H should have codespace ranges"
        );
    }

    #[test]
    fn test_korea_ucs2() {
        let cmap = get_predefined_cmap("Adobe-Korea1-UCS2");
        assert!(cmap.is_some(), "Adobe-Korea1-UCS2 should be available");
        let cmap = cmap.unwrap();
        assert!(
            cmap.unicode_mapping_count() > 0,
            "Adobe-Korea1-UCS2 should have Unicode mappings"
        );
    }

    #[test]
    fn test_nonexistent() {
        assert!(get_predefined_cmap("NoSuchCMap").is_none());
    }

    #[test]
    fn test_cache_works() {
        // First call parses
        let cmap1 = get_predefined_cmap("Identity-V");
        assert!(cmap1.is_some());
        // Second call should hit cache
        let cmap2 = get_predefined_cmap("Identity-V");
        assert!(cmap2.is_some());
    }

    #[test]
    fn test_has_predefined() {
        assert!(has_predefined_cmap("Identity-H"));
        assert!(has_predefined_cmap("Adobe-Korea1-UCS2"));
        assert!(!has_predefined_cmap("FakeCMap"));
    }
}
