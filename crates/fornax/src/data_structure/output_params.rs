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
///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible calls dcraw_process(), dcraw_ppm_tiff_writer(),
/// and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
pub struct OutputParams {
    /// 4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to calculate the white balance.
    /// X and Y are coordinates of the left-top rectangle corner; w and h are the rectangle's width and height, respectively.
    greybox: (usize,),
    demosaic_algorithm: DemosaicAlgorithm,
    half_size: bool,
    four_color_rgb: bool,
    dcb_iterations: usize,
    dcb_enhance: bool,
    fbdd_noise_reduction: FbddNoiseReductionMode,
    noise_thr: f64,
    median_filter_passes: usize,
    use_camera_wb: bool,
    use_auto_wb: bool,
    user_wb: (f64, f64, f64, f64),
    output_color: ColorSpace,
    output_bps: OutputBits,
    user_flip: UserFlip,
    user_black: u16,
    no_auto_bright: bool,
    auto_bright_thr: f64,
    adjust_maximum_thr: f64,
    bright: f64,
    highlight_mode: HighlightMode,
    exp_shift: f64,
    exp_preserve_highlights: f64,
    no_auto_scale: bool,
    gamma: (f64, f64),
    chromatic_aberration: (f64, f64),
    bad_pixels_path: String,
}
