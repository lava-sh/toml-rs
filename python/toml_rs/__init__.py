__all__ = (
    "TOMLDecodeError",
    "TOMLEncodeError",
    "__version__",
    "dump",
    "dumps",
    "load",
    "loads",
)

from collections.abc import Callable
from pathlib import Path
from typing import Any, BinaryIO, TextIO

from ._toml_rs import (
    _dumps,
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


def dump(obj: Any, /, file: Path | TextIO, *, pretty: bool = False) -> int:
    s = dumps(obj, pretty=pretty)
    if isinstance(file, Path):
        return file.write_text(s, encoding="UTF-8")
    else:
        return file.write(s)


def dumps(obj: Any, /, *, pretty: bool = False) -> str:
    return _dumps(obj, pretty=pretty)


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


class TOMLEncodeError(TypeError):
    def __init__(self, msg: str, obj_type: type | None = None, *args: Any):
        msg = msg.rstrip()
        super().__init__(msg)
        self.msg = msg
        self.obj_type = obj_type
