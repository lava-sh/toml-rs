import html
import re
import tomllib
import traceback
from collections.abc import Callable
from pathlib import Path

import toml_rs

COLORS = {
    "\x1b[1;35m": '<span style="color:#ff00ff;font-weight:bold">',
    "\x1b[35m": '<span style="color:#ff00ff">',
    "\x1b[36m": '<span style="color:#00ffff">',
    "\x1b[31m": '<span style="color:#ff0000">',
    "\x1b[1;31m": '<span style="color:#ff0000;font-weight:bold">',
    "\x1b[0m": "</span>",
}

ANSI = re.compile("|".join(map(re.escape, COLORS)))


def get_traceback(
    loader: Callable[[str], object],
    exception: type[BaseException],
) -> str:
    try:
        loader("x = ")
    except exception as exc:
        return "".join(traceback.format_exception(exc, colorize=True))  # ty: ignore[no-matching-overload]

    msg = "expected an exception"
    raise RuntimeError(msg)


def make_html(
    path: Path,
    loader: Callable[[str], object],
    exception: type[BaseException],
) -> None:
    text = get_traceback(loader, exception)
    text = re.sub(r'[^"\r\n]*[\\/]toml-rs([\\/])', r"toml-rs\1", text)
    text = re.sub(r'[^"\r\n]*[\\/]uv[\\/]python[\\/]', "", text)

    parts = ANSI.split(text)
    result = html.escape(parts[0])

    for ansi, part in zip(ANSI.findall(text), parts[1:], strict=True):
        result += COLORS[ansi] + html.escape(part)

    path.write_text(f"<pre><code>{result}</code></pre>", encoding="utf-8")


make_html(Path("tomllib.html"), tomllib.loads, tomllib.TOMLDecodeError)
make_html(Path("toml-rs.html"), toml_rs.loads, toml_rs.TOMLDecodeError)
