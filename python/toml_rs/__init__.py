__all__ = (
    "TOMLDecodeError",
    "__version__",
    "load",
    "loads",
)

from collections.abc import Callable
from typing import Any, BinaryIO

from ._toml_rs import (
    TOMLDecodeError,
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
