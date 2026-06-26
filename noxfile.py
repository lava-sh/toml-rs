import glob

import nox

nox.options.default_venv_backend = "uv"
nox.options.reuse_existing_virtualenvs = True


@nox.session
@nox.parametrize("group_name", ["tomli-1", "tomli-1-1"])
def test_compatibility(session: nox.Session, group_name: str) -> None:
    session.install("--group", "nox", "--group", f"{group_name}")
    session.install(glob.glob("wheel/*.whl")[0])

    session.run("pytest", "tests/")
