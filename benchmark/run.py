from __future__ import annotations

import sys
import timeit
from collections.abc import Callable
from pathlib import Path

import pytomlpp
import qtoml
import rtoml
import toml
import toml_rs
import tomlkit

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


def benchmark(
    name: str,
    run_count: int,
    func: Callable,
    col_width: tuple,
) -> float:
    placeholder = "Running..."
    print(f"{name:>{col_width[0]}} | {placeholder}", end="", flush=True)
    time_taken = timeit.timeit(func, number=run_count)
    print("\b" * len(placeholder), end="")
    time_suffix = " s"
    print(f"{time_taken:{col_width[1] - len(time_suffix)}.3g}{time_suffix}")
    return time_taken


def run(run_count: int) -> None:
    data_path = Path(__file__).resolve().parent.parent / "tests" / "data" / "example.toml"
    test_data = data_path.read_bytes().decode()

    # qtoml has a bug making it crash without this newline normalization
    test_data = test_data.replace("\r\n", "\n")

    col_width = (10, 10, 28)
    col_head = ("parser", "exec time", "performance (more is better)")
    print(f"Parsing data.toml {run_count} times:")
    print("-" * col_width[0] + "---" + "-" * col_width[1] + "---" + col_width[2] * "-")
    print(
        f"{col_head[0]:>{col_width[0]}} | {col_head[1]:>{col_width[1]}} | {col_head[2]}"
    )
    print("-" * col_width[0] + "-+-" + "-" * col_width[1] + "-+-" + col_width[2] * "-")

    parsers = {
        "toml_rs": lambda: toml_rs.loads(test_data),
        "rtoml": lambda: rtoml.loads(test_data),
        "pytomlpp": lambda: pytomlpp.loads(test_data),
        "tomllib": lambda: tomllib.loads(test_data),
        "toml": lambda: toml.loads(test_data),
        "qtoml": lambda: qtoml.loads(test_data),
        "tomlkit": lambda: tomlkit.parse(test_data),
    }

    results = {}
    for name, func in parsers.items():
        results[name] = benchmark(name, run_count, func, col_width)
    fastest_name = min(results, key=results.get)
    fastest_time = results[fastest_name]
    print(f"\nFastest parser: {fastest_name} ({fastest_time:.5f} s)\n")

    print("Performance relative to fastest parser:")
    for name, time_taken in results.items():
        delta = fastest_time / time_taken
        print(f"{name:>{col_width[0]}} | {delta:.2%}")


if __name__ == "__main__":
    run(10_000)
