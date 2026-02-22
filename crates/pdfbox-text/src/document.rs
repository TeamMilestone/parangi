//! PDF document wrapper around lopdf::Document.

use std::path::Path;
use std::sync::Arc;

use lopdf::Document;
use memmap2::Mmap;

use crate::page::PdfPage;
use crate::{PdfError, Result};

/// A loaded PDF document.
///
/// Wraps `lopdf::Document` and provides page-level access for text extraction.
/// The document is loaded via memory-mapping for efficient I/O.
pub struct PdfDocument {
    /// The underlying lopdf document, wrapped in Arc for sharing across threads.
    inner: Arc<Document>,
    /// Page object IDs in document order.
    page_ids: Vec<lopdf::ObjectId>,
    /// Keep the mmap alive for the lifetime of the document.
    _mmap: Option<Mmap>,
}

impl PdfDocument {
    /// Open a PDF file using memory-mapped I/O.
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        // SAFETY: The file is opened read-only and we keep the Mmap alive
        // for the lifetime of PdfDocument.
        let mmap = unsafe { Mmap::map(&file)? };
        let doc = Document::load_mem(&mmap)?;
        Self::from_document(doc, Some(mmap))
    }

    /// Load a PDF from an in-memory byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let doc = Document::load_mem(data)?;
        Self::from_document(doc, None)
    }

    fn from_document(doc: Document, mmap: Option<Mmap>) -> Result<Self> {
        let page_ids = doc.get_pages().into_values().collect::<Vec<_>>();

        // Sort by page number — lopdf's get_pages returns BTreeMap<u32, ObjectId>
        // which is already sorted by page number, but we re-collect as Vec.

        Ok(Self {
            inner: Arc::new(doc),
            page_ids,
            _mmap: mmap,
        })
    }

    /// Number of pages in this document.
    pub fn page_count(&self) -> u32 {
        self.page_ids.len() as u32
    }

    /// Get a page by 0-based index.
    pub fn page(&self, index: u32) -> Result<PdfPage> {
        let &object_id = self
            .page_ids
            .get(index as usize)
            .ok_or(PdfError::InvalidPage(index))?;
        PdfPage::new(Arc::clone(&self.inner), object_id)
    }

    /// Get a reference to the underlying lopdf document.
    pub fn inner(&self) -> &Document {
        &self.inner
    }

    /// Get a shared reference to the inner document Arc.
    pub fn inner_arc(&self) -> Arc<Document> {
        Arc::clone(&self.inner)
    }

    /// Get all page object IDs.
    pub fn page_ids(&self) -> &[lopdf::ObjectId] {
        &self.page_ids
    }
}

impl std::fmt::Debug for PdfDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocument")
            .field("page_count", &self.page_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Object};

    #[test]
    fn test_open_nonexistent_file() {
        let result = PdfDocument::open(Path::new("/nonexistent/file.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = PdfDocument::from_bytes(b"not a pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_minimal_pdf() {
        // Minimal valid PDF (empty, 1 page)
        let pdf_bytes = create_minimal_pdf();
        let doc = PdfDocument::from_bytes(&pdf_bytes).expect("should parse minimal PDF");
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn test_invalid_page_index() {
        let pdf_bytes = create_minimal_pdf();
        let doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();
        let result = doc.page(999);
        assert!(result.is_err());
    }

    /// Create a minimal valid PDF with one empty page.
    fn create_minimal_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let catalog_id = doc.new_object_id();

        let page = lopdf::dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects.insert(page_id, Object::Dictionary(page));

        let pages = lopdf::dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = lopdf::dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        };
        doc.objects.insert(catalog_id, Object::Dictionary(catalog));

        doc.trailer.set("Root", catalog_id);

        let mut buf = Vec::new();
        doc.save_to(&mut buf).expect("save minimal pdf");
        buf
    }
}

