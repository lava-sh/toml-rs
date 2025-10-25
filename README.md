# toml-rs — A High-Performance TOML Parser for Python written in Rust

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
from pprint import pprint

import toml_rs
import tomllib

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

assert tomllib_loads == toml_rs_loads

print("tomllib:")
pprint(tomllib_loads)
print("toml_rs:")
pprint(toml_rs_loads)
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
