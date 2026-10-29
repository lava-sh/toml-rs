#!/usr/bin/env bash
set -euo pipefail

: "${PYTHON_VERSIONS:?PYTHON_VERSIONS is required}"
: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"

paths=""
for v in $PYTHON_VERSIONS; do
  if [[ $v == pypy* ]]; then
    req="pypy@${v#pypy}"
  else
    req="cpython@$v"
  fi
  uv python install "$req"
  paths+="\"$(uv python find --managed-python "$req")\" "
done

echo "paths=${paths% }" >> "$GITHUB_OUTPUT"