use std::ffi::CString;
use std::path::PathBuf;

use miette::IntoDiagnostic;

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
impl TryFrom<i32> for OutputBps {
    type Error = miette::Report;

    fn try_from(value: i32) -> miette::Result<OutputBps> {
        match value {
            8 => Ok(OutputBps::_8bit),
            16 => Ok(OutputBps::_16bit),
            v => miette::bail!("Unknow `OutputBps`: {v}"),
        }
    }
}
impl From<OutputBps> for i32 {
    fn from(value: OutputBps) -> Self {
        match value {
            OutputBps::_8bit => 8,
            OutputBps::_16bit => 16,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum OutputTiff {
    None = -1,
    Ppm = 0,
    Tiff = 1,
}
impl TryFrom<i32> for OutputTiff {
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
impl From<OutputTiff> for i32 {
    fn from(value: OutputTiff) -> Self {
        match value {
            OutputTiff::None => -1,
            OutputTiff::Ppm => 0,
            OutputTiff::Tiff => 1,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UserFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 5,
    CW90 = 6,
}
impl TryFrom<i32> for UserFlip {
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
impl From<UserFlip> for i32 {
    fn from(value: UserFlip) -> Self {
        match value {
            UserFlip::None => 0,
            UserFlip::Rotate180 => 3,
            UserFlip::CCW90 => 5,
            UserFlip::CW90 => 6,
        }
    }
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
impl TryFrom<i32> for UserQual {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::LINEAR),
            1 => Ok(Self::VNG),
            2 => Ok(Self::PPG),
            3 => Ok(Self::AHD),
            4 => Ok(Self::DCB),
            11 => Ok(Self::DHT),
            12 => Ok(Self::AAHD),
            v => miette::bail!("Unknow `UserQual`: {v}"),
        }
    }
}
impl From<UserQual> for i32 {
    fn from(value: UserQual) -> Self {
        match value {
            UserQual::LINEAR => 0,
            UserQual::VNG => 1,
            UserQual::PPG => 2,
            UserQual::AHD => 3,
            UserQual::DCB => 4,
            UserQual::DHT => 11,
            UserQual::AAHD => 12,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum UseFujiRotate {
    UseRotate = -1,
    NotUse = 0,
}
impl TryFrom<i32> for UseFujiRotate {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::UseRotate),
            3 => Ok(Self::NotUse),
            v => miette::bail!("Unknow `UseFujiRotate`: {v}"),
        }
    }
}
impl From<UseFujiRotate> for i32 {
    fn from(value: UseFujiRotate) -> Self {
        match value {
            UseFujiRotate::UseRotate => -1,
            UseFujiRotate::NotUse => 0,
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum FbddNoiserd {
    OFF = 0,
    LIGHT = 1,
    FULL = 2,
}
impl TryFrom<i32> for FbddNoiserd {
    type Error = miette::Report;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::OFF),
            0 => Ok(Self::LIGHT),
            1 => Ok(Self::FULL),
            v => miette::bail!("Unknow `OutputTiff`: {v}"),
        }
    }
}
impl From<FbddNoiserd> for i32 {
    fn from(value: FbddNoiserd) -> Self {
        match value {
            FbddNoiserd::OFF => 0,
            FbddNoiserd::LIGHT => 1,
            FbddNoiserd::FULL => 2,
        }
    }
}

///Structure libraw_output_params_t (imgdata.params) is used for management of dcraw-compatible
/// calls dcraw_process(), dcraw_ppm_tiff_writer(), and dcraw_thumb_writer().
/// Fields of this structure correspond to command line keys of dcraw.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct Params {
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
    /// `gamm[0]/gamm[1]` to 1.0.
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
    ///   params.raw_processing_options (or for the rare specific case: no valid WB index was
    ///   parsed from CRW file).
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
    pub output_profile: Option<PathBuf>,
    ///Path to input (camera) profile ICC file (or 'embed' for embedded profile). Used only if
    /// LCMS support compiled in.
    pub camera_profile: Option<PathBuf>,
    ///Path to file with bad pixels map (in dcraw format: "column row
    /// date-of-pixel-death-in-UNIX-format", one pixel per row).
    pub bad_pixels: Option<PathBuf>,
    /// Path to dark frame file (in 16-bit PGM format)
    pub dark_frame: Option<PathBuf>,
    ///8 bit (default)/16 bit (key -4).
    pub output_bps: Option<OutputBps>,
    /// 0/1: output PPM/TIFF.
    pub output_tiff: Option<OutputTiff>,
    /// Bitfield that allows to set output file options:
    /// LIBRAW_OUTPUT_FLAGS_PPMMETA - write additional metadata into PPM/PGM output files
    // pub output_flags: i32,
    /// [0-7] Flip image (0=none, 3=180, 5=90CCW, 6=90CW). Default -1, which means taking the
    /// corresponding value from RAW.
    pub user_flip: Option<UserFlip>,
    /// 0-10: interpolation quality:
    /// 0 - linear interpolation
    /// 1 - VNG interpolation
    /// 2 - PPG interpolation
    /// 3 - AHD interpolation
    /// 4 - DCB interpolation
    /// 11 - DHT interpolation
    /// 12 - Modified AHD interpolation (by Anton Petrusevich)
    pub user_qual: Option<UserQual>,
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
    pub use_fuji_rotate: Option<UseFujiRotate>,
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
    pub fbdd_noiserd: Option<FbddNoiserd>,
    ///Exposure correction before demosaic.
    ///exp_correc: positive value turns the feature on (default: off).
    pub exp_correc: Option<i32>,
    ///Exposure correction before demosaic.
    ///exp_shift: exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0
    /// (3-stop lighter). Default: 1.0 (no exposure shift).
    pub exp_shift: Option<f32>,
    ///Exposure correction before demosaic.
    ///exp_preser: preserve highlights when lighten the image. Usable range from 0.0 (no
    /// preservation) to 1.0 (full preservation). 0.0 is the default value.
    pub exp_preser: Option<f32>,
    ///Turns on using RawSpeed library for data unpacking (only if RawSpeed support compiled in).
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
impl Params {
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
impl Params {
    /// Match output to cg workflow.
    /// - `gamm` = `[1.0, 1.0]`
    /// - `output_color`: ACES
    /// - `output_bps`: 16bit
    pub fn preset_cg() -> Self {
        Self {
            gamm: Some([1.0, 1.0]),
            output_color: Some(OutputColor::ACES),
            output_bps: Some(OutputBps::_16bit),
            ..Default::default()
        }
    }
}
