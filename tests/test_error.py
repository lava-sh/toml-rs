# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2021 Taneli Hukkinen
# Licensed to PSF under a Contributor Agreement.

from __future__ import annotations

from typing import Any

import pytest
import toml_rs as tomllib


def test_line_and_col():
    # invalid mantissa
    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads("val=.")
    msg = str(exc.value)
    assert "line 1, column 5" in msg
    assert "invalid mantissa" in msg

    # missing value
    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads(".")
    msg = str(exc.value)
    assert "line 1, column" in msg
    assert "missing value" in msg

    # multiple newlines
    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads("\n\nval=.")
    msg = str(exc.value)
    assert "line 3, column 5" in msg
    assert "invalid mantissa" in msg

    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads("\n\n.")
    msg = str(exc.value)
    assert "line 3, column" in msg
    assert "missing value" in msg


def test_missing_value():
    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads("\n\nfwfw=")
    msg = str(exc.value)
    assert "line 3, column 6" in msg
    assert "string values must be quoted" in msg


def test_invalid_char_quotes():
    with pytest.raises(tomllib.TOMLDecodeError) as exc:
        tomllib.loads("v = '\n'")
    assert "key with no value, expected `=`" in str(exc.value)


def test_type_error():
    with pytest.raises(TypeError) as exc:
        tomllib.loads(b"v = 1")  # type: ignore[arg-type]
    assert str(exc.value) in ("Expected str object, not 'bytes'", "str object expected; got bytes")

    with pytest.raises(TypeError) as exc:
        tomllib.loads(False)  # type: ignore[arg-type]  # noqa: FBT003
    assert str(exc.value) in ("Expected str object, not 'bool'", "str object expected; got bool")


def test_invalid_parse_float():
    def dict_returner(s: str) -> dict[Any, Any]:
        return {}

    def list_returner(s: str) -> list[Any]:
        return []

    err_msg = "parse_float must not return dicts or lists"

    for invalid_parse_float in (dict_returner, list_returner):
        with pytest.raises(ValueError, match=err_msg) as exc:
            tomllib.loads("f=0.1", parse_float=invalid_parse_float)
        assert str(exc.value) == err_msg


def test_tomldecodeerror_attributes():
    data = """\
title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T07:32:00-08:00

x =
"""
    with pytest.raises(tomllib.TOMLDecodeError) as exc_info:
        tomllib.loads(data)

    exc = exc_info.value

    assert exc.msg.startswith("TOML parse error")
    assert exc.doc == data
    assert exc.pos == 96
    assert exc.lineno == 7
    assert exc.colno == 4
    assert f"line {exc.lineno}, column {exc.colno}" in str(exc)
