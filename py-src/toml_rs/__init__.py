__all__ = (
    "TOMLDecodeError",
    "TOMLDocument",
    "TOMLEncodeError",
    "__version__",
    "dump",
    "dumps",
    "load",
    "load_with_metadata",
    "loads",
)

from ._lib import (
    TOMLDecodeError,
    TOMLDocument,
    TOMLEncodeError,
    dump,
    dumps,
    load,
    load_with_metadata,
    loads,
)
from ._toml_rs import (
    _VERSION as __version__,  # noqa: N811
)
