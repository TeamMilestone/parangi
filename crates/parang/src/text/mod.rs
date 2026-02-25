//! Text extraction and assembly.
//!
//! - `text_position`: Individual glyph/character positions (ported from TextPosition)
//! - `comparator`: Sorting TextPositions into reading order
//! - `stripper`: Assembling TextPositions into readable text (PDFTextStripper port)

pub mod comparator;
pub mod normalizer;
pub mod stripper;
pub mod text_position;

pub use normalizer::normalize_text;
pub use stripper::{assemble_text, StripperConfig};
pub use text_position::TextPosition;
