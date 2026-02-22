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
    // Use stable sort to preserve insertion order for identical positions
    positions.sort_by(|a, b| compare_positions(a, b));
}

/// Compare two TextPositions for reading order.
fn compare_positions(a: &TextPosition, b: &TextPosition) -> std::cmp::Ordering {
    // Step 1: Compare text direction
    let dir_cmp = a.direction().partial_cmp(&b.direction()).unwrap_or(std::cmp::Ordering::Equal);
    if dir_cmp != std::cmp::Ordering::Equal {
        return dir_cmp;
    }

    // Step 2: Compare Y positions (using direction-adjusted coordinates)
    let a_y_bottom = a.y_dir_adj();
    let b_y_bottom = b.y_dir_adj();
    let a_y_top = a_y_bottom - a.height_dir_adj();
    let b_y_top = b_y_bottom - b.height_dir_adj();

    let y_diff = (a_y_bottom - b_y_bottom).abs();

    // Check if texts are on the same line:
    // - Y values within 0.1 tolerance, OR
    // - Vertical ranges overlap
    let same_line = y_diff < 0.1
        || (b_y_bottom >= a_y_top && b_y_bottom <= a_y_bottom)
        || (a_y_bottom >= b_y_top && a_y_bottom <= b_y_bottom);

    if same_line {
        // Step 3: Same line — sort by X (left to right)
        let a_x = a.x_dir_adj();
        let b_x = b.x_dir_adj();
        a_x.partial_cmp(&b_x).unwrap_or(std::cmp::Ordering::Equal)
    } else if a_y_bottom < b_y_bottom {
        std::cmp::Ordering::Less // a is higher (earlier)
    } else {
        std::cmp::Ordering::Greater // b is higher
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
        // Slightly different Y but overlapping vertical ranges → same line
        let mut positions = vec![
            make_tp("B", 50.0, 699.0, 7.0, 12.0), // Y differs by 1pt
            make_tp("A", 10.0, 700.0, 7.0, 12.0),
        ];
        sort_positions(&mut positions);
        // The Y ranges overlap (height=12), so treated as same line → sort by X
        assert_eq!(positions[0].unicode, "A");
        assert_eq!(positions[1].unicode, "B");
    }
}
