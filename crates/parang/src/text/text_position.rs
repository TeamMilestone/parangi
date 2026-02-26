//! TextPosition — represents a single glyph/character with position info.
//!
//! Ported from org.apache.pdfbox.text.TextPosition.

use compact_str::CompactString;

/// A single character or glyph extracted from a PDF page,
/// along with its position, size, and rendering information.
///
/// Layout: 64 bytes = exactly 1 cache line.
/// CompactString(24) + 10×f32(40) = 64 bytes.
#[derive(Debug, Clone)]
pub struct TextPosition {
    /// Unicode text for this glyph (may be multi-char for ligatures).
    /// Uses CompactString for inline storage of short strings (most glyphs
    /// are 1-4 bytes), avoiding heap allocation.
    pub unicode: CompactString,

    // TRM (Text Rendering Matrix) components — only the 4 values actually used.
    // TRM = [fontSize*Hs 0 0; 0 fontSize 0; 0 rise 1] × Tm × CTM
    /// TRM scale_x (a): used for direction().
    pub trm_a: f32,
    /// TRM shear_y (b): used for direction().
    pub trm_b: f32,
    /// TRM translate_x (tx): glyph X position in device space.
    pub trm_tx: f32,
    /// TRM translate_y (ty): glyph Y position in device space.
    pub trm_ty: f32,

    /// Font height in device space (absolute value).
    pub max_height: f32,
    /// Character width in device space (endX - startX for horizontal text).
    pub individual_width: f32,
    /// Space character width in device space (for word segmentation).
    pub space_width: f32,

    /// Font size from the Tf operator.
    pub font_size: f32,

    /// Page width (from CropBox or MediaBox).
    pub page_width: f32,
    /// Page height.
    pub page_height: f32,
}

impl TextPosition {
    /// Starting X coordinate in device space.
    #[inline]
    pub fn x(&self) -> f32 {
        self.trm_tx
    }

    /// Starting Y coordinate in device space.
    #[inline]
    pub fn y(&self) -> f32 {
        self.trm_ty
    }

    /// Text direction in degrees (0, 90, 180, 270).
    ///
    /// Determined by examining the text rendering matrix components.
    pub fn direction(&self) -> f32 {
        let a = self.trm_a;
        let b = self.trm_b;

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

    /// Compute all direction-adjusted values in ONE direction() call.
    ///
    /// Returns (x_adj, y_adj, width_adj, height_adj).
    /// Used to avoid 4× redundant direction() calls in the assembly loop.
    #[inline]
    pub fn dir_adj_all(&self) -> (f32, f32, f32, f32) {
        let dir = self.direction();
        match dir as i32 {
            0 => (
                self.x(),
                self.page_height - self.y(),
                self.individual_width.abs(),
                self.max_height,
            ),
            90 => (
                self.page_height - self.y(),
                self.x(),
                self.max_height,
                self.individual_width.abs(),
            ),
            180 => (
                self.page_width - self.x(),
                self.y(),
                self.individual_width.abs(),
                self.max_height,
            ),
            270 => (
                self.y(),
                self.page_width - self.x(),
                self.max_height,
                self.individual_width.abs(),
            ),
            _ => (
                self.x(),
                self.page_height - self.y(),
                self.individual_width.abs(),
                self.max_height,
            ),
        }
    }

    /// Compute a sort key (dir, y_adj, x_adj) as (u32, u32, u32) for cached sorting.
    ///
    /// Uses f32 total-order bit representation so that (u32, u32, u32) implements Ord
    /// with the same semantics as total_cmp. Calls direction() exactly ONCE.
    #[inline]
    pub fn sort_key(&self) -> (u32, u32, u32) {
        let dir = self.direction();
        let (y_adj, x_adj) = match dir as i32 {
            0 => (self.page_height - self.y(), self.x()),
            90 => (self.x(), self.page_height - self.y()),
            180 => (self.y(), self.page_width - self.x()),
            270 => (self.page_width - self.x(), self.y()),
            _ => (self.page_height - self.y(), self.x()),
        };
        (f32_total_bits(dir), f32_total_bits(y_adj), f32_total_bits(x_adj))
    }
}

/// Convert f32 to u32 that preserves total order (equivalent to f32::total_cmp).
///
/// For positive f32 values (dir ∈ {0,90,180,270}, coords typically positive),
/// this is equivalent to .to_bits() with sign bit set. For negative values,
/// all bits are flipped so they sort before positive values.
#[inline]
fn f32_total_bits(f: f32) -> u32 {
    let bits = f.to_bits();
    // If sign bit is 0 (positive or +0): set sign bit to put after negatives
    // If sign bit is 1 (negative or -0): flip all bits to reverse order
    if bits >> 31 == 0 {
        bits | 0x8000_0000
    } else {
        !bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tp(unicode: &str, a: f32, b: f32, tx: f32, ty: f32, end_x: f32) -> TextPosition {
        TextPosition {
            unicode: unicode.into(),
            trm_a: a,
            trm_b: b,
            trm_tx: tx,
            trm_ty: ty,
            max_height: 10.0,
            individual_width: end_x - tx,
            space_width: 3.0,
            font_size: 12.0,
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    #[test]
    fn test_position_accessors() {
        let tp = make_tp("A", 12.0, 0.0, 100.0, 700.0, 107.0);

        assert!((tp.x() - 100.0).abs() < 0.001);
        assert!((tp.y() - 700.0).abs() < 0.001);
        assert!((tp.individual_width - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_direction_horizontal() {
        // Normal left-to-right text: a>0, b=0
        let tp = make_tp("A", 12.0, 0.0, 100.0, 700.0, 107.0);
        assert_eq!(tp.direction(), 0.0);
    }

    #[test]
    fn test_direction_rotated() {
        // 90-degree rotation: a=0, b>0
        let tp = make_tp("A", 0.0, 12.0, 100.0, 700.0, 100.0);
        assert_eq!(tp.direction(), 90.0);
    }
}
