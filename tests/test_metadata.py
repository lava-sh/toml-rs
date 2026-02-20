import datetime

import pytest
import toml_rs

from .helpers import _dedent


def test_metadata() -> None:
    nums = _dedent("""
    int = 999_999_999_999_9_9_9_999_999_999
    float = 999_999_999_999_999.9_9_9
    float_1 = +44121.4124124
    float_2 = -142414.12414_1
    float_3 = 5e+2_2
    float_4 = 1e0_6
    float_5 = -2E-2
    """)
    assert toml_rs.parse_from_string(nums, toml_version="1.1.0").meta == {
        "keys": {
            "float_3": {
                "key": "float_3",
                "key_line": 5,
                "key_col": 1,
                "key_span": (125, 132),
                "value": 5e22,
                "value_raw": "5e+2_2",
                "value_line": 5,
                "value_col": 11,
                "value_span": (135, 141),
            },
            "float_2": {
                "key": "float_2",
                "key_line": 4,
                "key_col": 1,
                "key_span": (99, 106),
                "value": -142414.124141,
                "value_raw": "-142414.12414_1",
                "value_line": 4,
                "value_col": 11,
                "value_span": (109, 124),
            },
            "int": {
                "key": "int",
                "key_line": 1,
                "key_col": 1,
                "key_span": (0, 3),
                "value": 999999999999999999999999,
                "value_raw": "999_999_999_999_9_9_9_999_999_999",
                "value_line": 1,
                "value_col": 7,
                "value_span": (6, 39),
            },
            "float_1": {
                "key": "float_1",
                "key_line": 3,
                "key_col": 1,
                "key_span": (74, 81),
                "value": 44121.4124124,
                "value_raw": "+44121.4124124",
                "value_line": 3,
                "value_col": 11,
                "value_span": (84, 98),
            },
            "float": {
                "key": "float",
                "key_line": 2,
                "key_col": 1,
                "key_span": (40, 45),
                "value": 1000000000000000.0,
                "value_raw": "999_999_999_999_999.9_9_9",
                "value_line": 2,
                "value_col": 9,
                "value_span": (48, 73),
            },
            "float_4": {
                "key": "float_4",
                "key_line": 6,
                "key_col": 1,
                "key_span": (142, 149),
                "value": 1000000.0,
                "value_raw": "1e0_6",
                "value_line": 6,
                "value_col": 11,
                "value_span": (152, 157),
            },
            "float_5": {
                "key": "float_5",
                "key_line": 7,
                "key_col": 1,
                "key_span": (158, 165),
                "value": -0.02,
                "value_raw": "-2E-2",
                "value_line": 7,
                "value_col": 11,
                "value_span": (168, 173),
            },
        },
    }
    strings = _dedent("""
    t1 = "text"
    t2 = 'text'
    t3 = '''text
    text1

    text 2



    text 3
    '''
    t4 = \"\"\"text
    text1
    text 2

    text 3
    \"\"\"
    """)
    assert toml_rs.parse_from_string(strings, toml_version="1.1.0").meta == {
        "keys": {
            "t1": {
                "key": "t1",
                "key_col": 1,
                "key_line": 1,
                "key_span": (0, 2),
                "value": "text",
                "value_col": 6,
                "value_line": 1,
                "value_raw": '"text"',
                "value_span": (5, 11),
            },
            "t2": {
                "key": "t2",
                "key_col": 1,
                "key_line": 2,
                "key_span": (12, 14),
                "value": "text",
                "value_col": 6,
                "value_line": 2,
                "value_raw": "'text'",
                "value_span": (17, 23),
            },
            "t3": {
                "key": "t3",
                "key_col": 1,
                "key_line": 3,
                "key_span": (24, 26),
                "value": "text\ntext1\n\ntext 2\n\n\n\ntext 3\n",
                "value_col": 6,
                "value_line": [
                    3,
                    11,
                ],
                "value_raw": "'''text\ntext1\n\ntext 2\n\n\n\ntext 3\n'''",
                "value_span": (29, 64),
            },
            "t4": {
                "key": "t4",
                "key_col": 1,
                "key_line": 12,
                "key_span": (65, 67),
                "value": "text\ntext1\ntext 2\n\ntext 3\n",
                "value_col": 6,
                "value_line": [
                    12,
                    17,
                ],
                "value_raw": '"""text\ntext1\ntext 2\n\ntext 3\n"""',
                "value_span": (70, 102),
            },
        },
    }
    example = _dedent("""
    title = "TOML Example"

    [owner]
    name = "Lance Uppercut"
    dob = 1979-05-27T07:32:00-08:00

    [database]
    server = "192.168.1.1"
    ports = [ 8001, 8001, 8002 ]
    connection_max = 5000
    enabled = true

    [servers]

      [servers.alpha]
      ip = "10.0.0.1"
      dc = "eqdc10"

      [servers.beta]
      ip = "10.0.0.2"
      dc = "eqdc10"

    [clients]
    data = [ ["gamma", "delta"], (1, 2) ]

    hosts = [
      "alpha",
      "omega"
    ]
    """)
    assert toml_rs.parse_from_string(example, toml_version="1.1.0").meta == {
        "keys": {
            "clients.data": {
                "key": "clients.data",
                "key_col": 1,
                "key_line": 24,
                "key_span": (316, 320),
                "value": [["gamma", "delta"], (1, 2)],
                "value_col": 8,
                "value_line": 24,
                "value_raw": '[ ["gamma", "delta"], (1, 2) ]',
                "value_span": (323, 353),
            },
            "clients.hosts": {
                "key": "clients.hosts",
                "key_col": 1,
                "key_line": 26,
                "key_span": (355, 360),
                "value": ["alpha", "omega"],
                "value_col": 9,
                "value_line": [
                    26,
                    29,
                ],
                "value_raw": '[\n  "alpha",\n  "omega"\n]',
                "value_span": (363, 387),
            },
            "database.connection_max": {
                "key": "database.connection_max",
                "key_col": 1,
                "key_line": 10,
                "key_span": (152, 166),
                "value": 5000,
                "value_col": 18,
                "value_line": 10,
                "value_raw": "5000",
                "value_span": (169, 173),
            },
            "database.enabled": {
                "key": "database.enabled",
                "key_col": 1,
                "key_line": 11,
                "key_span": (174, 181),
                "value": True,
                "value_col": 11,
                "value_line": 11,
                "value_raw": "true",
                "value_span": (184, 188),
            },
            "database.ports": {
                "key": "database.ports",
                "key_col": 1,
                "key_line": 9,
                "key_span": (123, 128),
                "value": [8001, 8001, 8002],
                "value_col": 9,
                "value_line": 9,
                "value_raw": "[ 8001, 8001, 8002 ]",
                "value_span": (131, 151),
            },
            "database.server": {
                "key": "database.server",
                "key_col": 1,
                "key_line": 8,
                "key_span": (100, 106),
                "value": "192.168.1.1",
                "value_col": 10,
                "value_line": 8,
                "value_raw": '"192.168.1.1"',
                "value_span": (109, 122),
            },
            "owner.dob": {
                "key": "owner.dob",
                "key_col": 1,
                "key_line": 5,
                "key_span": (56, 59),
                "value": datetime.datetime(
                    1979,
                    5,
                    27,
                    7,
                    32,
                    tzinfo=datetime.timezone(datetime.timedelta(days=-1, seconds=57600)),
                ),
                "value_col": 7,
                "value_line": 5,
                "value_raw": "1979-05-27T07:32:00-08:00",
                "value_span": (62, 87),
            },
            "owner.name": {
                "key": "owner.name",
                "key_col": 1,
                "key_line": 4,
                "key_span": (32, 36),
                "value": "Lance Uppercut",
                "value_col": 8,
                "value_line": 4,
                "value_raw": '"Lance Uppercut"',
                "value_span": (39, 55),
            },
            "servers.alpha.dc": {
                "key": "servers.alpha.dc",
                "key_col": 3,
                "key_line": 17,
                "key_span": (239, 241),
                "value": "eqdc10",
                "value_col": 8,
                "value_line": 17,
                "value_raw": '"eqdc10"',
                "value_span": (244, 252),
            },
            "servers.alpha.ip": {
                "key": "servers.alpha.ip",
                "key_col": 3,
                "key_line": 16,
                "key_span": (221, 223),
                "value": "10.0.0.1",
                "value_col": 8,
                "value_line": 16,
                "value_raw": '"10.0.0.1"',
                "value_span": (226, 236),
            },
            "servers.beta.dc": {
                "key": "servers.beta.dc",
                "key_col": 3,
                "key_line": 21,
                "key_span": (291, 293),
                "value": "eqdc10",
                "value_col": 8,
                "value_line": 21,
                "value_raw": '"eqdc10"',
                "value_span": (296, 304),
            },
            "servers.beta.ip": {
                "key": "servers.beta.ip",
                "key_col": 3,
                "key_line": 20,
                "key_span": (273, 275),
                "value": "10.0.0.2",
                "value_col": 8,
                "value_line": 20,
                "value_raw": '"10.0.0.2"',
                "value_span": (278, 288),
            },
            "title": {
                "key": "title",
                "key_col": 1,
                "key_line": 1,
                "key_span": (0, 5),
                "value": "TOML Example",
                "value_col": 9,
                "value_line": 1,
                "value_raw": '"TOML Example"',
                "value_span": (8, 22),
            },
        },
    }
    tbl = _dedent("""
    tbl = {
        key      = "a string",
        moar-tbl =  {
            key = 1,
        },
    }
    """)
    assert toml_rs.parse_from_string(tbl, toml_version="1.1.0").meta == {
        "keys": {
            "tbl": {
                "key": "tbl",
                "key_col": 0,
                "key_line": 0,
                "key_span": (6, 78),
                "value": {"key": "a string"},
                "value_col": 7,
                "value_line": (1, 6),
                "value_raw": "{\n"
                '    key      = "a string",\n'
                "    moar-tbl =  {\n"
                "        key = 1,\n"
                "    },\n"
                "}",
                "value_span": (6, 78),
            },
            "tbl.key": {
                "key": "tbl.key",
                "key_col": 5,
                "key_line": 2,
                "key_span": (12, 15),
                "value": "a string",
                "value_col": 16,
                "value_line": 2,
                "value_raw": '"a string"',
                "value_span": (23, 33),
            },
            "tbl.moar-tbl": {
                "key": "tbl.moar-tbl",
                "key_col": 0,
                "key_line": 0,
                "key_span": (51, 75),
                "value": {"key": 1},
                "value_col": 17,
                "value_line": [
                    3,
                    5,
                ],
                "value_raw": "{\n        key = 1,\n    }",
                "value_span": (51, 75),
            },
            "tbl.moar-tbl.key": {
                "key": "tbl.moar-tbl.key",
                "key_col": 9,
                "key_line": 4,
                "key_span": (61, 64),
                "value": 1,
                "value_col": 15,
                "value_line": 4,
                "value_raw": "1",
                "value_span": (67, 68),
            },
        },
    }


def test_document_item_accessors() -> None:
    toml = _dedent("""
    ".x" = "text"

    [a]
    b = 1

    [a.c]
    d = 2

    ".x" = 4

    [".m.".p.".e"]
    l = 99
    """)
    doc = toml_rs.parse_from_string(toml, toml_version="1.1.0")

    assert doc.value["a"]["b"] == 1
    assert doc.value["a"]["c"]["d"] == 2

    assert doc.value["a"]["c"][".x"] == 4
    assert doc.value[".m."]["p"][".e"]["l"] == 99

    assert doc["a.b"] == 1
    assert doc["a.c.d"] == 2

    with pytest.raises(KeyError):
        _ = doc[".m..p..e.l"]

    with pytest.raises(KeyError):
        _ = doc["a.err"]

    assert doc[".x"] == "text"

    doc["new.x.y"] = 3
    assert doc["new.x.y"] == 3

    del doc["a.c.d"]
    with pytest.raises(KeyError):
        _ = doc["a.c.d"]
    assert "d" not in doc.value["a"]["c"]

    with pytest.raises(KeyError):
        del doc["a.c.d"]

    with pytest.raises(KeyError):
        del doc["a.nope.x"]
