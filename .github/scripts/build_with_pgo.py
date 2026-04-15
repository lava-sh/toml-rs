import argparse
import os
import shutil
import subprocess
from pathlib import Path


def run(*args: str, env: dict[str, str] | None = None) -> None:
    subprocess.run(args, check=True, env=env)


def capture(*args: str) -> str:
    return subprocess.check_output(args, text=True).strip()


def uv_request(version: str, python_arch: str) -> str:
    arch = {"x64": "x86_64", "x86": "x86", "arm64": "aarch64"}[python_arch]

    if version.startswith("pypy"):
        return f"pypy-{version.removeprefix('pypy')}-windows-{arch}-none"

    if version.endswith("t"):
        return f"cpython-{version.removesuffix('t')}+freethreaded-windows-{arch}-none"

    return f"cpython-{version}-windows-{arch}-none"


def try_capture(*args: str) -> str | None:
    try:
        return capture(*args)
    except subprocess.CalledProcessError:
        return None


def log(message: str) -> None:
    print(message, flush=True)


def run_logged(*args: str, env: dict[str, str] | None = None) -> None:
    log(f"🛠️ Running: {' '.join(args)}")
    run(*args, env=env)


def phase(version: str, number: int, total: int, message: str) -> None:
    icons = {
        1: "📊",
        2: "📦",
        3: "🔬",
        4: "⚡",
    }
    log(f"  {icons[number]} Phase {number}/{total}: {message}")


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

    python_location = os.environ.get("pythonLocation")
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
    request = uv_request(version, python_arch)
    log(f"🐍 Resolving Python {version} for {python_arch} via uv request: {request}")

    managed_python = try_capture("uv", "python", "find", "--no-project", request)
    if managed_python is not None:
        log(f"🐍 Found Python {version} via uv: {managed_python}")
        return managed_python

    toolcache_python = find_python_in_toolcache(version, python_arch)
    if toolcache_python is not None:
        log(f"🐍 Found Python {version} in toolcache: {toolcache_python}")
        return toolcache_python

    launcher_version = f"{version}-32" if python_arch == "x86" else version
    log(f"🐍 Falling back to py launcher for Python {version}: -{launcher_version}")
    python = capture(
        "py",
        f"-{launcher_version}",
        "-c",
        "import sys; print(sys.executable)",
    )
    log(f"🐍 Found Python {version} via py launcher: {python}")
    return python


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interpreters", required=True)
    parser.add_argument("--python-arch", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--llvm-profdata", required=True)
    args = parser.parse_args()

    workspace = Path(os.environ["GITHUB_WORKSPACE"])
    dist_dir = workspace / "dist"
    instrumented_root = workspace / "dist-instrumented"
    dist_dir.mkdir(parents=True, exist_ok=True)
    instrumented_root.mkdir(parents=True, exist_ok=True)

    base_env = os.environ.copy()
    versions = args.interpreters.split()
    log(f"🚀 Starting manual PGO build for interpreters: {args.interpreters}")
    log(f"🧱 Python architecture: {args.python_arch}")
    log(f"🎯 Rust target: {args.target}")
    log(f"🔗 llvm-profdata: {args.llvm_profdata}")

    for index, version in enumerate(versions, start=1):
        safe_version = version.replace(".", "_")
        log("")
        log(
            f"📊 [{index}/{len(versions)}] PGO cycle for Python {version} ({args.python_arch})..."
        )
        python = find_python(version, args.python_arch)
        pgo_dir = workspace / "pgo-data" / safe_version
        instrumented_dir = instrumented_root / safe_version
        venv_dir = workspace / ".pgo-venv" / safe_version
        merged_profdata = pgo_dir / "merged.profdata"
        log(f"  🐍 Interpreter: {python}")
        log(f"  📁 PGO directory: {pgo_dir}")
        log(f"  📁 Instrumented wheel directory: {instrumented_dir}")
        log(f"  📁 Temporary venv: {venv_dir}")

        pgo_dir.mkdir(parents=True, exist_ok=True)
        instrumented_dir.mkdir(parents=True, exist_ok=True)
        if venv_dir.exists():
            shutil.rmtree(venv_dir)

        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-generate={pgo_dir}"
        phase(version, 1, 4, "Building instrumented wheel...")
        log(f"  🧪 RUSTFLAGS={env['RUSTFLAGS']}")
        run_logged(
            "maturin",
            "build",
            "--release",
            "--out",
            str(instrumented_dir),
            "--target",
            args.target,
            "--interpreter",
            python,
            "--features",
            "mimalloc",
            "--compatibility",
            "pypi",
            env=env,
        )

        phase(version, 2, 4, "Creating venv and installing instrumented wheel...")
        run_logged("uv", "venv", str(venv_dir), "--python", python)
        venv_python = venv_dir / "Scripts" / "python.exe"
        wheel = next(instrumented_dir.glob("*.whl"))
        log(f"  🐍 Venv Python: {venv_python}")
        log(f"  📦 Instrumented wheel: {wheel}")

        run_logged(
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
        phase(version, 3, 4, "Running benchmark/pgo.py...")
        log(f"  🔬 LLVM_PROFILE_FILE={env['LLVM_PROFILE_FILE']}")
        run_logged(str(venv_python), "benchmark/pgo.py", env=env)

        profraw = [str(path) for path in pgo_dir.glob("*.profraw")]
        log(f"  ✅ Collected {len(profraw)} raw profile(s)")
        log(f"  🔗 Merging profiles into: {merged_profdata}")
        run_logged(args.llvm_profdata, "merge", "-o", str(merged_profdata), *profraw)

        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-use={merged_profdata}"
        phase(version, 4, 4, "Building optimized wheel...")
        log(f"  ⚙️ RUSTFLAGS={env['RUSTFLAGS']}")
        run_logged(
            "maturin",
            "build",
            "--release",
            "--out",
            str(dist_dir),
            "--target",
            args.target,
            "--interpreter",
            python,
            "--features",
            "mimalloc",
            "--compatibility",
            "pypi",
            env=env,
        )
        built_wheels = sorted(
            dist_dir.glob("*.whl"),
            key=lambda path: path.stat().st_mtime,
            reverse=True,
        )
        if built_wheels:
            log(f"  📦 Built wheel: {built_wheels[0]}")
        log(f"  ✅ Finished Python {version}")

    log("")
    log(f"🎉 Manual PGO build completed. Wheels are in: {dist_dir}")


if __name__ == "__main__":
    main()
