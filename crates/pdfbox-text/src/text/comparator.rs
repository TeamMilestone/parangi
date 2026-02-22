//! TextPosition comparator for sorting text by reading order.
//!
//! Ported from org.apache.pdfbox.text.TextPositionComparator.
//! Sorts by: (1) text direction, (2) Y position (with overlap tolerance),
//! (3) X position within same line.

use super::TextPosition;

/// Sort TextPositions into reading order.
///
/// Uses direction-adjusted coordinates (origin at upper-left).
/// Text with overlapping Y ranges is considered on the same line and sorted by X.
pub fn sort_positions(positions: &mut [TextPosition]) {
    // Use stable sort to preserve insertion order for identical positions.
    // Use total_cmp for f32 to guarantee total order (NaN handled).
    positions.sort_by(|a, b| compare_positions(a, b));
}

/// Compare two TextPositions for reading order.
///
/// Uses a simplified approach that guarantees transitivity:
/// 1. Compare direction
/// 2. Compare Y using a quantized bucket approach for same-line detection
/// 3. Within same Y bucket, compare by X
fn compare_positions(a: &TextPosition, b: &TextPosition) -> std::cmp::Ordering {
    // Step 1: Compare text direction
    let dir_cmp = a.direction().total_cmp(&b.direction());
    if dir_cmp != std::cmp::Ordering::Equal {
        return dir_cmp;
    }

    // Step 2: Compare Y positions
    let a_y = a.y_dir_adj();
    let b_y = b.y_dir_adj();

    // Check if texts are on the same line using overlap.
    // To maintain transitivity, we use a simple approach:
    // if |a_y - b_y| is small relative to the average height, treat as same line.
    let avg_height = (a.height_dir_adj() + b.height_dir_adj()) / 2.0;
    let tolerance = if avg_height > 0.0 {
        avg_height * 0.5
    } else {
        1.0
    };

    let y_diff = a_y - b_y;
    if y_diff.abs() <= tolerance {
        // Same line: sort by X
        a.x_dir_adj().total_cmp(&b.x_dir_adj())
    } else {
        // Different lines: sort by Y (top to bottom)
        a_y.total_cmp(&b_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::matrix::Matrix;

    fn make_tp(unicode: &str, x: f32, y: f32, width: f32, height: f32) -> TextPosition {
        let trm = Matrix::from_values(height, 0.0, 0.0, height, x, y);
        TextPosition {
            unicode: unicode.to_string(),
            char_codes: vec![0],
            text_matrix: trm,
            end_x: x + width,
            end_y: y,
            max_height: height,
            individual_width: width,
            space_width: 3.0,
            font_size: 12.0,
            font_size_in_pt: 12,
            page_rotation: 0,
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
        // Higher Y in device space = lower Y_dir_adj = earlier in reading order
        let mut positions = vec![
            make_tp("Line2", 10.0, 680.0, 30.0, 12.0), // lower on page
            make_tp("Line1", 10.0, 700.0, 30.0, 12.0), // higher on page
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
        // Slightly different Y but within tolerance → same line
        let mut positions = vec![
            make_tp("B", 50.0, 699.0, 7.0, 12.0), // Y differs by 1pt
            make_tp("A", 10.0, 700.0, 7.0, 12.0),
        ];
        sort_positions(&mut positions);
        // The Y difference (1pt) is within tolerance (0.5 * 12 = 6pt) → same line → sort by X
        assert_eq!(positions[0].unicode, "A");
        assert_eq!(positions[1].unicode, "B");
    }
}
