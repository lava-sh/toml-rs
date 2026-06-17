#!/usr/bin/env python3

import glob
import os
import shutil
import subprocess
import sys


def die(message: str):
    print(f"Error: {message}", file=sys.stderr)
    raise SystemExit(1)


def run(*args, check=True, **kwargs):
    return subprocess.run(
        args,
        check=check,
        text=True,
        **kwargs,
    )


def output(*args):
    return subprocess.check_output(
        args,
        text=True,
    ).strip()


RUST_HOST = output("rustc", "--print", "host-tuple")

TARGET = os.environ["INPUTS_TARGET"]
INTERPRETERS = os.environ["INPUTS_INTERPRETER"].split()
WORKDIR = os.environ.get("INPUTS_WORKING_DIRECTORY", ".")


def target_arch(target: str) -> str:
    value = target.split("-", 1)[0]

    aliases = {
        "i686": "x86",
        "riscv64gc": "riscv64",
    }

    return aliases.get(value, value)


def python_download_request(version: str) -> str:
    runner_os = os.environ["RUNNER_OS"]

    arch = target_arch(TARGET)

    if runner_os == "Linux":
        os_name = "linux"

        if "-musl" in TARGET:
            libc = "musl"
        elif "-gnu" in TARGET:
            libc = "gnu"
        else:
            die(f"Unsupported Linux target: {TARGET}")

    elif runner_os == "Windows":
        os_name = "windows"
        libc = "none"

    elif runner_os == "macOS":
        os_name = "macos"
        libc = "none"

        if TARGET.startswith("universal2"):
            arch = "x86_64"

    else:
        die(f"Unsupported runner OS: {runner_os}")

    if version.startswith("pypy"):
        return (
            f"pypy-{version.removeprefix('pypy')}-"
            f"{os_name}-{arch}-{libc}"
        )

    if version.endswith("t"):
        return (
            f"cpython-{version[:-1]}+freethreaded-"
            f"{os_name}-{arch}-{libc}"
        )

    return (
        f"cpython-{version}-"
        f"{os_name}-{arch}-{libc}"
    )


def wheel_pattern(version: str) -> str:
    base = os.path.join(WORKDIR, "initial-wheel")

    if version.startswith("pypy"):
        minor = version.removeprefix("pypy").replace(".", "")
        return os.path.join(
            base,
            f"*-pp{minor}-*.whl",
        )

    compact = version.replace(".", "")

    if version.endswith("t"):
        compact = compact[:-1]
        return os.path.join(
            base,
            f"*-cp{compact}-cp{compact}t-*.whl",
        )

    return os.path.join(
        base,
        f"*-cp{compact}-cp{compact}-*.whl",
    )


def resolve_python_path(request: str) -> str:
    result = subprocess.run(
        [
            "uv",
            "python",
            "find",
            "--no-project",
            request,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )

    path = result.stdout.strip()

    if not path:
        run(
            "uv",
            "python",
            "install",
            request,
        )

        path = output(
            "uv",
            "python",
            "find",
            "--no-project",
            request,
        )

    if not path:
        die(f"Python not found: {request}")

    return path


def venv_python(path: str) -> str:
    if os.environ["RUNNER_OS"] == "Windows":
        return os.path.join(
            path,
            "Scripts",
            "python.exe",
        )

    return os.path.join(
        path,
        "bin",
        "python",
    )


def find_wheel(version: str) -> str:
    pattern = wheel_pattern(version)

    wheels = glob.glob(pattern)

    if len(wheels) != 1:
        print(
            f"Expected exactly one wheel for {version}",
            file=sys.stderr,
        )
        print(
            f"Pattern: {pattern}",
            file=sys.stderr,
        )

        for wheel in glob.glob(
            os.path.join(WORKDIR, "initial-wheel", "*")
        ):
            print(wheel, file=sys.stderr)

        raise SystemExit(1)

    return wheels[0]


def install_and_run(version: str, python: str):
    wheel = find_wheel(version)

    run(
        "uv",
        "pip",
        "install",
        "--python",
        python,
        "--force-reinstall",
        "--no-deps",
        wheel,
    )

    run(
        python,
        os.path.join(
            WORKDIR,
            "benchmark",
            "pgo.py",
        ),
    )


def setup(version: str):
    safe = version.replace(".", "_")

    venv = os.path.join(
        ".pgo-venv",
        safe,
    )

    shutil.rmtree(
        venv,
        ignore_errors=True,
    )

    request = python_download_request(version)

    python = resolve_python_path(request)

    run(
        "uv",
        "venv",
        venv,
        "--python",
        python,
    )

    executable = venv_python(venv)

    if not os.path.isfile(executable):
        die(
            f"Python executable not found: {executable}"
        )

    install_and_run(
        version,
        executable,
    )


for version in INTERPRETERS:
    setup(version)


sysroot = output(
    "rustc",
    "--print",
    "sysroot",
)

llvm_profdata = os.path.join(
    sysroot,
    "lib",
    "rustlib",
    RUST_HOST,
    "bin",
    "llvm-profdata",
)

if not os.path.isfile(llvm_profdata):
    fallback = shutil.which(
        "llvm-profdata"
    )

    if fallback:
        llvm_profdata = fallback
    else:
        die(
            f"llvm-profdata not found: {llvm_profdata}"
        )


with open(
    os.environ["GITHUB_ENV"],
    "a",
) as f:
    f.write(
        f"LLVM_PROFDATA={llvm_profdata}\n"
    )