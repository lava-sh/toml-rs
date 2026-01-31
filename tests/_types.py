__all__ = (
    "ParseFloat",
)

from collections.abc import Callable
from typing import Any, TypeAlias

ParseFloat: TypeAlias = Callable[[str], Any]
