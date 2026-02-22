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

use rayon::prelude::*;

/// Extract all text from a PDF file using the full stream engine and text assembly.
pub fn extract_text(path: &std::path::Path) -> Result<String> {
    extract_text_with_config(path, &StripperConfig::default())
}

/// Extract all text from a PDF file with custom configuration.
///
/// Uses rayon for page-level parallelism: each page is processed independently
/// on a separate thread, then results are merged in page order.
pub fn extract_text_with_config(
    path: &std::path::Path,
    config: &StripperConfig,
) -> Result<String> {
    let doc = PdfDocument::open(path)?;
    let page_count = doc.page_count();

    if page_count == 0 {
        return Ok(String::new());
    }

    // Collect per-page data sequentially (lopdf page tree traversal is not thread-safe)
    let page_data: Vec<_> = (0..page_count)
        .filter_map(|page_num| {
            let page = doc.page(page_num).ok()?;
            let content_bytes = page.content_bytes().ok()?;
            if content_bytes.is_empty() {
                return None;
            }
            let resources_dict = page
                .resources()
                .ok()
                .flatten()
                .map(|r| r.dictionary().clone());
            let rotation = page.rotation().unwrap_or(0) as i32;
            let media_box = page.media_box().ok()?;
            Some((page_num, content_bytes, resources_dict, rotation, media_box))
        })
        .collect();

    // Process pages in parallel
    let doc_arc = doc.inner_arc();
    let page_texts: Vec<(u32, String)> = page_data
        .into_par_iter()
        .filter_map(|(page_num, content_bytes, resources_dict, rotation, media_box)| {
            let mut engine = stream::engine::StreamEngine::new(doc_arc.clone());
            engine.set_page_info(rotation, media_box[2], media_box[3]);
            if let Some(ref res_dict) = resources_dict {
                engine.load_resources(res_dict);
            }
            if engine.process_content(&content_bytes).is_err() {
                return None;
            }
            let mut positions = engine.into_text_positions();
            if positions.is_empty() {
                return None;
            }
            let page_text = assemble_text(&mut positions, config);
            if page_text.is_empty() {
                return None;
            }
            Some((page_num, page_text))
        })
        .collect();

    // Merge results in page order
    let mut sorted_texts = page_texts;
    sorted_texts.sort_unstable_by_key(|(num, _)| *num);

    let mut output = String::new();
    for (_page_num, text) in sorted_texts {
        if !output.is_empty() {
            output.push_str(&config.page_separator);
        }
        output.push_str(&text);
    }

    Ok(output)
}

/// Result of extracting text from a single PDF file in a batch.
pub struct BatchResult {
    /// The path of the processed file.
    pub path: std::path::PathBuf,
    /// The extracted text, or an error.
    pub result: Result<String>,
}

/// Extract text from multiple PDF files in parallel.
///
/// Uses rayon for file-level parallelism. Each file internally uses
/// page-level parallelism, creating nested parallelism (file × page).
/// Rayon's work-stealing scheduler handles the nested parallelism efficiently.
pub fn extract_text_batch(
    paths: &[std::path::PathBuf],
    config: &StripperConfig,
) -> Vec<BatchResult> {
    paths
        .par_iter()
        .map(|path| BatchResult {
            path: path.clone(),
            result: extract_text_with_config(path, config),
        })
        .collect()
}

/// Extract all text from a PDF file sequentially (single-threaded).
///
/// Useful for debugging or when parallelism is not desired.
pub fn extract_text_sequential(
    path: &std::path::Path,
    config: &StripperConfig,
) -> Result<String> {
    let doc = PdfDocument::open(path)?;
    let mut output = String::new();

    for page_num in 0..doc.page_count() {
        let page = doc.page(page_num)?;
        let content_bytes = page.content_bytes()?;
        if content_bytes.is_empty() {
            continue;
        }

        let resources = page.resources()?;
        let rotation = page.rotation().unwrap_or(0) as i32;
        let media_box = page.media_box()?;

        let mut engine = stream::engine::StreamEngine::new(doc.inner_arc());
        engine.set_page_info(rotation, media_box[2], media_box[3]);
        if let Some(ref res) = resources {
            engine.load_resources(res.dictionary());
        }
        engine.process_content(&content_bytes)?;

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
