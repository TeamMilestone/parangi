//! Content stream processing engine.
//!
//! Ported from PDFStreamEngine + LegacyPDFStreamEngine.
//! Processes PDF content stream operators to update graphics/text state
//! and extract text positioning information.

use std::sync::Arc;

use lopdf::{content::Content, Document, Object};

use super::graphics_state::GraphicsStateStack;
use super::matrix::Matrix;
use super::operators::*;
use super::text_state::RenderingMode;
use crate::cos_helpers::obj_to_f32;
use crate::{PdfError, Result};

/// A raw text segment extracted from the content stream.
/// Later iterations will replace this with full TextPosition.
#[derive(Debug, Clone)]
pub struct RawTextSegment {
    /// The raw bytes from the PDF string operand.
    pub bytes: Vec<u8>,
    /// Font resource name active when this text was rendered.
    pub font_name: Option<Vec<u8>>,
    /// Font size.
    pub font_size: f32,
    /// Text rendering matrix at the start of this segment.
    /// This is text_matrix × CTM.
    pub text_rendering_matrix: Matrix,
}

/// Content stream processing engine.
///
/// Processes a single page's content stream, maintaining graphics state
/// and collecting text segments.
pub struct StreamEngine {
    doc: Arc<Document>,
    state_stack: GraphicsStateStack,
    /// Collected text segments from this content stream.
    text_segments: Vec<RawTextSegment>,
}

impl StreamEngine {
    /// Create a new stream engine.
    pub fn new(doc: Arc<Document>) -> Self {
        Self {
            doc,
            state_stack: GraphicsStateStack::new(),
            text_segments: Vec::new(),
        }
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

    /// Get the collected text segments.
    pub fn text_segments(&self) -> &[RawTextSegment] {
        &self.text_segments
    }

    /// Consume and return the collected text segments.
    pub fn into_text_segments(self) -> Vec<RawTextSegment> {
        self.text_segments
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
            self.show_text_bytes(bytes)?;
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
                        self.show_text_bytes(bytes)?;
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
        // First operand: word spacing
        self.state_stack.current_mut().text_state.word_spacing = obj_to_f32(&operands[0])?;
        // Second operand: character spacing
        self.state_stack.current_mut().text_state.character_spacing = obj_to_f32(&operands[1])?;
        // Third operand: text string (show via quote)
        self.op_t_star()?;
        if let Object::String(bytes, _) = &operands[2] {
            self.show_text_bytes(bytes)?;
        }
        Ok(())
    }

    // === ExtGState ===

    fn op_gs(&mut self, operands: &[Object]) -> Result<()> {
        // gs applies extended graphics state parameters.
        // For text extraction, we mainly care about font settings if present.
        // Full implementation deferred; this is a minimal placeholder.
        if operands.is_empty() {
            return Ok(());
        }
        // The operand is the name of the ExtGState resource.
        // We would look it up and apply relevant parameters.
        // For now, just log it.
        if let Object::Name(name) = &operands[0] {
            log::debug!("gs: ExtGState '{}'", String::from_utf8_lossy(name));
        }
        Ok(())
    }

    // === Internal helpers ===

    /// Record a text segment from raw bytes.
    fn show_text_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let gs = self.state_stack.current();
        let trm = self.compute_text_rendering_matrix(gs);
        let segment = RawTextSegment {
            bytes: bytes.to_vec(),
            font_name: gs.text_state.font_name.clone(),
            font_size: gs.text_state.font_size,
            text_rendering_matrix: trm,
        };
        self.text_segments.push(segment);
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
            let translate = Matrix::translate_instance(tx, 0.0);
            tm.concatenate(&translate);
        }
    }

    /// Compute the text rendering matrix: Tfs × Th × Tm × CTM.
    fn compute_text_rendering_matrix(
        &self,
        gs: &super::graphics_state::GraphicsState,
    ) -> Matrix {
        let font_size = gs.text_state.font_size;
        let hs = gs.text_state.horizontal_scaling_fraction();
        let rise = gs.text_state.rise;

        // Text rendering matrix per PDF spec (section 9.4.4):
        // TRM = [fontSize×Hs  0  0; 0  fontSize  0; 0  rise  1] × Tm × CTM
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
        assert!(engine.text_segments().is_empty());
    }

    #[test]
    fn test_bt_et() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);
        // Process BT/ET
        engine.process_operator(BT, &[]).unwrap();
        assert!(engine.state_stack.current().text_matrix.is_some());
        engine.process_operator(ET, &[]).unwrap();
        assert!(engine.state_stack.current().text_matrix.is_none());
    }

    #[test]
    fn test_text_state_operators() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        // Tc 2.0
        engine
            .process_operator(TC, &[Object::Real(2.0)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.character_spacing, 2.0);

        // Tw 1.5
        engine
            .process_operator(TW, &[Object::Real(1.5)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.word_spacing, 1.5);

        // Tz 80
        engine
            .process_operator(TZ, &[Object::Integer(80)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.horizontal_scaling, 80.0);

        // TL 14
        engine
            .process_operator(TL, &[Object::Integer(14)])
            .unwrap();
        assert_eq!(engine.state_stack.current().text_state.leading, 14.0);

        // Tf /F1 12
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

        // Tr 1
        engine
            .process_operator(TR, &[Object::Integer(1)])
            .unwrap();
        assert_eq!(
            engine.state_stack.current().text_state.rendering_mode,
            RenderingMode::Stroke
        );

        // Ts 5
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

        // Tm sets text matrix directly
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

        // Td moves relative
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
    fn test_show_text_collects_segments() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // Tj shows text
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(b"Hello".to_vec(), lopdf::StringFormat::Literal)],
            )
            .unwrap();

        assert_eq!(engine.text_segments().len(), 1);
        assert_eq!(engine.text_segments()[0].bytes, b"Hello");
        assert_eq!(engine.text_segments()[0].font_size, 12.0);
    }

    #[test]
    fn test_tj_array() {
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // TJ with adjustments: [("He") -120 ("llo")]
        let arr = Object::Array(vec![
            Object::String(b"He".to_vec(), lopdf::StringFormat::Literal),
            Object::Integer(-120),
            Object::String(b"llo".to_vec(), lopdf::StringFormat::Literal),
        ]);
        engine.process_operator(TJ_UPPER, &[arr]).unwrap();

        assert_eq!(engine.text_segments().len(), 2);
        assert_eq!(engine.text_segments()[0].bytes, b"He");
        assert_eq!(engine.text_segments()[1].bytes, b"llo");
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

        // cm with translation
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
}
