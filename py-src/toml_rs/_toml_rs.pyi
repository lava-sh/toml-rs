from collections.abc import Callable
from typing import Any, Literal, TypeAlias

_VERSION: str

TomlVersion: TypeAlias = Literal["1.0.0", "1.1.0"]
ParseFloat: TypeAlias = Callable[[str], Any]

def _loads(
    s: str,
    /,
    *,
    parse_float: ParseFloat = ...,
    toml_version: TomlVersion = ...,
) -> dict[str, Any]: ...

def _dumps(
    obj: Any,
    /,
    inline_tables: set[str] | None = None,
    *,
    pretty: bool = False,
    toml_version: TomlVersion = ...,
) -> str: ...
