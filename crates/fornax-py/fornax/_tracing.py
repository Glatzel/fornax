from enum import IntEnum

from .fornax_py import init_tracing  # type: ignore


class LogLevel(IntEnum):
    ERROR = 1
    WARN = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5


def set_log_level(level: LogLevel, color: bool):
    init_tracing(level, color)
