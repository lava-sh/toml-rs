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
from typing import Any, BinaryIO, Literal, TextIO, TypeAlias

from ._toml_rs import (
    _dumps,
    _loads,
    _version,
)

__version__: str = _version

TomlVersion: TypeAlias = Literal["1.0.0", "1.1.0"]

DEFAULT_TOML_VERSION = "1.0.0"


def load(
    fp: BinaryIO,
    /,
    *,
    parse_float: Callable[[str], Any] = float,
    toml_version: TomlVersion = DEFAULT_TOML_VERSION,
) -> dict[str, Any]:
    _bytes = fp.read()
    try:
        _str = _bytes.decode()
    except AttributeError:
        msg = "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
        raise TypeError(msg) from None
    return loads(_str, parse_float=parse_float, toml_version=toml_version)


def loads(
    s: str,
    /,
    *,
    parse_float: Callable[[str], Any] = float,
    toml_version: TomlVersion = DEFAULT_TOML_VERSION,
) -> dict[str, Any]:
    if not isinstance(s, str):
        raise TypeError(f"Expected str object, not '{type(s).__qualname__}'")
    return _loads(s, parse_float=parse_float, toml_version=toml_version)


def dump(
    obj: Any,
    /,
    file: str | Path | TextIO,
    inline_tables: set[str] | None = None,
    *,
    pretty: bool = False,
    toml_version: TomlVersion = DEFAULT_TOML_VERSION,
) -> int:
    _str = _dumps(
        obj,
        inline_tables=inline_tables,
        pretty=pretty,
        toml_version=toml_version,
    )
    if isinstance(file, str):
        file = Path(file)
    if isinstance(file, Path):
        return file.write_text(_str, encoding="utf-8")
    else:
        return file.write(_str)


def dumps(
    obj: Any,
    /,
    inline_tables: set[str] | None = None,
    *,
    pretty: bool = False,
    toml_version: TomlVersion = DEFAULT_TOML_VERSION,
) -> str:
    return _dumps(
        obj,
        inline_tables=inline_tables,
        pretty=pretty,
        toml_version=toml_version,
    )


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
    def __init__(self, msg: str, *args: Any):
        msg = msg.rstrip()
        super().__init__(msg)
        self.msg = msg
