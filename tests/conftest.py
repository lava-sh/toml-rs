import pytest
import toml_rs


@pytest.fixture(params=["1.0.0", "1.1.0"])
def toml_version(request: pytest.FixtureRequest) -> toml_rs._lib.TomlVersion:
    return request.param
