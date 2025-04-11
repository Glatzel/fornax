from enum import IntEnum

from .fornax_py import py_initialize_tracing  # type: ignore


class LogLevel(IntEnum):
    ERROR = 1
    WARN = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5


def initialize_tracing(level: LogLevel):
    py_initialize_tracing(level)
