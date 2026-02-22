//! TextPosition — represents a single glyph/character with position info.
//!
//! Ported from org.apache.pdfbox.text.TextPosition.

use crate::stream::matrix::Matrix;

/// A single character or glyph extracted from a PDF page,
/// along with its position, size, and rendering information.
#[derive(Debug, Clone)]
pub struct TextPosition {
    /// Unicode text for this glyph (may be multi-char for ligatures).
    pub unicode: String,
    /// PDF character codes (internal, not Unicode).
    pub char_codes: Vec<u32>,

    /// Text rendering matrix at the glyph's starting position.
    /// TRM = [fontSize*Hs 0 0; 0 fontSize 0; 0 rise 1] x Tm x CTM
    pub text_matrix: Matrix,

    /// Ending X position after glyph advance (device space).
    pub end_x: f32,
    /// Ending Y position after glyph advance (device space).
    pub end_y: f32,

    /// Font height in device space (absolute value).
    pub max_height: f32,
    /// Character width in device space (endX - startX for horizontal text).
    pub individual_width: f32,
    /// Space character width in device space (for word segmentation).
    pub space_width: f32,

    /// Font size from the Tf operator.
    pub font_size: f32,
    /// Approximate font size in points (fontSize * textMatrix X scale).
    pub font_size_in_pt: i32,

    /// Page rotation (0, 90, 180, 270).
    pub page_rotation: i32,
    /// Page width (from CropBox or MediaBox).
    pub page_width: f32,
    /// Page height.
    pub page_height: f32,
}

impl TextPosition {
    /// Starting X coordinate in device space.
    #[inline]
    pub fn x(&self) -> f32 {
        self.text_matrix.translate_x()
    }

    /// Starting Y coordinate in device space.
    #[inline]
    pub fn y(&self) -> f32 {
        self.text_matrix.translate_y()
    }

    /// X scaling factor of the text rendering matrix.
    #[inline]
    pub fn x_scale(&self) -> f32 {
        self.text_matrix.scaling_factor_x()
    }

    /// Y scaling factor of the text rendering matrix.
    #[inline]
    pub fn y_scale(&self) -> f32 {
        self.text_matrix.scaling_factor_y()
    }

    /// Text direction in degrees (0, 90, 180, 270).
    ///
    /// Determined by examining the text rendering matrix components.
    pub fn direction(&self) -> f32 {
        let a = self.text_matrix.scale_x();
        let b = self.text_matrix.shear_y();

        if a.abs() > b.abs() {
            if a > 0.0 {
                0.0
            } else {
                180.0
            }
        } else if b > 0.0 {
            90.0
        } else {
            270.0
        }
    }

    /// Direction-adjusted X coordinate.
    /// Normalizes position as if text were horizontal left-to-right.
    pub fn x_dir_adj(&self) -> f32 {
        let dir = self.direction();
        match dir as i32 {
            0 => self.x(),
            90 => self.page_height - self.y(),
            180 => self.page_width - self.x(),
            270 => self.y(),
            _ => self.x(),
        }
    }

    /// Direction-adjusted Y coordinate.
    pub fn y_dir_adj(&self) -> f32 {
        let dir = self.direction();
        match dir as i32 {
            0 => self.page_height - self.y(),
            90 => self.x(),
            180 => self.y(),
            270 => self.page_width - self.x(),
            _ => self.y(),
        }
    }

    /// Width of the character, direction-adjusted.
    pub fn width_dir_adj(&self) -> f32 {
        let dir = self.direction();
        if dir == 0.0 || dir == 180.0 {
            self.individual_width.abs()
        } else {
            self.max_height
        }
    }

    /// Height of the character, direction-adjusted.
    pub fn height_dir_adj(&self) -> f32 {
        let dir = self.direction();
        if dir == 0.0 || dir == 180.0 {
            self.max_height
        } else {
            self.individual_width.abs()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tp(unicode: &str, trm: Matrix, end_x: f32, end_y: f32) -> TextPosition {
        TextPosition {
            unicode: unicode.to_string(),
            char_codes: vec![0],
            text_matrix: trm,
            end_x,
            end_y,
            max_height: 10.0,
            individual_width: end_x - trm.translate_x(),
            space_width: 3.0,
            font_size: 12.0,
            font_size_in_pt: 12,
            page_rotation: 0,
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    #[test]
    fn test_position_accessors() {
        let trm = Matrix::from_values(12.0, 0.0, 0.0, 12.0, 100.0, 700.0);
        let tp = make_tp("A", trm, 107.0, 700.0);

        assert!((tp.x() - 100.0).abs() < 0.001);
        assert!((tp.y() - 700.0).abs() < 0.001);
        assert!((tp.individual_width - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_direction_horizontal() {
        // Normal left-to-right text: a>0, b=0
        let trm = Matrix::from_values(12.0, 0.0, 0.0, 12.0, 100.0, 700.0);
        let tp = make_tp("A", trm, 107.0, 700.0);
        assert_eq!(tp.direction(), 0.0);
    }

    #[test]
    fn test_direction_rotated() {
        // 90-degree rotation: a=0, b>0
        let trm = Matrix::from_values(0.0, 12.0, -12.0, 0.0, 100.0, 700.0);
        let tp = make_tp("A", trm, 100.0, 712.0);
        assert_eq!(tp.direction(), 90.0);
    }

    #[test]
    fn test_scaling_factors() {
        let trm = Matrix::from_values(12.0, 0.0, 0.0, 12.0, 100.0, 700.0);
        let tp = make_tp("A", trm, 107.0, 700.0);

        assert!((tp.x_scale() - 12.0).abs() < 0.001);
        assert!((tp.y_scale() - 12.0).abs() < 0.001);
    }
}
