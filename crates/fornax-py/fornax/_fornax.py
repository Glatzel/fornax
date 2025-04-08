from enum import Enum
from pathlib import Path

import numpy as np
from _base import BaseDecoder, BasePostProcessor

from .fornax_py import fornax  # type: ignore


class Decoder(Enum, str):
    Libraw = "libraw"
    Dnc = "dnc"


class PostProcessor(Enum, str):
    DCRaw = "dcraw"
    Null = "null"


class Fornax:
    def __init__(self, file: str | Path, decoder: BaseDecoder, post_processor: BasePostProcessor) -> None:
        self.file = Path(file).absolute()
        self.decoder = decoder
        self.post_processor = post_processor

    def process(self) -> np.ndarray:
        return 
