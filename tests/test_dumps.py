from datetime import datetime, timedelta, timezone
from re import escape as e

import pytest
import toml_rs


@pytest.mark.parametrize(
    ("v", "pattern"),
    [
        (
            pytest.param(
                type("_Class", (), {}),
                r"Cannot serialize <class '.*_Class'> \(<class 'type'>\)",
                id="class",
            )
        ),
        (
            {"x": lambda x: x},
            r"Cannot serialize <function <lambda> at 0x.*> \(<class 'function'>\)",
        ),
        (
            {"x": 1 + 2j},
            e("Cannot serialize (1+2j) (<class 'complex'>)"),
        ),
        (
            {"set": {1, 2, 3}},
            r"Cannot serialize {1, 2, 3} \(<class 'set'>\)",
        ),
        (
            {"valid": {"invalid": object()}},
            r"Cannot serialize <object object at 0x.*> \(<class 'object'>\)",
        ),
        (
            {42: "value"},
            e("TOML table keys must be strings, got 42 (<class 'int'>)"),
        ),
    ],
)
def test_incorrect_dumps(v, pattern):
    with pytest.raises(toml_rs.TOMLEncodeError, match=pattern):
        toml_rs.dumps(v)


def test_dumps():
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
    assert toml_rs.dumps(obj) == """\
title = "TOML Example"
float = -inf
float_2 = nan

[owner]
dob = 1979-05-27T07:32:00-08:00
name = "Tom Preston-Werner"

[database]
connection_max = 5000
enabled = true
ports = [8001, 8001, 8002]
server = "192.168.1.1"
"""


def test_dumps_inline_tables():
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
    dumps = toml_rs.dumps(obj)
    dumps_with_inline_tables = toml_rs.dumps(
        obj,
        inline_tables={
            "database.connection",
            "database.credentials",
            "service.parameters",
        },
    )
    assert dumps == """\
[database]

[database.connection]
host = "localhost"
port = 5432

[database.credentials]
user = "admin"
password = "secret"

[service]
endpoint = "https://api.example.com"

[service.parameters]
timeout = 30
retries = 3
"""
    assert dumps_with_inline_tables == """\
[database]
connection = { host = "localhost", port = 5432 }
credentials = { user = "admin", password = "secret" }

[service]
endpoint = "https://api.example.com"
parameters = { timeout = 30, retries = 3 }
"""
