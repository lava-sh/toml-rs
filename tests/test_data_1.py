import unittest
from pathlib import Path

import toml_rs
import tomllib


class TestSingleToml(unittest.TestCase):
    def test(self):
        toml_str = Path("data.toml").read_text(encoding="utf-8")
        self.assertEqual(tomllib.loads(toml_str), toml_rs.loads(toml_str))
