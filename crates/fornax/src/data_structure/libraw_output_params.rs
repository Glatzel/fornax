use std::path::PathBuf;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
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
impl TryFrom<i32> for HighlightMode {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<Self> {
        match value {
            0 => Ok(HighlightMode::CLIP),
            1 => Ok(HighlightMode::IGNORE),
            2 => Ok(HighlightMode::BLEND),
            3 => Ok(HighlightMode::RECONSTRUCT3),
            4 => Ok(HighlightMode::RECONSTRUCT4),
            5 => Ok(HighlightMode::RECONSTRUCT5),
            6 => Ok(HighlightMode::RECONSTRUCT6),
            7 => Ok(HighlightMode::RECONSTRUCT7),
            8 => Ok(HighlightMode::RECONSTRUCT8),
            9 => Ok(HighlightMode::RECONSTRUCT9),
            v => miette::bail!("Unknow highlight mode: {v}"),
        }
    }
}
impl From<HighlightMode> for i32 {
    fn from(value: HighlightMode) -> Self {
        match value {
            HighlightMode::CLIP => 0,
            HighlightMode::IGNORE => 1,
            HighlightMode::BLEND => 2,
            HighlightMode::RECONSTRUCT3 => 3,
            HighlightMode::RECONSTRUCT4 => 4,
            HighlightMode::RECONSTRUCT5 => 5,
            HighlightMode::RECONSTRUCT6 => 6,
            HighlightMode::RECONSTRUCT7 => 7,
            HighlightMode::RECONSTRUCT8 => 8,
            HighlightMode::RECONSTRUCT9 => 9,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UseCameraMatrix {
    NotUse = 0,
    EmbeddedProfile = 1,
    EmbeddedData = 3,
}
impl TryFrom<i32> for UseCameraMatrix {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<UseCameraMatrix> {
        match value {
            0 => Ok(UseCameraMatrix::NotUse),
            1 => Ok(UseCameraMatrix::EmbeddedProfile),
            3 => Ok(UseCameraMatrix::EmbeddedData),
            v => miette::bail!("Unknow UseCameraMatrix: {v}"),
        }
    }
}
impl From<UseCameraMatrix> for i32 {
    fn from(value: UseCameraMatrix) -> Self {
        match value {
            UseCameraMatrix::NotUse => 0,
            UseCameraMatrix::EmbeddedProfile => 1,
            UseCameraMatrix::EmbeddedData => 3,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
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
impl TryFrom<i32> for OutputColor {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<OutputColor> {
        match value {
            0 => Ok(OutputColor::RAW),
            1 => Ok(OutputColor::SRGB),
            2 => Ok(OutputColor::ADOBE),
            3 => Ok(OutputColor::WIDE),
            4 => Ok(OutputColor::PROPHOTO),
            5 => Ok(OutputColor::XYZ),
            6 => Ok(OutputColor::ACES),
            7 => Ok(OutputColor::P3D65),
            8 => Ok(OutputColor::REC2020),
            v => miette::bail!("Unknow `OutputColor`: {v}"),
        }
    }
}
impl From<OutputColor> for i32 {
    fn from(value: OutputColor) -> Self {
        match value {
            OutputColor::RAW => 0,
            OutputColor::SRGB => 1,
            OutputColor::ADOBE => 2,
            OutputColor::WIDE => 3,
            OutputColor::PROPHOTO => 4,
            OutputColor::XYZ => 5,
            OutputColor::ACES => 6,
            OutputColor::P3D65 => 7,
            OutputColor::REC2020 => 8,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum OutputBps {
    _8bit = 8,
    _16bit = 16,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum OutputTiff {
    None = -1,
    PPM = 0,
    TIFF = 1,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UserFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 5,
    CW90 = 6,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UseFujiRotate {
    UseRotate = -1,
    NotUse = 0,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum FbddNoiserd {
    OFF = 0,
    LIGHT = 1,
    FULL = 2,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UseDngSdk {
    NotUse = 0,
    Special = 1,
    All = 2,
}

///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible
/// calls dcraw_process(), dcraw_ppm_tiff_writer(), and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct LibrawOutputParams {
    /// 4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
    /// calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
    /// w and h are the rectangle's width and height, respectively.
    pub greybox: Option<[u32; 4]>,
    ///This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
    /// rectangle's top-left corner coordinates, remaining two values are width and height
    /// respectively. All coordinates are applied before any image rotation.
    pub cropbox: Option<[u32; 4]>,
    /// Correction of chromatic aberrations; the only specified values are
    ///
    /// - `aber[0]`, the red multiplier.
    /// - `aber[2]`, the blue multiplier.
    ///
    /// For some formats, it affects RAW data reading , since
    /// correction of aberrations changes the output size.
    ///
    /// The `aber[0]` will set `aber[0]`.
    /// The `aber[1]` will set `aber[2]`.
    pub aber: Option<[f64; 2]>,
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
    /// The `gamm[0]` will set `gamm[0]`.
    /// The `gamm[1]` will set `gamm[1]`.
    pub gamm: Option<[f64; 2]>,
    /// 4 multipliers (r,g,b,g) of the user's white balance.
    pub user_mul: Option<[f32; 4]>,
    /// Brightness (default 1.0).
    pub bright: Option<f32>,
    /// Parameter for noise reduction through wavelet denoising.
    pub threshold: Option<f32>,
    /// Outputs the image in 50% size. For some formats, it affects RAW data reading .
    pub half_size: Option<bool>,
    /// Switches on separate interpolations for two green components.
    pub four_color_rgb: Option<bool>,
    /// 0-9: Highlight mode (0=clip, 1=unclip, 2=blend, 3+=rebuild).
    pub highlight: Option<HighlightMode>,
    /// Use automatic white balance obtained after averaging over the entire image.
    pub use_auto_wb: Option<bool>,
    /// If possible, use the white balance from the camera.
    ///
    /// If camera-recorded WB is not available, dcraw_process() will fallback to:
    ///
    /// - Auto-WB if bit LIBRAW_PROCESSING_CAMERAWB_FALLBACK_TO_DAYLIGHT is not set in
    /// params.raw_processing_options (or for the rare specific case: no valid WB index was parsed
    /// from CRW file)
    /// - Daylight-WB if abovementioned bit is not set.
    pub use_camera_wb: Option<bool>,
    /// - 0: do not use embedded color profile
    /// - 1 (default): use embedded color profile (if present) for DNG files (always); for other
    ///   files only if use_camera_wb is set;
    /// - 3: use embedded color data (if present) regardless of white balance setting.
    pub use_camera_matrix: Option<UseCameraMatrix>,
    ///[0-8] Output colorspace (raw, sRGB, Adobe, Wide, ProPhoto, XYZ, ACES, DCI-P3, Rec. 2020).
    pub output_color: Option<OutputColor>,
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
    pub use_fuji_rotate: UseFujiRotate,
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
impl LibrawOutputParams {
    pub(crate) fn set_output_params(&self, imgdata: &mut crate::sys::libraw_data_t) {
        if let Some(graybox) = self.greybox {
            imgdata.params.greybox = graybox;
        }
        if let Some(cropbox) = self.cropbox {
            imgdata.params.cropbox = cropbox;
        }
        if let Some(aber) = self.aber {
            imgdata.params.aber[0] = aber[0];
            imgdata.params.aber[2] = aber[1];
        }
        if let Some(gamm) = self.gamm {
            imgdata.params.gamm[0] = gamm[0];
            imgdata.params.gamm[2] = gamm[1];
        }
        if let Some(user_mul) = self.user_mul {
            imgdata.params.user_mul = user_mul;
        }
        if let Some(bright) = self.bright {
            imgdata.params.bright = bright;
        }
        if let Some(threshold) = self.threshold {
            imgdata.params.threshold = threshold;
        }
        if let Some(half_size) = self.half_size {
            imgdata.params.half_size = half_size as i32;
        }
        if let Some(four_color_rgb) = self.four_color_rgb {
            imgdata.params.four_color_rgb = four_color_rgb as i32;
        }
        if let Some(highlight) = self.highlight {
            imgdata.params.highlight = i32::from(highlight);
        }
        if let Some(use_auto_wb) = self.use_auto_wb {
            imgdata.params.use_auto_wb = use_auto_wb as i32;
        }
        if let Some(use_camera_wb) = self.use_camera_wb {
            imgdata.params.use_camera_wb = use_camera_wb as i32;
        }
        if let Some(use_camera_matrix) = self.use_camera_matrix {
            imgdata.params.use_camera_matrix = i32::from(use_camera_matrix);
        }
        if let Some(output_color) = self.output_color {
            imgdata.params.output_color = i32::from(output_color);
        }
    }
}
