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

/// Extract all text from a PDF file.
pub fn extract_text(path: &std::path::Path) -> Result<String> {
    let doc = PdfDocument::open(path)?;
    let mut output = String::new();
    for page_num in 0..doc.page_count() {
        let page = doc.page(page_num)?;
        let page_text = page.extract_text_basic()?;
        if !page_text.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&page_text);
        }
    }
    Ok(output)
}
