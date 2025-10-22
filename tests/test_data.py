import json
from dataclasses import dataclass
from pathlib import Path

import pytest
import toml_rs as tomllib

from tests import _init_only, tests_path
from .burntsushi import convert, normalize


@dataclass(**_init_only)
class MissingFile:
    path: Path


DATA_DIR = tests_path / "data"


# Test files were taken from this commit:
# https://github.com/toml-lang/toml-test/commit/1d35870ef6783d86366ba55d7df703f3f60b3b55
def read_toml_files_file():
    lines = (
        (DATA_DIR / "files-toml-1.0.0")
        .read_text(encoding="utf-8", errors="ignore")
        .splitlines()
    )
    return (
        tuple(
            DATA_DIR / line.strip()
            for line in lines
            if line.strip().endswith(".toml") and line.strip().startswith("valid/")
        ),
        tuple(
            DATA_DIR / line.strip()
            for line in lines
            if line.strip().endswith(".toml") and line.strip().startswith("invalid/")
        ),
    )


VALID_FILES, INVALID_FILES = read_toml_files_file()
assert VALID_FILES, "Valid TOML test files not found"
assert INVALID_FILES, "Invalid TOML test files not found"

VALID_FILES_EXPECTED = tuple(
    json.loads((p.with_suffix(".json")).read_text()) for p in VALID_FILES
)


@pytest.mark.parametrize("invalid", INVALID_FILES, ids=lambda p: p.stem)
def test_invalid(invalid):
    toml_bytes = invalid.read_bytes()
    try:
        toml_str = toml_bytes.decode()
    except UnicodeDecodeError:
        # Some BurntSushi tests are not valid UTF-8. Skip those.
        pytest.skip(f"Invalid UTF-8: {invalid}")
    with pytest.raises(tomllib.TOMLDecodeError):
        tomllib.loads(toml_str)


VALID_PAIRS = list(zip(VALID_FILES, VALID_FILES_EXPECTED, strict=False))


@pytest.mark.parametrize(
    ("valid", "expected"),
    VALID_PAIRS,
    ids=[p[0].stem for p in VALID_PAIRS],
)
def test_valid(valid, expected):
    toml_str = valid.read_bytes().decode("utf-8")
    try:
        toml_str.encode("ascii")
    except UnicodeEncodeError:
        pytest.skip(f"Skipping Unicode content test: {valid.name}")
    actual = tomllib.loads(toml_str)
    actual = convert(actual)
    expected_normalized = normalize(expected)
    assert actual == expected_normalized
