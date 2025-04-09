from pathlib import Path

import numpy as np

from ._base import BaseDecoderParams, BasePostProcessorParams
from .decoder import DncParams, Libraw
from .fornax_py import py_process  # type: ignore
from .post_processor import DCRawParams


class Fornax:
    def __init__(
        self,
        file: str | Path,
        decoder_params: BaseDecoderParams,
        post_processor_params: BasePostProcessorParams,
    ) -> None:
        self.file = Path(file).absolute()
        match decoder_params:
            case DncParams():
                self.decoder = "dnc"
            case Libraw():
                self.decoder = "libraw"
            case _:
                TypeError("Unknown decoder")

        self.decoder_params = decoder_params
        match post_processor_params:
            case DCRawParams():
                self.post_processor = "dcraw"
            case _:
                TypeError("Unknown post processor")
        self.post_processor_params = post_processor_params

    def process(self):  # type: ignore
        buf, width, height, channels, bits = py_process(
            self.file,
            self.decoder,
            self.decoder_params.to_msgpack(),
            self.post_processor,
            self.post_processor_params.to_msgpack(),
        )
        match channels, bits:
            case 1, 8:
                img = np.frombuffer(buf, dtype=np.uint8).reshape(height, width)
            case 1, 16:
                img = np.frombuffer(buf, dtype=np.uint16).reshape(height, width)
            case 3, 8:
                img = np.frombuffer(buf, dtype=np.uint8).reshape(height, width, 3)
            case 3, 16:
                img = np.frombuffer(buf, dtype=np.uint16).reshape(height, width, 3)
        return img
