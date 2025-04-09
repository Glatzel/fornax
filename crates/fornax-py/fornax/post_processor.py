from enum import Enum

from pydantic import FilePath, PositiveInt, StrictBool

from ._base import BasePostProcessorParams


# region DCRaw
class DCRawHighlightMode(int, Enum):
    """Highlight modes."""

    Clip = 0
    Ignore = 1
    Blend = 2
    Reconstruct3 = 3
    Reconstruct4 = 4
    Reconstruct5 = 5
    Reconstruct6 = 6
    Reconstruct7 = 7
    Reconstruct8 = 8
    Reconstruct9 = 9


class DCRawUseCameraMatrix(int, Enum):
    NotUse = 0
    EmbeddedProfile = 1
    EmbeddedData = 3


class DCRawOutputBps(int, Enum):
    _8bit = 8
    _16bit = 6


class DCRawOutputColor(int, Enum):
    """Color spaces."""

    RAW = 0
    SRGB = 1
    ADOBE = 2
    WIDE = 3
    PROPHOTO = 4
    XYZ = 5
    ACES = 6
    P3D65 = 7
    REC2020 = 8


class DCRawOutputTiff(int, Enum):
    _None = -1
    Ppm = 0
    Tiff = 1


class DCRawUserFlip(int, Enum):
    _None = 0
    Rotate180 = 3
    CCW90 = 5
    CW90 = 6


class DCRawUserQual(int, Enum):
    """Identifiers for demosaic algorithms."""

    LINEAR = 0
    VNG = 1
    PPG = 2
    AHD = 3
    DCB = 4
    # comment GPL algorithm
    # MODIFIED_AHD = 5
    # AFD = 6
    # VCD = 7
    # VCD_MODIFIED_AHD = 8
    # LMMSE = 9
    # AMAZE = 10
    DHT = 11
    AAHD = 12


class DCRawUseFujiRotate(int, Enum):
    UseRotate = -1
    NotUse = 0


class DCRawFbddNoiserd(int, Enum):
    """FBDD noise reduction modes."""

    OFF = 0
    LIGHT = 1
    FULL = 2


class DCRawParams(BasePostProcessorParams):
    """
    DCRaw parameter.

    Attributes
    ----------
    greybox

    cropbox

    aber

    gamm

    user_mul

    bright

    threshold

    half_size

    four_color_rgb

    highlight

    use_auto_wb

    use_camera_wb

    use_camera_matrix

    output_color

    output_profile

    camera_profile

    bad_pixels

    dark_frame

    output_bps

    output_tiff

    user_flip

    user_qual

    user_black

    user_cblack

    user_sat

    med_passes

    no_auto_bright

    auto_bright_thr

    adjust_maximum_thr

    use_fuji_rotate

    green_matching

    dcb_iterations

    dcb_enhance_fl

    fbdd_noiserd

    exp_correc

    exp_shift

    exp_preser

    use_rawspeed

    no_auto_scale

    no_interpolation

    References
    ----------
    - `Libraw libraw_output_params_t <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_output_params_t>`_
    """

    greybox: tuple[PositiveInt, PositiveInt, PositiveInt, PositiveInt] | None = None
    cropbox: tuple[PositiveInt, PositiveInt, PositiveInt, PositiveInt] | None = None
    aber: tuple[float, float] | None = None
    gamm: tuple[float, float] | None = None
    user_mul: tuple[float, float, float, float] | None = None
    bright: float | None = None
    threshold: float | None = None
    half_size: StrictBool | None = None
    four_color_rgb: StrictBool | None = None
    highlight: DCRawHighlightMode | None = None
    use_auto_wb: StrictBool | None = None
    use_camera_wb: StrictBool | None = None
    use_camera_matrix: DCRawUseCameraMatrix | None = None
    output_color: DCRawOutputColor | None = None
    output_profile: FilePath | None = None
    camera_profile: FilePath | None = None
    bad_pixels: FilePath | None = None
    dark_frame: FilePath | None = None
    output_bps: DCRawOutputBps | None = None
    output_tiff: DCRawOutputTiff | None = None
    user_flip: DCRawUserFlip | None = None
    user_qual: DCRawUserQual | None = None
    user_black: int | None = None
    user_cblack: tuple[int, int, int, int] | None = None
    user_sat: int | None = None
    med_passes: int | None = None
    no_auto_bright: StrictBool | None = None
    auto_bright_thr: float | None = None
    adjust_maximum_thr: float | None = None
    use_fuji_rotate: DCRawUseFujiRotate | None = None
    green_matching: StrictBool | None = None
    dcb_iterations: int | None = None
    dcb_enhance_fl: int | None = None
    fbdd_noiserd: DCRawFbddNoiserd | None = None
    exp_correc: int | None = None
    exp_shift: float | None = None
    exp_preser: StrictBool | None = None
    use_rawspeed: StrictBool | None = None
    no_auto_scale: StrictBool | None = None
    no_interpolation: StrictBool | None = None
