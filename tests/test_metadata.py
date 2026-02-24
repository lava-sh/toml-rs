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
    assert toml_rs.load_with_metadata(nums, toml_version="1.1.0").meta == {
        "nodes": {
            "float": {
                "key": "float",
                "key_col": (1, 5),
                "key_line": 2,
                "key_raw": "float",
                "value": 1000000000000000.0,
                "value_col": (9, 33),
                "value_line": 2,
                "value_raw": "999_999_999_999_999.9_9_9",
            },
            "float_1": {
                "key": "float_1",
                "key_col": (1, 7),
                "key_line": 3,
                "key_raw": "float_1",
                "value": 44121.4124124,
                "value_col": (11, 24),
                "value_line": 3,
                "value_raw": "+44121.4124124",
            },
            "float_2": {
                "key": "float_2",
                "key_col": (1, 7),
                "key_line": 4,
                "key_raw": "float_2",
                "value": -142414.124141,
                "value_col": (11, 25),
                "value_line": 4,
                "value_raw": "-142414.12414_1",
            },
            "float_3": {
                "key": "float_3",
                "key_col": (1, 7),
                "key_line": 5,
                "key_raw": "float_3",
                "value": 5e22,
                "value_col": (11, 16),
                "value_line": 5,
                "value_raw": "5e+2_2",
            },
            "float_4": {
                "key": "float_4",
                "key_col": (1, 7),
                "key_line": 6,
                "key_raw": "float_4",
                "value": 1000000.0,
                "value_col": (11, 15),
                "value_line": 6,
                "value_raw": "1e0_6",
            },
            "float_5": {
                "key": "float_5",
                "key_col": (1, 7),
                "key_line": 7,
                "key_raw": "float_5",
                "value": -0.02,
                "value_col": (11, 15),
                "value_line": 7,
                "value_raw": "-2E-2",
            },
            "int": {
                "key": "int",
                "key_col": (1, 3),
                "key_line": 1,
                "key_raw": "int",
                "value": 999999999999999999999999,
                "value_col": (7, 39),
                "value_line": 1,
                "value_raw": "999_999_999_999_9_9_9_999_999_999",
            },
        },
    }

    strings = _dedent("""
    t1 = "text"
    't2' = 'text'
    "t3" = '''text
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
    assert toml_rs.load_with_metadata(strings, toml_version="1.1.0").meta == {
        "nodes": {
            "t1": {
                "key": "t1",
                "key_raw": "t1",
                "key_line": 1,
                "key_col": (1, 2),
                "value": "text",
                "value_raw": '"text"',
                "value_line": 1,
                "value_col": (6, 11),
            },
            "t2": {
                "key": "t2",
                "key_raw": "'t2'",
                "key_line": 2,
                "key_col": (1, 4),
                "value": "text",
                "value_raw": "'text'",
                "value_line": 2,
                "value_col": (8, 13),
            },
            "t3": {
                "key": "t3",
                "key_raw": '"t3"',
                "key_line": 3,
                "key_col": (1, 4),
                "value": "text\ntext1\n\ntext 2\n\n\n\ntext 3\n",
                "value_raw": "'''text\ntext1\n\ntext 2\n\n\n\ntext 3\n'''",
                "value_line": (3, 11),
                "value_col": (8, 14),
            },
            "t4": {
                "key": "t4",
                "key_raw": "t4",
                "key_line": 12,
                "key_col": (1, 2),
                "value": "text\ntext1\ntext 2\n\ntext 3\n",
                "value_raw": '"""text\ntext1\ntext 2\n\ntext 3\n"""',
                "value_line": (12, 17),
                "value_col": (6, 12),
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
    data = [ ["gamma", "delta"], [1, 2] ]

    hosts = [
      "alpha",
      "omega"
    ]
    """)

    assert toml_rs.load_with_metadata(example, toml_version="1.1.0").meta == {
        "nodes": {
            "clients": {
                "data": {
                    "key": "data",
                    "key_col": (1, 4),
                    "key_line": 24,
                    "key_raw": "data",
                    "value": [
                        {
                            "value": [
                                {
                                    "value": "gamma",
                                    "value_col": (11, 17),
                                    "value_line": 24,
                                    "value_raw": '"gamma"',
                                },
                                {
                                    "value": "delta",
                                    "value_col": (20, 26),
                                    "value_line": 24,
                                    "value_raw": '"delta"',
                                },
                            ],
                            "value_col": (10, 27),
                            "value_line": 24,
                            "value_raw": '["gamma", "delta"]',
                        },
                        {
                            "value": [
                                {
                                    "value": 1,
                                    "value_col": 31,
                                    "value_line": 24,
                                    "value_raw": "1",
                                },
                                {
                                    "value": 2,
                                    "value_col": 34,
                                    "value_line": 24,
                                    "value_raw": "2",
                                },
                            ],
                            "value_col": (30, 35),
                            "value_line": 24,
                            "value_raw": "[1, 2]",
                        },
                    ],
                    "value_col": (8, 37),
                    "value_line": 24,
                    "value_raw": '[ ["gamma", "delta"], [1, 2] ]',
                },
                "hosts": {
                    "key": "hosts",
                    "key_col": (1, 5),
                    "key_line": 26,
                    "key_raw": "hosts",
                    "value": [
                        {
                            "value": "alpha",
                            "value_col": (3, 9),
                            "value_line": 27,
                            "value_raw": '"alpha"',
                        },
                        {
                            "value": "omega",
                            "value_col": (3, 9),
                            "value_line": 28,
                            "value_raw": '"omega"',
                        },
                    ],
                    "value_col": 9,
                    "value_line": (26, 29),
                    "value_raw": '[\n  "alpha",\n  "omega"\n]',
                },
            },
            "database": {
                "connection_max": {
                    "key": "connection_max",
                    "key_col": (1, 14),
                    "key_line": 10,
                    "key_raw": "connection_max",
                    "value": 5000,
                    "value_col": (18, 21),
                    "value_line": 10,
                    "value_raw": "5000",
                },
                "enabled": {
                    "key": "enabled",
                    "key_col": (1, 7),
                    "key_line": 11,
                    "key_raw": "enabled",
                    "value": True,
                    "value_col": (11, 14),
                    "value_line": 11,
                    "value_raw": "true",
                },
                "ports": {
                    "key": "ports",
                    "key_col": (1, 5),
                    "key_line": 9,
                    "key_raw": "ports",
                    "value": [
                        {
                            "value": 8001,
                            "value_col": (11, 14),
                            "value_line": 9,
                            "value_raw": "8001",
                        },
                        {
                            "value": 8001,
                            "value_col": (17, 20),
                            "value_line": 9,
                            "value_raw": "8001",
                        },
                        {
                            "value": 8002,
                            "value_col": (23, 26),
                            "value_line": 9,
                            "value_raw": "8002",
                        },
                    ],
                    "value_col": (9, 28),
                    "value_line": 9,
                    "value_raw": "[ 8001, 8001, 8002 ]",
                },
                "server": {
                    "key": "server",
                    "key_col": (1, 6),
                    "key_line": 8,
                    "key_raw": "server",
                    "value": "192.168.1.1",
                    "value_col": (10, 22),
                    "value_line": 8,
                    "value_raw": '"192.168.1.1"',
                },
            },
            "owner": {
                "dob": {
                    "key": "dob",
                    "key_col": (1, 3),
                    "key_line": 5,
                    "key_raw": "dob",
                    "value": datetime.datetime(
                        1979,
                        5,
                        27,
                        7,
                        32,
                        tzinfo=datetime.timezone(
                            datetime.timedelta(days=-1, seconds=57600),
                        ),
                    ),
                    "value_col": (7, 31),
                    "value_line": 5,
                    "value_raw": "1979-05-27T07:32:00-08:00",
                },
                "name": {
                    "key": "name",
                    "key_col": (1, 4),
                    "key_line": 4,
                    "key_raw": "name",
                    "value": "Lance Uppercut",
                    "value_col": (8, 23),
                    "value_line": 4,
                    "value_raw": '"Lance Uppercut"',
                },
            },
            "servers": {
                "alpha": {
                    "dc": {
                        "key": "dc",
                        "key_col": (3, 4),
                        "key_line": 17,
                        "key_raw": "dc",
                        "value": "eqdc10",
                        "value_col": (8, 15),
                        "value_line": 17,
                        "value_raw": '"eqdc10"',
                    },
                    "ip": {
                        "key": "ip",
                        "key_col": (3, 4),
                        "key_line": 16,
                        "key_raw": "ip",
                        "value": "10.0.0.1",
                        "value_col": (8, 17),
                        "value_line": 16,
                        "value_raw": '"10.0.0.1"',
                    },
                },
                "beta": {
                    "dc": {
                        "key": "dc",
                        "key_col": (3, 4),
                        "key_line": 21,
                        "key_raw": "dc",
                        "value": "eqdc10",
                        "value_col": (8, 15),
                        "value_line": 21,
                        "value_raw": '"eqdc10"',
                    },
                    "ip": {
                        "key": "ip",
                        "key_col": (3, 4),
                        "key_line": 20,
                        "key_raw": "ip",
                        "value": "10.0.0.2",
                        "value_col": (8, 17),
                        "value_line": 20,
                        "value_raw": '"10.0.0.2"',
                    },
                },
            },
            "title": {
                "key": "title",
                "key_col": (1, 5),
                "key_line": 1,
                "key_raw": "title",
                "value": "TOML Example",
                "value_col": (9, 22),
                "value_line": 1,
                "value_raw": '"TOML Example"',
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
    assert toml_rs.load_with_metadata(tbl, toml_version="1.1.0").meta == {
        "nodes": {
            "tbl": {
                "key": "tbl",
                "key_col": (1, 3),
                "key_line": 1,
                "key_raw": "tbl",
                "value": {
                    "key": {
                        "key": "key",
                        "key_col": (5, 7),
                        "key_line": 2,
                        "key_raw": "key",
                        "value": "a string",
                        "value_col": (16, 25),
                        "value_line": 2,
                        "value_raw": '"a string"',
                    },
                    "moar-tbl": {
                        "key": "moar-tbl",
                        "key_col": (5, 12),
                        "key_line": 3,
                        "key_raw": "moar-tbl",
                        "value": {
                            "key": {
                                "key": "key",
                                "key_col": (9, 11),
                                "key_line": 4,
                                "key_raw": "key",
                                "value": 1,
                                "value_col": 15,
                                "value_line": 4,
                                "value_raw": "1",
                            },
                        },
                        "value_col": 17,
                        "value_line": (3, 5),
                        "value_raw": "{\n        key = 1,\n    }",
                    },
                },
                "value_col": 7,
                "value_line": (1, 6),
                "value_raw": "{\n"
                '    key      = "a string",\n'
                "    moar-tbl =  {\n"
                "        key = 1,\n"
                "    },\n"
                "}",
            },
        },
    }

    # https://toml.io/en/v1.1.0#array-of-tables
    points = _dedent("""
    points = [
        { x = 1, y = 2, z = 3 },
        { x = 7, y = 8, z = 9 },
        { x = 2, y = 4, z = 8 },
    ]
    """)
    doc = toml_rs.load_with_metadata(points, toml_version="1.1.0")
    assert doc.meta["nodes"]["points"]["value"][0]["value"]["x"]["value"] == 1
    assert doc.meta["nodes"]["points"]["value"][1]["value"]["z"]["value"] == 9
    assert doc.meta["nodes"]["points"]["value"][2]["value"]["z"]["value"] == 8

    assert doc.meta == {
        "nodes": {
            "points": {
                "key": "points",
                "key_col": (1, 6),
                "key_line": 1,
                "key_raw": "points",
                "value": [
                    {
                        "value": {
                            "x": {
                                "key": "x",
                                "key_col": 7,
                                "key_line": 2,
                                "key_raw": "x",
                                "value": 1,
                                "value_col": 11,
                                "value_line": 2,
                                "value_raw": "1",
                            },
                            "y": {
                                "key": "y",
                                "key_col": 14,
                                "key_line": 2,
                                "key_raw": "y",
                                "value": 2,
                                "value_col": 18,
                                "value_line": 2,
                                "value_raw": "2",
                            },
                            "z": {
                                "key": "z",
                                "key_col": 21,
                                "key_line": 2,
                                "key_raw": "z",
                                "value": 3,
                                "value_col": 25,
                                "value_line": 2,
                                "value_raw": "3",
                            },
                        },
                        "value_col": (5, 27),
                        "value_line": 2,
                        "value_raw": "{ x = 1, y = 2, z = 3 }",
                    },
                    {
                        "value": {
                            "x": {
                                "key": "x",
                                "key_col": 7,
                                "key_line": 3,
                                "key_raw": "x",
                                "value": 7,
                                "value_col": 11,
                                "value_line": 3,
                                "value_raw": "7",
                            },
                            "y": {
                                "key": "y",
                                "key_col": 14,
                                "key_line": 3,
                                "key_raw": "y",
                                "value": 8,
                                "value_col": 18,
                                "value_line": 3,
                                "value_raw": "8",
                            },
                            "z": {
                                "key": "z",
                                "key_col": 21,
                                "key_line": 3,
                                "key_raw": "z",
                                "value": 9,
                                "value_col": 25,
                                "value_line": 3,
                                "value_raw": "9",
                            },
                        },
                        "value_col": (5, 27),
                        "value_line": 3,
                        "value_raw": "{ x = 7, y = 8, z = 9 }",
                    },
                    {
                        "value": {
                            "x": {
                                "key": "x",
                                "key_col": 7,
                                "key_line": 4,
                                "key_raw": "x",
                                "value": 2,
                                "value_col": 11,
                                "value_line": 4,
                                "value_raw": "2",
                            },
                            "y": {
                                "key": "y",
                                "key_col": 14,
                                "key_line": 4,
                                "key_raw": "y",
                                "value": 4,
                                "value_col": 18,
                                "value_line": 4,
                                "value_raw": "4",
                            },
                            "z": {
                                "key": "z",
                                "key_col": 21,
                                "key_line": 4,
                                "key_raw": "z",
                                "value": 8,
                                "value_col": 25,
                                "value_line": 4,
                                "value_raw": "8",
                            },
                        },
                        "value_col": (5, 27),
                        "value_line": 4,
                        "value_raw": "{ x = 2, y = 4, z = 8 }",
                    },
                ],
                "value_col": 10,
                "value_line": (1, 5),
                "value_raw": "[\n"
                "    { x = 1, y = 2, z = 3 },\n"
                "    { x = 7, y = 8, z = 9 },\n"
                "    { x = 2, y = 4, z = 8 },\n"
                "]",
            },
        },
    }

    # https://toml.io/en/v1.1.0#array-of-tables
    product = _dedent("""
    [[product]]
    name = "Hammer"
    sku = 738594937

    [[product]]  # empty table within the array

    [x]
    t = 5

    [[product]]
    name = "Nail"
    sku = 284758393

    color = "gray"
    """)

    assert toml_rs.load_with_metadata(product, toml_version="1.1.0").meta == {
        "nodes": {
            "product": {
                "key": "product",
                "key_col": (3, 9),
                "key_line": 1,
                "key_raw": "product",
                "value": [
                    {
                        "value": {
                            "name": {
                                "key": "name",
                                "key_col": (1, 4),
                                "key_line": 2,
                                "key_raw": "name",
                                "value": "Hammer",
                                "value_col": (8, 15),
                                "value_line": 2,
                                "value_raw": '"Hammer"',
                            },
                            "sku": {
                                "key": "sku",
                                "key_col": (1, 3),
                                "key_line": 3,
                                "key_raw": "sku",
                                "value": 738594937,
                                "value_col": (7, 15),
                                "value_line": 3,
                                "value_raw": "738594937",
                            },
                        },
                        "value_col": (1, 11),
                        "value_line": 1,
                        "value_raw": "[[product]]",
                    },
                    {
                        "value": {},
                        "value_col": (1, 11),
                        "value_line": 5,
                        "value_raw": "[[product]]",
                    },
                    {
                        "value": {
                            "color": {
                                "key": "color",
                                "key_col": (1, 5),
                                "key_line": 14,
                                "key_raw": "color",
                                "value": "gray",
                                "value_col": (9, 14),
                                "value_line": 14,
                                "value_raw": '"gray"',
                            },
                            "name": {
                                "key": "name",
                                "key_col": (1, 4),
                                "key_line": 11,
                                "key_raw": "name",
                                "value": "Nail",
                                "value_col": (8, 13),
                                "value_line": 11,
                                "value_raw": '"Nail"',
                            },
                            "sku": {
                                "key": "sku",
                                "key_col": (1, 3),
                                "key_line": 12,
                                "key_raw": "sku",
                                "value": 284758393,
                                "value_col": (7, 15),
                                "value_line": 12,
                                "value_raw": "284758393",
                            },
                        },
                        "value_col": (1, 11),
                        "value_line": 10,
                        "value_raw": "[[product]]",
                    },
                ],
            },
            "x": {
                "t": {
                    "key": "t",
                    "key_col": 1,
                    "key_line": 8,
                    "key_raw": "t",
                    "value": 5,
                    "value_col": 5,
                    "value_line": 8,
                    "value_raw": "5",
                },
            },
        },
    }

    product_ = _dedent("""
    [[ product ]]
    name = "Hammer"

    [[ product ]]
    name = "Nail"
    """)
    doc = toml_rs.load_with_metadata(product_, toml_version="1.0.0").meta
    assert doc["nodes"]["product"]["value"][0]["value"]["name"]["value"] == "Hammer"
    assert doc["nodes"]["product"]["value"][1]["value"]["name"]["value"] == "Nail"
    assert doc["nodes"]["product"]["value"][0]["value_raw"] == "[[ product ]]"
    assert doc["nodes"]["product"]["value"][1]["value_raw"] == "[[ product ]]"


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
    doc = toml_rs.load_with_metadata(toml, toml_version="1.1.0")

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
    assert doc.value["new"]["x"]["y"] == 3
    assert "new.x.y" not in doc.value

    del doc["a.c.d"]
    with pytest.raises(KeyError):
        _ = doc["a.c.d"]
    assert "d" not in doc.value["a"]["c"]

    with pytest.raises(KeyError):
        del doc["a.c.d"]

    with pytest.raises(KeyError):
        del doc["a.nope.x"]
