import sys

from . import decoder, post_processor
from ._fornax import Fornax, FornaxOutputBits
from ._tracing import LogLevel, init_tracing

# Adobe DNC Converter only available on Windows or MacOS.
if sys.platform in ("win32", "darwin"):  # noqa: PYI007
    from . import dnc  # noqa: F401

__all__ = ["Fornax", "FornaxOutputBits", "LogLevel", "decoder", "init_tracing", "post_processor",]

if sys.platform in ("win32", "darwin"):  # noqa: PYI007
    __all__.append("dnc")  # noqa: PYI056

