__all__ = (
    "TOMLDecodeError",
    "__version__",
    "load",
    "loads",
)

from collections.abc import Callable
from typing import Any, BinaryIO

from ._toml_rs import (
    _load,
    _loads,
    _version,
)

__version__: str = _version


def load(fp: BinaryIO, /, *, parse_float: Callable[[str], Any] = float) -> dict[str, Any]:
    return _load(fp, parse_float=parse_float)


def loads(s: str, /, *, parse_float: Callable[[str], Any] = float) -> dict[str, Any]:
    if not isinstance(s, str):
        raise TypeError(f"Expected str object, not '{type(s).__name__}'")
    return _loads(s, parse_float=parse_float)


class TOMLDecodeError(ValueError):
    def __init__(self, msg: str, doc: str, pos: int, *args: Any):
        msg = msg.rstrip()
        super().__init__(msg)
        lineno = doc.count("\n", 0, pos) + 1
        if lineno == 1:
            colno = pos + 1
        else:
            colno = pos - doc.rindex("\n", 0, pos)
        self.msg = msg
        self.doc = doc
        self.pos = pos
        self.colno = colno
        self.lineno = lineno
