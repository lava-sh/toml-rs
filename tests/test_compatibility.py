import sys
from pathlib import Path

import pytest
import toml_rs

from .burntsushi import convert, normalize
from .helpers import TOML
from .test_data import VALID_PAIRS_1_0_0 as VALID_PAIRS

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib  # ty: ignore[unresolved-import, unused-ignore-comment]


def test_toml():
    toml_str = TOML.read_text(encoding="utf-8")
    assert tomllib.loads(toml_str) == toml_rs.loads(toml_str)


@pytest.mark.parametrize("lib", [tomllib, toml_rs])
def test_text_mode_typeerror(lib):
    err_msg = "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
    with Path(TOML).open(encoding="utf-8") as f, pytest.raises(TypeError) as exc:
        lib.load(f)
    assert err_msg in str(exc.value)


@pytest.mark.parametrize(
    ("valid", "expected"),
    VALID_PAIRS,
    ids=[p[0].stem for p in VALID_PAIRS],
)
def test_tomllib_vs_tomlrs(valid, expected):
    toml_str = valid.read_bytes().decode("utf-8")
    try:
        toml_str.encode("ascii")
    except UnicodeEncodeError:
        pytest.skip(f"Skipping Unicode content test: {valid.name}")
    tomllib_ = normalize(convert(tomllib.loads(toml_str)))
    toml_rs_ = normalize(convert(toml_rs.loads(toml_str)))

    assert tomllib_ == toml_rs_, f"Mismatch between tomllib and toml_rs for {valid.name}"
