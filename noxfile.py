import glob

import nox

nox.options.default_venv_backend = "uv"
nox.options.reuse_existing_virtualenvs = True


# https://pypi.org/project/tomli/2.3.0/
# It is fully compatible with TOML v1.0.0
#
# https://pypi.org/project/tomli/2.4.0/
# Version 2.4.0 and later are compatible with TOML v1.1.0
@nox.session
@nox.parametrize("tomli_version", ["2.3.0", "2.4.0"])
def test_compatibility(session: nox.Session, tomli_version: str) -> None:
    session.install(f"tomli=={tomli_version}")
    session.install("--group", "nox")

    session.install(glob.glob("*.whl")[0])

    session.run("pytest", "tests/")
