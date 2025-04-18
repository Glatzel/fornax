from enum import StrEnum
from pathlib import Path
from typing import TYPE_CHECKING

from ._base import BaseDecoderParams, BasePostProcessorParams
from .decoder import LibrawParams
from .dnc import DncParams
from .fornax_py import py_process  # type: ignore
from .post_processor import DalimParams, DCRawParams

if TYPE_CHECKING:
    import numpy as np


class FornaxOutputBits(StrEnum):
    u8 = "u8"
    u16 = "u16"
    f32 = "f32"


class Fornax:
    def __init__(
        self,
        *,
        output_bits: FornaxOutputBits,
        decoder_params: BaseDecoderParams,
        post_processor_params: BasePostProcessorParams,
        dnc_params: DncParams | None = None,
    ) -> None:
        self.output_bits = output_bits

        match decoder_params:
            case LibrawParams():
                self.decoder = "libraw"
            case _:
                TypeError("Unknown decoder")
        self.decoder_params = decoder_params

        match post_processor_params:
            case DalimParams():
                self.post_processor = "dalim"
            case DCRawParams():
                self.post_processor = "libraw"
            case _:
                TypeError("Unknown post processor")
        self.post_processor_params = post_processor_params

        self.dnc_params = dnc_params

    def process(self, file: str | Path) -> "np.ndarray":
        """
        Decode and process raw image.

        Parameters
        ----------
        file
            Raw image file.

        Returns
        -------
        np.ndarray
            Processed image of shape `(height, width, channels)`.
        """
        file = Path(file).absolute()
        return py_process(
            file,
            self.output_bits,
            self.decoder,
            self.decoder_params.to_msgpack(),
            self.post_processor,
            self.post_processor_params.to_msgpack(),
            self.dnc_params.to_msgpack() if self.dnc_params else None,
        )[0]
