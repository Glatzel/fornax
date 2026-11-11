from enum import IntEnum

from .fornax_py import py_init_tracing  # type: ignore


class LogLevel(IntEnum):
    ERROR = 1
    WARN = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5


class Inited:
    flag = False


def init_tracing(level: LogLevel, color: bool):
    if not Inited.flag:
        py_init_tracing(level, color)
    Inited.flag = True
