//! Content stream processing engine.
//!
//! Ported from PDFStreamEngine + LegacyPDFStreamEngine.
//! Processes PDF content stream operators to update graphics/text state
//! and extract text positioning information.

use std::collections::HashMap;
use std::sync::Arc;

use lopdf::{content::Content, Document, Object};

use super::graphics_state::GraphicsStateStack;
use super::matrix::Matrix;
use super::operators::*;
use super::text_state::RenderingMode;
use crate::cos_helpers::obj_to_f32;
use crate::font::PdfFont;
use crate::text::TextPosition;
use crate::{PdfError, Result};

/// Content stream processing engine.
///
/// Processes a single page's content stream, maintaining graphics state
/// and collecting TextPosition objects for each glyph.
pub struct StreamEngine {
    #[allow(dead_code)] // Used in iter 11 for Form XObject processing
    doc: Arc<Document>,
    state_stack: GraphicsStateStack,

    /// Loaded fonts keyed by resource name (e.g. b"F1").
    fonts: HashMap<Vec<u8>, PdfFont>,
    /// Collected text positions from this content stream.
    text_positions: Vec<TextPosition>,

    /// Page rotation (0, 90, 180, 270).
    page_rotation: i32,
    /// Page width (from CropBox or MediaBox).
    page_width: f32,
    /// Page height.
    page_height: f32,
}

impl StreamEngine {
    /// Create a new stream engine.
    pub fn new(doc: Arc<Document>) -> Self {
        Self {
            doc,
            state_stack: GraphicsStateStack::new(),
            fonts: HashMap::new(),
            text_positions: Vec::new(),
            page_rotation: 0,
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    /// Set page geometry for TextPosition creation.
    pub fn set_page_info(&mut self, rotation: i32, width: f32, height: f32) {
        self.page_rotation = rotation;
        self.page_width = width;
        self.page_height = height;
    }

    /// Set pre-loaded fonts for character decoding.
    pub fn set_fonts(&mut self, fonts: HashMap<Vec<u8>, PdfFont>) {
        self.fonts = fonts;
    }

    /// Process a content stream (raw bytes).
    pub fn process_content(&mut self, content_bytes: &[u8]) -> Result<()> {
        if content_bytes.is_empty() {
            return Ok(());
        }

        let content = Content::decode(content_bytes)
            .map_err(|e| PdfError::Parse(format!("content decode: {}", e)))?;

        for op in &content.operations {
            if let Err(e) = self.process_operator(&op.operator, &op.operands) {
                log::warn!("operator '{}' error: {}", op.operator, e);
            }
        }

        Ok(())
    }

    /// Get the collected text positions.
    pub fn text_positions(&self) -> &[TextPosition] {
        &self.text_positions
    }

    /// Consume and return the collected text positions.
    pub fn into_text_positions(self) -> Vec<TextPosition> {
        self.text_positions
    }

    /// Process a single operator with its operands.
    fn process_operator(&mut self, operator: &str, operands: &[Object]) -> Result<()> {
        match operator {
            // Graphics state
            Q_LOWER => self.op_save(),
            Q_UPPER => self.op_restore(),
            CM => self.op_cm(operands),

            // Text delimiters
            BT => self.op_bt(),
            ET => self.op_et(),

            // Text state
            TC => self.op_tc(operands),
            TW => self.op_tw(operands),
            TZ => self.op_tz(operands),
            TL => self.op_tl(operands),
            TF => self.op_tf(operands),
            TR => self.op_tr(operands),
            TS => self.op_ts(operands),

            // Text positioning
            TD_LOWER => self.op_td(operands),
            TD_UPPER => self.op_td_upper(operands),
            TM => self.op_tm(operands),
            T_STAR => self.op_t_star(),

            // Text showing
            TJ_LOWER => self.op_tj(operands),
            TJ_UPPER => self.op_tj_upper(operands),
            QUOTE => self.op_quote(operands),
            DOUBLE_QUOTE => self.op_double_quote(operands),

            // ExtGState
            GS => self.op_gs(operands),

            // XObject (placeholder — full implementation in iter 11)
            DO => {
                // TODO: handle Form XObjects
                Ok(())
            }

            // Marked content (placeholder — full implementation in iter 12)
            BMC | BDC | EMC => Ok(()),

            // All other operators: ignore (not needed for text extraction)
            _ => Ok(()),
        }
    }

    // === Graphics state operators ===

    fn op_save(&mut self) -> Result<()> {
        self.state_stack.save();
        Ok(())
    }

    fn op_restore(&mut self) -> Result<()> {
        self.state_stack.restore();
        Ok(())
    }

    fn op_cm(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 6 {
            return Err(PdfError::Parse("cm requires 6 operands".into()));
        }
        let a = obj_to_f32(&operands[0])?;
        let b = obj_to_f32(&operands[1])?;
        let c = obj_to_f32(&operands[2])?;
        let d = obj_to_f32(&operands[3])?;
        let e = obj_to_f32(&operands[4])?;
        let f = obj_to_f32(&operands[5])?;
        let matrix = Matrix::from_values(a, b, c, d, e, f);
        self.state_stack.current_mut().ctm.concatenate(&matrix);
        Ok(())
    }

    // === Text delimiter operators ===

    fn op_bt(&mut self) -> Result<()> {
        self.state_stack.current_mut().begin_text();
        Ok(())
    }

    fn op_et(&mut self) -> Result<()> {
        self.state_stack.current_mut().end_text();
        Ok(())
    }

    // === Text state operators ===

    fn op_tc(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(last) = operands.last() {
            self.state_stack.current_mut().text_state.character_spacing = obj_to_f32(last)?;
        }
        Ok(())
    }

    fn op_tw(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(first) = operands.first() {
            self.state_stack.current_mut().text_state.word_spacing = obj_to_f32(first)?;
        }
        Ok(())
    }

    fn op_tz(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(first) = operands.first() {
            self.state_stack.current_mut().text_state.horizontal_scaling = obj_to_f32(first)?;
        }
        Ok(())
    }

    fn op_tl(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(first) = operands.first() {
            self.state_stack.current_mut().text_state.leading = obj_to_f32(first)?;
        }
        Ok(())
    }

    fn op_tf(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 2 {
            return Err(PdfError::Parse("Tf requires 2 operands".into()));
        }
        let font_name = operands[0].as_name()?.to_vec();
        let font_size = obj_to_f32(&operands[1])?;
        let ts = &mut self.state_stack.current_mut().text_state;
        ts.font_name = Some(font_name);
        ts.font_size = font_size;
        Ok(())
    }

    fn op_tr(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(first) = operands.first() {
            let mode = obj_to_f32(first)? as i64;
            self.state_stack.current_mut().text_state.rendering_mode =
                RenderingMode::from_i64(mode);
        }
        Ok(())
    }

    fn op_ts(&mut self, operands: &[Object]) -> Result<()> {
        if let Some(first) = operands.first() {
            self.state_stack.current_mut().text_state.rise = obj_to_f32(first)?;
        }
        Ok(())
    }

    // === Text positioning operators ===

    /// Td: Move text position by (tx, ty).
    fn op_td(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 2 {
            return Err(PdfError::Parse("Td requires 2 operands".into()));
        }
        let tx = obj_to_f32(&operands[0])?;
        let ty = obj_to_f32(&operands[1])?;

        let gs = self.state_stack.current_mut();
        if let Some(ref mut tlm) = gs.text_line_matrix {
            let translate = Matrix::translate_instance(tx, ty);
            tlm.concatenate(&translate);
            gs.text_matrix = Some(tlm.clone());
        }
        Ok(())
    }

    /// TD: Move text position and set leading (= -ty then Td).
    fn op_td_upper(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 2 {
            return Err(PdfError::Parse("TD requires 2 operands".into()));
        }
        let ty = obj_to_f32(&operands[1])?;
        self.state_stack.current_mut().text_state.leading = -ty;
        self.op_td(operands)
    }

    /// Tm: Set text matrix directly.
    fn op_tm(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 6 {
            return Err(PdfError::Parse("Tm requires 6 operands".into()));
        }
        let a = obj_to_f32(&operands[0])?;
        let b = obj_to_f32(&operands[1])?;
        let c = obj_to_f32(&operands[2])?;
        let d = obj_to_f32(&operands[3])?;
        let e = obj_to_f32(&operands[4])?;
        let f = obj_to_f32(&operands[5])?;
        let matrix = Matrix::from_values(a, b, c, d, e, f);

        let gs = self.state_stack.current_mut();
        gs.text_matrix = Some(matrix.clone());
        gs.text_line_matrix = Some(matrix);
        Ok(())
    }

    /// T*: Move to start of next line (= Td 0 -leading).
    fn op_t_star(&mut self) -> Result<()> {
        let leading = self.state_stack.current().text_state.leading;
        let operands = vec![Object::Real(0.0), Object::Real(-leading)];
        self.op_td(&operands)
    }

    // === Text showing operators ===

    /// Tj: Show text string.
    fn op_tj(&mut self, operands: &[Object]) -> Result<()> {
        if operands.is_empty() {
            return Ok(());
        }
        if self.state_stack.current().text_matrix.is_none() {
            return Ok(()); // Not inside BT..ET
        }
        if let Object::String(bytes, _) = &operands[0] {
            self.show_text(bytes)?;
        }
        Ok(())
    }

    /// TJ: Show text with positioning adjustments.
    fn op_tj_upper(&mut self, operands: &[Object]) -> Result<()> {
        if operands.is_empty() {
            return Ok(());
        }
        if self.state_stack.current().text_matrix.is_none() {
            return Ok(());
        }
        if let Object::Array(arr) = &operands[0] {
            for item in arr {
                match item {
                    Object::String(bytes, _) => {
                        self.show_text(bytes)?;
                    }
                    Object::Integer(n) => {
                        self.apply_tj_adjustment(*n as f32);
                    }
                    Object::Real(n) => {
                        self.apply_tj_adjustment(*n as f32);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// ': Move to next line and show text.
    fn op_quote(&mut self, operands: &[Object]) -> Result<()> {
        self.op_t_star()?;
        self.op_tj(operands)
    }

    /// ": Set word/char spacing, move to next line, show text.
    fn op_double_quote(&mut self, operands: &[Object]) -> Result<()> {
        if operands.len() < 3 {
            return Err(PdfError::Parse("\" requires 3 operands".into()));
        }
        self.state_stack.current_mut().text_state.word_spacing = obj_to_f32(&operands[0])?;
        self.state_stack.current_mut().text_state.character_spacing = obj_to_f32(&operands[1])?;
        self.op_t_star()?;
        if let Object::String(bytes, _) = &operands[2] {
            self.show_text(bytes)?;
        }
        Ok(())
    }

    // === ExtGState ===

    fn op_gs(&mut self, operands: &[Object]) -> Result<()> {
        if operands.is_empty() {
            return Ok(());
        }
        if let Object::Name(name) = &operands[0] {
            log::debug!("gs: ExtGState '{}'", String::from_utf8_lossy(name));
        }
        Ok(())
    }

    // === Text processing (showText / showGlyph port) ===

    /// Process a text string byte-by-byte, creating TextPosition for each glyph.
    ///
    /// Ported from PDFStreamEngine.showText() + LegacyPDFStreamEngine.showGlyph().
    fn show_text(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }

        // Read text state values upfront to avoid borrow issues.
        let font_name = self.state_stack.current().text_state.font_name.clone();
        let font_size = self.state_stack.current().text_state.font_size;
        let hs = self.state_stack.current().text_state.horizontal_scaling_fraction();
        let char_spacing = self.state_stack.current().text_state.character_spacing;
        let word_spacing = self.state_stack.current().text_state.word_spacing;
        let _rise = self.state_stack.current().text_state.rise;

        // Check if we have a font loaded.
        let has_font = font_name
            .as_ref()
            .map(|n| self.fonts.contains_key(n))
            .unwrap_or(false);

        let mut offset = 0;
        while offset < bytes.len() {
            // --- Read character code ---
            let (code, code_length) = if has_font {
                let name = font_name.as_ref().unwrap();
                self.fonts[name].read_code(bytes, offset)
            } else {
                // No font: single-byte fallback
                (bytes[offset] as u32, 1)
            };

            if code_length == 0 {
                break;
            }
            offset += code_length;

            // --- Compute text rendering matrix BEFORE glyph ---
            let trm = self.compute_text_rendering_matrix();

            // --- Get glyph width (text space, 1/1000 units) ---
            let width_1000 = if has_font {
                let name = font_name.as_ref().unwrap();
                self.fonts[name].get_width(code)
            } else {
                0.0
            };
            let displacement_x = width_1000 / 1000.0;

            // --- Compute end position (visual glyph extent) ---
            // td = displacement × fontSize × horizontalScaling
            let tx_visual = displacement_x * font_size * hs;
            let tm = self.state_stack.current().text_matrix.as_ref().unwrap();
            let ctm = &self.state_stack.current().ctm;
            let td = Matrix::translate_instance(tx_visual, 0.0);
            let next_trm = td.multiply(tm).multiply(ctm);
            let end_x = next_trm.translate_x();
            let end_y = next_trm.translate_y();

            // --- Width in display space ---
            let dx_display = end_x - trm.translate_x();

            // --- Font height in display space ---
            // Simplified: use fontSize (text space) scaled by TRM Y factor.
            // Full implementation would use FontDescriptor CapHeight/BBox.
            let font_height_text = 1.0; // 1.0 = full em in text space
            let dy_display = (font_height_text * trm.scaling_factor_y()).abs();

            // --- Space width in display space ---
            let space_width_text = if has_font {
                let name = font_name.as_ref().unwrap();
                let sw = self.fonts[name].get_width(32) / 1000.0;
                if sw > 0.0 {
                    sw
                } else {
                    // Fallback: use average width heuristic
                    let avg = self.fonts[name].get_width(65) / 1000.0;
                    if avg > 0.0 {
                        avg * 0.80
                    } else {
                        0.25
                    }
                }
            } else {
                0.25
            };
            let space_width_display = (space_width_text * trm.scaling_factor_x()).abs();

            // --- Unicode mapping ---
            let unicode = if has_font {
                let name = font_name.as_ref().unwrap();
                self.fonts[name].to_unicode(code).unwrap_or_default()
            } else {
                // Fallback: interpret as Latin-1
                if let Some(ch) = char::from_u32(code) {
                    ch.to_string()
                } else {
                    String::new()
                }
            };

            // --- Font size in points ---
            let tm_ref = self.state_stack.current().text_matrix.as_ref().unwrap();
            let font_size_in_pt = (font_size * tm_ref.scaling_factor_x()) as i32;

            // --- Create TextPosition ---
            if !unicode.is_empty() {
                let tp = TextPosition {
                    unicode,
                    char_codes: vec![code],
                    text_matrix: trm,
                    end_x,
                    end_y,
                    max_height: dy_display,
                    individual_width: dx_display,
                    space_width: space_width_display,
                    font_size,
                    font_size_in_pt,
                    page_rotation: self.page_rotation,
                    page_width: self.page_width,
                    page_height: self.page_height,
                };
                self.text_positions.push(tp);
            }

            // --- Advance text matrix ---
            // tx = (displacement × fontSize + charSpacing + wordSpacing) × Hs
            let word_space = if code_length == 1 && code == 32 {
                word_spacing
            } else {
                0.0
            };
            let total_tx = (displacement_x * font_size + char_spacing + word_space) * hs;

            let gs = self.state_stack.current_mut();
            if let Some(ref mut tm) = gs.text_matrix {
                tm.translate(total_tx, 0.0);
            }
        }

        Ok(())
    }

    /// Apply TJ numeric adjustment to text matrix.
    /// Positive values move text backwards (left for horizontal text).
    fn apply_tj_adjustment(&mut self, adjustment: f32) {
        let gs = self.state_stack.current();
        let font_size = gs.text_state.font_size;
        let hs = gs.text_state.horizontal_scaling_fraction();

        // TJ adjustment: tx = -(adjustment / 1000) * fontSize * Hs
        let tx = -(adjustment / 1000.0) * font_size * hs;

        let gs = self.state_stack.current_mut();
        if let Some(ref mut tm) = gs.text_matrix {
            tm.translate(tx, 0.0);
        }
    }

    /// Compute the text rendering matrix: Tfs × Th × Tm × CTM.
    fn compute_text_rendering_matrix(&self) -> Matrix {
        let gs = self.state_stack.current();
        let font_size = gs.text_state.font_size;
        let hs = gs.text_state.horizontal_scaling_fraction();
        let rise = gs.text_state.rise;

        // Text rendering matrix per PDF spec (section 9.4.4):
        // TRM = [fontSize×Hs 0 0; 0 fontSize 0; 0 rise 1] × Tm × CTM
        let params = Matrix::from_values(font_size * hs, 0.0, 0.0, font_size, 0.0, rise);

        if let Some(ref tm) = gs.text_matrix {
            params.multiply(tm).multiply(&gs.ctm)
        } else {
            params.multiply(&gs.ctm)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);
        engine.process_content(b"").unwrap();
        assert!(engine.text_positions().is_empty());
    }

    #[test]
    fn test_bt_et() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);
        engine.process_operator(BT, &[]).unwrap();
        assert!(engine.state_stack.current().text_matrix.is_some());
        engine.process_operator(ET, &[]).unwrap();
        assert!(engine.state_stack.current().text_matrix.is_none());
    }

    #[test]
    fn test_text_state_operators() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine
            .process_operator(TC, &[Object::Real(2.0)])
            .unwrap();
        assert_eq!(
            engine.state_stack.current().text_state.character_spacing,
            2.0
        );

        engine
            .process_operator(TW, &[Object::Real(1.5)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.word_spacing, 1.5);

        engine
            .process_operator(TZ, &[Object::Integer(80)])
            .unwrap();
        assert_eq!(
            engine.state_stack.current().text_state.horizontal_scaling,
            80.0
        );

        engine
            .process_operator(TL, &[Object::Integer(14)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.leading, 14.0);

        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();
        assert_eq!(
            engine.state_stack.current().text_state.font_name.as_deref(),
            Some(b"F1".as_slice())
        );
        assert_eq!(engine.state_stack.current().text_state.font_size, 12.0);

        engine
            .process_operator(TR, &[Object::Integer(1)])
            .unwrap();
        assert_eq!(
            engine.state_stack.current().text_state.rendering_mode,
            RenderingMode::Stroke
        );

        engine
            .process_operator(TS, &[Object::Real(5.0)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.rise, 5.0);
    }

    #[test]
    fn test_text_positioning() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();

        engine
            .process_operator(
                TM,
                &[
                    Object::Integer(1),
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(1),
                    Object::Integer(100),
                    Object::Integer(700),
                ],
            )
            .unwrap();

        let tm = engine
            .state_stack
            .current()
            .text_matrix
            .as_ref()
            .unwrap();
        assert!((tm.translate_x() - 100.0).abs() < 0.001);
        assert!((tm.translate_y() - 700.0).abs() < 0.001);

        engine
            .process_operator(TD_LOWER, &[Object::Integer(50), Object::Integer(0)])
            .unwrap();

        let tm = engine
            .state_stack
            .current()
            .text_matrix
            .as_ref()
            .unwrap();
        assert!((tm.translate_x() - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_show_text_without_font() {
        // Without loaded fonts, show_text should still work (Latin-1 fallback)
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"Hello".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        // Each byte generates a TextPosition with Latin-1 fallback
        assert_eq!(engine.text_positions().len(), 5);
        assert_eq!(engine.text_positions()[0].unicode, "H");
        assert_eq!(engine.text_positions()[1].unicode, "e");
        assert_eq!(engine.text_positions()[4].unicode, "o");
    }

    #[test]
    fn test_show_text_with_font() {
        use crate::font::simple_font::SimpleFont;
        use lopdf::dictionary;

        let doc = Arc::new(Document::with_version("1.5"));
        let font_dict = dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type1".to_vec()),
            "BaseFont" => Object::Name(b"Helvetica".to_vec()),
            "Encoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "FirstChar" => Object::Integer(32),
            "Widths" => Object::Array(
                (0..224).map(|i| Object::Integer(500 + i)).collect()
            )
        };
        let font = PdfFont::Simple(SimpleFont::from_dict(&doc, &font_dict, "Type1").unwrap());

        let mut fonts = HashMap::new();
        fonts.insert(b"F1".to_vec(), font);

        let mut engine = StreamEngine::new(doc);
        engine.set_fonts(fonts);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TM,
                &[
                    Object::Integer(1),
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(1),
                    Object::Integer(100),
                    Object::Integer(700),
                ],
            )
            .unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"AB".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        assert_eq!(engine.text_positions().len(), 2);
        // First char 'A' (code 65) decoded via WinAnsiEncoding
        assert_eq!(engine.text_positions()[0].unicode, "A");
        assert_eq!(engine.text_positions()[0].font_size, 12.0);
        assert!((engine.text_positions()[0].x() - 100.0).abs() < 0.001);
        assert!((engine.text_positions()[0].y() - 700.0).abs() < 0.001);

        // Second char 'B' should be offset by first char's advance
        assert_eq!(engine.text_positions()[1].unicode, "B");
        // 'A' has code 65, first_char=32, so width index = 33, width = 500+33 = 533
        // advance = (533/1000 * 12 + 0 + 0) * 1.0 = 6.396
        let expected_x = 100.0 + (533.0 / 1000.0) * 12.0;
        assert!(
            (engine.text_positions()[1].x() - expected_x).abs() < 0.01,
            "expected x={}, got x={}",
            expected_x,
            engine.text_positions()[1].x()
        );
    }

    #[test]
    fn test_tj_array_with_font() {
        use crate::font::simple_font::SimpleFont;
        use lopdf::dictionary;

        let doc = Arc::new(Document::with_version("1.5"));
        let font_dict = dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type1".to_vec()),
            "BaseFont" => Object::Name(b"Helvetica".to_vec()),
            "Encoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "FirstChar" => Object::Integer(32),
            "Widths" => Object::Array(
                (0..224).map(|_| Object::Integer(600)).collect()
            )
        };
        let font = PdfFont::Simple(SimpleFont::from_dict(&doc, &font_dict, "Type1").unwrap());

        let mut fonts = HashMap::new();
        fonts.insert(b"F1".to_vec(), font);

        let mut engine = StreamEngine::new(doc);
        engine.set_fonts(fonts);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(10)],
            )
            .unwrap();

        // TJ: [("A") -500 ("B")]
        // The -500 adjustment should move text position by +500/1000 * 10 * 1.0 = +5.0 units
        let arr = Object::Array(vec![
            Object::String(b"A".to_vec(), lopdf::StringFormat::Literal),
            Object::Integer(-500),
            Object::String(b"B".to_vec(), lopdf::StringFormat::Literal),
        ]);
        engine.process_operator(TJ_UPPER, &[arr]).unwrap();

        assert_eq!(engine.text_positions().len(), 2);
        assert_eq!(engine.text_positions()[0].unicode, "A");
        assert_eq!(engine.text_positions()[1].unicode, "B");

        // 'A' starts at x=0 (identity matrix)
        // 'A' advance = 600/1000 * 10 = 6.0
        // TJ adjustment = -(-500)/1000 * 10 = +5.0
        // 'B' starts at x = 6.0 + 5.0 = 11.0
        let b_x = engine.text_positions()[1].x();
        assert!(
            (b_x - 11.0).abs() < 0.01,
            "expected B at x=11.0, got x={}",
            b_x
        );
    }

    #[test]
    fn test_save_restore_state() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();
        engine.process_operator(Q_LOWER, &[]).unwrap();

        engine
            .process_operator(
                TF,
                &[Object::Name(b"F2".to_vec()), Object::Integer(24)],
            )
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.font_size, 24.0);

        engine.process_operator(Q_UPPER, &[]).unwrap();
        assert_eq!(engine.state_stack.current().text_state.font_size, 12.0);
    }

    #[test]
    fn test_cm_concatenates_ctm() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine
            .process_operator(
                CM,
                &[
                    Object::Integer(1),
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(1),
                    Object::Integer(100),
                    Object::Integer(200),
                ],
            )
            .unwrap();

        let ctm = &engine.state_stack.current().ctm;
        assert!((ctm.translate_x() - 100.0).abs() < 0.001);
        assert!((ctm.translate_y() - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_text_matrix_advances() {
        // Verify text matrix advances correctly after each glyph
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(10)],
            )
            .unwrap();

        // Show "ABC" without font — each byte advances by 0 (no width info)
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"ABC".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        // Without a font, width is 0, so all positions should be at x=0
        assert_eq!(engine.text_positions().len(), 3);
        assert!((engine.text_positions()[0].x() - 0.0).abs() < 0.001);
        assert!((engine.text_positions()[1].x() - 0.0).abs() < 0.001);
        assert!((engine.text_positions()[2].x() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_word_spacing() {
        use crate::font::simple_font::SimpleFont;
        use lopdf::dictionary;

        let doc = Arc::new(Document::with_version("1.5"));
        let font_dict = dictionary! {
            "Type" => Object::Name(b"Font".to_vec()),
            "Subtype" => Object::Name(b"Type1".to_vec()),
            "BaseFont" => Object::Name(b"Helvetica".to_vec()),
            "Encoding" => Object::Name(b"WinAnsiEncoding".to_vec()),
            "FirstChar" => Object::Integer(32),
            "Widths" => Object::Array(
                (0..224).map(|_| Object::Integer(500)).collect()
            )
        };
        let font = PdfFont::Simple(SimpleFont::from_dict(&doc, &font_dict, "Type1").unwrap());

        let mut fonts = HashMap::new();
        fonts.insert(b"F1".to_vec(), font);

        let mut engine = StreamEngine::new(doc);
        engine.set_fonts(fonts);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(10)],
            )
            .unwrap();
        // Set word spacing = 5.0
        engine
            .process_operator(TW, &[Object::Real(5.0)])
            .unwrap();

        // "A B" — space at code 32 should get extra word spacing
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"A B".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        // 'A' at x=0, advance = 500/1000 * 10 = 5.0
        // ' ' at x=5.0, advance = (500/1000 * 10 + 0 + 5.0) * 1.0 = 10.0
        // 'B' at x=15.0
        assert_eq!(engine.text_positions().len(), 3);
        assert!((engine.text_positions()[2].x() - 15.0).abs() < 0.1);
    }
}
