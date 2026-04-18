#!/usr/bin/env bash
set -euo pipefail

export RUST_HOST
RUST_HOST="$(rustc --print host-tuple)"
shopt -s nullglob
read -r -a interpreters <<<"$INPUTS_INTERPRETER"

wheel_pattern() {
  local version="$1"
  if [[ "$version" == pypy* ]]; then
    local pypy_minor="${version#pypy}"
    local compact="${pypy_minor/./}"
    echo "initial-wheel/*-pp${compact}-*.whl"
    return
  fi

  local compact="${version//./}"
  if [[ "$version" == *t ]]; then
    compact="${compact%t}"
    echo "initial-wheel/*-cp${compact}-cp${compact}t-*.whl"
  else
    echo "initial-wheel/*-cp${compact}-cp${compact}-*.whl"
  fi
}

for version in "${interpreters[@]}"; do
  safe_version="${version//./_}"
  venv_dir=".pgo-venv/${safe_version}"
  rm -rf "$venv_dir"
  uv venv "$venv_dir" --python "$version"

  if [[ "$RUNNER_OS" == "Windows" ]]; then
    pgo_python="$venv_dir/Scripts/python.exe"
  else
    pgo_python="$venv_dir/bin/python"
  fi

  pattern="$(wheel_pattern "$version")"
  mapfile -t wheels < <(compgen -G "$pattern")
  if [[ "${#wheels[@]}" -ne 1 ]]; then
    echo "Expected exactly one wheel for ${version}, found ${#wheels[@]} using pattern: ${pattern}" >&2
    ls -lh initial-wheel
    exit 1
  fi

  uv pip install --python "$pgo_python" --force-reinstall --no-deps "${wheels[0]}"
  "$pgo_python" benchmark/pgo.py
done

sysroot="$(rustc --print sysroot)"
llvm_profdata="$sysroot/lib/rustlib/$RUST_HOST/bin/llvm-profdata"
printf 'LLVM_PROFDATA=%s\n' "$llvm_profdata" >> "$GITHUB_ENV"
