import html
import re
import tomllib
import traceback
from collections.abc import Callable
from pathlib import Path

import toml_rs

COLORS = {
    "\x1b[1;35m": ("#ff00ff", True),
    "\x1b[35m": ("#ff00ff", False),
    "\x1b[36m": ("#00ffff", False),
    "\x1b[31m": ("#ff0000", False),
    "\x1b[1;31m": ("#ff0000", True),
    "\x1b[0m": (None, False),
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


def render_line(line: str) -> str:
    parts = ANSI.split(line)
    matches = ANSI.findall(line)
    result = html.escape(parts[0]).replace(" ", "&#160;")

    for ansi, part in zip(matches, parts[1:], strict=True):
        color, bold = COLORS[ansi]

        if color is None:
            result += html.escape(part).replace(" ", "&#160;")
            continue

        weight = ' font-weight="bold"' if bold else ""
        result += (
            f'<tspan fill="{color}"{weight}>'
            f"{html.escape(part).replace(' ', '&#160;')}"
            "</tspan>"
        )

    return result


def make_svg(
    path: Path,
    loader: Callable[[str], object],
    exception: type[BaseException],
) -> None:
    text = get_traceback(loader, exception)
    text = re.sub(r'[^"\r\n]*[\\/]toml-rs([\\/])', r"toml-rs\1", text)
    text = re.sub(r'[^"\r\n]*[\\/]uv[\\/]python[\\/]', "", text)

    lines = text.rstrip("\r\n").splitlines()
    rendered = [render_line(line) for line in lines]

    line_height = 22
    padding = 12
    width = max(map(len, lines), default=1) * 9 + padding * 2
    height = len(lines) * line_height + padding * 2

    elements = "\n".join(
        f'<text x="{padding}" y="{i * line_height + padding}">{line}</text>'
        for i, line in enumerate(rendered)
    )

    svg = f"""\
<svg xmlns="http://www.w3.org/2000/svg"
     width="{width}"
     height="{height}"
     viewBox="0 0 {width} {height}">
<rect width="100%" height="100%" fill="#0d1117"/>
<g font-family="ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace"
   font-size="15"
   dominant-baseline="hanging"
   fill="#ffffff">
{elements}
</g>
</svg>
"""

    path.write_text(svg, encoding="utf-8")


make_svg(
    Path(__file__).resolve().parents[1] / ".assets" / "tomllib-error.svg",
    tomllib.loads,
    tomllib.TOMLDecodeError,
)

make_svg(
    Path(__file__).resolve().parents[1] / ".assets" / "toml-rs-error.svg",
    toml_rs.loads,
    toml_rs.TOMLDecodeError,
)
