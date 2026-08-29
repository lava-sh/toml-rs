#!/usr/bin/env bash
set -uo pipefail

failed=0

read -ra interpreters <<< "$INTERPRETERS"

for interpreter in "${interpreters[@]}"; do
    venv=".venv-$interpreter"

    uv venv "$venv" --python "$interpreter"
    source "$venv/bin/activate"

    uv pip install --group nox

    if ! nox --parallel 4; then
        echo "❌ Tests failed for $interpreter"
        failed=1
    fi

    deactivate
done

exit "$failed"