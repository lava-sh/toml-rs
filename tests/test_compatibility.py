import platform
import sys
import types
from decimal import Decimal
from pathlib import Path

import pytest
import toml_rs

from ._types import ParseFloat
from .burntsushi import convert, normalize
from .helpers import TOML
from .test_data import VALID_PAIRS_1_0_0 as VALID_PAIRS

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib  # ty: ignore


def test_example_toml() -> None:
    toml_str = TOML.read_text(encoding="utf-8")
    assert tomllib.loads(toml_str) == toml_rs.loads(toml_str)


@pytest.mark.parametrize("lib", [tomllib, toml_rs])
def test_text_mode_typeerror(lib: types.ModuleType) -> None:
    err_msg = "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
    with (
        Path(TOML).open(encoding="utf-8") as f,
        pytest.raises(TypeError) as exc,
    ):
        lib.load(f)
    assert err_msg in str(exc.value)


@pytest.mark.parametrize(
    ("valid", "expected"),
    VALID_PAIRS,
    ids=[p[0].stem for p in VALID_PAIRS],
)
def test_tomllib_vs_tomlrs(valid: Path, expected: Path) -> None:
    toml_str = valid.read_bytes().decode("utf-8")
    try:
        toml_str.encode("ascii")
    except UnicodeEncodeError:
        pytest.skip(f"Skipping Unicode content test: {valid.name}")

    tomllib_ = normalize(convert(tomllib.loads(toml_str)))
    toml_rs_ = normalize(convert(toml_rs.loads(toml_str)))

    assert tomllib_ == toml_rs_, f"Mismatch between tomllib and toml_rs for {valid.name}"


@pytest.mark.skipif(
    platform.python_implementation() == "PyPy",
    reason="PyPy's `Decimal` parsing hits the int string "
           "conversion digit limit for very large numbers.",
)
@pytest.mark.parametrize(
    "parse_float",
    [float, Decimal],
    ids=["float", "Decimal"],
)
def test_parse_float(parse_float: ParseFloat) -> None:
    num = "9" * 47
    f = f"{num}.{num}"
    t = f"x = {f}"

    tomllib_ = tomllib.loads(t, parse_float=parse_float)
    toml_rs_1 = toml_rs.loads(t, toml_version="1.0.0", parse_float=parse_float)
    toml_rs_1_1 = toml_rs.loads(t, toml_version="1.1.0", parse_float=parse_float)

    assert tomllib_ == toml_rs_1
    assert tomllib_ == toml_rs_1_1
