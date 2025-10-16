# `toml-rs` — A High-Performance TOML Parser for Python

## Features

• Fast parsing using [`Rust’s toml crate`](https://github.com/toml-rs/toml)

• Drop-in compatibility with most [`tomllib`](https://docs.python.org/3/library/tomllib.html) use cases

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
