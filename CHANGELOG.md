# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Cargo aliases: `cargo build-plugin` and `cargo install-plugin` to ensure `zjctl-zrpc` is built for `wasm32-wasip1`.

### Fixed
- Building `zjctl-zrpc` for a non-WASI target now fails fast with a clear error message (instead of a linker error).

## [0.1.2] - 2026-01-26

### Added
- `zjctl pane resize` now supports `--cols`/`--rows` to resize a pane to an exact terminal size.

### Fixed
- CI workflows now target the renamed plugin crate (`zjctl-zrpc`).
- `zjctl status` and the `focused` selector now use the active tab’s focused pane (instead of the per-tab focused pane), avoiding ambiguous/random “focused” results when multiple tabs exist.
- `zjctl help` no longer conflicts with clap’s built-in help subcommand, so it consistently shows the agent-friendly quickstart.
- `zjctl status` and `focused` selection no longer get stuck at `Focused: none` after plugin reloads; focus is derived from Zellij client metadata (with a deterministic fallback when no client is marked “current”), and is refreshed periodically so floating-pane focus is reported correctly.
- Pane selectors now accept `terminal:N` / `plugin:N` as shorthand for `id:terminal:N` / `id:plugin:N` (so copying IDs from `status` output works).
- `zjctl pane resize --cols/--rows` now tries both relevant borders (left/right or up/down) when `--direction` is omitted, avoiding no-op resizes on edge panes.

## [0.1.1] - 2026-01-17

### Added
- Cargo-first install steps and clearer manual install guidance in the README.
- Repository README links in crate docs.
- Conventional Commit guidance in `AGENTS.md`.

### Changed
- Tagline and crate description refined.

### Fixed
- Missing plugin error now suggests running `zjctl install`.

## [0.1.0] - 2026-01-17

### Added
- Initial release with CLI + Zellij plugin workflow.
- Pane selectors, pane operations, and JSON output for automation.
- `install` and `doctor` setup helpers.

[Unreleased]: https://github.com/mrshu/zjctl/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/mrshu/zjctl/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/mrshu/zjctl/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mrshu/zjctl/releases/tag/v0.1.0
