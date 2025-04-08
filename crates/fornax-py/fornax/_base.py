import msgspec
from pydantic import BaseModel


class BaseParams(BaseModel):
    def to_msgpack(self) -> bytes:
        return msgspec.msgpack.encode(self.model_dump())


class BaseDecoder(BaseParams): ...


class BasePostProcessor(BaseParams): ...
