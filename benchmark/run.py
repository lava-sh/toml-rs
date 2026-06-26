import platform
import time
from collections.abc import Callable
from importlib.metadata import version
from pathlib import Path
from pprint import pprint

import altair as alt
import cpuinfo
import polars as pl
import pytomlpp
import qtoml
import rtoml
import toml
import toml_rs
import tomli as tomllib
import tomli_w
import tomlkit

N = 500

CPU_INFO = cpuinfo.get_cpu_info()
pprint({k: v for k, v in CPU_INFO.items() if k != "flags"})
print()
PY_VERSION = f"{platform.python_version()} ({platform.system()} {platform.release()})"
print(PY_VERSION)


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


def plot_benchmark(
    results: dict[str, float],
    run_type: str,
    save_path: Path,
) -> None:
    df = pl.DataFrame({
        "parser": list(results.keys()),
        "exec_time": list(results.values()),
    }).sort("exec_time")

    df = df.with_columns(
        slowdown=(pl.col("exec_time") / pl.col("exec_time").min()),
        parser_label=pl.col("parser").map_elements(
            lambda parser: f"{parser}\n{get_lib_version(parser.split()[0])}",
            return_dtype=pl.String,
        ),
    )

    max_time = df.select(pl.max("exec_time")).item()

    chart = (
        alt
        .Chart(df)
        .mark_bar(cornerRadiusTopLeft=6, cornerRadiusTopRight=6)
        .encode(
            x=alt.X(
                "parser_label:N",
                sort=None,
                title="Parser",
                axis=alt.Axis(
                    labelAngle=0,
                    labelExpr="split(datum.label, '\\n')",
                    labelLineHeight=14,
                ),
            ),
            y=alt.Y(
                "exec_time:Q",
                title="Execution Time (seconds, lower=better)",
                scale=alt.Scale(domain=(0, max_time * 1.05)),
                axis=alt.Axis(grid=False),
            ),
            color=alt.Color("parser:N", legend=None, scale=alt.Scale(scheme="dark2")),
            tooltip=[
                alt.Tooltip("parser:N", title=""),
                alt.Tooltip("exec_time:Q", title="Execution Time (s)", format=".4f"),
                alt.Tooltip("slowdown:Q", title="Slowdown", format=".2f"),
            ],
        )
    )

    text = (
        chart
        .mark_text(
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

    (chart + text).properties(
        width=800,
        height=600,
        title={
            "text": f"TOML parsers benchmark ({run_type})",
            "subtitle": f"Python: {PY_VERSION} | CPU: {CPU_INFO['brand_raw']}",
        },
    ).save(save_path)


file = Path(__file__).resolve().parent
example_toml = file.parent / "tests" / "data" / "example.toml"
data = example_toml.read_bytes().decode()
fixed_data = data.replace("\r\n", "\n")

obj = tomllib.loads(example_toml.read_text())


def run(run_count: int) -> None:
    loads = {
        "toml_rs": lambda: toml_rs.loads(data, toml_version="1.1.0"),
        "rtoml": lambda: rtoml.loads(data),
        "pytomlpp": lambda: pytomlpp.loads(data),
        "tomllib": lambda: tomllib.loads(data),
        "toml": lambda: toml.loads(data),
        "qtoml": lambda: qtoml.loads(fixed_data),
        "tomlkit": lambda: tomlkit.parse(data),
    }
    dumps = {
        "toml_rs": lambda: toml_rs.dumps(obj, toml_version="1.1.0"),
        "rtoml": lambda: rtoml.dumps(obj),
        "pytomlpp": lambda: pytomlpp.dumps(obj),
        "toml": lambda: toml.dumps(obj),
        "qtoml": lambda: qtoml.dumps(obj),
        "tomlkit": lambda: tomlkit.dumps(obj),
        "tomli-w": lambda: tomli_w.dumps(obj),
    }
    loads = {name: benchmark(func, run_count) for name, func in loads.items()}
    dumps = {name: benchmark(func, run_count) for name, func in dumps.items()}
    plot_benchmark(loads, run_type="loads", save_path=file / "loads.svg")
    plot_benchmark(dumps, run_type="dumps", save_path=file / "dumps.svg")


if __name__ == "__main__":
    run(N)
