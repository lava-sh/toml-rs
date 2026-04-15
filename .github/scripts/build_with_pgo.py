import argparse
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


def run(*args: str, env: dict[str, str] | None = None) -> None:
    subprocess.run(args, check=True, env=env)


def capture(*args: str) -> str:
    return subprocess.check_output(args, text=True).strip()


def try_capture(*args: str) -> str | None:
    try:
        return capture(*args)
    except subprocess.CalledProcessError:
        return None


def log(message: str) -> None:
    try:
        print(message, flush=True)
    except UnicodeEncodeError:
        fallback = message.encode("ascii", errors="replace").decode("ascii")
        print(fallback, flush=True)


def run_logged(*args: str, env: dict[str, str] | None = None) -> None:
    log(f"🛠️ Running: {' '.join(args)}")
    run(*args, env=env)


def phase(number: int, total: int, message: str) -> None:
    icons = {
        1: "📊",
        2: "📦",
        3: "🔬",
        4: "⚡",
    }
    log(f"  {icons[number]} Phase {number}/{total}: {message}")


def rust_target_triple(target: str) -> str:
    return {
        "x64": "x86_64-pc-windows-msvc",
        "x86": "i686-pc-windows-msvc",
        "aarch64": "aarch64-pc-windows-msvc",
    }.get(target, target)


def uv_request(version: str, python_arch: str) -> str:
    arch = {"x64": "x86_64", "x86": "x86", "arm64": "aarch64"}[python_arch]

    if version.startswith("pypy"):
        return f"pypy-{version.removeprefix('pypy')}-windows-{arch}-none"

    if version.endswith("t"):
        return f"cpython-{version.removesuffix('t')}+freethreaded-windows-{arch}-none"

    return f"cpython-{version}-windows-{arch}-none"


def find_python_in_toolcache(version: str, python_arch: str) -> str | None:
    is_pypy = version.startswith("pypy")
    is_freethreaded = version.endswith("t") and not is_pypy
    version_prefix = version.removeprefix("pypy").removesuffix("t")
    arch_dir = f"{python_arch}-freethreaded" if is_freethreaded else python_arch
    family = "PyPy" if is_pypy else "Python"
    roots: list[Path] = []

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


@dataclass(slots=True)
class PgoBuild:
    version: str
    python: str
    python_arch: str
    target: str
    workspace: Path
    dist_dir: Path
    instrumented_root: Path

    @property
    def safe_version(self) -> str:
        return self.version.replace(".", "_")

    @property
    def rust_target(self) -> str:
        return rust_target_triple(self.target)

    @property
    def pgo_dir(self) -> Path:
        return self.workspace / "pgo-data" / self.safe_version

    @property
    def instrumented_dir(self) -> Path:
        return self.instrumented_root / self.safe_version

    @property
    def venv_dir(self) -> Path:
        return self.workspace / ".pgo-venv" / self.safe_version

    @property
    def merged_profdata(self) -> Path:
        return self.pgo_dir / "merged.profdata"

    @property
    def venv_python(self) -> Path:
        return self.venv_dir / "Scripts" / "python.exe"

    @property
    def profile_pattern(self) -> str:
        return str(self.pgo_dir / "%m_%p.profraw")

    def log_summary(self) -> None:
        log(f"  🐍 Interpreter: {self.python}")
        log(f"  🎯 Rust target: {self.rust_target}")
        log(f"  📁 PGO directory: {self.pgo_dir}")
        log(f"  📁 Instrumented wheel directory: {self.instrumented_dir}")
        log(f"  📁 Temporary venv: {self.venv_dir}")

    def prepare_dirs(self) -> None:
        self.pgo_dir.mkdir(parents=True, exist_ok=True)
        self.instrumented_dir.mkdir(parents=True, exist_ok=True)
        if self.venv_dir.exists():
            shutil.rmtree(self.venv_dir)

    def instrumented_env(self, base_env: dict[str, str]) -> dict[str, str]:
        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-generate={self.pgo_dir}"
        return env

    def optimized_env(self, base_env: dict[str, str]) -> dict[str, str]:
        env = base_env.copy()
        env["RUSTFLAGS"] = f"-Cprofile-use={self.merged_profdata}"
        return env

    def instrumentation_env(self, base_env: dict[str, str]) -> dict[str, str]:
        env = base_env.copy()
        env["LLVM_PROFILE_FILE"] = self.profile_pattern
        return env

    def latest_instrumented_wheel(self) -> Path:
        return next(self.instrumented_dir.glob("*.whl"))

    def latest_built_wheel(self) -> Path | None:
        wheels = sorted(
            self.dist_dir.glob("*.whl"),
            key=lambda path: path.stat().st_mtime,
            reverse=True,
        )
        return wheels[0] if wheels else None


def build_instrumented_wheel(build: PgoBuild, base_env: dict[str, str]) -> None:
    env = build.instrumented_env(base_env)
    phase(1, 4, "Building instrumented wheel...")
    log(f"  🧪 RUSTFLAGS={env['RUSTFLAGS']}")
    run_logged(
        "maturin",
        "build",
        "--release",
        "--out",
        str(build.instrumented_dir),
        "--target",
        build.rust_target,
        "--interpreter",
        build.python,
        "--features",
        "mimalloc",
        "--compatibility",
        "pypi",
        env=env,
    )


def run_instrumentation(build: PgoBuild, base_env: dict[str, str]) -> None:
    phase(2, 4, "Creating venv and installing instrumented wheel...")
    run_logged("uv", "venv", str(build.venv_dir), "--python", build.python)
    wheel = build.latest_instrumented_wheel()
    log(f"  🐍 Venv Python: {build.venv_python}")
    log(f"  📦 Instrumented wheel: {wheel}")
    run_logged(
        "uv",
        "pip",
        "install",
        "--python",
        str(build.venv_python),
        "--force-reinstall",
        "--no-deps",
        str(wheel),
    )

    env = build.instrumentation_env(base_env)
    phase(3, 4, "Running benchmark/pgo.py...")
    log(f"  🔬 LLVM_PROFILE_FILE={env['LLVM_PROFILE_FILE']}")
    run_logged(str(build.venv_python), "benchmark/pgo.py", env=env)


def merge_profiles(build: PgoBuild, llvm_profdata: str) -> None:
    profraw = [str(path) for path in build.pgo_dir.glob("*.profraw")]
    log(f"  ✅ Collected {len(profraw)} raw profile(s)")
    log(f"  🔗 Merging profiles into: {build.merged_profdata}")
    run_logged(llvm_profdata, "merge", "-o", str(build.merged_profdata), *profraw)


def build_optimized_wheel(build: PgoBuild, base_env: dict[str, str]) -> None:
    env = build.optimized_env(base_env)
    phase(4, 4, "Building optimized wheel...")
    log(f"  ⚙️ RUSTFLAGS={env['RUSTFLAGS']}")
    run_logged(
        "maturin",
        "build",
        "--release",
        "--out",
        str(build.dist_dir),
        "--target",
        build.rust_target,
        "--interpreter",
        build.python,
        "--features",
        "mimalloc",
        "--compatibility",
        "pypi",
        env=env,
    )
    wheel = build.latest_built_wheel()
    if wheel is not None:
        log(f"  📦 Built wheel: {wheel}")


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

    versions = args.interpreters.split()
    base_env = os.environ.copy()
    log(f"🚀 Starting PGO build for interpreters: {args.interpreters}")
    log(f"🧱 Python architecture: {args.python_arch}")
    log(f"🎯 Rust target: {rust_target_triple(args.target)}")
    log(f"🔗 llvm-profdata: {args.llvm_profdata}")

    for index, version in enumerate(versions, start=1):
        log("")
        log(
            f"📊 [{index}/{len(versions)}] "
            f"PGO cycle for Python {version} ({args.python_arch})...",
        )
        build = PgoBuild(
            version=version,
            python=find_python(version, args.python_arch),
            python_arch=args.python_arch,
            target=args.target,
            workspace=workspace,
            dist_dir=dist_dir,
            instrumented_root=instrumented_root,
        )
        build.log_summary()
        build.prepare_dirs()
        build_instrumented_wheel(build, base_env)
        run_instrumentation(build, base_env)
        merge_profiles(build, args.llvm_profdata)
        build_optimized_wheel(build, base_env)
        log(f"  ✅ Finished Python {version}")

    log("")
    log(f"🎉 PGO build completed. Wheels are in: {dist_dir}")


if __name__ == "__main__":
    main()
