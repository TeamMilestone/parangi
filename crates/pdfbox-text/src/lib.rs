//! PDF text extraction library ported from Apache PDFBox.
//!
//! This crate provides high-performance text extraction from PDF documents,
//! with full CJK support and multi-core parallelism via rayon.

pub mod cos_helpers;
pub mod document;
pub mod page;
pub mod resources;

pub mod encoding;
pub mod font;
pub mod stream;
pub mod text;

mod error;

pub use document::PdfDocument;
pub use error::{PdfError, Result};
pub use text::{StripperConfig, assemble_text};

/// Extract all text from a PDF file using the full stream engine and text assembly.
pub fn extract_text(path: &std::path::Path) -> Result<String> {
    extract_text_with_config(path, &StripperConfig::default())
}

/// Extract all text from a PDF file with custom configuration.
pub fn extract_text_with_config(
    path: &std::path::Path,
    config: &StripperConfig,
) -> Result<String> {
    let doc = PdfDocument::open(path)?;
    let mut output = String::new();

    for page_num in 0..doc.page_count() {
        let page = doc.page(page_num)?;

        // Get page geometry
        let content_bytes = page.content_bytes()?;
        if content_bytes.is_empty() {
            continue;
        }

        let resources = page.resources()?;
        let rotation = page.rotation().unwrap_or(0) as i32;
        let media_box = page.media_box()?;

        // Process content stream
        let mut engine = stream::engine::StreamEngine::new(doc.inner_arc());
        engine.set_page_info(rotation, media_box[2], media_box[3]);
        if let Some(ref res) = resources {
            engine.load_resources(res.dictionary());
        }
        engine.process_content(&content_bytes)?;

        // Assemble text from positions
        let mut positions = engine.into_text_positions();
        if positions.is_empty() {
            continue;
        }

        let page_text = assemble_text(&mut positions, config);
        if !page_text.is_empty() {
            if !output.is_empty() {
                output.push_str(&config.page_separator);
            }
            output.push_str(&page_text);
        }
    }

    Ok(output)
}
