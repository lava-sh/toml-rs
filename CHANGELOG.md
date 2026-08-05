# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0).

## [Unreleased] - ReleaseDate

### Features

- Bump [pyodide](https://github.com/pyodide/pyodide) (`314.0.0` -> `314.0.2`)

### Internal

- Bump

## [0.3.16] - 07.07.2026

### Bug Fixes

- Fix `macOS` subprocess failures after import by disabling mimalloc's malloc override ([#231][gh-pull-231]
  by [@chirizxc][gh-chirizxc], reported in [#229][gh-issue-229] by [@mermyly][gh-mermyly])

### Features

- Bump [pyodide](https://github.com/pyodide/pyodide) (`314.0.0` -> `314.0.2`)

[gh-issue-229]: https://github.com/lava-sh/toml-rs/issues/229

[gh-pull-231]: https://github.com/lava-sh/toml-rs/pull/231

[gh-chirizxc]: https://github.com/chirizxc
[gh-mermyly]: https://github.com/mermyly