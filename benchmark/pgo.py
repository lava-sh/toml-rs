from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

import toml_rs

ROOT = Path(__file__).resolve().parents[1]


def build_obj() -> dict[str, Any]:
    return {
        "title": "PGO profile",
        "numbers": list(range(64)),
        "tuple": tuple(range(16)),
        "nested": {
            "date": date(1979, 5, 27),
            "time": time(7, 32),
            "datetime": datetime(
                1979,
                5,
                27,
                7,
                32,
                tzinfo=timezone(timedelta(hours=-8)),
            ),
            "decimal": Decimal("12345.6789"),
            "items": [
                {"value": Decimal("1.50")},
                {"value": Decimal("1E+3")},
                {"value": Decimal("Infinity")},
                {"value": Decimal("sNaN")},
            ],
        },
    }


def main() -> None:
    example = (ROOT / "tests" / "data" / "example.toml").read_text(encoding="utf-8")
    obj = build_obj()

    with TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir) / "profile.toml"

        for toml_version in ("1.0.0", "1.1.0"):
            for _ in range(400):
                dumped = toml_rs.dumps(obj, toml_version=toml_version)
                toml_rs.loads(dumped, toml_version=toml_version)
                doc = toml_rs.load_with_metadata(example, toml_version=toml_version)
                _ = doc.meta
                _ = doc.value

            for _ in range(200):
                buffer = StringIO()
                toml_rs.dump(obj, buffer, pretty=True, toml_version=toml_version)
                toml_rs.loads(buffer.getvalue(), toml_version=toml_version)

            for _ in range(200):
                toml_rs.dump(obj, tmp_path, toml_version=toml_version)
                with tmp_path.open("rb") as fp:
                    toml_rs.load(fp, toml_version=toml_version)
                with tmp_path.open("rb") as fp:
                    doc = toml_rs.load_with_metadata(fp, toml_version=toml_version)
                    _ = doc.meta


if __name__ == "__main__":
    main()
