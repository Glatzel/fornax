from enum import Enum

from pydantic import DirectoryPath, PositiveInt, StrictBool

from ._base import BaseDecoder


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


class DncParams(BaseDecoder):
    """
    DNG converter parameter.

    Attributes
    ----------
    compressed
        Output lossless compressed DNG files
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
        Output converted files to the specified directory.
        Default is the same directory as the input file.
    compatibility
        Set Camera Raw compatibility
    directory
        Output converted files to the specified directory.
        Default is the same directory as the input file.
    filename
        Specify the name of the output DNG file.
        Default is the name of the input file with the extension changed to “.dng”.
    overwrite

    References
    ----------
    - `DNG Converter Command Line <https://community.adobe.com/havfw69955/attachments/havfw69955/camera-raw/23452/1/DNG%20Converter%20Command%20Line.pdf>`_

    """

    compressed: StrictBool = True
    linear: StrictBool = False
    embed: StrictBool = False
    preview: DncPreview = DncPreview.Medium
    fast_load: StrictBool = False
    side: PositiveInt | None = None
    count: PositiveInt | None = None
    compatibility: DncCompatibility = DncCompatibility.CR16_0
    directory: DirectoryPath | None = None
    filename: str | None = None
    overwrite: StrictBool = False


# region libraw
class Libraw(BaseDecoder): ...
