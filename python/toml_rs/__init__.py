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
    b = fp.read()
    try:
        s = b.decode("utf-8")
    except AttributeError:
        raise TypeError(
            "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
        ) from None

    return _loads(s, parse_float=parse_float)


def loads(s: str, /, *, parse_float: Callable[[str], Any] = float) -> dict[str, Any]:
    if not isinstance(s, str):
        raise TypeError(f"Expected str object, not '{type(s).__name__}'")
    return _loads(s, parse_float=parse_float)
