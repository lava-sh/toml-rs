import nox

nox.options.default_venv_backend = "uv"
nox.options.reuse_existing_virtualenvs = True
nox.options.allow_parallel = True


@nox.session
@nox.parametrize("group_name", ["tomli-1", "tomli-1-1"])
def test_compatibility(session: nox.Session, group_name: str) -> None:
    session.install(
        "--group", "pytest",
        "--group", "tomli-w",
        "--group", f"{group_name}",
    )  # fmt: skip
    session.install(
        "toml-rs",
        "--no-index",
        "--find-links",
        "wheels",
        "--force-reinstall",
    )
    session.run("pytest", "tests/")
