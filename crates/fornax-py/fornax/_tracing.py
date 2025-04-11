from enum import IntEnum

from .fornax_py import py_set_log_level  # type: ignore


class LogLevel(IntEnum):
    ERROR = 1
    WARN = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5


def set_log_level(level: LogLevel):
    py_set_log_level(level)
