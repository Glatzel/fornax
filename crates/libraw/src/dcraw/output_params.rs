use std::ffi::CString;
use std::path::PathBuf;

use miette::IntoDiagnostic;

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]

pub enum DCRawHighlightMode {
    Clip = 0,
    Ignore = 1,
    Blend = 2,
    Reconstruct3 = 3,
    Reconstruct4 = 4,
    Reconstruct5 = 5, //default
    Reconstruct6 = 6,
    Reconstruct7 = 7,
    Reconstruct8 = 8,
    Reconstruct9 = 9,
}
impl TryFrom<i32> for DCRawHighlightMode {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<Self> {
        match value {
            0 => Ok(DCRawHighlightMode::Clip),
            1 => Ok(DCRawHighlightMode::Ignore),
            2 => Ok(DCRawHighlightMode::Blend),
            3 => Ok(DCRawHighlightMode::Reconstruct3),
            4 => Ok(DCRawHighlightMode::Reconstruct4),
            5 => Ok(DCRawHighlightMode::Reconstruct5),
            6 => Ok(DCRawHighlightMode::Reconstruct6),
            7 => Ok(DCRawHighlightMode::Reconstruct7),
            8 => Ok(DCRawHighlightMode::Reconstruct8),
            9 => Ok(DCRawHighlightMode::Reconstruct9),
            v => miette::bail!("Unknow highlight mode: {v}"),
        }
    }
}
impl From<DCRawHighlightMode> for i32 {
    fn from(value: DCRawHighlightMode) -> Self {
        match value {
            DCRawHighlightMode::Clip => 0,
            DCRawHighlightMode::Ignore => 1,
            DCRawHighlightMode::Blend => 2,
            DCRawHighlightMode::Reconstruct3 => 3,
            DCRawHighlightMode::Reconstruct4 => 4,
            DCRawHighlightMode::Reconstruct5 => 5,
            DCRawHighlightMode::Reconstruct6 => 6,
            DCRawHighlightMode::Reconstruct7 => 7,
            DCRawHighlightMode::Reconstruct8 => 8,
            DCRawHighlightMode::Reconstruct9 => 9,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum DCRawUseCameraMatrix {
    NotUse = 0,
    EmbeddedProfile = 1,
    EmbeddedData = 3,
}
impl TryFrom<i32> for DCRawUseCameraMatrix {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<DCRawUseCameraMatrix> {
        match value {
            0 => Ok(DCRawUseCameraMatrix::NotUse),
            1 => Ok(DCRawUseCameraMatrix::EmbeddedProfile),
            3 => Ok(DCRawUseCameraMatrix::EmbeddedData),
            v => miette::bail!("Unknow UseCameraMatrix: {v}"),
        }
    }
}
impl From<DCRawUseCameraMatrix> for i32 {
    fn from(value: DCRawUseCameraMatrix) -> Self {
        match value {
            DCRawUseCameraMatrix::NotUse => 0,
            DCRawUseCameraMatrix::EmbeddedProfile => 1,
            DCRawUseCameraMatrix::EmbeddedData => 3,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawOutputColor {
    Raw = 0,
    SRgb = 1,
    Adobe = 2,
    Wide = 3,
    ProPhoto = 4,
    XYZ = 5,
    ACES = 6,
    DciP3 = 7,
    Rec2020 = 8,
}
impl TryFrom<i32> for DCRawOutputColor {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<DCRawOutputColor> {
        match value {
            0 => Ok(DCRawOutputColor::Raw),
            1 => Ok(DCRawOutputColor::SRgb),
            2 => Ok(DCRawOutputColor::Adobe),
            3 => Ok(DCRawOutputColor::Wide),
            4 => Ok(DCRawOutputColor::ProPhoto),
            5 => Ok(DCRawOutputColor::XYZ),
            6 => Ok(DCRawOutputColor::ACES),
            7 => Ok(DCRawOutputColor::DciP3),
            8 => Ok(DCRawOutputColor::Rec2020),
            v => miette::bail!("Unknow `OutputColor`: {v}"),
        }
    }
}
impl From<DCRawOutputColor> for i32 {
    fn from(value: DCRawOutputColor) -> Self {
        match value {
            DCRawOutputColor::Raw => 0,
            DCRawOutputColor::SRgb => 1,
            DCRawOutputColor::Adobe => 2,
            DCRawOutputColor::Wide => 3,
            DCRawOutputColor::ProPhoto => 4,
            DCRawOutputColor::XYZ => 5,
            DCRawOutputColor::ACES => 6,
            DCRawOutputColor::DciP3 => 7,
            DCRawOutputColor::Rec2020 => 8,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawOutputBps {
    _8bit = 8,
    _16bit = 16,
}
impl TryFrom<i32> for DCRawOutputBps {
    type Error = miette::Report;

    fn try_from(value: i32) -> miette::Result<DCRawOutputBps> {
        match value {
            8 => Ok(DCRawOutputBps::_8bit),
            16 => Ok(DCRawOutputBps::_16bit),
            v => miette::bail!("Unknow `OutputBps`: {v}"),
        }
    }
}
impl From<DCRawOutputBps> for i32 {
    fn from(value: DCRawOutputBps) -> Self {
        match value {
            DCRawOutputBps::_8bit => 8,
            DCRawOutputBps::_16bit => 16,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawOutputTiff {
    None = -1,
    Ppm = 0,
    Tiff = 1,
}
impl TryFrom<i32> for DCRawOutputTiff {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::None),
            0 => Ok(Self::Ppm),
            1 => Ok(Self::Tiff),
            v => miette::bail!("Unknow `OutputTiff`: {v}"),
        }
    }
}
impl From<DCRawOutputTiff> for i32 {
    fn from(value: DCRawOutputTiff) -> Self {
        match value {
            DCRawOutputTiff::None => -1,
            DCRawOutputTiff::Ppm => 0,
            DCRawOutputTiff::Tiff => 1,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawUserFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 5,
    CW90 = 6,
}
impl TryFrom<i32> for DCRawUserFlip {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            3 => Ok(Self::Rotate180),
            5 => Ok(Self::CCW90),
            6 => Ok(Self::CW90),
            v => miette::bail!("Unknow `UserFlip`: {v}"),
        }
    }
}
impl From<DCRawUserFlip> for i32 {
    fn from(value: DCRawUserFlip) -> Self {
        match value {
            DCRawUserFlip::None => 0,
            DCRawUserFlip::Rotate180 => 3,
            DCRawUserFlip::CCW90 => 5,
            DCRawUserFlip::CW90 => 6,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawUserQual {
    Linear = 0,
    VNG = 1,
    PPG = 2,
    AHD = 3,
    DCB = 4,
    DHT = 11,
    ModifiedAHD = 12,
}
impl TryFrom<i32> for DCRawUserQual {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Linear),
            1 => Ok(Self::VNG),
            2 => Ok(Self::PPG),
            3 => Ok(Self::AHD),
            4 => Ok(Self::DCB),
            11 => Ok(Self::DHT),
            12 => Ok(Self::ModifiedAHD),
            v => miette::bail!("Unknow `UserQual`: {v}"),
        }
    }
}
impl From<DCRawUserQual> for i32 {
    fn from(value: DCRawUserQual) -> Self {
        match value {
            DCRawUserQual::Linear => 0,
            DCRawUserQual::VNG => 1,
            DCRawUserQual::PPG => 2,
            DCRawUserQual::AHD => 3,
            DCRawUserQual::DCB => 4,
            DCRawUserQual::DHT => 11,
            DCRawUserQual::ModifiedAHD => 12,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawUseFujiRotate {
    UseRotate = -1,
    NotUse = 0,
}
impl TryFrom<i32> for DCRawUseFujiRotate {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::UseRotate),
            3 => Ok(Self::NotUse),
            v => miette::bail!("Unknow `UseFujiRotate`: {v}"),
        }
    }
}
impl From<DCRawUseFujiRotate> for i32 {
    fn from(value: DCRawUseFujiRotate) -> Self {
        match value {
            DCRawUseFujiRotate::UseRotate => -1,
            DCRawUseFujiRotate::NotUse => 0,
        }
    }
}
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
#[derive(Debug, Copy, Clone)]
pub enum DCRawFbddNoiserd {
    Off = 0,
    Light = 1,
    Full = 2,
}
impl TryFrom<i32> for DCRawFbddNoiserd {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::Off),
            0 => Ok(Self::Light),
            1 => Ok(Self::Full),
            v => miette::bail!("Unknow `OutputTiff`: {v}"),
        }
    }
}
impl From<DCRawFbddNoiserd> for i32 {
    fn from(value: DCRawFbddNoiserd) -> Self {
        match value {
            DCRawFbddNoiserd::Off => 0,
            DCRawFbddNoiserd::Light => 1,
            DCRawFbddNoiserd::Full => 2,
        }
    }
}

///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible
/// calls dcraw_process(), dcraw_ppm_tiff_writer(), and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct DCRawParams {
    /// 4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
    /// calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
    /// w and h are the rectangle's width and height, respectively.
    pub greybox: Option<[u32; 4]>,
    /// This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
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
    ///
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
    /// `gamm[0]/gamm[1]` to 1.0.
    ///
    /// The `gamm[0]` will set `gamm[0]`.
    ///
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
    pub highlight: Option<DCRawHighlightMode>,
    /// Use automatic white balance obtained after averaging over the entire image.
    pub use_auto_wb: Option<bool>,
    /// If possible, use the white balance from the camera.
    ///
    /// If camera-recorded WB is not available, dcraw_process() will fallback to:
    ///
    /// - Auto-WB if bit LIBRAW_PROCESSING_CAMERAWB_FALLBACK_TO_DAYLIGHT is not set in
    ///   params.raw_processing_options (or for the rare specific case: no valid WB index was
    ///   parsed from CRW file).
    /// - Daylight-WB if abovementioned bit is not set.
    pub use_camera_wb: Option<bool>,
    /// - 0: do not use embedded color profile
    /// - 1 (default): use embedded color profile (if present) for DNG files (always); for other
    ///   files only if use_camera_wb is set;
    /// - 3: use embedded color data (if present) regardless of white balance setting.
    pub use_camera_matrix: Option<DCRawUseCameraMatrix>,
    ///[0-8] Output colorspace (raw, sRGB, Adobe, Wide, ProPhoto, XYZ, ACES, DCI-P3, Rec. 2020).
    pub output_color: Option<DCRawOutputColor>,
    ///Path to output profile ICC file (used only if LibRaw compiled with LCMS support)
    pub output_profile: Option<PathBuf>,
    ///Path to input (camera) profile ICC file (or 'embed' for embedded profile). Used only if
    /// LCMS support compiled in.
    pub camera_profile: Option<PathBuf>,
    ///Path to file with bad pixels map (in dcraw format: "column row
    /// date-of-pixel-death-in-UNIX-format", one pixel per row).
    pub bad_pixels: Option<PathBuf>,
    /// Path to dark frame file (in 16-bit PGM format).
    pub dark_frame: Option<PathBuf>,
    ///8 bit (default)/16 bit (key -4).
    pub output_bps: Option<DCRawOutputBps>,
    /// 0/1: output PPM/TIFF.
    pub output_tiff: Option<DCRawOutputTiff>,
    /// Bitfield that allows to set output file options:
    /// LIBRAW_OUTPUT_FLAGS_PPMMETA - write additional metadata into PPM/PGM output files
    // pub output_flags: i32,
    /// [0-7] Flip image (0=none, 3=180, 5=90CCW, 6=90CW). Default -1, which means taking the
    /// corresponding value from RAW.
    pub user_flip: Option<DCRawUserFlip>,
    /// 0-10: interpolation quality:
    /// 0 - linear interpolation
    /// 1 - VNG interpolation
    /// 2 - PPG interpolation
    /// 3 - AHD interpolation
    /// 4 - DCB interpolation
    /// 11 - DHT interpolation
    /// 12 - Modified AHD interpolation (by Anton Petrusevich)
    pub user_qual: Option<DCRawUserQual>,
    /// User black level.
    pub user_black: Option<i32>,
    /// Per-channel corrections to user_black.
    pub user_cblack: Option<[i32; 4]>,
    ///Saturation adjustment.
    pub user_sat: Option<i32>,
    ///Number of median filter passes.
    pub med_passes: Option<i32>,
    ///Don't use automatic increase of brightness by histogram.
    pub no_auto_bright: Option<bool>,
    ///  Portion of clipped pixels when auto brightness increase is used. Default value is 0.01
    /// (1%) for dcraw compatibility. Recommended value for modern low-noise multimegapixel cameras
    /// depends on shooting style. Values in 0.001-0.00003 range looks reasonable.
    pub auto_bright_thr: Option<f32>,
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
    pub adjust_maximum_thr: Option<f32>,
    /// Default -1 (use), 0 - don't use rotation for cameras on a Fuji sensor.
    pub use_fuji_rotate: Option<DCRawUseFujiRotate>,
    ///Turns on fixing of green channels disbalance. dcraw keys: none
    ///
    /// Default: 0 (not use), 1 - turns on this postprocessing stage. green_matching requires
    /// additional memory for image data.
    pub green_matching: Option<bool>,
    ///Number of DCB correction passes. Default is -1 (no correction). Useful only for DCB
    /// interpolation.
    pub dcb_iterations: Option<i32>,
    /// nonzero: DCB interpolation with enhance interpolated colors.
    pub dcb_enhance_fl: Option<i32>,
    ///Controls FBDD noise reduction before demosaic.
    /// - 0 - do not use FBDD noise reduction
    /// - 1 - light FBDD reduction
    /// - 2 (and more) - full FBDD reduction
    pub fbdd_noiserd: Option<DCRawFbddNoiserd>,
    ///Exposure correction before demosaic.
    ///
    ///exp_correc: positive value turns the feature on (default: off).
    pub exp_correc: Option<i32>,
    ///Exposure correction before demosaic.
    ///
    ///exp_shift: exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0
    /// (3-stop lighter). Default: 1.0 (no exposure shift).
    pub exp_shift: Option<f32>,
    ///Exposure correction before demosaic.
    ///
    ///exp_preser: preserve highlights when lighten the image. Usable range from 0.0 (no
    /// preservation) to 1.0 (full preservation). 0.0 is the default value.
    pub exp_preser: Option<f32>,
    /// Turns on using RawSpeed library for data unpacking (only if RawSpeed support compiled in).
    pub use_rawspeed: Option<bool>,
    ///Disables pixel values scaling (call to LibRaw::scale_colors()) in LibRaw::dcraw_process().
    ///
    ///This is special use value because white balance is performed in scale_colors(), so skipping
    /// it will result in non-balanced image.
    ///
    ///This setting is targeted to use with no_interpolation, or with own interpolation callback
    /// call.
    pub no_auto_scale: Option<bool>,
    ///Disables call to demosaic code in LibRaw::dcraw_process()
    pub no_interpolation: Option<bool>,
}
impl DCRawParams {
    pub(crate) fn set_output_params(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<()> {
        if let Some(graybox) = self.greybox {
            unsafe { (*imgdata).params.greybox = graybox };
        }
        if let Some(cropbox) = self.cropbox {
            unsafe { (*imgdata).params.cropbox = cropbox };
        }
        if let Some(aber) = self.aber {
            unsafe {
                (*imgdata).params.aber[0] = aber[0];
                (*imgdata).params.aber[2] = aber[1];
            }
        }
        if let Some(gamm) = self.gamm {
            unsafe {
                (*imgdata).params.gamm[0] = gamm[0];
                (*imgdata).params.gamm[1] = gamm[1];
            }
        }
        if let Some(user_mul) = self.user_mul {
            unsafe { (*imgdata).params.user_mul = user_mul };
        }
        if let Some(bright) = self.bright {
            unsafe { (*imgdata).params.bright = bright };
        }
        if let Some(threshold) = self.threshold {
            unsafe { (*imgdata).params.threshold = threshold };
        }
        if let Some(half_size) = self.half_size {
            unsafe { (*imgdata).params.half_size = half_size as i32 };
        }
        if let Some(four_color_rgb) = self.four_color_rgb {
            unsafe { (*imgdata).params.four_color_rgb = four_color_rgb as i32 };
        }
        if let Some(highlight) = self.highlight {
            unsafe { (*imgdata).params.highlight = i32::from(highlight) };
        }
        if let Some(use_auto_wb) = self.use_auto_wb {
            unsafe { (*imgdata).params.use_auto_wb = use_auto_wb as i32 };
        }
        if let Some(use_camera_wb) = self.use_camera_wb {
            unsafe { (*imgdata).params.use_camera_wb = use_camera_wb as i32 };
        }
        if let Some(use_camera_matrix) = self.use_camera_matrix {
            unsafe { (*imgdata).params.use_camera_matrix = i32::from(use_camera_matrix) };
        }
        if let Some(output_color) = self.output_color {
            unsafe { (*imgdata).params.output_color = i32::from(output_color) };
        }
        if let Some(output_profile) = &self.output_profile {
            unsafe {
                (*imgdata).params.output_profile = CString::new(output_profile.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(camera_profile) = &self.camera_profile {
            unsafe {
                (*imgdata).params.camera_profile = CString::new(camera_profile.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(bad_pixels) = &self.bad_pixels {
            unsafe {
                (*imgdata).params.bad_pixels = CString::new(bad_pixels.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(dark_frame) = &self.dark_frame {
            unsafe {
                (*imgdata).params.dark_frame = CString::new(dark_frame.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(output_bps) = self.output_bps {
            unsafe { (*imgdata).params.output_bps = i32::from(output_bps) };
        }
        if let Some(output_tiff) = self.output_tiff {
            unsafe { (*imgdata).params.output_tiff = i32::from(output_tiff) };
        }
        if let Some(user_flip) = self.user_flip {
            unsafe { (*imgdata).params.user_flip = i32::from(user_flip) };
        }
        if let Some(user_qual) = self.user_qual {
            unsafe { (*imgdata).params.user_qual = i32::from(user_qual) };
        }
        if let Some(user_black) = self.user_black {
            unsafe { (*imgdata).params.user_black = user_black };
        }
        if let Some(user_cblack) = self.user_cblack {
            unsafe { (*imgdata).params.user_cblack = user_cblack };
        }
        if let Some(user_black) = self.user_black {
            unsafe { (*imgdata).params.user_black = user_black };
        }
        if let Some(user_sat) = self.user_sat {
            unsafe { (*imgdata).params.user_sat = user_sat };
        }
        if let Some(med_passes) = self.med_passes {
            unsafe { (*imgdata).params.med_passes = med_passes };
        }
        if let Some(no_auto_bright) = self.no_auto_bright {
            unsafe { (*imgdata).params.no_auto_bright = no_auto_bright as i32 };
        }
        if let Some(auto_bright_thr) = self.auto_bright_thr {
            unsafe { (*imgdata).params.auto_bright_thr = auto_bright_thr };
        }
        if let Some(adjust_maximum_thr) = self.adjust_maximum_thr {
            unsafe { (*imgdata).params.adjust_maximum_thr = adjust_maximum_thr };
        }
        if let Some(use_fuji_rotate) = self.use_fuji_rotate {
            unsafe { (*imgdata).params.use_fuji_rotate = use_fuji_rotate as i32 };
        }
        if let Some(dcb_iterations) = self.dcb_iterations {
            unsafe { (*imgdata).params.dcb_iterations = dcb_iterations };
        }
        if let Some(dcb_enhance_fl) = self.dcb_enhance_fl {
            unsafe { (*imgdata).params.dcb_enhance_fl = dcb_enhance_fl };
        }
        if let Some(fbdd_noiserd) = self.fbdd_noiserd {
            unsafe { (*imgdata).params.fbdd_noiserd = i32::from(fbdd_noiserd) };
        }
        if let Some(exp_correc) = self.exp_correc {
            unsafe { (*imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_shift) = self.exp_shift {
            unsafe { (*imgdata).params.exp_shift = exp_shift };
        }
        if let Some(exp_correc) = self.exp_correc {
            unsafe { (*imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_preser) = self.exp_preser {
            unsafe { (*imgdata).params.exp_preser = exp_preser };
        }
        if let Some(no_auto_scale) = self.no_auto_scale {
            unsafe { (*imgdata).params.no_auto_scale = no_auto_scale as i32 };
        }
        if let Some(no_interpolation) = self.no_interpolation {
            unsafe { (*imgdata).params.no_interpolation = no_interpolation as i32 };
        }
        Ok(())
    }
}
// presets
impl DCRawParams {
    /// Match output to cg workflow.
    /// - `gamm` = `[1.0, 1.0]`
    /// - `output_color`: ACES
    /// - `output_bps`: 16bit
    pub fn preset_cg() -> Self {
        Self {
            gamm: Some([1.0, 1.0]),
            output_color: Some(DCRawOutputColor::ACES),
            output_bps: Some(DCRawOutputBps::_16bit),
            ..Default::default()
        }
    }
}
