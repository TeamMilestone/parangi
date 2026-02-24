//! Content stream processing engine.
//!
//! Ported from PDFStreamEngine + LegacyPDFStreamEngine.
//! Processes PDF content stream operators to update graphics/text state
//! and extract text positioning information.

use std::collections::HashMap;
use std::sync::Arc;

use lopdf::{content::Content, Document, Object, ObjectId};

use super::graphics_state::GraphicsStateStack;
use super::matrix::Matrix;
use super::operators::*;
use super::text_state::RenderingMode;
use crate::cos_helpers::{name_to_string, obj_to_f32};
use crate::font::PdfFont;
use crate::text::TextPosition;
use crate::{PdfError, Result};

/// A marked content entry on the stack (for BMC/BDC/EMC tracking).
#[derive(Debug)]
struct MarkedContentEntry {
    /// The tag name (e.g. "Span", "P", "Artifact").
    #[allow(dead_code)]
    tag: String,
    /// The ActualText value from properties, if present.
    actual_text: Option<String>,
}

/// Pre-extracted Form XObject data (avoids borrow conflicts).
struct FormData {
    content_bytes: Vec<u8>,
    matrix: Option<Matrix>,
    resources_fonts: Option<HashMap<Vec<u8>, PdfFont>>,
    resources_xobject_refs: Option<HashMap<Vec<u8>, ObjectId>>,
}

/// Content stream processing engine.
///
/// Processes a single page's content stream, maintaining graphics state
/// and collecting TextPosition objects for each glyph.
pub struct StreamEngine {
    doc: Arc<Document>,
    state_stack: GraphicsStateStack,

    /// Loaded fonts keyed by resource name (e.g. b"F1").
    fonts: HashMap<Vec<u8>, PdfFont>,
    /// XObject references in current scope (name → ObjectId).
    xobject_refs: HashMap<Vec<u8>, ObjectId>,
    /// Collected text positions from this content stream.
    text_positions: Vec<TextPosition>,

    /// Page rotation (0, 90, 180, 270).
    page_rotation: i32,
    /// Page width (from CropBox or MediaBox).
    page_width: f32,
    /// Page height.
    page_height: f32,

    /// Recursion depth for Form XObject nesting.
    nesting_level: u32,

    /// Stack of marked content entries (BMC/BDC push, EMC pop).
    marked_content_stack: Vec<MarkedContentEntry>,
    /// Current ActualText replacement (from innermost BDC with ActualText).
    actual_text: Option<String>,
    /// Whether we're still waiting for the first TextPosition within an ActualText span.
    first_actual_text_position: bool,
}

impl StreamEngine {
    /// Create a new stream engine.
    pub fn new(doc: Arc<Document>) -> Self {
        Self {
            doc,
            state_stack: GraphicsStateStack::new(),
            fonts: HashMap::new(),
            xobject_refs: HashMap::new(),
            text_positions: Vec::new(),
            page_rotation: 0,
            page_width: 612.0,
            page_height: 792.0,
            nesting_level: 0,
            marked_content_stack: Vec::new(),
            actual_text: None,
            first_actual_text_position: false,
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

    /// Load fonts and XObject references from a resources dictionary.
    pub fn load_resources(&mut self, resources_dict: &lopdf::Dictionary) {
        self.fonts = Self::load_fonts_from_resources(&self.doc, resources_dict);
        self.xobject_refs = Self::load_xobject_refs(&self.doc, resources_dict);
    }

    /// Process a content stream (raw bytes).
    pub fn process_content(&mut self, content_bytes: &[u8]) -> Result<()> {
        if content_bytes.is_empty() {
            return Ok(());
        }

        let has_nul = content_bytes.contains(&0u8);

        if !has_nul {
            // Fast path: no NUL bytes, try direct decode then strip inline images
            if let Ok(content) = Content::decode(content_bytes) {
                return self.process_operations(&content.operations);
            }
            let cleaned = Self::strip_unparseable_content(content_bytes);
            if let Ok(content) = Content::decode(&cleaned) {
                return self.process_operations(&content.operations);
            }
            return Err(PdfError::Parse(
                "content decode: invalid content stream".into(),
            ));
        }

        // Content has NUL bytes. lopdf's Content::decode may silently truncate
        // at NUL positions, so we must clean the content first.

        // Try 1: strip inline images and NUL bytes, then decode
        let cleaned = Self::strip_unparseable_content(content_bytes);
        if let Ok(content) = Content::decode(&cleaned) {
            return self.process_operations(&content.operations);
        }

        // Try 2: split on NUL runs and decode each chunk independently
        self.process_chunks(content_bytes)
    }

    /// Process decoded operations.
    fn process_operations(
        &mut self,
        operations: &[lopdf::content::Operation],
    ) -> Result<()> {
        for op in operations {
            if let Err(e) = self.process_operator(&op.operator, &op.operands) {
                log::warn!("operator '{}' error: {}", op.operator, e);
            }
        }
        Ok(())
    }

    /// Strip inline images (BI...ID...EI) and NUL bytes from content streams.
    ///
    /// lopdf's Content::decode cannot parse inline image data (binary content
    /// between ID and EI operators). Since we only need text operators, we can
    /// safely remove inline images. NUL byte padding from HWP→PDF converters
    /// is also stripped — but NUL bytes inside parenthesized string literals
    /// `(...)` are preserved, as they may be valid CID character codes.
    fn strip_unparseable_content(content_bytes: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(content_bytes.len());
        let mut i = 0;
        let mut paren_depth = 0u32;

        while i < content_bytes.len() {
            let b = content_bytes[i];

            // Track parenthesized string literal depth (handling escapes)
            if paren_depth > 0 {
                // Inside a string literal: keep everything including NUL
                result.push(b);
                if b == b'\\' {
                    // Escape sequence: copy next byte too
                    i += 1;
                    if i < content_bytes.len() {
                        result.push(content_bytes[i]);
                    }
                } else if b == b'(' {
                    paren_depth += 1;
                } else if b == b')' {
                    paren_depth -= 1;
                }
                i += 1;
                continue;
            }

            // Outside string literals
            if b == b'(' {
                paren_depth = 1;
                result.push(b);
                i += 1;
                continue;
            }

            // Skip NUL bytes outside strings
            if b == 0 {
                i += 1;
                continue;
            }

            // Check for BI (Begin Inline Image) operator
            if i + 2 < content_bytes.len()
                && b == b'B'
                && content_bytes[i + 1] == b'I'
                && (i == 0 || content_bytes[i - 1].is_ascii_whitespace())
                && content_bytes[i + 2].is_ascii_whitespace()
            {
                if let Some(ei_pos) = Self::find_ei(&content_bytes[i..]) {
                    i += ei_pos;
                    if i < content_bytes.len() && content_bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    continue;
                }
            }

            result.push(b);
            i += 1;
        }

        result
    }

    /// Find the position after EI (End Inline Image) in the given slice.
    /// Returns the offset past "EI\n" or "EI " relative to the start of the slice.
    fn find_ei(data: &[u8]) -> Option<usize> {
        // Skip past BI
        let mut i = 2;
        // Find ID (Image Data) marker
        while i + 2 < data.len() {
            if data[i] == b'I'
                && data[i + 1] == b'D'
                && (i == 0 || data[i - 1].is_ascii_whitespace())
                && data[i + 2].is_ascii_whitespace()
            {
                // Skip past ID + whitespace + binary data
                i += 3; // past "ID "
                // Now scan for EI preceded by whitespace
                while i + 2 < data.len() {
                    if data[i] == b'E'
                        && data[i + 1] == b'I'
                        && data[i - 1].is_ascii_whitespace()
                        && (i + 2 >= data.len() || data[i + 2].is_ascii_whitespace())
                    {
                        return Some(i + 2);
                    }
                    i += 1;
                }
                // No EI found — skip to end
                return Some(data.len());
            }
            i += 1;
        }
        None
    }

    /// Split content on NUL byte runs and decode each chunk independently.
    fn process_chunks(&mut self, content_bytes: &[u8]) -> Result<()> {
        let mut processed_any = false;
        let mut start = 0;

        while start < content_bytes.len() {
            // Skip NUL bytes
            if content_bytes[start] == 0 {
                start += 1;
                continue;
            }

            // Find end of non-NUL chunk (stop at runs of 3+ NUL bytes)
            let mut end = start + 1;
            while end < content_bytes.len() {
                if content_bytes[end] == 0 {
                    let nul_run = content_bytes[end..]
                        .iter()
                        .take_while(|&&b| b == 0)
                        .count();
                    if nul_run >= 3 {
                        break;
                    }
                    end += nul_run;
                } else {
                    end += 1;
                }
            }

            let chunk = &content_bytes[start..end];
            if !chunk.is_empty() {
                // Try direct decode, then with inline image stripping
                let content = Content::decode(chunk).or_else(|_| {
                    let cleaned = Self::strip_unparseable_content(chunk);
                    Content::decode(&cleaned)
                });
                if let Ok(content) = content {
                    for op in &content.operations {
                        if let Err(e) = self.process_operator(&op.operator, &op.operands) {
                            log::warn!("operator '{}' error: {}", op.operator, e);
                        }
                    }
                    processed_any = true;
                }
            }
            start = end;
        }

        if processed_any {
            Ok(())
        } else {
            Err(PdfError::Parse(
                "content decode: no valid chunks found".into(),
            ))
        }
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

            // XObject (Form XObject processing)
            DO => self.op_do(operands),

            // Marked content
            BMC => self.op_bmc(operands),
            BDC => self.op_bdc(operands),
            EMC => self.op_emc(),
            MP | DP => Ok(()), // Marked content points: no-op for text extraction

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

    // === Form XObject (Do operator) ===

    /// Do: Invoke a named XObject.
    fn op_do(&mut self, operands: &[Object]) -> Result<()> {
        if operands.is_empty() {
            return Ok(());
        }
        let name = match &operands[0] {
            Object::Name(n) => n.clone(),
            _ => return Ok(()),
        };

        // Look up XObject reference
        let oid = match self.xobject_refs.get(&name) {
            Some(&oid) => oid,
            None => return Ok(()), // Unknown XObject, skip
        };

        // Extract Form data (all immutable operations, avoids borrow conflicts)
        let form_data = self.extract_form_data(oid)?;
        if let Some(data) = form_data {
            self.nesting_level += 1;
            if self.nesting_level > 50 {
                self.nesting_level -= 1;
                log::warn!("Form XObject nesting too deep (>50), skipping");
                return Ok(());
            }
            let result = self.process_form(data);
            self.nesting_level -= 1;
            return result;
        }

        Ok(())
    }

    /// Extract all data from a Form XObject without mutable self borrow.
    fn extract_form_data(&self, oid: ObjectId) -> Result<Option<FormData>> {
        let obj = self
            .doc
            .get_object(oid)
            .map_err(|e| PdfError::Parse(format!("XObject {:?}: {}", oid, e)))?;

        let stream = match obj {
            Object::Stream(s) => s,
            _ => return Ok(None),
        };

        // Check Subtype = Form
        let subtype = stream
            .dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| match o {
                Object::Name(n) => Some(name_to_string(n)),
                _ => None,
            })
            .unwrap_or_default();

        if subtype != "Form" {
            return Ok(None);
        }

        // Decompress content (fall back to raw bytes for streams without /Filter)
        let content_bytes = match stream.decompressed_content() {
            Ok(data) => data,
            Err(_) => stream.content.clone(),
        };

        if content_bytes.is_empty() {
            return Ok(None);
        }

        // Read Form matrix
        let matrix = Self::read_form_matrix(&self.doc, &stream.dict);

        // Load Form's resources
        let (res_fonts, res_xobjects) = self.load_form_resources(&stream.dict);

        Ok(Some(FormData {
            content_bytes,
            matrix,
            resources_fonts: res_fonts,
            resources_xobject_refs: res_xobjects,
        }))
    }

    /// Process a pre-extracted Form XObject.
    fn process_form(&mut self, data: FormData) -> Result<()> {
        let has_own_resources = data.resources_fonts.is_some();

        // 1. Swap in Form's resources if it has its own; otherwise inherit parent's.
        let saved_fonts;
        let saved_xobject_refs;

        if has_own_resources {
            saved_fonts =
                Some(std::mem::replace(&mut self.fonts, data.resources_fonts.unwrap()));
            saved_xobject_refs = Some(std::mem::replace(
                &mut self.xobject_refs,
                data.resources_xobject_refs.unwrap_or_default(),
            ));
        } else {
            saved_fonts = None;
            saved_xobject_refs = None;
        }

        // 2. Save entire graphics state stack
        let saved_stack = self.state_stack.save_full();

        // 3. Apply Form's matrix to CTM
        if let Some(form_matrix) = data.matrix {
            self.state_stack
                .current_mut()
                .ctm
                .concatenate(&form_matrix);
        }

        // 4. Process Form's content stream
        let result = self.process_content(&data.content_bytes);

        // 5. Restore graphics state
        self.state_stack.restore_full(saved_stack);

        // 6. Restore resources if we swapped them
        if let Some(fonts) = saved_fonts {
            self.fonts = fonts;
        }
        if let Some(xobjs) = saved_xobject_refs {
            self.xobject_refs = xobjs;
        }

        result
    }

    /// Read the Matrix entry from a Form XObject dictionary.
    fn read_form_matrix(doc: &Document, form_dict: &lopdf::Dictionary) -> Option<Matrix> {
        let matrix_obj = form_dict.get(b"Matrix").ok()?;
        let arr = match matrix_obj {
            Object::Array(arr) if arr.len() >= 6 => arr,
            Object::Reference(id) => match doc.get_object(*id).ok()? {
                Object::Array(arr) if arr.len() >= 6 => arr,
                _ => return None,
            },
            _ => return None,
        };

        let vals: Vec<f32> = arr
            .iter()
            .take(6)
            .map(|o| obj_to_f32(o).unwrap_or(0.0))
            .collect();
        Some(Matrix::from_values(
            vals[0], vals[1], vals[2], vals[3], vals[4], vals[5],
        ))
    }

    /// Load resources (fonts + xobjects) from a Form XObject's dictionary.
    /// Returns (Some(fonts), Some(xobjects)) if the Form has a Resources entry,
    /// or (None, None) to inherit parent's resources.
    fn load_form_resources(
        &self,
        form_dict: &lopdf::Dictionary,
    ) -> (
        Option<HashMap<Vec<u8>, PdfFont>>,
        Option<HashMap<Vec<u8>, ObjectId>>,
    ) {
        let res_obj = match form_dict.get(b"Resources") {
            Ok(obj) => obj,
            Err(_) => return (None, None), // No Resources → inherit parent
        };

        let res_dict = match res_obj {
            Object::Reference(id) => match self.doc.get_object(*id) {
                Ok(obj) => match obj.as_dict() {
                    Ok(d) => d,
                    Err(_) => return (None, None),
                },
                Err(_) => return (None, None),
            },
            Object::Dictionary(d) => d,
            _ => return (None, None),
        };

        let fonts = Self::load_fonts_from_resources(&self.doc, res_dict);
        let xobject_refs = Self::load_xobject_refs(&self.doc, res_dict);
        (Some(fonts), Some(xobject_refs))
    }

    /// Load fonts from a Resources dictionary's Font sub-entry.
    fn load_fonts_from_resources(
        doc: &Document,
        resources_dict: &lopdf::Dictionary,
    ) -> HashMap<Vec<u8>, PdfFont> {
        let mut fonts = HashMap::new();

        let font_dict = match resources_dict.get(b"Font") {
            Ok(obj) => {
                let resolved = match obj {
                    Object::Reference(id) => doc.get_object(*id).ok(),
                    _ => Some(obj),
                };
                match resolved.and_then(|o| o.as_dict().ok()) {
                    Some(d) => d,
                    None => return fonts,
                }
            }
            Err(_) => return fonts,
        };

        for (name, obj) in font_dict.iter() {
            let (dict, oid) = match obj {
                Object::Reference(id) => match doc.get_object(*id) {
                    Ok(o) => match o.as_dict() {
                        Ok(d) => (d, *id),
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                },
                Object::Dictionary(d) => (d, (0, 0)),
                _ => continue,
            };
            match PdfFont::from_dict(doc, dict, oid) {
                Ok(font) => {
                    fonts.insert(name.clone(), font);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to load font {:?}: {}",
                        String::from_utf8_lossy(name),
                        e
                    );
                }
            }
        }
        fonts
    }

    /// Load XObject references from a Resources dictionary's XObject sub-entry.
    fn load_xobject_refs(
        doc: &Document,
        resources_dict: &lopdf::Dictionary,
    ) -> HashMap<Vec<u8>, ObjectId> {
        let mut refs = HashMap::new();

        let xobj_dict = match resources_dict.get(b"XObject") {
            Ok(obj) => {
                let resolved = match obj {
                    Object::Reference(id) => doc.get_object(*id).ok(),
                    _ => Some(obj),
                };
                match resolved.and_then(|o| o.as_dict().ok()) {
                    Some(d) => d,
                    None => return refs,
                }
            }
            Err(_) => return refs,
        };

        for (name, obj) in xobj_dict.iter() {
            if let Object::Reference(id) = obj {
                refs.insert(name.clone(), *id);
            }
        }
        refs
    }

    // === Marked content operators (BMC/BDC/EMC) ===

    /// BMC: Begin marked content (tag only, no properties).
    fn op_bmc(&mut self, operands: &[Object]) -> Result<()> {
        let tag = if let Some(Object::Name(n)) = operands.first() {
            name_to_string(n)
        } else {
            String::new()
        };
        self.marked_content_stack.push(MarkedContentEntry {
            tag,
            actual_text: None,
        });
        Ok(())
    }

    /// BDC: Begin marked content with properties dict.
    /// Extracts ActualText from properties if present.
    fn op_bdc(&mut self, operands: &[Object]) -> Result<()> {
        let tag = if let Some(Object::Name(n)) = operands.first() {
            name_to_string(n)
        } else {
            String::new()
        };

        // Extract properties dictionary (inline or referenced)
        let actual_text = self.extract_actual_text(operands);

        let entry = MarkedContentEntry {
            tag,
            actual_text: actual_text.clone(),
        };
        self.marked_content_stack.push(entry);

        // If this BDC has ActualText, activate it
        if let Some(text) = actual_text {
            // Remove soft hyphens (U+00AD), matching PDFBox behavior
            let cleaned = text.replace('\u{00ad}', "");
            self.actual_text = Some(cleaned);
            self.first_actual_text_position = true;
        }

        Ok(())
    }

    /// EMC: End marked content.
    fn op_emc(&mut self) -> Result<()> {
        if let Some(entry) = self.marked_content_stack.pop() {
            // If this entry had ActualText, clear it
            if entry.actual_text.is_some() {
                self.actual_text = None;
                self.first_actual_text_position = false;
            }
        }
        Ok(())
    }

    /// Extract ActualText from BDC operands.
    /// Operands are [tag_name, properties_dict_or_name].
    fn extract_actual_text(&self, operands: &[Object]) -> Option<String> {
        if operands.len() < 2 {
            return None;
        }

        let props = match &operands[1] {
            Object::Dictionary(d) => d,
            Object::Name(name) => {
                // Properties referenced by name from page's Properties resource
                // For now, skip property resource lookup (rare in practice)
                log::debug!(
                    "BDC properties reference: {:?}",
                    String::from_utf8_lossy(name)
                );
                return None;
            }
            Object::Reference(id) => {
                // Resolve indirect reference
                match self.doc.get_object(*id) {
                    Ok(Object::Dictionary(d)) => d,
                    _ => return None,
                }
            }
            _ => return None,
        };

        // Look for /ActualText in properties
        match props.get(b"ActualText") {
            Ok(Object::String(bytes, _format)) => {
                // Try UTF-16BE first (starts with BOM FE FF)
                if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                    let utf16: Vec<u16> = bytes[2..]
                        .chunks(2)
                        .map(|chunk| {
                            if chunk.len() == 2 {
                                u16::from_be_bytes([chunk[0], chunk[1]])
                            } else {
                                0
                            }
                        })
                        .collect();
                    String::from_utf16(&utf16).ok()
                } else {
                    // PDFDocEncoding (roughly Latin-1 for most chars)
                    Some(bytes.iter().map(|&b| b as char).collect())
                }
            }
            _ => None,
        }
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
        let gs = self.state_stack.current();
        let font_name = gs.text_state.font_name.clone();
        let font_size = gs.text_state.font_size;
        let hs = gs.text_state.horizontal_scaling_fraction();
        let char_spacing = gs.text_state.character_spacing;
        let word_spacing = gs.text_state.word_spacing;

        // Check if we have a font loaded.
        let has_font = font_name
            .as_ref()
            .map(|n| self.fonts.contains_key(n))
            .unwrap_or(false);

        // Cache space_width and actual_text for use in loop.
        let cached_space_width = if has_font {
            let name = font_name.as_ref().unwrap();
            let sw = self.fonts[name].get_width(32) / 1000.0;
            if sw > 0.0 {
                sw
            } else {
                let avg = self.fonts[name].get_width(65) / 1000.0;
                if avg > 0.0 { avg * 0.80 } else { 0.25 }
            }
        } else {
            0.25
        };
        let actual_text_value = self.actual_text.clone();

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

            // --- Space width in display space (cached) ---
            let space_width_display = (cached_space_width * trm.scaling_factor_x()).abs();

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

            // --- Apply ActualText replacement if active ---
            let final_unicode = if actual_text_value.is_some() {
                if self.first_actual_text_position {
                    // First glyph in ActualText span: use the ActualText value
                    self.first_actual_text_position = false;
                    actual_text_value.clone().unwrap_or_default()
                } else {
                    // Subsequent glyphs in ActualText span: suppress (empty string)
                    String::new()
                }
            } else {
                unicode
            };

            // --- Create TextPosition ---
            if !final_unicode.is_empty() {
                let tp = TextPosition {
                    unicode: final_unicode,
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

    // === Marked content tests ===

    #[test]
    fn test_bmc_emc_no_effect_on_text() {
        // BMC/EMC without ActualText should not affect text extraction
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // BMC /Span
        engine
            .process_operator(BMC, &[Object::Name(b"Span".to_vec())])
            .unwrap();

        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"ABC".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap();

        // All three characters should appear normally
        assert_eq!(engine.text_positions().len(), 3);
        assert_eq!(engine.text_positions()[0].unicode, "A");
        assert_eq!(engine.text_positions()[1].unicode, "B");
        assert_eq!(engine.text_positions()[2].unicode, "C");
    }

    #[test]
    fn test_bdc_actual_text_replaces_glyphs() {
        // BDC with ActualText should replace glyph text
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // BDC /Span << /ActualText (fi) >>
        let props = lopdf::Dictionary::from_iter(vec![(
            b"ActualText".to_vec(),
            Object::String(b"fi".to_vec(), lopdf::StringFormat::Literal),
        )]);
        engine
            .process_operator(
                BDC,
                &[
                    Object::Name(b"Span".to_vec()),
                    Object::Dictionary(props),
                ],
            )
            .unwrap();

        // Content stream has 2 glyphs representing the ligature
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"\x01\x02".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap();

        // Only one TextPosition should be produced, with "fi" as unicode
        assert_eq!(engine.text_positions().len(), 1);
        assert_eq!(engine.text_positions()[0].unicode, "fi");
    }

    #[test]
    fn test_bdc_actual_text_utf16be() {
        // BDC with UTF-16BE ActualText
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // UTF-16BE encoded "가" (U+AC00): FE FF AC 00
        let utf16_bytes = vec![0xFE, 0xFF, 0xAC, 0x00];
        let props = lopdf::Dictionary::from_iter(vec![(
            b"ActualText".to_vec(),
            Object::String(utf16_bytes, lopdf::StringFormat::Hexadecimal),
        )]);
        engine
            .process_operator(
                BDC,
                &[
                    Object::Name(b"Span".to_vec()),
                    Object::Dictionary(props),
                ],
            )
            .unwrap();

        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"\x01".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap();

        assert_eq!(engine.text_positions().len(), 1);
        assert_eq!(engine.text_positions()[0].unicode, "가");
    }

    #[test]
    fn test_bdc_actual_text_soft_hyphen_removed() {
        // Soft hyphens (U+00AD) in ActualText should be removed
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // "a\u{00AD}b" in Latin-1 encoding (00AD = soft hyphen)
        let text_bytes = vec![b'a', 0xAD, b'b'];
        let props = lopdf::Dictionary::from_iter(vec![(
            b"ActualText".to_vec(),
            Object::String(text_bytes, lopdf::StringFormat::Literal),
        )]);
        engine
            .process_operator(
                BDC,
                &[
                    Object::Name(b"Span".to_vec()),
                    Object::Dictionary(props),
                ],
            )
            .unwrap();

        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"\x01".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap();

        assert_eq!(engine.text_positions().len(), 1);
        assert_eq!(engine.text_positions()[0].unicode, "ab");
    }

    #[test]
    fn test_nested_marked_content() {
        // Nested BMC/BDC blocks should work correctly
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        // Outer BMC /P (no ActualText)
        engine
            .process_operator(BMC, &[Object::Name(b"P".to_vec())])
            .unwrap();

        // Show "A" — should appear normally
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"A".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        // Inner BDC with ActualText
        let props = lopdf::Dictionary::from_iter(vec![(
            b"ActualText".to_vec(),
            Object::String(b"XY".to_vec(), lopdf::StringFormat::Literal),
        )]);
        engine
            .process_operator(
                BDC,
                &[
                    Object::Name(b"Span".to_vec()),
                    Object::Dictionary(props),
                ],
            )
            .unwrap();

        // Show 3 glyphs — only first should produce TextPosition with "XY"
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"\x01\x02\x03".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap(); // End inner BDC

        // Show "B" — back to normal (outer BMC has no ActualText)
        engine
            .process_operator(
                TJ_LOWER,
                &[Object::String(
                    b"B".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            )
            .unwrap();

        engine.process_operator(EMC, &[]).unwrap(); // End outer BMC

        // Expect: "A", "XY", "B"
        assert_eq!(engine.text_positions().len(), 3);
        assert_eq!(engine.text_positions()[0].unicode, "A");
        assert_eq!(engine.text_positions()[1].unicode, "XY");
        assert_eq!(engine.text_positions()[2].unicode, "B");
    }

    #[test]
    fn test_emc_without_bmc_is_safe() {
        // EMC without matching BMC should not panic
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);
        engine.process_operator(EMC, &[]).unwrap();
        // Should just be a no-op
    }

    #[test]
    fn test_actual_text_with_no_glyphs() {
        // ActualText with no glyphs inside should not produce TextPositions
        let doc = Arc::new(Document::with_version("1.5"));
        let mut engine = StreamEngine::new(doc);

        engine.process_operator(BT, &[]).unwrap();
        engine
            .process_operator(
                TF,
                &[Object::Name(b"F1".to_vec()), Object::Integer(12)],
            )
            .unwrap();

        let props = lopdf::Dictionary::from_iter(vec![(
            b"ActualText".to_vec(),
            Object::String(b"phantom".to_vec(), lopdf::StringFormat::Literal),
        )]);
        engine
            .process_operator(
                BDC,
                &[
                    Object::Name(b"Span".to_vec()),
                    Object::Dictionary(props),
                ],
            )
            .unwrap();

        // No text operators inside BDC/EMC
        engine.process_operator(EMC, &[]).unwrap();

        // No TextPositions should be produced
        assert!(engine.text_positions().is_empty());
    }
}
