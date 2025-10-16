from pathlib import Path

import pytest
import toml_rs
import tomllib

from tests import tests_path

TOML = tests_path / "data" / "example.toml"


def test_toml():
    toml_str = TOML.read_text(encoding="utf-8")
    assert tomllib.loads(toml_str) == toml_rs.loads(toml_str)


def test_toml_text_mode_typeerror():
    with Path(TOML).open(encoding="utf-8") as f, pytest.raises(TypeError) as exc1:
        tomllib.load(f)
    tomllib_err = str(exc1.value)

    with Path(TOML).open(encoding="utf-8") as f, pytest.raises(TypeError) as exc2:
        toml_rs.load(f)
    toml_rs_err = str(exc2.value)

    err_msg = "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
    assert err_msg in tomllib_err
    assert err_msg in toml_rs_err
    assert tomllib_err == toml_rs_err
