# PGO Bug Report: maturin v1.13.1 interpreter mismatch in Docker containers

## Summary

When building wheels with `--pgo` and multiple interpreters inside manylinux/musllinux Docker containers,
maturin v1.13.1 creates the instrumentation venv with the **first available Python** in the container
(e.g. CPython 3.10), then installs a wheel built for a **different interpreter** (e.g. CPython 3.11).
This causes `ModuleNotFoundError: No module named 'toml_rs._toml_rs'` due to ABI mismatch
(`cp311` wheel installed into `cp310` venv).

## Reproduction

Run inside manylinux/musllinux Docker container:

```bash
maturin build --release --pgo --interpreter "3.10 3.11 3.12 3.13 3.14"
```

The first PGO cycle (CPython 3.10) succeeds. The second cycle (CPython 3.11) fails:

```
📊 [2/5] PGO cycle for CPython 3.11...
  🔬 Phase 2/3: Running PGO instrumentation...
📦 Installing instrumented wheel into temporary venv...
 + toml-rs==0.3.11 (from .../toml_rs-0.3.11-cp311-cp311-manylinux_2_17_x86_64.whl)
🏃 Running instrumentation command: python benchmark/pgo.py
ModuleNotFoundError: No module named 'toml_rs._toml_rs'
```

## Root Cause

In `src/pgo.rs`, `run_instrumentation()` creates a venv using the **first Python interpreter**
found in the container rather than the interpreter that matches the wheel being installed.

On Windows/macOS (native execution), `setup-python` action installs all requested Python versions
and maturin correctly matches interpreters. Inside Docker containers, the issue occurs because:

1. Manylinux containers ship with multiple Python versions pre-installed
2. `find_uv_python()` or the fallback `python -m venv` may resolve to a different interpreter
   than the one used to build the current wheel
3. The installed wheel's ABI tag (e.g. `cp311`) doesn't match the venv's Python ABI (e.g. `cp310`)

## Expected Behavior

Each PGO cycle should create a venv using the **same interpreter** that was used to build the wheel
in Phase 1.

## Environment

- maturin: 1.13.1
- manylinux: 2014 / musllinux_1_2
- OS: Docker containers (quay.io/pypa/manylinux2014_*, quay.io/pypa/musllinux_1_2_*)
- Works correctly on: Windows (native), macOS (native)

## Workaround

Build wheels without `--pgo` flag, or use `--pgo` with a single interpreter per maturin invocation.
