import argparse
import os
import shutil
import subprocess
from pathlib import Path


def run(*args: str, env: dict[str, str] | None = None) -> None:
    subprocess.run(args, check=True, env=env)


def capture(*args: str) -> str:
    return subprocess.check_output(args, text=True).strip()


def find_python_in_toolcache(version: str, python_arch: str) -> str | None:
    is_pypy = version.startswith("pypy")
    is_freethreaded = version.endswith("t") and not is_pypy
    version_prefix = version.removeprefix("pypy").removesuffix("t")
    arch_dir = f"{python_arch}-freethreaded" if is_freethreaded else python_arch
    family = "PyPy" if is_pypy else "Python"
    roots = []

    runner_tool_cache = os.environ.get("RUNNER_TOOL_CACHE")
    if runner_tool_cache:
        roots.append(Path(runner_tool_cache) / family)

    python_location = os.environ.get("PYTHONLOCATION")
    if python_location:
        location_path = Path(python_location)
        if location_path.parent.name == family:
            roots.append(location_path.parent)

    for root in roots:
        if not root.exists():
            continue

        for version_dir in sorted(root.glob(f"{version_prefix}*"), reverse=True):
            candidate = version_dir / arch_dir / "python.exe"
            if candidate.exists():
                return str(candidate)

    return None


def find_python(version: str, python_arch: str) -> str:
    toolcache_python = find_python_in_toolcache(version, python_arch)
    if toolcache_python is not None:
        return toolcache_python

    launcher_version = f"{version}-32" if python_arch == "x86" else version
    return capture(
        "py",
        f"-{launcher_version}",
        "-c",
        "import sys; print(sys.executable)",
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interpreters", required=True)
    parser.add_argument("--python-arch", required=True)
    parser.add_argument("--llvm-profdata", required=True)
    args = parser.parse_args()

    workspace = Path(os.environ["GITHUB_WORKSPACE"])
    dist_dir = workspace / "dist"
    instrumented_root = workspace / "dist-instrumented"
    dist_dir.mkdir(parents=True, exist_ok=True)
    instrumented_root.mkdir(parents=True, exist_ok=True)

    base_env = os.environ.copy()

    for version in args.interpreters.split():
        safe_version = version.replace(".", "_")
        python = find_python(version, args.python_arch)
        pgo_dir = workspace / "pgo-data" / safe_version
        instrumented_dir = instrumented_root / safe_version
        venv_dir = workspace / ".pgo-venv" / safe_version
        merged_profdata = pgo_dir / "merged.profdata"

        pgo_dir.mkdir(parents=True, exist_ok=True)
        instrumented_dir.mkdir(parents=True, exist_ok=True)
        if venv_dir.exists():
            shutil.rmtree(venv_dir)

        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-generate={pgo_dir}"
        run(
            "maturin",
            "build",
            "--release",
            "--out",
            str(instrumented_dir),
            "--interpreter",
            python,
            "--features",
            "mimalloc",
            "--compatibility",
            "pypi",
            env=env,
        )

        run("uv", "venv", str(venv_dir), "--python", python)
        venv_python = venv_dir / "Scripts" / "python.exe"
        wheel = next(instrumented_dir.glob("*.whl"))

        run(
            "uv",
            "pip",
            "install",
            "--python",
            str(venv_python),
            "--force-reinstall",
            "--no-deps",
            str(wheel),
        )

        env = base_env.copy()
        env["LLVM_PROFILE_FILE"] = str(pgo_dir / "%m_%p.profraw")
        run(str(venv_python), "benchmark/pgo.py", env=env)

        profraw = [str(path) for path in pgo_dir.glob("*.profraw")]
        run(args.llvm_profdata, "merge", "-o", str(merged_profdata), *profraw)

        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-use={merged_profdata}"
        run(
            "maturin",
            "build",
            "--release",
            "--out",
            str(dist_dir),
            "--interpreter",
            python,
            "--features",
            "mimalloc",
            "--compatibility",
            "pypi",
            env=env,
        )


if __name__ == "__main__":
    main()
