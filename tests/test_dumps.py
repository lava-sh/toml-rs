import re
from datetime import datetime, timedelta, timezone
from typing import Any

import pytest
import toml_rs
import tomli_w

from .helpers import read_toml


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
