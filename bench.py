import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EXAMPLE = ROOT / "tests" / "data" / "example.toml"
PYTHON_VERSION = os.environ["PYTHON_VERSION"]

PYTHONS = {
    "default": ROOT / f".venv-{PYTHON_VERSION}-default" / "bin" / "python",
    "mimalloc": ROOT / f".venv-{PYTHON_VERSION}-mimalloc" / "bin" / "python",
    "snmalloc": ROOT / f".venv-{PYTHON_VERSION}-snmalloc" / "bin" / "python",
    "jemalloc": ROOT / f".venv-{PYTHON_VERSION}-jemalloc" / "bin" / "python",
}

SAMPLES = 15
RUNS = 10_000
WARMUP = 2_000


BENCH_CODE = r"""
import pathlib
import resource
import statistics
import sys
import time

import toml_rs


def get_memory():
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

    if sys.platform == "darwin":
        return value

    return value * 1024


def percentile(values, percentile):
    values = sorted(values)

    index = (len(values) - 1) * percentile
    lower = int(index)
    upper = min(lower + 1, len(values) - 1)

    fraction = index - lower

    return (
        values[lower]
        + (values[upper] - values[lower]) * fraction
    )


def main():
    data = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")

    for _ in range(%d):
        toml_rs.loads(data)

    samples = []
    peak_memory = get_memory()

    for _ in range(%d):
        start = time.perf_counter_ns()

        for _ in range(%d):
            toml_rs.loads(data)

        elapsed = time.perf_counter_ns() - start

        samples.append(elapsed)
        peak_memory = max(peak_memory, get_memory())

    mean = statistics.mean(samples)
    median = statistics.median(samples)
    stdev = statistics.stdev(samples)

    print(
        median,
        mean,
        stdev,
        min(samples),
        max(samples),
        percentile(samples, 0.90),
        percentile(samples, 0.95),
        percentile(samples, 0.99),
        peak_memory,
    )


if __name__ == "__main__":
    main()
""" % (WARMUP, SAMPLES, RUNS)


def run_bench(python: Path) -> tuple[float, ...]:
    result = subprocess.run(
        [
            str(python),
            "-c",
            BENCH_CODE,
            str(EXAMPLE),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )

    return tuple(float(value) for value in result.stdout.split())


def format_time(ns: float) -> str:
    return f"{ns / 1_000_000_000:.4f}s"


def format_memory(value: float) -> str:
    return f"{value / 1024 / 1024:.2f} MB"


def format_ratio(value: float) -> str:
    return f"{value:.2f}x"


def print_header() -> None:
    print()
    print("╭──────────────────────────────────────────────────────────────╮")
    print("│                     TOML-RS BENCHMARK                       │")
    print("╰──────────────────────────────────────────────────────────────╯")
    print()

    print(f"  Platform     : {platform.system()} {platform.release()}")
    print(f"  Architecture : {platform.machine()}")
    print(f"  CPU          : {platform.processor() or 'unknown'}")
    print(f"  CPUs         : {os.cpu_count()}")
    print(f"  Python       : {PYTHON_VERSION}")
    print(f"  File         : {EXAMPLE}")
    print(f"  File size    : {EXAMPLE.stat().st_size / 1024:.2f} KiB")
    print(f"  Samples      : {SAMPLES:,}")
    print(f"  Runs/sample  : {RUNS:,}")
    print(f"  Warmup       : {WARMUP:,}")
    print()


def print_table(results: dict[str, tuple[float, ...]]) -> None:
    baseline = results["default"][0]

    rows = []

    for name, values in results.items():
        (
            median,
            mean,
            stdev,
            minimum,
            maximum,
            p90,
            p95,
            p99,
            memory,
        ) = values

        rows.append(
            (
                name,
                format_time(median),
                format_ratio(baseline / median),
                format_time(minimum),
                format_time(mean),
                format_time(stdev),
                format_time(p90),
                format_time(p95),
                format_time(p99),
                format_memory(memory),
            ),
        )

    headers = (
        "allocator",
        "median",
        "vs default",
        "min",
        "mean",
        "stdev",
        "p90",
        "p95",
        "p99",
        "peak RSS",
    )

    widths = [
        max(
            len(headers[i]),
            *(len(row[i]) for row in rows),
        )
        for i in range(len(headers))
    ]

    print("  ┌" + "┬".join("─" * (width + 2) for width in widths) + "┐")

    print(
        "  │"
        + "│".join(
            f" {header:^{width}} "
            for header, width in zip(headers, widths)
        )
        + "│",
    )

    print("  ├" + "┼".join("─" * (width + 2) for width in widths) + "┤")

    for row in rows:
        print(
            "  │"
            + "│".join(
                (
                    f" {value:<{width}} "
                    if i == 0
                    else f" {value:>{width}} "
                )
                for i, (value, width) in enumerate(zip(row, widths))
            )
            + "│",
        )

    print("  └" + "┴".join("─" * (width + 2) for width in widths) + "┘")


def print_summary(results: dict[str, tuple[float, ...]]) -> None:
    baseline = results["default"][0]

    fastest = min(
        results.items(),
        key=lambda item: item[1][0],
    )

    lowest_memory = min(
        results.items(),
        key=lambda item: item[1][8],
    )

    most_stable = min(
        results.items(),
        key=lambda item: item[1][2],
    )

    print()
    print("╭────────────────────── SUMMARY ──────────────────────────────╮")
    print("│")

    print(
        f"│  Fastest       : {fastest[0]} "
        f"({format_time(fastest[1][0])}, "
        f"{baseline / fastest[1][0]:.2f}x default)",
    )

    print(
        f"│  Lowest memory : {lowest_memory[0]} "
        f"({format_memory(lowest_memory[1][8])})",
    )

    print(
        f"│  Most stable   : {most_stable[0]} "
        f"(stdev {format_time(most_stable[1][2])})",
    )

    print("│")

    for name, values in results.items():
        median = values[0]
        memory = values[8]

        delta = ((median / baseline) - 1) * 100

        if name == "default":
            print("│  default       : baseline")
        elif delta < 0:
            print(
                f"│  {name:<14}: "
                f"{abs(delta):.2f}% faster than default",
            )
        else:
            print(
                f"│  {name:<14}: "
                f"{delta:.2f}% slower than default",
            )

        print(
            f"│                  "
            f"time={format_time(median)}, "
            f"RSS={format_memory(memory)}",
        )

    print("│")
    print("╰────────────────────────────────────────────────────────────╯")


def main() -> None:
    print_header()

    results = {}

    for name, python in PYTHONS.items():
        if not python.exists():
            raise SystemExit(
                f"Python interpreter not found: {python}",
            )

        print(
            f"  Benchmarking {name:<10} ...",
            flush=True,
        )

        start = time.perf_counter()

        values = run_bench(python)

        wall = time.perf_counter() - start

        results[name] = values

        print(
            f"  done ({wall:.2f}s)",
            flush=True,
        )

    print()
    print_table(results)
    print_summary(results)


if __name__ == "__main__":
    main()