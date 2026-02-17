__all__ = (
    "TOML",
    "_dedent",
    "_init_only",
    "tests_path",
)

from pathlib import Path
from textwrap import dedent

_init_only = {
    "eq": False,
    "repr": False,
    "match_args": False,
}

tests_path = Path(__file__).resolve().parent
TOML = tests_path / "data" / "example.toml"


def _dedent(str_: str, /) -> str:
    return dedent(str_).strip()
