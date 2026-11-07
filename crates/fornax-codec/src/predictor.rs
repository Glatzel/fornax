/// Reversible per-chunk predictor applied before sample reconstruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Predictor {
    /// Samples are not predicted.
    None,
    /// Horizontal integer differencing using the previous sample.
    HorizontalInteger,
    /// TIFF floating-point byte shuffling and differencing.
    FloatingPoint {
        /// Distance between differenced bytes.
        byte_stride: usize,
    },
}
