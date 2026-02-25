//! PDF content stream operator definitions.
//!
//! Operator name constants matching PDF spec and PDFBox OperatorName.java.

// Text delimiters
pub const BT: &str = "BT"; // Begin text object
pub const ET: &str = "ET"; // End text object

// Text positioning
pub const TD_LOWER: &str = "Td"; // Move text position
pub const TD_UPPER: &str = "TD"; // Move text position, set leading
pub const TM: &str = "Tm"; // Set text matrix
pub const T_STAR: &str = "T*"; // Move to next line

// Text state
pub const TC: &str = "Tc"; // Set character spacing
pub const TW: &str = "Tw"; // Set word spacing
pub const TZ: &str = "Tz"; // Set horizontal scaling
pub const TL: &str = "TL"; // Set text leading
pub const TF: &str = "Tf"; // Set font and size
pub const TR: &str = "Tr"; // Set text rendering mode
pub const TS: &str = "Ts"; // Set text rise

// Text showing
pub const TJ_LOWER: &str = "Tj"; // Show text
pub const TJ_UPPER: &str = "TJ"; // Show text with adjustments
pub const QUOTE: &str = "'"; // Move to next line and show text
pub const DOUBLE_QUOTE: &str = "\""; // Set spacing, move to next line, show text

// Graphics state
pub const Q_LOWER: &str = "q"; // Save graphics state
pub const Q_UPPER: &str = "Q"; // Restore graphics state
pub const CM: &str = "cm"; // Concatenate matrix

// XObject
pub const DO: &str = "Do"; // Paint XObject

// Graphics state parameters
pub const GS: &str = "gs"; // Set graphics state from ExtGState

// Marked content
pub const BMC: &str = "BMC"; // Begin marked content
pub const BDC: &str = "BDC"; // Begin marked content with properties
pub const EMC: &str = "EMC"; // End marked content
pub const MP: &str = "MP"; // Marked content point
pub const DP: &str = "DP"; // Marked content point with properties
