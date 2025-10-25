import platform
import sys
import time
from collections.abc import Callable
from importlib.metadata import version
from pathlib import Path

import altair as alt
import cpuinfo
import polars as pl
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

N = 10_000


def get_lib_version(lib: str) -> str:
    if lib == "tomllib":
        return "built-in"
    return version(lib)


def benchmark(func: Callable, count: int) -> float:
    start = time.perf_counter()
    for _ in range(count):
        func()
    end = time.perf_counter()
    return end - start


def run(run_count: int) -> None:
    file_path = Path(__file__).resolve().parent
    path = file_path.parent / "tests" / "data" / "example.toml"
    data = path.read_bytes().decode()
    fixed_data = data.replace("\r\n", "\n")

    parsers = {
        "toml_rs": lambda: toml_rs.loads(data),
        "rtoml": lambda: rtoml.loads(data),
        "pytomlpp": lambda: pytomlpp.loads(data),
        "tomllib": lambda: tomllib.loads(data),
        "toml": lambda: toml.loads(data),
        "qtoml": lambda: qtoml.loads(fixed_data),
        "tomlkit": lambda: tomlkit.parse(data),
    }

    results = {name: benchmark(func, run_count) for name, func in parsers.items()}

    df = (
        pl.DataFrame({
            "parser": [f"{name} ({get_lib_version(name)})" for name in results],
            "exec_time": list(results.values()),
        })
        .sort("exec_time")
        .with_columns([
            (pl.col("exec_time") / pl.col("exec_time").min()).alias("slowdown"),
        ])
    )

    chart = (
        alt.Chart(df)
        .mark_bar(cornerRadiusTopLeft=6, cornerRadiusTopRight=6)
        .encode(
            x=alt.X(
                "parser:N",
                sort=None,
                title="Libraries",
                axis=alt.Axis(labelAngle=0),
            ),
            y=alt.Y(
                "exec_time:Q",
                title="Execution Time (seconds, lower=better)",
                scale=alt.Scale(domain=(0, df["exec_time"].max() * 1.1)),
            ),
            color=alt.Color(
                "parser:N",
                legend=None,
                scale=alt.Scale(scheme="dark2"),
            ),
            tooltip=[
                alt.Tooltip("parser:N", title=""),
                alt.Tooltip("exec_time:Q", title="Execution Time (s)", format=".4f"),
                alt.Tooltip("slowdown:Q", title="Slowdown", format=".2f"),
            ],
        )
    )
    text = (
        chart.mark_text(
            align="center",
            baseline="bottom",
            dy=-2,
            fontSize=9,
            fontWeight="bold",
        )
        .transform_calculate(
            label='format(datum.exec_time, ".4f") + '
            '"s (x" + format(datum.slowdown, ".2f") + ")"',
        )
        .encode(text="label:N")
    )
    os = f"{platform.system()} {platform.release()}"
    cpu = cpuinfo.get_cpu_info()["brand_raw"]
    py = platform.python_version()
    (chart + text).properties(
        width=600,
        height=400,
        title={
            "text": "TOML parsers benchmark",
            "subtitle": f"Python: {py} ({os}) | CPU: {cpu}",
        },
    ).save(file_path / "benchmark.svg")


if __name__ == "__main__":
    run(N)
