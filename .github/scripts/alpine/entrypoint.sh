#!/bin/sh
set -e

. "$HOME/.cargo/env"

cd /app

uv pip install --group maturin --system

maturin build --out dist --features mimalloc

uv pip install dist/*.whl --system

python -c 'import toml_rs; print(toml_rs.__version__)'