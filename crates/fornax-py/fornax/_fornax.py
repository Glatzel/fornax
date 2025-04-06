from enum import Enum


class Decoder(Enum, str):
    libraw = "libraw"
    dnc = "dnc"


class PostProcessor(Enum, str):
    dcraw = "dcraw"
