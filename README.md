<div align="center">

# toml-rs

*A High-Performance TOML v1.0.0 and v1.1.0 parser for Python written in Rust*

[![PyPI License](https://img.shields.io/pypi/l/toml_rs.svg?style=flat-square)](https://pypi.org/project/toml_rs/)
[![Python version](https://img.shields.io/pypi/pyversions/toml_rs.svg?style=flat-square)](https://pypi.org/project/toml_rs/)
[![Implementation](https://img.shields.io/pypi/implementation/toml_rs.svg?style=flat-square)](https://pypi.org/project/toml_rs/)

[![Monthly downloads](https://img.shields.io/pypi/dm/toml_rs.svg?style=)](https://pypi.org/project/toml_rs/)
[![Github Repository size](https://img.shields.io/github/repo-size/lava-sh/toml-rs?style=flat-square)](https://github.com/lava-sh/toml-rs)

</div>

## Features

* The fastest TOML parser in Python (see [benchmarks](https://github.com/lava-sh/toml-rs/tree/main/benchmark))

* Drop-in compatibility with most [`tomllib`](https://docs.python.org/3/library/tomllib.html) use cases (see [below](#differences-with-tomllib))

## Installation

```bash
# Using pip
pip install toml-rs

# Using uv
uv pip install toml-rs
```

## Examples

```python
import tomllib
from pprint import pprint

import toml_rs

toml = """\
title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T07:32:00-08:00

[database]
enabled = true
ports = [ 8000, 8001, 8002 ]
data = [ ["delta", "phi"], [3.14] ]
temp_targets = { cpu = 79.5, case = 72.0 }

[servers]
[servers.alpha]
ip = "10.0.0.1"
role = "frontend"
[servers.beta]
ip = "10.0.0.2"
role = "backend"
"""

tomllib_loads = tomllib.loads(toml)
toml_rs_loads = toml_rs.loads(toml)
toml_rs_dumps = toml_rs.dumps(toml_rs_loads)

assert tomllib_loads == toml_rs_loads

print("toml_rs.loads:")
pprint(toml_rs_loads)
print("toml_rs.dumps:")
print(toml_rs_dumps)
```

## Differences with [`tomllib`](https://docs.python.org/3/library/tomllib.html)

1. More understandable errors

```python
import tomllib

t = """\
x = 1
y = 2
v = 
"""
print(tomllib.loads(t))
# tomllib.TOMLDecodeError: Invalid value (at line 3, column 5)
```

```python
import toml_rs

t = """\
x = 1
y = 2
v = 
"""
print(toml_rs.loads(t))
# toml_rs.TOMLDecodeError: TOML parse error at line 3, column 5
#   |
# 3 | v = 
#   |     ^
# string values must be quoted, expected literal string
```

2. Strict compliance with TOML v1.0.0

From [TOML spec](https://toml.io/en/v1.0.0#integer):

> Arbitrary 64-bit signed integers (from `−2^63` to `2^63−1`) should be accepted and handled losslessly. If an integer cannot be represented losslessly, an error must be thrown.

```python
import tomllib

t = "x = 999_999_999_999_999_999_999_999"
print(tomllib.loads(t))
# {'x': 999999999999999999999999} <== speс violation
```

```python
import toml_rs

t = "x = 999_999_999_999_999_999_999_999"
print(toml_rs.loads(t))
# toml_rs.TOMLDecodeError: TOML parse error at line 1, column 5
#   |
# 1 | x = 999_999_999_999_999_999_999_999
#   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
# invalid type: integer `999999999999999999999999` as i128, expected any valid TOML value
```

3. Supports serialization (`toml_rs.dumps` and `toml_rs.dump`)

```python
from pathlib import Path

import toml_rs

data = {
    "title": "TOML Example",
    "owner": {"name": "Alice", "age": 30},
}

print(toml_rs.dumps(data))

toml_rs.dump(data, Path("example.toml"))  
# or `toml_rs.dump(data, "example.toml")`
```
