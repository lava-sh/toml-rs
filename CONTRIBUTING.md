# Contributing

## Getting started

1. Fork [toml-rs](https://github.com/lava-sh/toml-rs)

2. Clone your fork

<p>
  <img
    src="https://thesvg.org/icons/git/default.svg"
    alt="Python"
    height="14"
  />
  using <a href="https://git-scm.com/install">git</a>:
</p>

```console
git clone https://github.com/<USERNAME>/toml-rs.git
cd toml-rs
```

<p>
  <img
    src="https://thesvg.org/icons/refined-github/default.svg"
    alt="GitHub"
    height="14"
  />
  using <a href="https://cli.github.com">GitHub CLI</a>:
</p>

```console
gh repo clone <USERNAME>/toml-rs
cd toml-rs
```

3. Create and activate a [virtual environment](https://docs.python.org/3/library/venv.html):

<p>
  <span style="white-space: nowrap;">
    <img
      src="https://thesvg.org/icons/linux/default.svg"
      alt="linux"
      height="14"
    />
    Linux /
    <picture>
      <source
        media="(prefers-color-scheme: dark)"
        srcset="https://thesvg.org/icons/apple/default.svg"
      />
      <img
        src="https://thesvg.org/icons/apple/mono.svg"
        alt="macos"
        height="14"
      />
    </picture>
    MacOS:
  </span>
</p>

```console
python3 -m venv .venv
# or uv venv .venv --seed

source .venv/bin/activate
```

<p>
  <img
    src="https://thesvg.org/icons/windows/default.svg"
    alt="windows"
    height="14"
  />
  Windows:
</p>

```console
py -m venv .venv
# or uv venv .venv --seed

.venv\scripts\activate
```

4. Install development dependencies and the project itself:

<p>
  <img
    src="https://thesvg.org/icons/python/default.svg"
    alt="Python"
    height="14"
  />
  using <a href="https://github.com/pypa/pip">pip</a>:
</p>

```console
pip install -e . --group dev
```

<p>
  <img
    src="https://thesvg.org/icons/uv/default.svg"
    alt="uv"
    height="14"
  />
  using <a href="https://github.com/astral-sh/uv">uv</a>:
</p>

```console
uv pip install -e . --group dev
```

## Running linters, code formatters, type checkers

### Python

We use [ruff](https://github.com/astral-sh/ruff) to lint Python code. To run it, do:

```console
ruff check
```

We use [ty](https://github.com/astral-sh/ty) to check Python types. To run it, do:

```console
ty check
```

<h3>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://thesvg.org/icons/rust/dark.svg">
    <img src="https://thesvg.org/icons/rust/light.svg" alt="Rust" height="16">
  </picture>
  Rust
</h3>

We use [rustfmt](https://github.com/rust-lang/rustfmt) to format Rust code. To run it, do:

```console
cargo fmt-nightly
```

We use [clippy](https://github.com/rust-lang/rust-clippy) to lint Rust code. To run it, do:

```console
cargo clippy
```

### Markdown

We use [rumdl](https://github.com/rvben/rumdl) to lint Markdown files. To run it, do:

```console
rumdl check
```

## Running tests

We use [nox](https://github.com/wntrblm/nox) for tests. To run it, do:

```console
nox
```

## Running security audit for GitHub Actions

We use [zizmor](https://github.com/zizmorcore/zizmor) to audit our
GitHub Actions workflows for security issues. To run it, do:

```console
zizmor .github/
```

## Running spell check

We use [typos](https://github.com/crate-ci/typos) to check our code for spelling mistakes. To run it, do:

```console
typos
```

## Building with alternative allocators

Project supports compilation with alternative memory allocators,
such as [mimalloc](https://github.com/microsoft/mimalloc/tree/dev3).

[Mimalloc](https://github.com/microsoft/mimalloc/tree/dev3) is used where possible.

Building with [mimalloc](https://github.com/microsoft/mimalloc/tree/dev3):

```console
maturin develop --release --features mimalloc
```

Building with the default (system) allocator:

```console
maturin develop --release
```
