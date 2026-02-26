//! TextPosition comparator for sorting text by reading order.
//!
//! Ported from org.apache.pdfbox.text.TextPositionComparator.
//! Sorts by: (1) text direction, (2) Y position, (3) X position.
//!
//! Note: Same-line grouping (Y overlap tolerance) is handled downstream
//! by the text assembler (stripper.rs), not here. The comparator must
//! provide a strict total order to satisfy Rust's sort requirements.

use super::TextPosition;

/// Sort TextPositions into reading order.
///
/// Uses direction-adjusted coordinates (origin at upper-left).
/// Strict total order: direction → Y (top to bottom) → X (left to right).
pub fn sort_positions(positions: &mut [TextPosition]) {
    positions.sort_unstable_by(|a, b| compare_positions(a, b));
}

/// Compare two TextPositions for reading order.
///
/// Strict total order using f32::total_cmp (handles NaN).
fn compare_positions(a: &TextPosition, b: &TextPosition) -> std::cmp::Ordering {
    a.direction()
        .total_cmp(&b.direction())
        .then_with(|| a.y_dir_adj().total_cmp(&b.y_dir_adj()))
        .then_with(|| a.x_dir_adj().total_cmp(&b.x_dir_adj()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tp(unicode: &str, x: f32, y: f32, width: f32, height: f32) -> TextPosition {
        TextPosition {
            unicode: unicode.into(),
            trm_a: height,
            trm_b: 0.0,
            trm_tx: x,
            trm_ty: y,
            max_height: height,
            individual_width: width,
            space_width: 3.0,
            font_size: 12.0,
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    #[test]
    fn test_sort_same_line() {
        let mut positions = vec![
            make_tp("B", 50.0, 700.0, 7.0, 12.0),
            make_tp("A", 10.0, 700.0, 7.0, 12.0),
            make_tp("C", 90.0, 700.0, 7.0, 12.0),
        ];
        sort_positions(&mut positions);
        assert_eq!(positions[0].unicode, "A");
        assert_eq!(positions[1].unicode, "B");
        assert_eq!(positions[2].unicode, "C");
    }

    #[test]
    fn test_sort_different_lines() {
        let mut positions = vec![
            make_tp("Line2", 10.0, 680.0, 30.0, 12.0),
            make_tp("Line1", 10.0, 700.0, 30.0, 12.0),
        ];
        sort_positions(&mut positions);
        // Line1 (y=700) has y_dir_adj = 792-700 = 92
        // Line2 (y=680) has y_dir_adj = 792-680 = 112
        // Lower y_dir_adj comes first → Line1 first
        assert_eq!(positions[0].unicode, "Line1");
        assert_eq!(positions[1].unicode, "Line2");
    }

    #[test]
    fn test_sort_overlapping_y_same_line() {
        // With strict Y sort, slight Y difference means different Y order.
        // But the assembler handles same-line grouping via overlap check.
        let mut positions = vec![
            make_tp("B", 50.0, 699.0, 7.0, 12.0),
            make_tp("A", 10.0, 700.0, 7.0, 12.0),
        ];
        sort_positions(&mut positions);
        // A has y_dir_adj = 792-700 = 92, B has y_dir_adj = 792-699 = 93
        // A (92) < B (93) → A comes first
        assert_eq!(positions[0].unicode, "A");
        assert_eq!(positions[1].unicode, "B");
    }
}
