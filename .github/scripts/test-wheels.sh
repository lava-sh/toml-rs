#!/usr/bin/env bash
set -uo pipefail

failed=0

read -ra interpreters <<< "$_PYTHON_INTERPRETERS"

for interpreter in "${interpreters[@]}"; do
    venv=".venv-$interpreter"

    uv venv "$venv" --python "$interpreter"

    if [[ -d "$venv/Scripts" ]]; then
        source "$venv/Scripts/activate"
    else
        source "$venv/bin/activate"
    fi

    uv pip install --group nox

    if ! nox --parallel 4; then
        failed=1
    fi

    deactivate
done

exit "$failed"