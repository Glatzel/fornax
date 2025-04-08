from pathlib import Path

import numpy as np

from ._base import BaseDecoder, BasePostProcessor
from .decoder import DncParams, Libraw
from .fornax_py import py_process  # type: ignore
from .post_processor import DCRawParams


class Fornax:
    def __init__(self, file: str | Path, decoder_params: BaseDecoder, post_processor_params: BasePostProcessor) -> None:
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

    def process(self) -> np.ndarray:  # type: ignore
        buf = py_process(self.file, self.decoder, self.decoder_params, self.post_processor, self.post_processor_params)
