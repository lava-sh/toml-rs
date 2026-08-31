<!-- rumdl-disable MD036 MD041 -->
<div align="center">

# toml-rs

_A High-Performance TOML v1.0.0 / v1.1.0 parser for Python written in Rust_
<!-- rumdl-enable MD036 MD041 -->

[![PyPI version][pypi-version-badge]][pypi]
[![PyPI downloads][pypi-downloads-badge]][pypistats]
[![PyPI requires python][pypi-requires-python-badge]][pypi]
[![PyPI licence][pypi-licence-badge]][pypi]

<a href="https://github.com/lava-sh/toml-rs/actions?query=branch%3Amain"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?workflow=ci.yaml&branch=main&variant=outline&size=xs&animate=pulse&logo=github&label=CI&mode=dark"><img alt="CI" src="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?workflow=ci.yaml&branch=main&variant=outline&size=xs&animate=pulse&mode=light&theme=zinc&logo=github&label=CI"></picture></a>
<a href="https://github.com/lava-sh/toml-rs/commits/main"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&logo=github&mode=dark"><img alt="Last Commit" src="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&mode=light&theme=zinc&logo=github"></picture></a>
</div>

## Features

* The fastest TOML parser in Python (see [benchmarks](https://github.com/lava-sh/toml-rs/tree/main/benchmark))

* Drop-in compatibility with most [`tomllib`][tomllib-docs] use cases
  (see [below](#differences-with-tomllib))

## Installation

<p>
  <img
    src="https://thesvg.org/icons/python/default.svg"
    alt="Python"
    height="14"
  />
  Using <a href="https://github.com/pypa/pip">pip</a>:
</p>

```bash
pip install toml-rs
```

<p>
  <img
    src="https://thesvg.org/icons/uv/default.svg"
    alt="uv"
    height="14"
  />
  Using <a href="https://github.com/astral-sh/uv">uv</a>:
</p>

```bash
uv pip install toml-rs
```

<p>
  <img
    src="https://thesvg.org/icons/poetry/default.svg"
    alt="Poetry"
    height="14"
  />
  Using <a href="https://github.com/python-poetry/poetry">poetry</a>:
</p>

```bash
poetry add toml-rs
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

## Differences with [`tomllib`][tomllib-docs]

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

2. Supports serialization like [tomli-w](https://github.com/hukkin/tomli-w) (`toml_rs.dumps` and `toml_rs.dump`)

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

<div align="center">

## Contributors

[![lava-sh/toml-rs contributors][contributors-badge]][github-contributors]

</div>

[github-contributors]: https://github.com/lava-sh/toml-rs/graphs/contributors

[tomllib-docs]: https://docs.python.org/3/library/tomllib.html

[pypi]: https://pypi.org/project/toml-rs
[pypistats]: https://pypistats.org/packages/toml-rs

[pypi-version-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypi.org%2Fpypi%2Ftoml-rs%2Fjson&query=%24.info.version&variant=branded&size=xs&mode=light&logo=python&label=pypi+version
[pypi-downloads-badge]: https://shieldcn.dev/pypi/dm/toml-rs.svg?variant=branded&size=xs&logo=python&logoColor=ffffff
[pypi-requires-python-badge]: https://shieldcn.dev/pypi/python/toml-rs.svg?variant=branded&size=xs&logo=python&logoColor=ffffff&label=requires+python
[pypi-licence-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypi.org%2Fpypi%2Ftoml-rs%2Fjson&query=%24.info.license_expression&variant=branded&size=xs&mode=light&logo=python&logoColor=ffffff&label=license

[contributors-badge]: https://shieldcn.dev/contributors/lava-sh/toml-rs.svg?title=false&theme=slate&size=80&bots=true&titleAlign=center&mode=light&font=geist&border=false&image=https%3A%2F%2Fimages.wallpaperscraft.ru%2Fimage%2Fsingle%2Foblaka_nebo_ogni_1647475_3840x2400.jpg&overlay=0.3
