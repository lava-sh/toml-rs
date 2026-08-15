# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0).

## [Unreleased] - ReleaseDate

## [0.4.0] - xx.10.2026

### Breaking Changes

- Change default [TOML][site-toml] version from `1.0.0` to `1.1.0`. Functions that do not explicitly specify
  `toml_version` now parse and emit [TOML][site-toml] `1.1.0` by default

- Drop support for Python 3.10 ([#245][gh-pull-245] by [@chirizxc][gh-chirizxc])

### Fixes

- Fix incorrect benchmark `.svg` link (by [@chirizxc][gh-chirizxc])

### Internal

- Bump [pyodide][gh-pyodide] (`314.0.2` -> `314.0.4`)
- Bump [pyo3][gh-pyo3] (`=0.29.0` -> `=0.29.2`)
- Bump [memchr][gh-memchr] (`=2.8.2` -> `=2.8.3`)
- Bump [toml-test][gh-toml-test] (`229ce2e7` -> `9eef1b95`)
- Drop [snmalloc-rs][gh-snmalloc-rs]

## [0.3.16] - 07.07.2026

### Bug Fixes

- Fix `macOS` subprocess failures after import by disabling mimalloc's malloc override ([#231][gh-pull-231]
  by [@chirizxc][gh-chirizxc], reported in [#229][gh-issue-229] by [@mermyly][gh-mermyly])

### Internal

- Bump [pyodide][gh-pyodide] (`314.0.0` -> `314.0.2`)

[gh-issue-229]: https://github.com/lava-sh/toml-rs/issues/229

[gh-pull-231]: https://github.com/lava-sh/toml-rs/pull/231
[gh-pull-245]: https://github.com/lava-sh/toml-rs/pull/245

[gh-chirizxc]: https://github.com/chirizxc
[gh-mermyly]: https://github.com/mermyly

[gh-pyodide]: https://github.com/pyodide/pyodide
[gh-pyo3]: https://github.com/pyo3/pyo3
[gh-memchr]: https://github.com/BurntSushi/memchr
[gh-snmalloc-rs]: https://github.com/microsoft/snmalloc/tree/main/snmalloc-rs
[gh-toml-test]: https://github.com/toml-lang/toml-test

[site-toml]: https://toml.io/en

[Unreleased]: https://github.com/lava-sh/toml-rs/compare/0.4.0...HEAD


[0.4.0]: https://github.com/lava-sh/toml-rs/compare/0.3.16...0.4.0
[0.3.16]: https://github.com/lava-sh/toml-rs/compare/0.3.15...0.3.16
