//! Text stripper — assembles TextPositions into readable text.
//!
//! Ported from org.apache.pdfbox.text.PDFTextStripper.
//! Handles word boundary detection, line detection, and paragraph separation.

use std::collections::HashMap;

use super::comparator::sort_positions;
use super::TextPosition;

/// Configuration for text stripping.
#[derive(Debug, Clone)]
pub struct StripperConfig {
    /// Fraction of space width to use as word boundary threshold.
    /// Default: 0.5 (PDFBox default).
    pub spacing_tolerance: f32,
    /// Fraction of average character width to use as word boundary threshold.
    /// Default: 0.3 (PDFBox default).
    pub average_char_tolerance: f32,
    /// Multiplier of line height for paragraph drop detection.
    /// Default: 2.5 (PDFBox default).
    pub drop_threshold: f32,
    /// Multiplier of space width for paragraph indent detection.
    /// Default: 2.0 (PDFBox default).
    pub indent_threshold: f32,
    /// Whether to sort positions by reading order.
    /// Default: true.
    pub sort_by_position: bool,
    /// Word separator string.
    /// Default: " ".
    pub word_separator: String,
    /// Line separator string.
    /// Default: "\n".
    pub line_separator: String,
    /// Paragraph separator string.
    /// Default: "\n" (same as line; set to "\n\n" for double-spaced paragraphs).
    pub paragraph_separator: String,
    /// Page separator string.
    /// Default: "\n".
    pub page_separator: String,
    /// Whether to suppress duplicate overlapping text.
    /// Default: true.
    pub suppress_duplicates: bool,
}

impl Default for StripperConfig {
    fn default() -> Self {
        Self {
            spacing_tolerance: 0.5,
            average_char_tolerance: 0.3,
            drop_threshold: 2.5,
            indent_threshold: 2.0,
            sort_by_position: true,
            word_separator: " ".to_string(),
            line_separator: "\n".to_string(),
            paragraph_separator: "\n".to_string(),
            page_separator: "\n".to_string(),
            suppress_duplicates: true,
        }
    }
}

/// Assemble TextPositions into readable text.
///
/// This is the core text assembly function, ported from PDFTextStripper.writePage().
#[allow(unused_assignments)] // line-break resets are read in subsequent loop iterations
pub fn assemble_text(positions: &mut Vec<TextPosition>, config: &StripperConfig) -> String {
    if positions.is_empty() {
        return String::new();
    }

    // Sort positions into reading order
    if config.sort_by_position {
        sort_positions(positions);
    }

    // Remove duplicate overlapping text
    if config.suppress_duplicates {
        suppress_duplicate_positions(positions);
    }

    // Remove spaces that are contained within other characters
    remove_contained_spaces(positions);

    let mut output = String::new();
    let mut line = Vec::<&TextPosition>::new();

    // Line tracking state
    let mut max_y_for_line: f32 = f32::MIN;
    let mut max_height_for_line: f32 = -1.0;
    let mut end_of_last_text_x: f32 = f32::MIN;
    let mut last_word_spacing: f32 = -1.0;
    let mut previous_ave_char_width: f32 = -1.0;
    let mut last_position: Option<&TextPosition> = None;
    let mut last_line_start_y: Option<f32> = None;
    let mut last_line_start_x: Option<f32> = None;

    // Word separator tracking
    let mut pending_word_separator = false;

    for pos in positions.iter() {
        let pos_x = pos.x_dir_adj();
        let pos_y = pos.y_dir_adj();
        let pos_width = pos.width_dir_adj();
        let pos_height = pos.height_dir_adj();
        let word_spacing = pos.space_width;

        if let Some(last) = last_position {
            // --- Line break detection ---
            if !overlap(pos_y, pos_height, max_y_for_line, max_height_for_line) {
                // Write the current line
                flush_line(&line, &mut output, pending_word_separator, config);
                pending_word_separator = false;
                line.clear();

                // Check for paragraph break
                if let Some(_last_start_y) = last_line_start_y {
                    let y_gap = (pos_y - last.y_dir_adj()).abs();
                    if max_height_for_line > 0.0
                        && y_gap > config.drop_threshold * max_height_for_line
                    {
                        // Paragraph break (large vertical gap)
                        output.push_str(&config.paragraph_separator);
                    } else {
                        // Regular line break
                        output.push_str(&config.line_separator);
                    }

                    // Check for indent-based paragraph
                    if let Some(last_start_x) = last_line_start_x {
                        let x_indent = pos_x - last_start_x;
                        if x_indent > config.indent_threshold * word_spacing.max(1.0) {
                            // Significant indent — could be paragraph start
                            // (handled by the paragraph separator above)
                        }
                    }

                }

                // Reset line tracking
                max_y_for_line = f32::MIN;
                max_height_for_line = -1.0;
                end_of_last_text_x = f32::MIN;
                last_word_spacing = -1.0;
                previous_ave_char_width = -1.0;
                last_line_start_y = Some(pos_y);
                last_line_start_x = Some(pos_x);
            } else {
                // --- Word boundary detection (same line) ---
                let delta_space = if word_spacing <= 0.0 || word_spacing.is_nan() {
                    f32::MAX
                } else if last_word_spacing < 0.0 {
                    word_spacing * config.spacing_tolerance
                } else {
                    (word_spacing + last_word_spacing) / 2.0 * config.spacing_tolerance
                };

                let char_count = pos.unicode.len().max(1) as f32;
                let ave_char_width = if previous_ave_char_width < 0.0 {
                    pos_width / char_count
                } else {
                    (previous_ave_char_width + pos_width / char_count) / 2.0
                };
                let delta_char = ave_char_width * config.average_char_tolerance;

                let expected_x = end_of_last_text_x + delta_space.min(delta_char);

                if end_of_last_text_x > f32::MIN && expected_x < pos_x {
                    // Gap exceeds threshold — insert word separator
                    if !last
                        .unicode
                        .ends_with(&config.word_separator)
                    {
                        pending_word_separator = true;
                    }
                }

                // Check for large X gap that indicates line dimension reset
                if (pos_x - last.x_dir_adj()).abs() > (word_spacing + delta_space) {
                    max_y_for_line = f32::MIN;
                    max_height_for_line = -1.0;
                }

                previous_ave_char_width = ave_char_width;
            }
        } else {
            // First position
            last_line_start_y = Some(pos_y);
            last_line_start_x = Some(pos_x);
        }

        // Update line tracking
        if pos_y > max_y_for_line {
            max_y_for_line = pos_y;
        }
        if pos_height > max_height_for_line {
            max_height_for_line = pos_height;
        }

        end_of_last_text_x = pos_x + pos_width;
        last_word_spacing = word_spacing;
        last_position = Some(pos);
        line.push(pos);
    }

    // Flush remaining line
    if !line.is_empty() {
        flush_line(&line, &mut output, pending_word_separator, config);
    }

    output
}

/// Write a line of TextPositions to the output, inserting word separators
/// where word boundaries were detected during assembly.
fn flush_line(
    line: &[&TextPosition],
    output: &mut String,
    _pending_word_sep: bool,
    config: &StripperConfig,
) {
    if line.is_empty() {
        return;
    }

    // Re-process line to detect word boundaries (we need to insert separators)
    let mut prev_end_x: f32 = f32::MIN;
    let mut last_word_spacing: f32 = -1.0;
    let mut previous_ave_char_width: f32 = -1.0;
    let mut first = true;

    for pos in line {
        let pos_x = pos.x_dir_adj();
        let pos_width = pos.width_dir_adj();
        let word_spacing = pos.space_width;

        if !first {
            // Word boundary detection
            let delta_space = if word_spacing <= 0.0 || word_spacing.is_nan() {
                f32::MAX
            } else if last_word_spacing < 0.0 {
                word_spacing * 0.5
            } else {
                (word_spacing + last_word_spacing) / 2.0 * 0.5
            };

            let char_count = pos.unicode.len().max(1) as f32;
            let ave_char_width = if previous_ave_char_width < 0.0 {
                pos_width / char_count
            } else {
                (previous_ave_char_width + pos_width / char_count) / 2.0
            };
            let delta_char = ave_char_width * 0.3;

            let expected_x = prev_end_x + delta_space.min(delta_char);

            if prev_end_x > f32::MIN && expected_x < pos_x {
                output.push_str(&config.word_separator);
            }

            previous_ave_char_width = ave_char_width;
        }

        output.push_str(&pos.unicode);
        prev_end_x = pos_x + pos_width;
        last_word_spacing = word_spacing;
        first = false;
    }
}

/// Suppress duplicate overlapping text positions.
///
/// Ported from PDFTextStripper.processTextPosition() duplicate suppression.
/// When the same character appears at the same position (within a tolerance
/// of 1/3 character width), only the first occurrence is kept.
/// This handles PDFs that overlay text for bold/shadow effects.
fn suppress_duplicate_positions(positions: &mut Vec<TextPosition>) {
    if positions.len() < 2 {
        return;
    }

    // Map: character → list of (x, y) positions seen
    let mut seen: HashMap<String, Vec<(f32, f32)>> = HashMap::new();
    let mut keep = vec![true; positions.len()];

    for (i, pos) in positions.iter().enumerate() {
        if pos.unicode.is_empty() {
            continue;
        }

        let tolerance = if pos.unicode.len() > 0 {
            pos.individual_width.abs() / pos.unicode.len().max(1) as f32 / 3.0
        } else {
            0.0
        };

        if tolerance <= 0.0 {
            continue;
        }

        let x = pos.x();
        let y = pos.y();

        let positions_for_char = seen.entry(pos.unicode.clone()).or_default();

        // Check if any existing position is within tolerance
        let is_duplicate = positions_for_char.iter().any(|&(px, py)| {
            (x - px).abs() < tolerance && (y - py).abs() < tolerance
        });

        if is_duplicate {
            keep[i] = false;
        } else {
            positions_for_char.push((x, y));
        }
    }

    // Remove duplicates (iterate in reverse to preserve indices)
    let mut i = positions.len();
    while i > 0 {
        i -= 1;
        if !keep[i] {
            positions.remove(i);
        }
    }
}

/// Remove space characters whose X range is contained within another character.
///
/// Ported from PDFTextStripper.removeContainedSpaces().
/// This handles cases where PDF producers insert space characters that overlap
/// with adjacent characters, creating unwanted gaps.
fn remove_contained_spaces(positions: &mut Vec<TextPosition>) {
    if positions.len() < 2 {
        return;
    }

    let mut remove_indices = Vec::new();

    for i in 0..positions.len() {
        if positions[i].unicode != " " {
            continue;
        }

        let space_x = positions[i].x_dir_adj();
        let space_end_x = space_x + positions[i].width_dir_adj();
        let space_y = positions[i].y_dir_adj();
        let space_height = positions[i].height_dir_adj();

        // Check if this space is contained within a neighboring character
        for j in (i.saturating_sub(5))..((i + 6).min(positions.len())) {
            if i == j || positions[j].unicode == " " {
                continue;
            }

            // Must be on the same line
            if !overlap(
                space_y,
                space_height,
                positions[j].y_dir_adj(),
                positions[j].height_dir_adj(),
            ) {
                continue;
            }

            let char_x = positions[j].x_dir_adj();
            let char_end_x = char_x + positions[j].width_dir_adj();

            // Space is contained if its X range falls entirely within the character's X range
            if space_x >= char_x && space_end_x <= char_end_x {
                remove_indices.push(i);
                break;
            }
        }
    }

    // Remove in reverse order
    for &i in remove_indices.iter().rev() {
        positions.remove(i);
    }
}

/// Check if two vertical ranges overlap (same line detection).
///
/// Returns true if the Y/height ranges overlap or are within 0.1pt tolerance.
fn overlap(y1: f32, height1: f32, y2: f32, height2: f32) -> bool {
    // Within 0.1pt tolerance
    if (y1 - y2).abs() < 0.1 {
        return true;
    }
    // y2's bottom overlaps y1's range
    if y2 <= y1 && y2 >= y1 - height1 {
        return true;
    }
    // y1's bottom overlaps y2's range
    if y1 <= y2 && y1 >= y2 - height2 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::matrix::Matrix;

    fn make_tp(
        unicode: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        space_width: f32,
    ) -> TextPosition {
        // Note: y in device space. y_dir_adj = page_height - y for direction 0
        let trm = Matrix::from_values(height, 0.0, 0.0, height, x, y);
        TextPosition {
            unicode: unicode.to_string(),
            char_codes: vec![0],
            text_matrix: trm,
            end_x: x + width,
            end_y: y,
            max_height: height,
            individual_width: width,
            space_width,
            font_size: 12.0,
            font_size_in_pt: 12,
            page_rotation: 0,
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    #[test]
    fn test_overlap_function() {
        assert!(overlap(10.0, 12.0, 10.0, 12.0)); // Same position
        assert!(overlap(10.0, 12.0, 10.05, 12.0)); // Within tolerance
        assert!(overlap(10.0, 12.0, 5.0, 12.0)); // Ranges overlap
        assert!(!overlap(10.0, 12.0, 30.0, 12.0)); // No overlap
    }

    #[test]
    fn test_simple_word_assembly() {
        let config = StripperConfig::default();
        // Characters "H" "e" "l" "l" "o" tightly spaced
        let mut positions = vec![
            make_tp("H", 100.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("e", 107.0, 700.0, 6.0, 12.0, 4.0),
            make_tp("l", 113.0, 700.0, 3.0, 12.0, 4.0),
            make_tp("l", 116.0, 700.0, 3.0, 12.0, 4.0),
            make_tp("o", 119.0, 700.0, 6.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_word_separation() {
        let config = StripperConfig::default();
        // "Hello" followed by a gap, then "World"
        let mut positions = vec![
            make_tp("H", 100.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("e", 107.0, 700.0, 6.0, 12.0, 4.0),
            make_tp("l", 113.0, 700.0, 3.0, 12.0, 4.0),
            make_tp("l", 116.0, 700.0, 3.0, 12.0, 4.0),
            make_tp("o", 119.0, 700.0, 6.0, 12.0, 4.0),
            // Gap: "o" ends at 125, "W" starts at 135 (gap = 10, >> space_width * 0.5 = 2.0)
            make_tp("W", 135.0, 700.0, 8.0, 12.0, 4.0),
            make_tp("o", 143.0, 700.0, 6.0, 12.0, 4.0),
            make_tp("r", 149.0, 700.0, 4.0, 12.0, 4.0),
            make_tp("l", 153.0, 700.0, 3.0, 12.0, 4.0),
            make_tp("d", 156.0, 700.0, 6.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_line_separation() {
        let config = StripperConfig::default();
        // Two lines of text
        let mut positions = vec![
            make_tp("Line1", 100.0, 700.0, 30.0, 12.0, 4.0),
            // Y drops by 14pt (> height) → different line
            make_tp("Line2", 100.0, 686.0, 30.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "Line1\nLine2");
    }

    #[test]
    fn test_paragraph_detection() {
        let config = StripperConfig {
            drop_threshold: 2.5,
            paragraph_separator: "\n\n".to_string(),
            ..Default::default()
        };
        // Two lines, then a large gap (paragraph)
        let mut positions = vec![
            make_tp("Para1", 100.0, 700.0, 30.0, 12.0, 4.0),
            // Large gap: 700 - 650 = 50 > 2.5 * 12 = 30 → paragraph
            make_tp("Para2", 100.0, 650.0, 30.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "Para1\n\nPara2");
    }

    #[test]
    fn test_empty_positions() {
        let config = StripperConfig::default();
        let mut positions = vec![];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "");
    }

    #[test]
    fn test_single_character() {
        let config = StripperConfig::default();
        let mut positions = vec![make_tp("A", 100.0, 700.0, 7.0, 12.0, 4.0)];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "A");
    }

    #[test]
    fn test_korean_text_no_spaces() {
        let config = StripperConfig::default();
        // Korean characters are typically tightly packed (no word spaces)
        let mut positions = vec![
            make_tp("안", 100.0, 700.0, 12.0, 12.0, 6.0),
            make_tp("녕", 112.0, 700.0, 12.0, 12.0, 6.0),
            make_tp("하", 124.0, 700.0, 12.0, 12.0, 6.0),
            make_tp("세", 136.0, 700.0, 12.0, 12.0, 6.0),
            make_tp("요", 148.0, 700.0, 12.0, 12.0, 6.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "안녕하세요");
    }

    #[test]
    fn test_duplicate_suppression() {
        let config = StripperConfig {
            suppress_duplicates: true,
            ..Default::default()
        };
        // Same character "A" at the same position (bold shadow effect)
        let mut positions = vec![
            make_tp("A", 100.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("A", 100.1, 700.0, 7.0, 12.0, 4.0), // Duplicate (0.1pt offset)
            make_tp("B", 107.0, 700.0, 7.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "AB");
    }

    #[test]
    fn test_duplicate_suppression_different_chars() {
        let config = StripperConfig {
            suppress_duplicates: true,
            ..Default::default()
        };
        // Different characters at the same position should NOT be suppressed
        let mut positions = vec![
            make_tp("A", 100.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("B", 100.0, 700.0, 7.0, 12.0, 4.0), // Different char
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "AB");
    }

    #[test]
    fn test_contained_space_removal() {
        let config = StripperConfig::default();
        // A wide character with a space contained entirely within it
        let mut positions = vec![
            make_tp("W", 100.0, 700.0, 14.0, 12.0, 4.0), // Wide char: 100-114
            make_tp(" ", 104.0, 700.0, 3.0, 12.0, 4.0),   // Space: 104-107 (inside W)
            make_tp("x", 114.0, 700.0, 6.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "Wx");
    }

    #[test]
    fn test_unsorted_positions() {
        let config = StripperConfig {
            sort_by_position: true,
            ..Default::default()
        };
        // Out of order — should be sorted to "ABC" (tightly packed, no gaps)
        let mut positions = vec![
            make_tp("C", 114.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("A", 100.0, 700.0, 7.0, 12.0, 4.0),
            make_tp("B", 107.0, 700.0, 7.0, 12.0, 4.0),
        ];
        let text = assemble_text(&mut positions, &config);
        assert_eq!(text, "ABC");
    }
}
