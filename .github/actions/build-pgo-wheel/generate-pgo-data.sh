#!/usr/bin/env bash
set -euo pipefail

export RUST_HOST
RUST_HOST="$(rustc --print host-tuple)"
shopt -s nullglob
read -r -a interpreters <<<"$INPUTS_INTERPRETER"

uv_request() {
  local version="$1"
  local os arch libc

  case "$RUNNER_OS" in
    Windows)
      os="windows"
      libc="none"
      case "$INPUTS_PYTHON_ARCH" in
        x64) arch="x86_64" ;;
        x86) arch="x86" ;;
        arm64) arch="aarch64" ;;
        *)
          echo "Unsupported Windows python arch: $INPUTS_PYTHON_ARCH" >&2
          exit 1
          ;;
      esac
      ;;
    Linux)
      os="linux"
      libc="gnu"
      case "$INPUTS_TARGET" in
        x86_64) arch="x86_64" ;;
        x86) arch="x86" ;;
        aarch64) arch="aarch64" ;;
        armv7) arch="armv7" ;;
        s390x) arch="s390x" ;;
        ppc64le) arch="powerpc64le" ;;
        riscv64) arch="riscv64" ;;
        *)
          echo "Unsupported Linux target: $INPUTS_TARGET" >&2
          exit 1
          ;;
      esac
      ;;
    macOS)
      os="macos"
      libc="none"
      case "$INPUTS_TARGET" in
        x86_64) arch="x86_64" ;;
        aarch64) arch="aarch64" ;;
        universal2) arch="x86_64" ;;
        *)
          echo "Unsupported macOS target: $INPUTS_TARGET" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "Unsupported runner OS: $RUNNER_OS" >&2
      exit 1
      ;;
  esac

  if [[ "$version" == pypy* ]]; then
    echo "pypy-${version#pypy}-${os}-${arch}-${libc}"
    return
  fi

  if [[ "$version" == *t ]]; then
    echo "cpython-${version%t}+freethreaded-${os}-${arch}-${libc}"
  else
    echo "cpython-${version}-${os}-${arch}-${libc}"
  fi
}

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
  request="$(uv_request "$version")"
  rm -rf "$venv_dir"
  python_path="$(uv python find --no-project "$request" 2> /dev/null || true)"
  if [[ -z "$python_path" ]]; then
    uv python install "$request"
    python_path="$(uv python find --no-project "$request")"
  fi

  uv venv "$venv_dir" --python "$python_path"

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
