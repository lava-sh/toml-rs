import json
from dataclasses import dataclass
from pathlib import Path

import pytest
import toml_rs as tomllib

from tests import _init_only, tests_path
from .burntsushi import (
    convert as burntsushi_convert,
    normalize as burntsushi_normalize,
)


@dataclass(**_init_only)
class MissingFile:
    path: Path


DATA_DIR = tests_path / "data"

VALID_FILES = tuple((DATA_DIR / "valid").glob("**/*.toml"))
assert VALID_FILES, "Valid TOML test files not found"

INVALID_FILES = tuple((DATA_DIR / "invalid").glob("**/*.toml"))
assert INVALID_FILES, "Invalid TOML test files not found"

_expected_files = []
for p in VALID_FILES:
    json_path = p.with_suffix(".json")
    try:
        text = json.loads(json_path.read_bytes().decode())
    except FileNotFoundError:
        text = MissingFile(json_path)
    _expected_files.append(text)
VALID_FILES_EXPECTED = tuple(_expected_files)


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
    ("valid_file", "expected"),
    VALID_PAIRS,
    ids=[p[0].stem for p in VALID_PAIRS],
)
def test_valid(valid_file, expected):
    if isinstance(expected, MissingFile):
        # For a poor man's xfail, assert that this is one of the
        # test cases where expected data is known to be missing.
        assert valid_file.stem in {
            "qa-array-inline-nested-1000",
            "qa-table-inline-nested-1000",
        }
        pytest.xfail(f"Expected JSON missing for {valid_file.stem}")
    toml_str = valid_file.read_bytes().decode()
    actual = tomllib.loads(toml_str)
    actual = burntsushi_convert(actual)
    expected_normalized = burntsushi_normalize(expected)
    assert actual == expected_normalized
