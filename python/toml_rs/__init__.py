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


class TOMLDecodeError(ValueError):
    """
    An error raised if a document is not valid TOML.

    Adds the following attributes to ValueError:
    msg: The unformatted error message
    doc: The TOML document being parsed
    pos: The index of doc where parsing failed
    lineno: The line corresponding to pos
    colno: The column corresponding to pos
    """

    def __init__(
        self,
        msg: str,
        doc: str,
        pos: int,
        *args: Any,
    ):
        lineno = doc.count("\n", 0, pos) + 1
        if lineno == 1:
            colno = pos + 1
        else:
            colno = pos - doc.rindex("\n", 0, pos)

        if pos >= len(doc):
            coord_repr = "end of document"
        else:
            coord_repr = f"line {lineno}, column {colno}"
        errmsg = f"{msg} (at {coord_repr})"
        ValueError.__init__(self, errmsg)

        self.msg = msg
        self.doc = doc
        self.pos = pos
        self.lineno = lineno
        self.colno = colno
