//! 3x3 affine transformation matrix for PDF coordinate transformations.
//!
//! Ported from org.apache.pdfbox.util.Matrix.
//!
//! Layout (row-major):
//! ```text
//! | a   b   0 |     | [0] [1] [2] |
//! | c   d   0 |  =  | [3] [4] [5] |
//! | tx  ty  1 |     | [6] [7] [8] |
//! ```

/// A 3x3 affine transformation matrix stored as a flat 9-element array (row-major).
#[derive(Clone, Debug)]
pub struct Matrix {
    /// Row-major storage: [a, b, 0, c, d, 0, tx, ty, 1]
    single: [f32; 9],
}

impl Matrix {
    /// Identity matrix.
    pub const IDENTITY: Matrix = Matrix {
        single: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    /// Create a new identity matrix.
    pub fn new() -> Self {
        Self::IDENTITY.clone()
    }

    /// Create a matrix from 6 values [a, b, c, d, tx, ty].
    /// This matches the PDF operator format (cm, Tm, etc.).
    pub fn from_values(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) -> Self {
        Self {
            single: [a, b, 0.0, c, d, 0.0, tx, ty, 1.0],
        }
    }

    /// Create from a 6-element array [a, b, c, d, tx, ty].
    pub fn from_array(arr: &[f32]) -> Self {
        assert!(arr.len() >= 6, "Matrix::from_array requires at least 6 elements");
        Self::from_values(arr[0], arr[1], arr[2], arr[3], arr[4], arr[5])
    }

    /// Get element at (row, col). Both 0-indexed.
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.single[row * 3 + col]
    }

    /// Set element at (row, col).
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        self.single[row * 3 + col] = value;
    }

    // Named getters matching PDFBox API

    #[inline]
    pub fn scale_x(&self) -> f32 {
        self.single[0]
    }

    #[inline]
    pub fn shear_y(&self) -> f32 {
        self.single[1]
    }

    #[inline]
    pub fn shear_x(&self) -> f32 {
        self.single[3]
    }

    #[inline]
    pub fn scale_y(&self) -> f32 {
        self.single[4]
    }

    #[inline]
    pub fn translate_x(&self) -> f32 {
        self.single[6]
    }

    #[inline]
    pub fn translate_y(&self) -> f32 {
        self.single[7]
    }

    /// Multiply this matrix by another: result = self × other.
    pub fn multiply(&self, other: &Matrix) -> Matrix {
        let a = &self.single;
        let b = &other.single;
        let mut result = [0.0f32; 9];
        for row in 0..3 {
            for col in 0..3 {
                result[row * 3 + col] = a[row * 3] * b[col]
                    + a[row * 3 + 1] * b[3 + col]
                    + a[row * 3 + 2] * b[6 + col];
            }
        }
        Matrix { single: result }
    }

    /// Pre-concatenate: self = other × self.
    /// This matches PDFBox's concatenate() semantics.
    pub fn concatenate(&mut self, other: &Matrix) {
        *self = other.multiply(self);
    }

    /// Translate in place.
    pub fn translate(&mut self, tx: f32, ty: f32) {
        self.single[6] += tx * self.single[0] + ty * self.single[3];
        self.single[7] += tx * self.single[1] + ty * self.single[4];
    }

    /// Scale in place.
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.single[0] *= sx;
        self.single[1] *= sx;
        self.single[3] *= sy;
        self.single[4] *= sy;
    }

    /// Transform a point (x, y) and return (tx, ty).
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        let tx = x * self.single[0] + y * self.single[3] + self.single[6];
        let ty = x * self.single[1] + y * self.single[4] + self.single[7];
        (tx, ty)
    }

    /// Get the X scaling factor (magnitude of the first column vector).
    pub fn scaling_factor_x(&self) -> f32 {
        let a = self.single[0];
        let b = self.single[1];
        (a * a + b * b).sqrt()
    }

    /// Get the Y scaling factor (magnitude of the second column vector).
    pub fn scaling_factor_y(&self) -> f32 {
        let c = self.single[3];
        let d = self.single[4];
        (c * c + d * d).sqrt()
    }

    /// Create a scale matrix.
    pub fn scale_instance(sx: f32, sy: f32) -> Self {
        Self::from_values(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Create a translation matrix.
    pub fn translate_instance(tx: f32, ty: f32) -> Self {
        Self::from_values(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// Create a rotation matrix (angle in radians).
    pub fn rotate_instance(theta: f32) -> Self {
        let cos = theta.cos();
        let sin = theta.sin();
        Self::from_values(cos, sin, -sin, cos, 0.0, 0.0)
    }

    /// Get all 9 values as a flat array.
    pub fn values(&self) -> &[f32; 9] {
        &self.single
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        self.single == other.single
    }
}

impl std::fmt::Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[[{:.4}, {:.4}, {:.4}], [{:.4}, {:.4}, {:.4}], [{:.4}, {:.4}, {:.4}]]",
            self.single[0], self.single[1], self.single[2],
            self.single[3], self.single[4], self.single[5],
            self.single[6], self.single[7], self.single[8],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let m = Matrix::new();
        assert_eq!(m.scale_x(), 1.0);
        assert_eq!(m.scale_y(), 1.0);
        assert_eq!(m.shear_x(), 0.0);
        assert_eq!(m.shear_y(), 0.0);
        assert_eq!(m.translate_x(), 0.0);
        assert_eq!(m.translate_y(), 0.0);
    }

    #[test]
    fn test_from_values() {
        let m = Matrix::from_values(2.0, 0.0, 0.0, 3.0, 10.0, 20.0);
        assert_eq!(m.scale_x(), 2.0);
        assert_eq!(m.scale_y(), 3.0);
        assert_eq!(m.translate_x(), 10.0);
        assert_eq!(m.translate_y(), 20.0);
    }

    #[test]
    fn test_multiply_identity() {
        let a = Matrix::from_values(2.0, 1.0, 3.0, 4.0, 5.0, 6.0);
        let identity = Matrix::new();
        let result = a.multiply(&identity);
        assert_eq!(result, a);
    }

    #[test]
    fn test_multiply() {
        // PDF uses row vectors: point × matrix.
        // To "scale first, then translate", compute point × scale × translate.
        // This means we call scale.multiply(&translate).
        let scale = Matrix::scale_instance(2.0, 1.0);
        let translate = Matrix::translate_instance(10.0, 0.0);
        let result = scale.multiply(&translate);
        let (px, py) = result.transform_point(5.0, 0.0);
        // (5 * 2) + 10 = 20
        assert!((px - 20.0).abs() < 0.001);
        assert!((py - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_point() {
        let m = Matrix::from_values(1.0, 0.0, 0.0, 1.0, 100.0, 200.0);
        let (x, y) = m.transform_point(10.0, 20.0);
        assert!((x - 110.0).abs() < 0.001);
        assert!((y - 220.0).abs() < 0.001);
    }

    #[test]
    fn test_scaling_factor() {
        let m = Matrix::from_values(3.0, 4.0, 0.0, 5.0, 0.0, 0.0);
        assert!((m.scaling_factor_x() - 5.0).abs() < 0.001); // sqrt(9+16) = 5
        assert!((m.scaling_factor_y() - 5.0).abs() < 0.001); // sqrt(0+25) = 5
    }

    #[test]
    fn test_concatenate() {
        let mut m = Matrix::translate_instance(10.0, 20.0);
        let scale = Matrix::scale_instance(2.0, 3.0);
        m.concatenate(&scale);
        // concatenate(scale) sets m = scale × translate
        // Point (1,1) × m = (1,1) × scale × translate
        // = (2, 3) × translate = (12, 23)
        let (x, y) = m.transform_point(1.0, 1.0);
        assert!((x - 12.0).abs() < 0.001);
        assert!((y - 23.0).abs() < 0.001);
    }

    #[test]
    fn test_translate_in_place() {
        let mut m = Matrix::new();
        m.translate(10.0, 20.0);
        assert!((m.translate_x() - 10.0).abs() < 0.001);
        assert!((m.translate_y() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_scale_in_place() {
        let mut m = Matrix::new();
        m.scale(2.0, 3.0);
        assert!((m.scale_x() - 2.0).abs() < 0.001);
        assert!((m.scale_y() - 3.0).abs() < 0.001);
    }
}
