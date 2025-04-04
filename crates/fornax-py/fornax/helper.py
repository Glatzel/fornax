from enum import Enum
from pathlib import Path
from typing import Annotated, Literal, Optional, Self

from pydantic import BaseModel, Field, StrictBool, field_serializer  # type: ignore


class DEMOSAIC_ALGORITHM(int, Enum):
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


class COLOR_SPACE(int, Enum):
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


class HIGHLIGHT_MODE(int, Enum):
    """Highlight modes."""

    CLIP = 0
    IGNORE = 1
    BLEND = 2
    RECONSTRUCT3 = 3
    RECONSTRUCT4 = 4
    RECONSTRUCT5 = 5
    RECONSTRUCT6 = 6
    RECONSTRUCT7 = 7
    RECONSTRUCT8 = 8
    RECONSTRUCT9 = 9


class FBDD_NOISE_REDUCTION_MODE(int, Enum):
    """FBDD noise reduction modes."""

    OFF = 0
    LIGHT = 1
    FULL = 2


class RawpyParams(BaseModel):
    """
    Create `rawpy.Params`.

    Attributes
    ----------
    demosaic_algorithm
        Demosaic algorithms.
        See https://letmaik.github.io/rawpy/api/enums.html#rawpy.DemosaicAlgorithm.
    half_size
        Outputs image in half size by reducing each 2x2 block to one pixel instead of interpolating.
    four_color_rgb
        Whether to use separate interpolations for two green channels.
    dcb_iterations
        Number of DCB correction passes, requires DCB demosaicing algorithm.
    dcb_enhance
        DCB interpolation with enhanced interpolated colors.
    fbdd_noise_reduction
        Controls FBDD noise reduction before demosaicing.
        See https://letmaik.github.io/rawpy/api/enums.html#rawpy.DemosaicAlgorithm.
    noise_thr
        Threshold for wavelet denoising (default disabled).
    median_filter_passes
        Number of median filter passes after demosaicing to reduce color artifacts.
    use_camera_wb
        Whether to use the as-shot white balance values.
    use_auto_wb
        Whether to try automatically calculating the white balance.
    user_wb
        List of length 4 with white balance multipliers for each color.
    output_color
        Output color space.
    output_bps
        8 or 16.
    user_flip
        Default is to use image orientation from the RAW image if available.

        - 0: none
        - 3: 180
        - 5: 90CCW
        - 6: 90CW
    user_black
        Custom black level.
    user_sat
        Aaturation adjustment.
    no_auto_bright
        Whether to disable automatic increase of brightness.
    auto_bright_thr
        Ratio of clipped pixels when automatic brighness increase is used (see no_auto_bright). Default is 0.01 (1%).
    adjust_maximum_thr
        See libraw docs.
    bright
        Brightness scaling.
    highlight_mode
        Highlight mode.
    exp_shift
        Exposure shift in linear scale. Usable range from 0.25 (2-stop darken) to 8.0 (3-stop lighter).
    exp_preserve_highlights
        Preserve highlights when lightening the image with exp_shift. From 0.0 to 1.0 (full preservation).
    no_auto_scale
        Whether to disable pixel value scaling.
    gamma
        Pair (power,slope), default is `(1, 1)`.
    chromatic_aberration
        Pair (red_scale, blue_scale),
        Default is (1,1),
        Corrects chromatic aberration by scaling the red and blue channels.
    bad_pixels_path
        Path to dcraw bad pixels file. Each bad pixel will be corrected using the mean of the neighbor pixels.

    References
    ----------
    - `rawpy.Params <https://letmaik.github.io/rawpy/api/rawpy.Params.html>`_
    """

    demosaic_algorithm: DEMOSAIC_ALGORITHM = DEMOSAIC_ALGORITHM.AHD
    half_size: StrictBool = False
    four_color_rgb: StrictBool = False
    dcb_iterations: Annotated[int, Field(ge=0)] = 0
    dcb_enhance: StrictBool = False
    fbdd_noise_reduction: FBDD_NOISE_REDUCTION_MODE = FBDD_NOISE_REDUCTION_MODE.OFF
    noise_thr: Optional[float] = None
    median_filter_passes: int = 0
    use_camera_wb: StrictBool = False
    use_auto_wb: StrictBool = False
    user_wb: Optional[tuple[float, float, float, float]] = None
    output_color: COLOR_SPACE = COLOR_SPACE.ACES
    output_bps: Literal[8, 16] = 16
    user_flip: Optional[Literal[0, 3, 5, 6]] = None
    user_black: Optional[int] = None
    user_sat: Optional[int] = None
    no_auto_bright: StrictBool = False
    auto_bright_thr: float = 0.01
    adjust_maximum_thr: float = 0.75
    bright: float = 1.0
    highlight_mode: HIGHLIGHT_MODE = HIGHLIGHT_MODE.CLIP
    exp_shift: Optional[float] = None
    exp_preserve_highlights: Annotated[float, Field(ge=0, le=1)] = 0.0
    no_auto_scale: StrictBool = False
    gamma: Optional[tuple[float, float]] = (1, 1)
    chromatic_aberration: Optional[tuple[float, float]] = None
    bad_pixels_path: Optional[str] = None

    @field_serializer(
        "demosaic_algorithm",
        "fbdd_noise_reduction",
        "output_color",
        "highlight_mode",
        when_used="json",
    )
    def _serialize_special_field(self, v: Enum):
        return v.value

    @classmethod
    def read_json(cls, json_file: str | Path) -> Self:
        """
        Create a model from json file.

        Parameters
        ----------
        json_file
            Path of json file.

        Returns
        -------
        RawpyParams
        """
        json_file = Path(json_file)
        assert json_file.exists()
        assert json_file.is_file()
        assert json_file.suffix.endswith("json")
        return cls.model_validate_json(json_data=json_file.read_text("utf-8"))

    def write_json(self, json_file: str | Path):
        """
        Dump a model to json file.

        Parameters
        ----------
        json_file
            Path of json file.
        """
        json_file = Path(json_file)
        assert json_file.suffix.endswith("json")
        content = self.model_dump_json(indent=4)
        json_file.write_text(content, "utf-8")

    def build_params(self) -> rawpy.Params:  # type: ignore
        """
        Create a ``rawpy.Params`` object.

        Returns
        -------
        rawpy.Params
        """
        return rawpy.Params(  # type: ignore
            demosaic_algorithm=rawpy.DemosaicAlgorithm(self.demosaic_algorithm),  # type: ignore
            half_size=self.half_size,
            four_color_rgb=self.four_color_rgb,
            dcb_iterations=self.dcb_iterations,
            dcb_enhance=self.dcb_enhance,
            fbdd_noise_reduction=rawpy.FBDDNoiseReductionMode(  # type: ignore
                self.fbdd_noise_reduction
            ),  # type: ignore
            noise_thr=self.noise_thr,
            median_filter_passes=self.median_filter_passes,
            use_camera_wb=self.use_camera_wb,
            use_auto_wb=self.use_auto_wb,
            user_wb=self.user_wb,
            output_color=rawpy.ColorSpace(self.output_color),  # type: ignore
            output_bps=self.output_bps,
            user_flip=self.user_flip,
            user_black=self.user_black,
            user_sat=self.user_sat,
            no_auto_bright=self.no_auto_bright,
            auto_bright_thr=self.auto_bright_thr,
            adjust_maximum_thr=self.adjust_maximum_thr,
            bright=self.bright,
            highlight_mode=self.highlight_mode,
            exp_shift=self.exp_shift,
            exp_preserve_highlights=self.exp_preserve_highlights,
            no_auto_scale=self.no_auto_scale,
            gamma=self.gamma,
            chromatic_aberration=self.chromatic_aberration,
            bad_pixels_path=self.bad_pixels_path,
        )
