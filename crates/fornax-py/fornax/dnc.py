from __future__ import annotations

from enum import Enum
from pathlib import Path

from pydantic import PositiveInt, StrictBool, field_validator

from ._base import BaseParams


# region Dnc
class DncPreview(str, Enum):
    _None = "-p0"
    Medium = "-p1"
    Full = "-p2"


class DncCompatibility(str, Enum):
    CR2_4 = "-cr2.4"
    CR4_1 = "-cr4.1"
    CR4_6 = "-cr4.6"
    CR5_4 = "-cr5.4"
    CR6_6 = "-cr6.6"
    CR7_1 = "-cr7.1"
    CR11_2 = "-cr11.2"
    CR12_4 = "-cr12.4"
    CR13_2 = "-cr13.2"
    CR14_0 = "-cr14.0"
    CR15_3 = "-cr15.3"
    CR16_0 = "-cr16.0"
    DNG1_1 = "-dng1.1"
    DNG1_3 = "-dng1.3"
    DNG1_4 = "-dng1.4"
    DNG1_5 = "-dng1.5"
    DNG1_6 = "-dng1.6"
    DNG1_7 = "-dng1.7"
    DNG1_7_1 = "-dng1.7.1"


class DncParams(BaseParams):
    """
    DNG converter parameter.

    Attributes
    ----------
    compressed
        Output lossless compressed DNG files.
    linear
        Output linear DNG files.
    embed
        Embed original raw file inside DNG files.
    preview
        Set JPEG preview size.
    fast_load
        Embed fast load data inside DNG files.
    side
        Limit size to `num` pixels/side.
    count
        Limit pixel count to `num` pixels/image.
    compatibility
        Set Camera Raw compatibility.
    directory
        Output converted files to the specified directory.

        Default is the same directory as the input file.
    filename
        Specify the name of the output DNG file.

        Default is the name of the input file with the extension changed to “.dng”.
    overwrite
        Overwrite existing dng.

    References
    ----------
    - `DNG Converter Command Line <https://community.adobe.com/havfw69955/attachments/havfw69955/camera-raw/23452/1/DNG%20Converter%20Command%20Line.pdf>`_
    """

    compressed: StrictBool = True
    """Output lossless compressed DNG files."""

    linear: StrictBool = False
    """Output linear DNG files."""

    embed: StrictBool = False
    """Embed original raw file inside DNG files."""

    preview: DncPreview = DncPreview.Medium
    """Set JPEG preview size."""

    fast_load: StrictBool = False
    """Embed fast load data inside DNG files."""

    side: PositiveInt | None = None
    """Limit size to `num` pixels/side."""

    count: PositiveInt | None = None
    """
    Limit pixel count to `num` pixels/image.
    """

    compatibility: DncCompatibility = DncCompatibility.CR16_0
    """Set Camera Raw compatibility"""

    directory: str | Path | None = None
    """
    Output converted files to the specified directory.

    Default is the same directory as the input file.
    """

    filename: str | None = None
    """
    Specify the name of the output DNG file.

    Default is the name of the input file with the extension changed to “.dng”.
    """

    overwrite: StrictBool = False
    """Overwrite existing dng."""

    @field_validator("directory")
    def convert_path_to_string(cls, v):
        if v:
            return str(Path(v).absolute())
        else:
            return None
