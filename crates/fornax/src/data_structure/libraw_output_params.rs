use std::path::PathBuf;

pub enum UserQual {
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

pub enum OutputColor {
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

pub enum FbddNoiserd {
    OFF = 0,
    LIGHT = 1,
    FULL = 2,
}
pub enum OutputBps {
    _8bit = 8,
    _16bit = 16,
}
pub enum UserFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 5,
    CW90 = 6,
}
pub enum UseCameraMatrix {
    NotUse = 0,
    EmbeddedProfile = 1,
    EmbeddedData = 3,
}
pub enum OutputTiff {
    None = -1,
    PPM = 0,
    TIFF = 1,
}
pub enum UseDngSdk {
    NotUse = 0,
    Special = 1,
    All = 2,
}
pub enum UseFujiRotate {
    UseRotate = -1,
    NotUse = 0,
}
///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible
/// calls dcraw_process(), dcraw_ppm_tiff_writer(), and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
pub struct LibrawOutputParams {
    /// 4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
    /// calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
    /// w and h are the rectangle's width and height, respectively.
    pub greybox: (u32, u32, u32, u32),
    ///This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
    /// rectangle's top-left corner coordinates, remaining two values are width and height
    /// respectively. All coordinates are applied before any image rotation.
    pub cropbox: (u32, u32, u32, u32),
    /// Correction of chromatic aberrations; the only specified values are
    ///
    /// - `aber[0]`, the red multiplier.
    /// - `aber[2]`, the blue multiplier.
    ///
    /// For some formats, it affects RAW data reading , since
    /// correction of aberrations changes the output size.
    ///
    /// The `aber.0` will set `aber[0]`.
    /// The `aber.2` will set `aber[0]`.
    pub aber: (f64, f64),
    /// Sets user gamma-curve. Library user should set first two fields of gamm array:
    /// - `gamm[0]` - inverted gamma value)
    /// - `gamm[1]` - slope for linear part (so called toe slope). Set to zero for simple power
    ///   curve.
    ///
    /// Remaining 4 values are filled automatically.
    ///
    /// By default settings for rec. BT.709 are used: power 2.222 (i.e. `gamm[0]=1/2.222`) and
    /// slope 4.5. For sRGB curve use `gamm[0]=1/2.4` and `gamm[1]=12.92`, for linear curve set
    /// gamm[0]/gamm[1] to 1.0.
    ///
    /// The `gamm.0` will set `gamm[0]`.
    /// The `gamm.1` will set `gamm[1]`.
    pub gamm: (f64, f64),
    /// 4 multipliers (r,g,b,g) of the user's white balance.
    pub user_mul: (f32, f32, f32, f32),
    /// Brightness (default 1.0).
    pub bright: f32,
    /// Parameter for noise reduction through wavelet denoising.
    pub threshold: f32,
    /// Outputs the image in 50% size. For some formats, it affects RAW data reading .
    pub half_size: bool,
    /// Switches on separate interpolations for two green components.
    pub four_color_rgb: bool,
    /// 0-9: Highlight mode (0=clip, 1=unclip, 2=blend, 3+=rebuild).
    pub highlight: HighlightMode,
    /// Use automatic white balance obtained after averaging over the entire image.
    pub use_auto_wb: bool,
    /// If possible, use the white balance from the camera.
    ///
    /// If camera-recorded WB is not available, dcraw_process() will fallback to:
    ///
    /// - Auto-WB if bit LIBRAW_PROCESSING_CAMERAWB_FALLBACK_TO_DAYLIGHT is not set in
    /// params.raw_processing_options (or for the rare specific case: no valid WB index was parsed
    /// from CRW file)
    /// - Daylight-WB if abovementioned bit is not set.
    pub use_camera_wb: bool,
    /// - 0: do not use embedded color profile
    /// - 1 (default): use embedded color profile (if present) for DNG files (always); for other
    ///   files only if use_camera_wb is set;
    /// - 3: use embedded color data (if present) regardless of white balance setting.
    pub use_camera_matrix: UseCameraMatrix,
    ///[0-8] Output colorspace (raw, sRGB, Adobe, Wide, ProPhoto, XYZ, ACES, DCI-P3, Rec. 2020).
    pub output_color: OutputColor,
    ///Path to output profile ICC file (used only if LibRaw compiled with LCMS support)
    pub output_profile: PathBuf,
    ///Path to input (camera) profile ICC file (or 'embed' for embedded profile). Used only if
    /// LCMS support compiled in.
    pub camera_profile: PathBuf,
    ///Path to file with bad pixels map (in dcraw format: "column row
    /// date-of-pixel-death-in-UNIX-format", one pixel per row).
    pub bad_pixelse: PathBuf,
    /// Path to dark frame file (in 16-bit PGM format)
    pub dark_frame: PathBuf,
    ///8 bit (default)/16 bit (key -4).
    pub output_bps: OutputBps,
    /// 0/1: output PPM/TIFF.
    pub output_tiff: OutputTiff,
    /// Bitfield that allows to set output file options:
    /// LIBRAW_OUTPUT_FLAGS_PPMMETA - write additional metadata into PPM/PGM output files
    pub output_flags: i32,
    /// [0-7] Flip image (0=none, 3=180, 5=90CCW, 6=90CW). Default -1, which means taking the
    /// corresponding value from RAW.
    pub user_flip: UserFlip,
    /// 0-10: interpolation quality:
    /// 0 - linear interpolation
    /// 1 - VNG interpolation
    /// 2 - PPG interpolation
    /// 3 - AHD interpolation
    /// 4 - DCB interpolation
    /// 11 - DHT interpolation
    /// 12 - Modified AHD interpolation (by Anton Petrusevich)
    pub user_qual: UserQual,
    /// User black level.
    pub user_black: i32,
    /// Per-channel corrections to user_black.
    pub user_cblack: (i32, i32, i32, i32),
    ///Saturation adjustment.
    pub user_sat: i32,
    ///Number of median filter passes.
    pub med_passes: i32,
    ///Don't use automatic increase of brightness by histogram.
    pub no_auto_bright: i32,
    ///  Portion of clipped pixels when auto brightness increase is used. Default value is 0.01
    /// (1%) for dcraw compatibility. Recommended value for modern low-noise multimegapixel cameras
    /// depends on shooting style. Values in 0.001-0.00003 range looks reasonable.
    pub auto_bright_thr: f32,
    ///This parameters controls auto-adjusting of maximum value based on channel_maximum[] data,
    /// calculated from real frame data. If calculated maximum is greater than
    /// adjust_maximum_thr*maximum, than maximum is set to calculated_maximum.
    ///
    /// Default: 0.75. If you set this value above 0.99999, than default value will be used. If you
    /// set this value below 0.00001, than no maximum adjustment will be performed.
    ///
    /// Adjusting maximum should not damage any picture (esp. if you use default value) and is very
    /// useful for correcting channel overflow problems (magenta clouds on landscape shots,
    /// green-blue highlights for indoor shots).
    pub adjust_maximum_thr: f32,
    /// Default -1 (use), 0 - don't use rotation for cameras on a Fuji sensor.
    pub use_fuji_rotate: bool,
    ///Turns on fixing of green channels disbalance. dcraw keys: none
    ///
    /// Default: 0 (not use), 1 - turns on this postprocessing stage. green_matching requires
    /// additional memory for image data.
    pub green_matching: bool,
    ///Number of DCB correction passes. Default is -1 (no correction). Useful only for DCB
    /// interpolation.
    pub dcb_iterations: i32,
    /// nonzero: DCB interpolation with enhance interpolated colors.
    pub dcb_enhance_fl: i32,
    ///Controls FBDD noise reduction before demosaic.
    /// - 0 - do not use FBDD noise reduction
    /// - 1 - light FBDD reduction
    /// - 2 (and more) - full FBDD reduction
    pub fbdd_noiserd: FbddNoiserd,
    ///Exposure correction before demosaic.
    ///exp_correc: positive value turns the feature on (default: off).
    pub exp_correc: i32,
    ///Exposure correction before demosaic.
    ///exp_shift: exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0
    /// (3-stop lighter). Default: 1.0 (no exposure shift).
    pub exp_shift: f32,
    ///Exposure correction before demosaic.
    ///exp_preser: preserve highlights when lighten the image. Usable range from 0.0 (no
    /// preservation) to 1.0 (full preservation). 0.0 is the default value.
    pub exp_preser: f32,
    ///Turns on using RawSpeed library for data unpacking (only if RawSpeed support compiled in).
    pub use_rawspeed: bool,
    /// Turns on using Adobe DNG SDK (if compiled with it and dng host is set:
    /// - 0 - do not use
    /// - 1 - use for speciality formats (Float, Linear DNG, deflate compression, 8 bit)
    /// - 2 - use for all DNG files
    pub use_dng_sdk: UseDngSdk,
    ///Disables pixel values scaling (call to LibRaw::scale_colors()) in LibRaw::dcraw_process().
    ///
    ///This is special use value because white balance is performed in scale_colors(), so skipping
    /// it will result in non-balanced image.
    ///
    ///This setting is targeted to use with no_interpolation, or with own interpolation callback
    /// call.
    pub no_auto_scale: bool,
    ///Disables call to demosaic code in LibRaw::dcraw_process()
    pub no_interpolation: bool,
}
