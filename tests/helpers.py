__all__ = (
    "TOML",
    "TOML_VERSION",
    "_dedent",
    "_init_only",
    "read_toml",
    "tests_path",
)

from pathlib import Path
from textwrap import dedent

import tomli  # ty: ignore

_init_only = {
    "eq": False,
    "repr": False,
    "match_args": False,
}

tests_path = Path(__file__).resolve().parent
TOML = tests_path / "data" / "example.toml"


def _dedent(str_: str, /) -> str:
    return dedent(str_).strip()


def read_toml(file: str) -> str:
    path = tests_path / "data" / "dumps" / file
    return path.read_text(encoding="utf-8")


TOML_VERSION = "1.1.0" if tomli.__version__ >= "2.4.0" else "1.0.0"
