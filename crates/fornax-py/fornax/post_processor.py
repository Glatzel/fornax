from enum import StrEnum

from pydantic import FilePath, PositiveInt, StrictBool

from ._base import BasePostProcessorParams


# region Dalim
class DalimDemosaicer(StrEnum):
    Linear = "Linear"


class DalimParams(BasePostProcessorParams):
    demosaicer: DalimDemosaicer


# region DCRaw
class DCRawHighlightMode(StrEnum):
    """Highlight modes."""

    Clip = "Clip"
    Ignore = "Ignore"
    Blend = "Blend"
    Reconstruct3 = "Reconstruct3"
    Reconstruct4 = "Reconstruct4"
    Reconstruct5 = "Reconstruct5"
    Reconstruct6 = "Reconstruct6"
    Reconstruct7 = "Reconstruct7"
    Reconstruct8 = "Reconstruct8"
    Reconstruct9 = "Reconstruct9"


class DCRawUseCameraMatrix(StrEnum):
    NotUse = "NotUse"
    EmbeddedProfile = "EmbeddedProfile"
    EmbeddedData = "EmbeddedData"


class DCRawOutputColor(StrEnum):
    """Color spaces."""

    Raw = "Raw"
    SRgb = "SRgb"
    Adobe = "Adobe"
    Wide = "Wide"
    ProPhoto = "ProPhoto"
    XYZ = "XYZ"
    ACES = "ACES"
    DciP3 = "DciP3"
    Rec2020 = "Rec2020"


class DCRawOutputBps(StrEnum):
    _8bit = "_8bit"
    _16bit = "_16bit"


class DCRawOutputTiff(StrEnum):
    _None = "None"
    Ppm = "Ppm"
    Tiff = "Tiff"


class DCRawUserFlip(StrEnum):
    _None = "None"
    Rotate180 = "Rotate180"
    CCW90 = "CCW90"
    CW90 = "CW90"


class DCRawUserQual(StrEnum):
    """Identifiers for demosaic algorithms."""

    Linear = "Linear"
    VNG = "VNG"
    PPG = "PPG"
    AHD = "AHD"
    DCB = "DCB"
    DHT = "DHT"
    ModifiedAHD = "ModifiedAHD"


class DCRawUseFujiRotate(StrEnum):
    UseRotate = "UseRotate"
    NotUse = "NotUse"


class DCRawFbddNoiserd(StrEnum):
    """FBDD noise reduction modes."""

    Off = "Off"
    Light = "Light"
    Full = "Full"


class DCRawParams(BasePostProcessorParams):
    """
    DCRaw parameter.

    Attributes
    ----------
    greybox
        4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
        calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
        w and h are the rectangle's width and height, respectively.
    cropbox
        This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
        rectangle's top-left corner coordinates, remaining two values are width and height
        respectively. All coordinates are applied before any image rotation.
    aber
        Correction of chromatic aberrations; the only specified values are

        - `aber[0]`, the red multiplier.

        - `aber[2]`, the blue multiplier.

        For some formats, it affects RAW data reading , since
        correction of aberrations changes the output size.

        The `aber[0]` will set `aber[0]`.

        The `aber[1]` will set `aber[2]`.
    gamm
        Sets user gamma-curve. Library user should set first two fields of gamm array:
        - `gamm[0]` - inverted gamma value)
        - `gamm[1]` - slope for linear part (so called toe slope). Set to zero for simple power
          curve.

        Remaining 4 values are filled automatically.

        By default settings for rec. BT.709 are used: power 2.222 (i.e. `gamm[0]=1/2.222`) and
        slope 4.5. For sRGB curve use `gamm[0]=1/2.4` and `gamm[1]=12.92`, for linear curve set
        `gamm[0]/gamm[1]` to 1.0.

        The `gamm[0]` will set `gamm[0]`.

        The `gamm[1]` will set `gamm[1]`.
    user_mul
        4 multipliers (r,g,b,g) of the user's white balance.
    bright
        Brightness (default 1.0).
    threshold
        Parameter for noise reduction through wavelet denoising.
    half_size
        Outputs the image in 50% size. For some formats, it affects RAW data reading .
    four_color_rgb
        Switches on separate interpolations for two green components.
    highlight
        0-9: Highlight mode (0=clip, 1=unclip, 2=blend, 3+=rebuild).
    use_auto_wb
        Use automatic white balance obtained after averaging over the entire image.
    use_camera_wb
        If possible, use the white balance from the camera.

        If camera-recorded WB is not available, dcraw_process() will fallback to:
        - Auto-WB if bit LIBRAW_PROCESSING_CAMERAWB_FALLBACK_TO_DAYLIGHT is not set in
            params.raw_processing_options (or for the rare specific case: no valid WB index was
            parsed from CRW file).
         - Daylight-WB if abovementioned bit is not set.
    use_camera_matrix
        If camera-recorded WB is not available, dcraw_process() will fallback to:
    output_color
        [0-8] Output colorspace (raw, sRGB, Adobe, Wide, ProPhoto, XYZ, ACES, DCI-P3, Rec. 2020).
    output_profile
        - 0: do not use embedded color profile
        - 1 (default): use embedded color profile (if present) for DNG files (always); for other
          files only if use_camera_wb is set;
        - 3: use embedded color data (if present) regardless of white balance setting.
    camera_profile
        Path to output profile ICC file (used only if LibRaw compiled with LCMS support)
    bad_pixels
        Path to file with bad pixels map (in dcraw format: "column row
        date-of-pixel-death-in-UNIX-format", one pixel per row).
    dark_frame
        Path to dark frame file (in 16-bit PGM format).
    output_bps
        8 bit (default)/16 bit (key -4).
    output_tiff
        0/1: output PPM/TIFF.
    user_flip
        [0-7] Flip image (0=none, 3=180, 5=90CCW, 6=90CW). Default -1, which means taking the
        corresponding value from RAW.
    user_qual
        0-10: interpolation quality:
        0 - linear interpolation
        1 - VNG interpolation
        2 - PPG interpolation
        3 - AHD interpolation
        4 - DCB interpolation
        11 - DHT interpolation
        12 - Modified AHD interpolation (by Anton Petrusevich)
    user_black
        User black level.
    user_cblack
        Per-channel corrections to user_black.
    user_sat
        Saturation adjustment.
    med_passes
        Number of median filter passes.
    no_auto_bright
        Don't use automatic increase of brightness by histogram.
    auto_bright_thr
        Portion of clipped pixels when auto brightness increase is used. Default value is 0.01
        (1%) for dcraw compatibility. Recommended value for modern low-noise multimegapixel cameras
        depends on shooting style. Values in 0.001-0.00003 range looks reasonable.
    adjust_maximum_thr
        This parameters controls auto-adjusting of maximum value based on channel_maximum[] data,
        calculated from real frame data. If calculated maximum is greater than
        adjust_maximum_thr*maximum, than maximum is set to calculated_maximum.

        Default: 0.75. If you set this value above 0.99999, than default value will be used. If you
        set this value below 0.00001, than no maximum adjustment will be performed.

        Adjusting maximum should not damage any picture (esp. if you use default value) and is very
        useful for correcting channel overflow problems (magenta clouds on landscape shots,
        green-blue highlights for indoor shots).
    use_fuji_rotate
        Default -1 (use), 0 - don't use rotation for cameras on a Fuji sensor.
    green_matching
        Turns on fixing of green channels disbalance. dcraw keys: none.

        Default: 0 (not use), 1 - turns on this postprocessing stage. green_matching requires
        additional memory for image data.
    dcb_iterations
        Number of DCB correction passes. Default is -1 (no correction). Useful only for DCB
        interpolation.
    dcb_enhance_fl
        nonzero: DCB interpolation with enhance interpolated colors.
    fbdd_noiserd
        Controls FBDD noise reduction before demosaic.
        - 0 - do not use FBDD noise reduction
        - 1 - light FBDD reduction
        - 2 (and more) - full FBDD reduction
    exp_correct
        Exposure correction before demosaic.

        exp_correct: positive value turns the feature on (default: off).
    exp_shift
        Exposure correction before demosaic.

        exp_shift: exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0
        (3-stop lighter). Default: 1.0 (no exposure shift).
    exp_preser
        Exposure correction before demosaic.

        exp_preser: preserve highlights when lighten the image. Usable range from 0.0 (no
        preservation) to 1.0 (full preservation). 0.0 is the default value.
    use_rawspeed
        Turns on using RawSpeed library for data unpacking (only if RawSpeed support compiled in).
    no_auto_scale
        Disables pixel values scaling (call to LibRaw::scale_colors()) in LibRaw::dcraw_process().

        This is special use value because white balance is performed in scale_colors(), so skipping
        it will result in non-balanced image.

        This setting is targeted to use with no_interpolation, or with own interpolation callback
        call.
    no_interpolation
        Disables call to demosaic code in LibRaw::dcraw_process()

    References
    ----------
    - `Libraw libraw_output_params_t <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_output_params_t>`_
    """

    greybox: tuple[PositiveInt, PositiveInt, PositiveInt, PositiveInt] | None = None
    """
    4 numbers corresponding to the coordinates (in pixels) of the rectangle that is used to
    calculate the white balance. X and Y are coordinates of the left-top rectangle corner;
    w and h are the rectangle's width and height, respectively.
    """

    cropbox: tuple[PositiveInt, PositiveInt, PositiveInt, PositiveInt] | None = None
    """
    This field sets the image cropping rectangle. `Cropbox[0]` and` cropbox[1]` are the
    rectangle's top-left corner coordinates, remaining two values are width and height
    respectively. All coordinates are applied before any image rotation.
    """

    aber: tuple[float, float] | None = None
    """
    Correction of chromatic aberrations; the only specified values are

    - `aber[0]`, the red multiplier.

    - `aber[2]`, the blue multiplier.

    For some formats, it affects RAW data reading , since
    correction of aberrations changes the output size.

    The `aber[0]` will set `aber[0]`.

    The `aber[1]` will set `aber[2]`.
    """

    gamm: tuple[float, float] | None = None
    """
    Sets user gamma-curve. Library user should set first two fields of gamm array:
    - `gamm[0]` - inverted gamma value)
    - `gamm[1]` - slope for linear part (so called toe slope). Set to zero for simple power
      curve.

    Remaining 4 values are filled automatically.

    By default settings for rec. BT.709 are used: power 2.222 (i.e. `gamm[0]=1/2.222`) and
    slope 4.5. For sRGB curve use `gamm[0]=1/2.4` and `gamm[1]=12.92`, for linear curve set
    `gamm[0]/gamm[1]` to 1.0.

    The `gamm[0]` will set `gamm[0]`.

    The `gamm[1]` will set `gamm[1]`.
    """

    user_mul: tuple[float, float, float, float] | None = None
    """4 multipliers (r,g,b,g) of the user's white balance."""

    bright: float | None = None
    """Brightness (default 1.0)."""

    threshold: float | None = None
    """Parameter for noise reduction through wavelet denoising."""

    half_size: StrictBool | None = None
    """Outputs the image in 50% size. For some formats, it affects RAW data reading ."""

    four_color_rgb: StrictBool | None = None
    """Switches on separate interpolations for two green components."""

    highlight: DCRawHighlightMode | None = None
    """0-9: Highlight mode (0=clip, 1=unclip, 2=blend, 3+=rebuild)."""

    use_auto_wb: StrictBool | None = None
    """Use automatic white balance obtained after averaging over the entire image."""

    use_camera_wb: StrictBool | None = None
    """
    If possible, use the white balance from the camera.

    If camera-recorded WB is not available, dcraw_process() will fallback to:
    - Auto-WB if bit LIBRAW_PROCESSING_CAMERAWB_FALLBACK_TO_DAYLIGHT is not set in
        params.raw_processing_options (or for the rare specific case: no valid WB index was
        parsed from CRW file).
     - Daylight-WB if abovementioned bit is not set.
    """

    use_camera_matrix: DCRawUseCameraMatrix | None = None
    """If camera-recorded WB is not available, dcraw_process() will fallback to:"""

    output_color: DCRawOutputColor | None = None
    """[0-8] Output colorspace (raw, sRGB, Adobe, Wide, ProPhoto, XYZ, ACES, DCI-P3, Rec. 2020)."""

    output_profile: FilePath | None = None
    """
    - 0: do not use embedded color profile
    - 1 (default): use embedded color profile (if present) for DNG files (always); for other
      files only if use_camera_wb is set;
    - 3: use embedded color data (if present) regardless of white balance setting.
    """

    camera_profile: FilePath | None = None
    """Path to output profile ICC file (used only if LibRaw compiled with LCMS support)"""

    bad_pixels: FilePath | None = None
    """
    Path to file with bad pixels map (in dcraw format: "column row
    date-of-pixel-death-in-UNIX-format", one pixel per row).
    """

    dark_frame: FilePath | None = None
    """ Path to dark frame file (in 16-bit PGM format)."""

    output_bps: DCRawOutputBps | None = None
    """8 bit (default)/16 bit (key -4)."""

    output_tiff: DCRawOutputTiff | None = None
    """0/1: output PPM/TIFF."""

    user_flip: DCRawUserFlip | None = None
    """
    [0-7] Flip image (0=none, 3=180, 5=90CCW, 6=90CW). Default -1, which means taking the
    corresponding value from RAW.
    """

    user_qual: DCRawUserQual | None = None
    """
    0-10: interpolation quality:
    0 - linear interpolation
    1 - VNG interpolation
    2 - PPG interpolation
    3 - AHD interpolation
    4 - DCB interpolation
    11 - DHT interpolation
    12 - Modified AHD interpolation (by Anton Petrusevich)
    """

    user_black: int | None = None
    """User black level."""

    user_cblack: tuple[int, int, int, int] | None = None
    """Per-channel corrections to user_black."""

    user_sat: int | None = None
    """Saturation adjustment."""

    med_passes: int | None = None
    """Number of median filter passes."""

    no_auto_bright: StrictBool | None = None
    """Don't use automatic increase of brightness by histogram."""

    auto_bright_thr: float | None = None
    """
    Portion of clipped pixels when auto brightness increase is used. Default value is 0.01
    (1%) for dcraw compatibility. Recommended value for modern low-noise multimegapixel cameras
    depends on shooting style. Values in 0.001-0.00003 range looks reasonable.
    """

    adjust_maximum_thr: float | None = None
    """
    This parameters controls auto-adjusting of maximum value based on channel_maximum[] data,
    calculated from real frame data. If calculated maximum is greater than
    adjust_maximum_thr*maximum, than maximum is set to calculated_maximum.

    Default: 0.75. If you set this value above 0.99999, than default value will be used. If you
    set this value below 0.00001, than no maximum adjustment will be performed.

    Adjusting maximum should not damage any picture (esp. if you use default value) and is very
    useful for correcting channel overflow problems (magenta clouds on landscape shots,
    green-blue highlights for indoor shots).
    """

    use_fuji_rotate: DCRawUseFujiRotate | None = None
    """Default -1 (use), 0 - don't use rotation for cameras on a Fuji sensor."""

    green_matching: StrictBool | None = None
    """
    Turns on fixing of green channels disbalance. dcraw keys: none.

    Default: 0 (not use), 1 - turns on this postprocessing stage. green_matching requires
    additional memory for image data.
    """

    dcb_iterations: int | None = None
    """
    Number of DCB correction passes. Default is -1 (no correction). Useful only for DCB
    interpolation.
    """

    dcb_enhance_fl: int | None = None
    """nonzero: DCB interpolation with enhance interpolated colors."""

    fbdd_noiserd: DCRawFbddNoiserd | None = None
    """
    Controls FBDD noise reduction before demosaic.
    - 0 - do not use FBDD noise reduction
    - 1 - light FBDD reduction
    - 2 (and more) - full FBDD reduction
    """

    exp_correct: int | None = None
    """
    Exposure correction before demosaic.

    exp_correct: positive value turns the feature on (default: off).
    """

    exp_shift: float | None = None
    """
    Exposure correction before demosaic.

    exp_shift: exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0
    (3-stop lighter). Default: 1.0 (no exposure shift).
    """

    exp_preser: float | None = None
    """
    Exposure correction before demosaic.

    exp_preser: preserve highlights when lighten the image. Usable range from 0.0 (no
    preservation) to 1.0 (full preservation). 0.0 is the default value.
    """

    use_rawspeed: StrictBool | None = None
    """Turns on using RawSpeed library for data unpacking (only if RawSpeed support compiled in)."""

    no_auto_scale: StrictBool | None = None
    """
    Disables pixel values scaling (call to LibRaw::scale_colors()) in LibRaw::dcraw_process().

    This is special use value because white balance is performed in scale_colors(), so skipping
    it will result in non-balanced image.

    This setting is targeted to use with no_interpolation, or with own interpolation callback
    call.
    """

    no_interpolation: StrictBool | None = None
    """ Disables call to demosaic code in LibRaw::dcraw_process()."""
