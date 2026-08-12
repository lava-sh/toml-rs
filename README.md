<!-- rumdl-disable MD036 MD041 -->
<div align="center">

# toml-rs

_A High-Performance TOML v1.0.0 / v1.1.0 parser for Python written in Rust_
<!-- rumdl-enable MD036 MD041 -->

[![PyPI version][pypi-version-badge]][pypi]
[![PyPI downloads][pypi-downloads-badge]][pypistats]
[![PyPI requires python][pypi-requires-python-badge]][pypi]

<a href="https://github.com/lava-sh/toml-rs/actions?query=branch%3Amain"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&animate=pulse&mode=dark"><img alt="CI" src="https://shieldcn.dev/github/ci/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&animate=pulse&mode=light"></picture></a>
<a href="https://github.com/lava-sh/toml-rs/commits/main"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&mode=dark"><img alt="Last Commit" src="https://shieldcn.dev/github/last-commit/lava-sh/toml-rs.svg?variant=outline&font=geist&size=xs&mode=light"></picture></a>
<a href="https://github.com/lava-sh/toml-rs/blob/main/UNLICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/lava-sh/toml-rs/license.svg?variant=outline&font=geist&size=xs&mode=dark"><img alt="License" src="https://shieldcn.dev/github/lava-sh/toml-rs/license.svg?variant=outline&font=geist&size=xs&mode=light"></picture></a>

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

toml = "x = "

print(tomllib.loads(toml))
```

<pre><code>Traceback (most recent call last):
  File <span style="color:#ff00ff">&quot;toml-rs\scripts\readme_traceback.py&quot;</span>, line <span style="color:#ff00ff">27</span>, in <span style="color:#ff00ff">get_traceback</span>
    <span style="color:#ff0000">loader</span><span style="color:#ff0000;font-weight:bold">(&quot;x = &quot;)</span>
    <span style="color:#ff0000">~~~~~~</span><span style="color:#ff0000;font-weight:bold">^^^^^^^^</span>
  File <span style="color:#ff00ff">&quot;cpython-3.14.4-windows-x86_64-none\Lib\tomllib\_parser.py&quot;</span>, line <span style="color:#ff00ff">174</span>, in <span style="color:#ff00ff">loads</span>
    pos = key_value_rule(src, pos, out, header, parse_float)
  File <span style="color:#ff00ff">&quot;cpython-3.14.4-windows-x86_64-none\Lib\tomllib\_parser.py&quot;</span>, line <span style="color:#ff00ff">403</span>, in <span style="color:#ff00ff">key_value_rule</span>
    pos, key, value = <span style="color:#ff0000">parse_key_value_pair</span><span style="color:#ff0000;font-weight:bold">(src, pos, parse_float)</span>
                      <span style="color:#ff0000">~~~~~~~~~~~~~~~~~~~~</span><span style="color:#ff0000;font-weight:bold">^^^^^^^^^^^^^^^^^^^^^^^</span>
  File <span style="color:#ff00ff">&quot;cpython-3.14.4-windows-x86_64-none\Lib\tomllib\_parser.py&quot;</span>, line <span style="color:#ff00ff">446</span>, in <span style="color:#ff00ff">parse_key_value_pair</span>
    pos, value = <span style="color:#ff0000">parse_value</span><span style="color:#ff0000;font-weight:bold">(src, pos, parse_float)</span>
                 <span style="color:#ff0000">~~~~~~~~~~~</span><span style="color:#ff0000;font-weight:bold">^^^^^^^^^^^^^^^^^^^^^^^</span>
  File <span style="color:#ff00ff">&quot;cpython-3.14.4-windows-x86_64-none\Lib\tomllib\_parser.py&quot;</span>, line <span style="color:#ff00ff">728</span>, in <span style="color:#ff00ff">parse_value</span>
    raise TOMLDecodeError(&quot;Invalid value&quot;, src, pos)
<span style="color:#ff00ff;font-weight:bold">tomllib.TOMLDecodeError</span>: <span style="color:#ff00ff">Invalid value (at end of document)</span>
</code></pre>

<div align="center">

### vs

</div>

```python
import toml_rs

toml = "x = "

print(toml_rs.loads(toml))
``` 

<pre><code>Traceback (most recent call last):
  File <span style="color:#ff00ff">&quot;toml-rs\scripts\readme_traceback.py&quot;</span>, line <span style="color:#ff00ff">27</span>, in <span style="color:#ff00ff">get_traceback</span>
    <span style="color:#ff0000">loader</span><span style="color:#ff0000;font-weight:bold">(&quot;x = &quot;)</span>
    <span style="color:#ff0000">~~~~~~</span><span style="color:#ff0000;font-weight:bold">^^^^^^^^</span>
  File <span style="color:#ff00ff">&quot;toml-rs\.venv\Lib\site-packages\toml_rs\_lib.py&quot;</span>, line <span style="color:#ff00ff">46</span>, in <span style="color:#ff00ff">loads</span>
    return _loads(str_obj, parse_float=parse_float, toml_version=toml_version)
<span style="color:#ff00ff;font-weight:bold">toml_rs._lib.TOMLDecodeError</span>: <span style="color:#ff00ff">TOML parse error at line 1, column 5
  |
1 | x =
  |     ^
string values must be quoted, expected literal string</span>
</code></pre>

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
[pypi-downloads-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypistats.org%2Fapi%2Fpackages%2Ftoml-rs%2Frecent&query=%24.data.last_month&suffix=%2Fmonth&size=xs&mode=light&logo=python&logoColor=ffffff&label=downloads&color=3775A9
[pypi-requires-python-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypi.org%2Fpypi%2Ftoml-rs%2Fjson&query=%24.info.requires_python&size=xs&mode=light&logo=python&logoColor=ffffff&label=requires+python&color=3775A9
[contributors-badge]: https://shieldcn.dev/contributors/lava-sh/toml-rs.svg?title=false&theme=slate&size=80&bots=true&titleAlign=center&mode=light&font=geist&border=false&image=https%3A%2F%2Fimages.wallpaperscraft.ru%2Fimage%2Fsingle%2Foblaka_nebo_ogni_1647475_3840x2400.jpg&overlay=0.3
