__all__ = (
    "TOML",
    "_init_only",
    "tests_path",
)
from pathlib import Path

_init_only = {
    "eq": False,
    "repr": False,
    "match_args": False,
}

tests_path = Path(__file__).resolve().parent
TOML = tests_path / "data" / "example.toml"
