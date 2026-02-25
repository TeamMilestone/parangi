//! PDF text state parameters.
//!
//! Ported from org.apache.pdfbox.pdmodel.graphics.state.PDTextState.

/// Text rendering modes (PDF spec Table 106).
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum RenderingMode {
    Fill = 0,
    Stroke = 1,
    FillStroke = 2,
    Neither = 3,
    FillClip = 4,
    StrokeClip = 5,
    FillStrokeClip = 6,
    NeitherClip = 7,
}

impl RenderingMode {
    pub fn from_i64(val: i64) -> Self {
        match val {
            0 => Self::Fill,
            1 => Self::Stroke,
            2 => Self::FillStroke,
            3 => Self::Neither,
            4 => Self::FillClip,
            5 => Self::StrokeClip,
            6 => Self::FillStrokeClip,
            7 => Self::NeitherClip,
            _ => Self::Fill, // default
        }
    }

    /// Returns true if this mode includes filling.
    pub fn is_fill(&self) -> bool {
        matches!(self, Self::Fill | Self::FillStroke | Self::FillClip | Self::FillStrokeClip)
    }

    /// Returns true if this mode includes stroking.
    pub fn is_stroke(&self) -> bool {
        matches!(
            self,
            Self::Stroke | Self::FillStroke | Self::StrokeClip | Self::FillStrokeClip
        )
    }

    /// Returns true if this mode includes clipping.
    pub fn is_clip(&self) -> bool {
        matches!(
            self,
            Self::FillClip | Self::StrokeClip | Self::FillStrokeClip | Self::NeitherClip
        )
    }
}

/// PDF text state parameters tracked during content stream processing.
///
/// Corresponds to PDFBox's PDTextState.
#[derive(Clone, Debug)]
pub struct TextState {
    /// Character spacing (Tc). Default: 0.
    pub character_spacing: f32,

    /// Word spacing (Tw). Default: 0.
    pub word_spacing: f32,

    /// Horizontal scaling (Th) as percentage. Default: 100.
    /// NOTE: This is 0-100 scale, not 0-1.
    pub horizontal_scaling: f32,

    /// Text leading (Tl). Default: 0.
    pub leading: f32,

    /// Font resource name (from Tf operator). None if not set.
    pub font_name: Option<Vec<u8>>,

    /// Font size (from Tf operator). Default: 0.
    pub font_size: f32,

    /// Text rendering mode (Tr). Default: Fill.
    pub rendering_mode: RenderingMode,

    /// Text rise (Ts). Default: 0.
    pub rise: f32,

    /// Knockout flag. Default: true.
    pub knockout: bool,
}

impl TextState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get horizontal scaling as a fraction (0-1 range).
    /// PDFBox stores this as percentage (0-100).
    pub fn horizontal_scaling_fraction(&self) -> f32 {
        self.horizontal_scaling / 100.0
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            font_name: None,
            font_size: 0.0,
            rendering_mode: RenderingMode::Fill,
            rise: 0.0,
            knockout: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_text_state() {
        let ts = TextState::new();
        assert_eq!(ts.character_spacing, 0.0);
        assert_eq!(ts.word_spacing, 0.0);
        assert_eq!(ts.horizontal_scaling, 100.0);
        assert_eq!(ts.leading, 0.0);
        assert!(ts.font_name.is_none());
        assert_eq!(ts.font_size, 0.0);
        assert_eq!(ts.rendering_mode, RenderingMode::Fill);
        assert_eq!(ts.rise, 0.0);
        assert!(ts.knockout);
    }

    #[test]
    fn test_horizontal_scaling_fraction() {
        let mut ts = TextState::new();
        assert!((ts.horizontal_scaling_fraction() - 1.0).abs() < 0.001);

        ts.horizontal_scaling = 50.0;
        assert!((ts.horizontal_scaling_fraction() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rendering_mode() {
        assert!(RenderingMode::Fill.is_fill());
        assert!(!RenderingMode::Fill.is_stroke());
        assert!(!RenderingMode::Fill.is_clip());

        assert!(RenderingMode::FillStrokeClip.is_fill());
        assert!(RenderingMode::FillStrokeClip.is_stroke());
        assert!(RenderingMode::FillStrokeClip.is_clip());

        assert!(!RenderingMode::Neither.is_fill());
        assert!(!RenderingMode::Neither.is_stroke());
        assert!(!RenderingMode::Neither.is_clip());
    }

    #[test]
    fn test_rendering_mode_from_i64() {
        assert_eq!(RenderingMode::from_i64(0), RenderingMode::Fill);
        assert_eq!(RenderingMode::from_i64(2), RenderingMode::FillStroke);
        assert_eq!(RenderingMode::from_i64(7), RenderingMode::NeitherClip);
        assert_eq!(RenderingMode::from_i64(99), RenderingMode::Fill); // invalid → default
    }
}
