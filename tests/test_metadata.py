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
    assert toml_rs.load_with_metadata(nums, toml_version="1.1.0").meta == {}
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
        "tree": {
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
                "value_line": (3, 10),
                "value_col": (8, 14),
            },
            "t4": {
                "key": "t4",
                "key_raw": "t4",
                "key_line": 11,
                "key_col": (1, 2),
                "value": "text\ntext1\ntext 2\n\ntext 3\n",
                "value_raw": '"""text\ntext1\ntext 2\n\ntext 3\n"""',
                "value_line": (11, 16),
                "value_col": (6, 13),
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
    assert toml_rs.load_with_metadata(example, toml_version="1.1.0").meta == {}
    tbl = _dedent("""
    tbl = {
        key      = "a string",
        moar-tbl =  {
            key = 1,
        },
    }
    """)
    assert toml_rs.load_with_metadata(tbl, toml_version="1.1.0").meta == {}

    # https://toml.io/en/v1.1.0#array-of-tables
    points = _dedent("""
    points = [
        { x = 1, y = 2, z = 3 },
        { x = 7, y = 8, z = 9 },
        { x = 2, y = 4, z = 8 },
    ]
    """)
    assert toml_rs.load_with_metadata(points, toml_version="1.1.0").meta == {}

    # https://toml.io/en/v1.1.0#array-of-tables
    product = _dedent("""
    [[product]]
    name = "Hammer"
    sku = 738594937

    [[product]]  # empty table within the array

    [[product]]
    name = "Nail"
    sku = 284758393

    color = "gray"
    """)

    assert toml_rs.load_with_metadata(product, toml_version="1.1.0").meta == {}


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

    del doc["a.c.d"]
    with pytest.raises(KeyError):
        _ = doc["a.c.d"]
    assert "d" not in doc.value["a"]["c"]

    with pytest.raises(KeyError):
        del doc["a.c.d"]

    with pytest.raises(KeyError):
        del doc["a.nope.x"]
