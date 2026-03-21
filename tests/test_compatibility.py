import platform
import types
from decimal import Decimal
from pathlib import Path

import pytest
import toml_rs
import tomli as tomllib  # ty: ignore

from .burntsushi import convert, normalize
from .helpers import TOML, TOML_VERSION
from .test_data import VALID_PAIRS_1_0_0, VALID_PAIRS_1_1_0


def test_example_toml() -> None:
    toml_str = TOML.read_text(encoding="utf-8")
    assert tomllib.loads(toml_str) == toml_rs.loads(
        toml_str,
        toml_version=TOML_VERSION,
    )


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
    VALID_PAIRS_1_1_0 if TOML_VERSION == "1.1.0" else VALID_PAIRS_1_0_0,
    ids=lambda p: p[0].stem,
)
def test_tomllib_vs_tomlrs(valid: Path, expected: Path) -> None:
    toml_str = valid.read_bytes().decode("utf-8")
    try:
        toml_str.encode("ascii")
    except UnicodeEncodeError:
        pytest.skip(f"Skipping Unicode content test: {valid.name}")

    tomllib_ = normalize(convert(tomllib.loads(toml_str)))
    toml_rs_ = normalize(convert(toml_rs.loads(
        toml_str,
        toml_version=TOML_VERSION,
    )))

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
def test_parse_float(parse_float: toml_rs._lib.ParseFloat) -> None:
    num = "9" * 47
    f = f"{num}.{num}"
    t = f"x = {f}"

    tomllib_ = tomllib.loads(t, parse_float=parse_float)

    toml_rs_ = toml_rs.loads(
        t,
        toml_version=TOML_VERSION,
        parse_float=parse_float,
    )

    assert tomllib_ == toml_rs_
