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
use stream::engine::FontCache;
use hashbrown::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

/// Extract all text from a PDF file using the full stream engine and text assembly.
pub fn extract_text(path: &std::path::Path) -> Result<String> {
    extract_text_with_config(path, &StripperConfig::default())
}

/// Extract all text from a PDF file with custom configuration.
///
/// Uses rayon for page-level parallelism: each page is processed independently
/// on a separate thread, then results are merged in page order.
/// Global timing counters for profiling (nanoseconds, atomic for thread safety).
/// Enabled via PARANG_PROFILE=1 environment variable.
pub static PROF_DOC_OPEN: AtomicU64 = AtomicU64::new(0);
pub static PROF_CONTENT_BYTES: AtomicU64 = AtomicU64::new(0);
pub static PROF_LOAD_RESOURCES: AtomicU64 = AtomicU64::new(0);
pub static PROF_PROCESS_CONTENT: AtomicU64 = AtomicU64::new(0);
pub static PROF_ASSEMBLE_TEXT: AtomicU64 = AtomicU64::new(0);
pub static PROF_PAGE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Print profiling summary and reset counters.
pub fn print_profile_summary() {
    let doc_open = PROF_DOC_OPEN.swap(0, Ordering::Relaxed);
    let content = PROF_CONTENT_BYTES.swap(0, Ordering::Relaxed);
    let resources = PROF_LOAD_RESOURCES.swap(0, Ordering::Relaxed);
    let process = PROF_PROCESS_CONTENT.swap(0, Ordering::Relaxed);
    let assemble = PROF_ASSEMBLE_TEXT.swap(0, Ordering::Relaxed);
    let pages = PROF_PAGE_COUNT.swap(0, Ordering::Relaxed);
    let total = doc_open + content + resources + process + assemble;
    if total == 0 { return; }
    let ms = |ns: u64| ns as f64 / 1_000_000.0;
    let pct = |ns: u64| ns as f64 / total as f64 * 100.0;
    eprintln!("=== PARANG PROFILE ({} pages) ===", pages);
    eprintln!("  doc_open (lopdf parse):  {:>10.1}ms  ({:>5.1}%)", ms(doc_open), pct(doc_open));
    eprintln!("  content_bytes (decomp):  {:>10.1}ms  ({:>5.1}%)", ms(content), pct(content));
    eprintln!("  load_resources (fonts):  {:>10.1}ms  ({:>5.1}%)", ms(resources), pct(resources));
    eprintln!("  process_content (eng):   {:>10.1}ms  ({:>5.1}%)", ms(process), pct(process));
    eprintln!("  assemble_text (strip):   {:>10.1}ms  ({:>5.1}%)", ms(assemble), pct(assemble));
    eprintln!("  TOTAL (sum of phases):   {:>10.1}ms", ms(total));
    // Also print lopdf sub-phase breakdown
    lopdf::print_lopdf_profile();
}

pub fn extract_text_with_config(
    path: &std::path::Path,
    config: &StripperConfig,
) -> Result<String> {
    let profiling = std::env::var("PARANG_PROFILE").is_ok();

    let t0 = std::time::Instant::now();
    let doc = PdfDocument::open(path)?;
    let page_count = doc.page_count();
    if profiling {
        PROF_DOC_OPEN.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }

    if page_count == 0 {
        return Ok(String::new());
    }

    // Shared font cache across pages within the same document
    let font_cache: FontCache = Arc::new(RwLock::new(HashMap::new()));
    let doc_arc = doc.inner_arc();

    // Process all pages in parallel (page data collection + text extraction in one pass).
    // All Document operations are read-only via Arc<Document>, so this is thread-safe.
    let page_texts: Vec<(u32, String)> = (0..page_count)
        .into_par_iter()
        .filter_map(|page_num| {
            let t1 = std::time::Instant::now();
            let page = doc.page(page_num).ok()?;
            let content_bytes = page.content_bytes().ok()?;
            if content_bytes.is_empty() {
                return None;
            }
            // Borrow resources from page without cloning the Dictionary.
            let resources = page.resources().ok().flatten();
            let rotation = page.rotation().unwrap_or(0) as i32;
            let media_box = page.media_box().ok()?;
            if profiling {
                PROF_CONTENT_BYTES.fetch_add(t1.elapsed().as_nanos() as u64, Ordering::Relaxed);
                PROF_PAGE_COUNT.fetch_add(1, Ordering::Relaxed);
            }

            let t2 = std::time::Instant::now();
            let mut engine = stream::engine::StreamEngine::with_font_cache(
                doc_arc.clone(),
                font_cache.clone(),
            );
            engine.set_page_info(rotation, media_box[2], media_box[3]);
            if let Some(ref res) = resources {
                engine.load_resources(res.dictionary());
            }
            if profiling {
                PROF_LOAD_RESOURCES.fetch_add(t2.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }

            let t3 = std::time::Instant::now();
            if engine.process_content(&content_bytes).is_err() {
                return None;
            }
            let mut positions = engine.into_text_positions();
            if profiling {
                PROF_PROCESS_CONTENT.fetch_add(t3.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }

            if positions.is_empty() {
                return None;
            }

            let t4 = std::time::Instant::now();
            let page_text = assemble_text(&mut positions, config);
            if profiling {
                PROF_ASSEMBLE_TEXT.fetch_add(t4.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }

            if page_text.is_empty() {
                return None;
            }
            Some((page_num, page_text))
        })
        .collect();

    // Merge results in page order
    let mut sorted_texts = page_texts;
    sorted_texts.sort_unstable_by_key(|(num, _)| *num);

    let total_len: usize = sorted_texts.iter().map(|(_, t)| t.len()).sum::<usize>()
        + config.page_separator.len() * sorted_texts.len().saturating_sub(1);
    let mut output = String::with_capacity(total_len);
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
    let font_cache: FontCache = Arc::new(RwLock::new(HashMap::new()));

    for page_num in 0..doc.page_count() {
        let page = doc.page(page_num)?;
        let content_bytes = page.content_bytes()?;
        if content_bytes.is_empty() {
            continue;
        }

        let resources = page.resources()?;
        let rotation = page.rotation().unwrap_or(0) as i32;
        let media_box = page.media_box()?;

        let mut engine = stream::engine::StreamEngine::with_font_cache(
            doc.inner_arc(),
            font_cache.clone(),
        );
        engine.set_page_info(rotation, media_box[2], media_box[3]);
        if let Some(ref res) = resources {
            engine.load_resources(res.dictionary());
        }
        if engine.process_content(&content_bytes).is_err() {
            continue;
        }

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
