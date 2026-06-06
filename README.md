<div align="center">

# toml-rs

*A High-Performance TOML v1.0.0 / v1.1.0 parser for Python written in Rust*

<p>
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?variant=outline&font=geist-mono&size=xs&animate=pulse&mode=dark"><img alt="CI" src="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?variant=outline&font=geist-mono&size=xs&animate=pulse&mode=light"></picture>
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="Last Commit" src="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture>
  <a href="https://github.com/lava-sh/toml-rs/blob/main/UNLICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/lava-sh/toml-rs/license.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="License" src="https://shieldcn.dev/github/lava-sh/toml-rs/license.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture></a>
  <a href="https://pypi.org/project/toml-rs"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/pypi/dm/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="Monthly downloads" src="https://shieldcn.dev/pypi/dm/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture></a>
  <a href="https://pypi.org/project/toml-rs"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/pypi/python/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="Python version" src="https://shieldcn.dev/pypi/python/toml-rs.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture></a>
</p>

![CI](https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?variant=outline&animate=pulse&font=geist-mono&size=xs)
![Last Commit](https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist-mono&size=xs)
[![License](https://shieldcn.dev/github/lava-sh/toml-rs/license.svg?variant=outline&font=geist-mono&size=xs)](https://github.com/lava-sh/toml-rs/blob/main/UNLICENSE)
[![Monthly downloads](https://shieldcn.dev/pypi/dm/toml-rs.svg?variant=outline&font=geist-mono&size=xs)](https://pypi.org/project/toml-rs)
[![Python version](https://shieldcn.dev/pypi/python/toml-rs.svg?variant=outline&font=geist-mono&size=xs)](https://pypi.org/project/toml-rs)

</div>

## Features

* The fastest TOML parser in Python (see [benchmarks](https://github.com/lava-sh/toml-rs/tree/main/benchmark))

* Drop-in compatibility with most [`tomllib`](https://docs.python.org/3/library/tomllib.html) use cases
  (see [below](#differences-with-tomllib))

## Installation

Using [pip](https://github.com/pypa/pip):

```bash
pip install toml-rs
```

Using [uv](https://github.com/astral-sh/uv):

```bash
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

2. Supports serialization (`toml_rs.dumps` and `toml_rs.dump`)

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
