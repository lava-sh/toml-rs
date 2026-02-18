from collections.abc import Callable
from typing import Any, Literal, Protocol, TypeAlias, TypedDict

_VERSION: str

TomlVersion: TypeAlias = Literal["1.0.0", "1.1.0"]
ParseFloat: TypeAlias = Callable[[str], Any]

class KeyMeta(TypedDict, total=False):
    key: str
    key_line: int
    key_col: int
    key_span: list[int]
    value: Any
    value_raw: str
    value_line: int | list[int]
    value_col: int
    value_span: list[int]

class DocumentMeta(TypedDict):
    keys: dict[str, KeyMeta]

class TOMLDocument(Protocol):
    value: dict[str, Any]
    meta: DocumentMeta

    def __getitem__(self, key: str, /) -> Any: ...
    def __setitem__(self, key: str, value: Any, /) -> None: ...
    def __delitem__(self, key: str, /) -> None: ...

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

def _parse_from_string(
    toml_string: str,
    toml_version: TomlVersion = ...,
) -> TOMLDocument: ...
