//! PDF graphics state — the state stack managed by q/Q operators.
//!
//! Ported from org.apache.pdfbox.pdmodel.graphics.state.PDGraphicsState.
//! Only text-extraction-relevant fields are included.

use super::matrix::Matrix;
use super::text_state::TextState;

/// Graphics state for text extraction.
///
/// This is a simplified version that only tracks state relevant to text extraction,
/// omitting color, clipping, line dash, and other rendering-only state.
#[derive(Clone, Debug)]
pub struct GraphicsState {
    /// Current transformation matrix (CTM).
    pub ctm: Matrix,

    /// Text state parameters.
    pub text_state: TextState,

    /// Text matrix (set by BT, updated by text positioning operators).
    pub text_matrix: Option<Matrix>,

    /// Text line matrix (updated by T*, ', " operators).
    pub text_line_matrix: Option<Matrix>,

    /// Line width (used for some rendering decisions).
    pub line_width: f32,

    /// Stroking alpha.
    pub alpha_constant: f64,

    /// Non-stroking alpha.
    pub non_stroking_alpha_constant: f64,
}

impl GraphicsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize text matrices at the start of a text object (BT).
    pub fn begin_text(&mut self) {
        self.text_matrix = Some(Matrix::new());
        self.text_line_matrix = Some(Matrix::new());
    }

    /// Clear text matrices at the end of a text object (ET).
    pub fn end_text(&mut self) {
        self.text_matrix = None;
        self.text_line_matrix = None;
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::new(),
            text_state: TextState::new(),
            text_matrix: None,
            text_line_matrix: None,
            line_width: 1.0,
            alpha_constant: 1.0,
            non_stroking_alpha_constant: 1.0,
        }
    }
}

/// A stack of graphics states, managed by q (save) and Q (restore) operators.
#[derive(Debug)]
pub struct GraphicsStateStack {
    stack: Vec<GraphicsState>,
    current: GraphicsState,
}

impl GraphicsStateStack {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: GraphicsState::new(),
        }
    }

    /// Get the current graphics state.
    pub fn current(&self) -> &GraphicsState {
        &self.current
    }

    /// Get a mutable reference to the current graphics state.
    pub fn current_mut(&mut self) -> &mut GraphicsState {
        &mut self.current
    }

    /// Save the current state (q operator).
    pub fn save(&mut self) {
        self.stack.push(self.current.clone());
    }

    /// Restore the previous state (Q operator).
    /// If the stack is empty, the current state is reset to default.
    pub fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.current = state;
        } else {
            log::warn!("GraphicsStateStack::restore called on empty stack");
        }
    }

    /// Depth of the stack (not counting current state).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl Default for GraphicsStateStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let gs = GraphicsState::new();
        assert_eq!(gs.ctm, Matrix::IDENTITY);
        assert!(gs.text_matrix.is_none());
        assert!(gs.text_line_matrix.is_none());
        assert_eq!(gs.line_width, 1.0);
    }

    #[test]
    fn test_begin_end_text() {
        let mut gs = GraphicsState::new();
        gs.begin_text();
        assert!(gs.text_matrix.is_some());
        assert!(gs.text_line_matrix.is_some());

        gs.end_text();
        assert!(gs.text_matrix.is_none());
        assert!(gs.text_line_matrix.is_none());
    }

    #[test]
    fn test_state_stack_save_restore() {
        let mut stack = GraphicsStateStack::new();
        assert_eq!(stack.depth(), 0);

        // Modify current state
        stack.current_mut().line_width = 5.0;
        stack.current_mut().text_state.font_size = 12.0;

        // Save
        stack.save();
        assert_eq!(stack.depth(), 1);

        // Modify again
        stack.current_mut().line_width = 10.0;
        stack.current_mut().text_state.font_size = 24.0;
        assert_eq!(stack.current().line_width, 10.0);
        assert_eq!(stack.current().text_state.font_size, 24.0);

        // Restore
        stack.restore();
        assert_eq!(stack.depth(), 0);
        assert_eq!(stack.current().line_width, 5.0);
        assert_eq!(stack.current().text_state.font_size, 12.0);
    }

    #[test]
    fn test_state_stack_nested() {
        let mut stack = GraphicsStateStack::new();

        stack.current_mut().line_width = 1.0;
        stack.save();

        stack.current_mut().line_width = 2.0;
        stack.save();

        stack.current_mut().line_width = 3.0;
        assert_eq!(stack.current().line_width, 3.0);
        assert_eq!(stack.depth(), 2);

        stack.restore();
        assert_eq!(stack.current().line_width, 2.0);

        stack.restore();
        assert_eq!(stack.current().line_width, 1.0);
    }

    #[test]
    fn test_restore_empty_stack() {
        let mut stack = GraphicsStateStack::new();
        // Should not panic, just warn
        stack.restore();
        assert_eq!(stack.depth(), 0);
    }
}
