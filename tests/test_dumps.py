import re
from collections import OrderedDict
from collections.abc import Mapping
from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal
from types import MappingProxyType
from typing import Any

import pytest
import toml_rs
import tomli_w

from .helpers import read_toml


class MyMap(Mapping[str, Any]):
    def __init__(self, data: dict[str, Any]) -> None:
        self._data = data

    def __getitem__(self, key: str) -> Any:
        return self._data[key]

    def __iter__(self) -> Any:
        return iter(self._data)

    def __len__(self) -> int:
        return len(self._data)


@pytest.mark.parametrize(
    ("value", "pattern", "kwargs"),
    [
        (
            type("_Class", (), {}),
            r"Cannot serialize <class '.*_Class'> \(<class 'type'>\)",
            {},
        ),
        (
            {"x": lambda x: x},
            r"Cannot serialize <function <lambda> at 0x.*> \(<class 'function'>\)",
            {},
        ),
        (
            {"x": 1 + 2j},
            re.escape("Cannot serialize (1+2j) (<class 'complex'>)"),
            {},
        ),
        (
            {"set": {1, 2, 3}},
            r"Cannot serialize {1, 2, 3} \(<class 'set'>\)",
            {},
        ),
        (
            {"valid": {"invalid": object()}},
            r"Cannot serialize <object object at 0x.*> \(<class 'object'>\)",
            {},
        ),
        (
            {42: "value"},
            re.escape("TOML table keys must be strings, got 42 (<class 'int'>)"),
            {},
        ),
        (
            {"database": {"connection": {"host": "localhost"}}},
            re.escape(
                "Path 'database.connectio' specified in"
                " inline_tables does not exist in the toml",
            ),
            {"inline_tables": {"database.connectio"}},
        ),
        (
            {"database": {"connection": {"host": "localhost"}, "port": 8080}},
            re.escape("Path 'database.port' does not point to a table"),
            {"inline_tables": {"database.port"}},
        ),
        (
            {"decimal": Decimal("NaN123")},
            re.escape("Cannot serialize invalid decimal.Decimal('NaN123') to TOML"),
            {},
        ),
        (
            {"decimal": Decimal("-NaN123")},
            re.escape("Cannot serialize invalid decimal.Decimal('-NaN123') to TOML"),
            {},
        ),
        (
            {"decimal": Decimal("sNaN789")},
            re.escape("Cannot serialize invalid decimal.Decimal('sNaN789') to TOML"),
            {},
        ),
        (
            {"decimal": Decimal("-sNaN789")},
            re.escape("Cannot serialize invalid decimal.Decimal('-sNaN789') to TOML"),
            {},
        ),
    ],
)
def test_incorrect_dumps(
        value: Any,
        pattern: str | re.Pattern[str],
        kwargs: dict[str, Any],
) -> None:
    with pytest.raises(toml_rs.TOMLEncodeError, match=pattern):
        toml_rs.dumps(value, **kwargs)


def test_dumps() -> None:
    obj = {
        "title": "TOML Example",
        "float": float("-inf"),
        "float_2": float("+nan"),
        "owner": {
            "dob": datetime(1979, 5, 27, 7, 32, tzinfo=timezone(timedelta(hours=-8))),
            "name": "Tom Preston-Werner",
        },
        "database": {
            "connection_max": 5000,
            "enabled": True,
            "ports": [8001, 8001, 8002],
            "server": "192.168.1.1",
        },
    }
    assert toml_rs.dumps(obj) == read_toml("test_dumps.toml")


def test_dumps_inline_tables() -> None:
    obj = {
        "database": {
            "connection": {"host": "localhost", "port": 5432},
            "credentials": {"user": "admin", "password": "secret"},
        },
        "service": {
            "endpoint": "https://api.example.com",
            "parameters": {"timeout": 30, "retries": 3},
        },
    }
    assert toml_rs.dumps(obj) == read_toml("test_dumps_inline_tables.toml")

    with_inline_tables = toml_rs.dumps(
        obj,
        inline_tables={
            "database.connection",
            "database.credentials",
            "service.parameters",
        },
    )
    assert with_inline_tables == read_toml("test_dumps_inline_tables[1].toml")

    with_inline_tables_2 = toml_rs.dumps(
        obj,
        inline_tables={
            "database.connection",
            "service.parameters",
        },
    )
    assert with_inline_tables_2 == read_toml("test_dumps_inline_tables[2].toml")


def test_dumps_pretty() -> None:
    obj = {
        "example": {
            "array": ["item 1", "item 2", "item 3"],
        },
        "x": [
            {"name": "foo", "value": 1},
            {"name": "bar", "value": 2},
        ],
    }
    assert (
        toml_rs.dumps(obj, pretty=False)
        ==
        read_toml("test_dumps_pretty[pretty=False].toml")
    )

    assert (
        toml_rs.dumps(obj, pretty=True)
        ==
        read_toml("test_dumps_pretty[pretty=True].toml")
    )


def test_dumps_pretty_with_inline_tables() -> None:
    obj = {
        "array": ["item 1", "item 2", "item 3"],
        "database": {
            "connection": {"host": "localhost", "port": 5432},
            "credentials": {"user": "admin", "password": "secret"},
        },
        "x": [
            {"name": "foo", "value": 1},
            {"name": "bar", "value": 2},
        ],
    }

    assert (
        toml_rs.dumps(
            obj,
            inline_tables={"database.connection", "database.credentials"},
            pretty=True,
        )
        ==
        read_toml("test_dumps_pretty_with_inline_tables.toml")
    )


def test_big_nums(toml_version: toml_rs._lib.TomlVersion) -> None:
    num = 999999999999999999999999999999999999999999999999999999999

    # https://github.com/lava-sh/toml-rs/issues/117
    big_int = {"int": num}
    assert tomli_w.dumps(big_int) == toml_rs.dumps(big_int, toml_version=toml_version)

    # https://github.com/lava-sh/toml-rs/issues/118
    big_float = {"float": float(f"{num}.{num}")}
    assert tomli_w.dumps(big_float) == toml_rs.dumps(big_float, toml_version=toml_version)


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (
            {"empty_tuple": ()},
            {
                "1.0.0": "empty_tuple = []\n",
                "1.1.0": "empty_tuple = []\n",
            },
        ),
        (
            {"date": date(1979, 5, 27)},
            {
                "1.0.0": "date = 1979-05-27\n",
                "1.1.0": "date = 1979-05-27\n",
            },
        ),
        (
            {"ordered_dict": OrderedDict([("x", 1), ("y", 2)])},
            {
                "1.0.0": "[ordered_dict]\nx = 1\ny = 2\n",
                "1.1.0": "[ordered_dict]\nx = 1\ny = 2\n",
            },
        ),
        (
            {"mapping_proxy": MappingProxyType({"x": 1, "y": 2})},
            {
                "1.0.0": "[mapping_proxy]\nx = 1\ny = 2\n",
                "1.1.0": "[mapping_proxy]\nx = 1\ny = 2\n",
            },
        ),
        (
            {"custom_mapping": MyMap({"x": 1, "y": 2})},
            {
                "1.0.0": "[custom_mapping]\nx = 1\ny = 2\n",
                "1.1.0": "[custom_mapping]\nx = 1\ny = 2\n",
            },
        ),
        (
            {"decimal": Decimal("1.50")},
            {
                "1.0.0": "decimal = 1.50\n",
                "1.1.0": "decimal = 1.50\n",
            },
        ),
        (
            {"decimal": Decimal(1)},
            {
                "1.0.0": "decimal = 1.0\n",
                "1.1.0": "decimal = 1.0\n",
            },
        ),
        (
            {"decimal": Decimal("1E+3")},
            {
                "1.0.0": "decimal = 1e+3\n",
                "1.1.0": "decimal = 1e+3\n",
            },
        ),
        (
            {"decimal": Decimal("NaN")},
            {
                "1.0.0": "decimal = nan\n",
                "1.1.0": "decimal = nan\n",
            },
        ),
        (
            {"decimal": Decimal("-NaN")},
            {
                "1.0.0": "decimal = nan\n",
                "1.1.0": "decimal = nan\n",
            },
        ),
        (
            {"decimal": Decimal("sNaN")},
            {
                "1.0.0": "decimal = nan\n",
                "1.1.0": "decimal = nan\n",
            },
        ),
        (
            {"decimal": Decimal("-sNaN")},
            {
                "1.0.0": "decimal = nan\n",
                "1.1.0": "decimal = nan\n",
            },
        ),
        (
            {"decimal": Decimal("Infinity")},
            {
                "1.0.0": "decimal = inf\n",
                "1.1.0": "decimal = inf\n",
            },
        ),
        (
            {"decimal": Decimal("-Infinity")},
            {
                "1.0.0": "decimal = -inf\n",
                "1.1.0": "decimal = -inf\n",
            },
        ),
        (
            {"decimal": Decimal("Inf")},
            {
                "1.0.0": "decimal = inf\n",
                "1.1.0": "decimal = inf\n",
            },
        ),
        (
            {"decimal": Decimal("-Inf")},
            {
                "1.0.0": "decimal = -inf\n",
                "1.1.0": "decimal = -inf\n",
            },
        ),
        (
            {"tuple": (2, 3)},
            {
                "1.0.0": "tuple = [2, 3]\n",
                "1.1.0": "tuple = [2, 3]\n",
            },
        ),
        (
            {"nested_tuple": ((1, 2), (3, 4))},
            {
                "1.0.0": "nested_tuple = [[1, 2], [3, 4]]\n",
                "1.1.0": "nested_tuple = [[1, 2], [3, 4]]\n",
            },
        ),
        (
            {"mixed_sequence": [1, (2, 3), [4, 5]]},
            {
                "1.0.0": "mixed_sequence = [1, [2, 3], [4, 5]]\n",
                "1.1.0": "mixed_sequence = [1, [2, 3], [4, 5]]\n",
            },
        ),
        (
            {"time": time(7, 32)},
            {
                "1.0.0": "time = 07:32:00\n",
                "1.1.0": "time = 07:32:00.0\n",
            },
        ),
        (
            {
                "datetime": datetime(
                    1979,
                    5,
                    27,
                    7,
                    32,
                    tzinfo=timezone(timedelta(hours=-8)),
                ),
            },
            {
                "1.0.0": "datetime = 1979-05-27T07:32:00-08:00\n",
                "1.1.0": "datetime = 1979-05-27T07:32:00.0-08:00\n",
            },
        ),
    ],
)
def test_dumps_direct(
        value: dict[str, Any],
        expected: dict[toml_rs._lib.TomlVersion, str],
        toml_version: toml_rs._lib.TomlVersion,
) -> None:
    assert toml_rs.dumps(value, toml_version=toml_version) == expected[toml_version]
