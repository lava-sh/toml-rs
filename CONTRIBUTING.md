# Contributing

## Getting started

1. Fork [toml-rs](https://github.com/lava-sh/toml-rs)

2. Clone your fork

via [git](https://git-scm.com/install):

```bash
git clone https://github.com/<USERNAME>/toml-rs.git
cd toml-rs
```

via [GitHub CLI](https://cli.github.com):

```bash
gh repo clone <USERNAME>/toml-rs
cd toml-rs
```

3. Create and activate [virtual environment](https://docs.python.org/3/library/venv.html):

on Linux / MacOS:

```bash
python3 -m venv .venv  # or uv venv .venv --seed
source .venv/bin/activate
```

on Windows:

```bash
py -m venv .venv  # or uv venv .venv --seed
.venv\scripts\activate
```

4. Install development dependencies and project itself:

via pip:

```bash
uv pip install -e . --group dev
```

via [uv](https://github.com/astral-sh/uv):

```text
uv pip install -e . --group dev
```

## Running linters

We use [ruff](https://github.com/astral-sh/ruff) to check code. To run it do:

```bash
ruff check
```

We use [rumdl](https://github.com/rvben/rumdl) to lint Markdown files. To run it do:

```bash
rumdl check .
```

## Running type checker

We use [ty](https://github.com/astral-sh/ty) to check types. To run it do:

```bash
ty check
```

## Running tests

We use [pytest](https://github.com/pytest-dev/pytest) for tests. To run it do:

```bash
pytest tests/
```

## Running security audit for GitHub Actions

We use [zizmor](https://github.com/zizmorcore/zizmor) to audit our GitHub Actions workflows for security issues. To run it do:

```bash
zizmor .github/
```

## Running spell check

We use [typos](https://github.com/crate-ci/typos) to check our code for spelling mistakes. To run it locally:

```bash
typos
```
