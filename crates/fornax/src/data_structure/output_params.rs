///Identifiers for demosaic algorithms.
pub enum DemosaicAlgorithm {
    LINEAR = 0,
    VNG = 1,
    PPG = 2,
    AHD = 3,
    DCB = 4,
    // comment GPL algorithm
    // MODIFIED_AHD = 5
    // AFD = 6
    // VCD = 7
    // VCD_MODIFIED_AHD = 8
    // LMMSE = 9
    // AMAZE = 10
    DHT = 11,
    AAHD = 12,
}
/// Color spaces.
pub enum ColorSpace {
    RAW = 0,
    SRGB = 1,
    ADOBE = 2,
    WIDE = 3,
    PROPHOTO = 4,
    XYZ = 5,
    ACES = 6,
    P3D65 = 7,
    REC2020 = 8,
}
/// Highlight modes.
pub enum HighlightMode {
    CLIP = 0,
    IGNORE = 1,
    BLEND = 2,
    RECONSTRUCT3 = 3,
    RECONSTRUCT4 = 4,
    RECONSTRUCT5 = 5, //default
    RECONSTRUCT6 = 6,
    RECONSTRUCT7 = 7,
    RECONSTRUCT8 = 8,
    RECONSTRUCT9 = 9,
}

/// FBDD noise reduction modes.
pub enum FbddNoiseReductionMode {
    OFF = 0,
    LIGHT = 1,
    FULL = 2,
}
pub enum OutputBits {
    _8bit = 8,
    _16bit = 16,
}
pub enum UserFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 5,
    CW90 = 6,
}
///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible
/// calls dcraw_process(), dcraw_ppm_tiff_writer(), and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
pub struct OutputParams {
    /// 4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
    /// calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
    /// w and h are the rectangle's width and height, respectively.
    greybox: (usize, usize, usize, usize),
    ///This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
    /// rectangle's top-left corner coordinates, remaining two values are width and height
    /// respectively. All coordinates are applied before any image rotation.
    cropbox: (usize, usize, usize, usize),
    /// Correction of chromatic aberrations; the only specified values are
    /// `aber[0]`, the red multiplier
    /// `aber[2]`, the blue multiplier. For some formats, it affects RAW data reading , since
    /// correction of aberrations changes the output size.
    aber: (f64, f64, f64, f64),
    /// Sets user gamma-curve. Library user should set first two fields of gamm array:
    ///gamm[0] - inverted gamma value)
    ///gamm[1] - slope for linear part (so called toe slope). Set to zero for simple power curve.
    ///Remaining 4 values are filled automatically.
    ///By default settings for rec. BT.709 are used: power 2.222 (i.e. gamm[0]=1/2.222) and slope
    /// 4.5. For sRGB curve use gamm[0]=1/2.4 and gamm[1]=12.92, for linear curve set
    /// gamm[0]/gamm[1] to 1.0.
    gamm: (f64, f64, f64, f64, f64, f64),
    ///4 multipliers (r,g,b,g) of the user's white balance.
    user_mul: (f32, f32, f32, f32),
    ///Brightness (default 1.0).
    bright: f32,
    /// Parameter for noise reduction through wavelet denoising.
    threshold: f32,
    /// Outputs the image in 50% size. For some formats, it affects RAW data reading .
    half_size: bool,
    four_color_rgb: bool,
}
