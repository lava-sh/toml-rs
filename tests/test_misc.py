import copy
import datetime
import sys
from decimal import Decimal
from pathlib import Path
from tempfile import TemporaryDirectory

import pytest
import toml_rs as tomllib


def test_load():
    content = "one=1 \n two='two' \n arr=[]"
    expected = {"one": 1, "two": "two", "arr": []}
    with TemporaryDirectory() as tmp_dir_path:
        file_path = Path(tmp_dir_path) / "test.toml"
        file_path.write_text(content)

        with Path(file_path).open("rb") as bin_f:
            actual = tomllib.load(bin_f)
    assert actual == expected


def test_incorrect_load():
    content = "one=1"
    with TemporaryDirectory() as tmp_dir_path:
        file_path = Path(tmp_dir_path) / "test.toml"
        file_path.write_text(content)

        with (
            Path(file_path).open(encoding="utf-8") as txt_f,
            pytest.raises(TypeError) as exc,
        ):
            tomllib.load(txt_f)  # type: ignore[arg-type]
        assert str(exc.value) in (
            "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`",
            "bytes object expected; got str",
        )


def test_parse_float():
    doc = """
          val=0.1
          biggest1=inf
          biggest2=+inf
          smallest=-inf
          notnum1=nan
          notnum2=-nan
          notnum3=+nan
          """
    obj = tomllib.loads(doc, parse_float=Decimal)
    expected = {
        "val": Decimal("0.1"),
        "biggest1": Decimal("inf"),
        "biggest2": Decimal("inf"),
        "smallest": Decimal("-inf"),
        "notnum1": Decimal("nan"),
        "notnum2": Decimal("-nan"),
        "notnum3": Decimal("nan"),
    }
    for k, expected_val in expected.items():
        actual_val = obj[k]
        assert isinstance(actual_val, Decimal)
        if actual_val.is_nan():
            assert expected_val.is_nan()
        else:
            assert actual_val == expected_val


def test_deepcopy():
    doc = """
          [bliibaa.diibaa]
          offsettime=[1979-05-27T00:32:00.999999-07:00]
          """
    obj = tomllib.loads(doc)
    obj_copy = copy.deepcopy(obj)
    assert obj_copy == obj

    expected_obj = {
        "bliibaa": {
            "diibaa": {
                "offsettime": [
                    datetime.datetime(
                        1979,
                        5,
                        27,
                        0,
                        32,
                        0,
                        999999,
                        tzinfo=datetime.timezone(datetime.timedelta(hours=-7)),
                    ),
                ],
            },
        },
    }
    assert obj_copy == expected_obj


def test_inline_array_recursion_limit():
    nest_count = 470
    recursive_array_toml = "arr = " + nest_count * "[" + nest_count * "]"
    tomllib.loads(recursive_array_toml)

    nest_count = sys.getrecursionlimit() + 2
    recursive_array_toml = "arr = " + nest_count * "[" + nest_count * "]"
    with pytest.raises(
        RecursionError,
        match=r"max recursion depth met",
    ):
        tomllib.loads(recursive_array_toml)
