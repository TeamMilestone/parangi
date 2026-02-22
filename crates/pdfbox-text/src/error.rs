use thiserror::Error;

/// Result type for pdfbox-text operations.
pub type Result<T> = std::result::Result<T, PdfError>;

/// Errors that can occur during PDF text extraction.
#[derive(Error, Debug)]
pub enum PdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF parsing error: {0}")]
    Parse(String),

    #[error("lopdf error: {0}")]
    Lopdf(#[from] lopdf::Error),

    #[error("invalid page number: {0}")]
    InvalidPage(u32),

    #[error("missing required entry: {0}")]
    MissingEntry(String),

    #[error("unsupported feature: {0}")]
    Unsupported(String),

    #[error("font error: {0}")]
    Font(String),

    #[error("encoding error: {0}")]
    Encoding(String),
}
