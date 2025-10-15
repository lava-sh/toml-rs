# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2021 Taneli Hukkinen
# Licensed to PSF under a Contributor Agreement.

from __future__ import annotations

import unittest
from typing import Any

from . import tomllib


class TestError(unittest.TestCase):
    def test_line_and_col(self):
        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads("val=.")
        msg = str(exc_info.exception)
        self.assertIn("line 1, column 5", msg)
        self.assertIn("invalid mantissa", msg)

        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads(".")
        msg = str(exc_info.exception)
        self.assertIn("line 1, column", msg)
        self.assertIn("missing value", msg)

        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads("\n\nval=.")
        msg = str(exc_info.exception)
        self.assertIn("line 3, column 5", msg)
        self.assertIn("invalid mantissa", msg)

        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads("\n\n.")
        msg = str(exc_info.exception)
        self.assertIn("line 3, column", msg)
        self.assertIn("missing value", msg)

    def test_missing_value(self):
        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads("\n\nfwfw=")
        msg = str(exc_info.exception)
        self.assertIn("line 3, column 6", msg)
        self.assertIn("string values must be quoted", msg)

    def test_invalid_char_quotes(self):
        with self.assertRaises(tomllib.TOMLDecodeError) as exc_info:
            tomllib.loads("v = '\n'")
        self.assertRegex(str(exc_info.exception), r"key with no value, expected `=`")

    def test_type_error(self):
        with self.assertRaises(TypeError) as exc_info:
            tomllib.loads(b"v = 1")  # type: ignore[arg-type]
        # Mypyc extension leads to different message than pure Python
        self.assertIn(
            str(exc_info.exception),
            ("Expected str object, not 'bytes'", "str object expected; got bytes"),
        )

        with self.assertRaises(TypeError) as exc_info:
            tomllib.loads(False)  # type: ignore[arg-type]
        # Mypyc extension leads to different message than pure Python
        self.assertIn(
            str(exc_info.exception),
            ("Expected str object, not 'bool'", "str object expected; got bool"),
        )

    def test_invalid_parse_float(self):
        def dict_returner(s: str) -> dict[Any, Any]:
            return {}

        def list_returner(s: str) -> list[Any]:
            return []

        for invalid_parse_float in (dict_returner, list_returner):
            with self.assertRaises(ValueError) as exc_info:
                tomllib.loads("f=0.1", parse_float=invalid_parse_float)
            self.assertEqual(
                str(exc_info.exception), "parse_float must not return dicts or lists"
            )
