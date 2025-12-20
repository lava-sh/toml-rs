import json
import math
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

import pytest
import toml_rs as tomllib

from .burntsushi import convert, normalize
from .helpers import _init_only, tests_path


@dataclass(**_init_only)
class MissingFile:
    path: Path


DATA_DIR = tests_path / "data"


# Test files were taken from this commit:
# https://github.com/toml-lang/toml-test/commit/229ce2e7bb565d1704eac5f41e939870d4b1bce7
def read_toml_files_file(version: str) -> tuple:
    lines = (
        (DATA_DIR / f"files-toml-{version}")
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


VALID_TOML_V_1_0_0, INVALID_TOML_V_1_0_0 = read_toml_files_file("1.0.0")
VALID_TOML_V_1_1_0, INVALID_TOML_V_1_1_0 = read_toml_files_file("1.1.0")

VALID_V_1_0_0_EXPECTED = tuple(
    json.loads(
        (p.with_suffix(".json")).read_text(encoding="utf-8"),
    )
    for p in VALID_TOML_V_1_0_0
)
VALID_V_1_1_0_EXPECTED = tuple(
    json.loads(
        (p.with_suffix(".json")).read_text(encoding="utf-8"),
    )
    for p in VALID_TOML_V_1_1_0
)

VALID_PAIRS_1_0_0 = list(zip(VALID_TOML_V_1_0_0, VALID_V_1_0_0_EXPECTED, strict=False))
VALID_PAIRS_1_1_0 = list(zip(VALID_TOML_V_1_1_0, VALID_V_1_1_0_EXPECTED, strict=False))


@pytest.mark.parametrize(
    ("invalid", "toml_version"),
    [
        *[(pytest.param(p, "1.0.0", id=p.stem)) for p in INVALID_TOML_V_1_0_0],
        *[(pytest.param(p, "1.1.0", id=p.stem)) for p in INVALID_TOML_V_1_1_0],
    ],
)
def test_invalid_tomls(invalid: Path, toml_version: str) -> None:
    toml_bytes = invalid.read_bytes()
    try:
        toml_str = toml_bytes.decode()
    except UnicodeDecodeError:
        # Some BurntSushi tests are not valid UTF-8. Skip those.
        pytest.skip(f"Invalid UTF-8: {invalid}")
    with pytest.raises(tomllib.TOMLDecodeError):
        tomllib.loads(toml_str)


@pytest.mark.parametrize(
    ("valid", "expected", "toml_version"),
    [
        *[(p[0], p[1], "1.0.0") for p in VALID_PAIRS_1_0_0],
        *[(p[0], p[1], "1.1.0") for p in VALID_PAIRS_1_1_0],
    ],
    ids=[p[0].stem for p in VALID_PAIRS_1_0_0 + VALID_PAIRS_1_1_0],
)
def test_valid_tomls(valid: Path, expected: dict, toml_version: str) -> None:
    toml_str = valid.read_bytes().decode("utf-8")
    try:
        toml_str.encode(encoding="ascii")
    except UnicodeEncodeError:
        pytest.skip(f"Skipping Unicode content test: {valid.name}")
    actual = tomllib.loads(toml_str, toml_version=toml_version)
    actual = convert(actual)
    expected_normalized = normalize(expected)
    assert actual == expected_normalized
    # Ensure that parsed toml's can be serialized back without error
    tomllib.dumps(actual)


def test_parse_big_nums() -> None:
    big_int = 999 ** 999
    big_float = float(f"{big_int}.{big_int}")

    t = f"x = {big_int}"
    t2 = f"x = {big_float}"
    t3 = f"x = {big_int + big_int}.{big_int}"

    assert tomllib.loads(t, toml_version="1.1.0")["x"] == big_int
    assert math.isclose(
        tomllib.loads(t3, toml_version="1.1.0")["x"],
        big_float,
        abs_tol=1e-9,
    )
    assert math.isclose(
        tomllib.loads(t3, toml_version="1.1.0", parse_float=Decimal)["x"],
        Decimal(big_float),
        abs_tol=1e-9,
    )
    assert math.isclose(
        tomllib.loads(t2, toml_version="1.1.0")["x"],
        float("inf"),
        abs_tol=1e-9,
    )
