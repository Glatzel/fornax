from __future__ import annotations

from pathlib import Path
from typing import Self

import msgspec
from pydantic import BaseModel


class BaseParams(BaseModel):
    def to_msgpack(self) -> bytes:
        return msgspec.msgpack.encode(self.model_dump())

    @classmethod
    def read_json(cls, json_file: str | Path) -> Self:
        """
        Create a model from json file.

        Parameters
        ----------
        json_file
            Path of json file.

        Returns
        -------
        Params
        """
        json_file = Path(json_file)
        assert json_file.exists()
        assert json_file.is_file()
        assert json_file.suffix.endswith("json")
        return cls.model_validate_json(json_data=json_file.read_text("utf-8"))

    def write_json(self, json_file: str | Path):
        """
        Dump a model to json file.

        Parameters
        ----------
        json_file
            Path of json file.
        """
        json_file = Path(json_file)
        assert json_file.suffix.endswith("json")
        content = self.model_dump_json(indent=4)
        json_file.write_text(content, "utf-8")


class BaseDecoderParams(BaseParams): ...


class BasePostProcessorParams(BaseParams): ...
