# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- CI workflows now target the renamed plugin crate (`zjctl-zrpc`).

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

[Unreleased]: https://github.com/mrshu/zjctl/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/mrshu/zjctl/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mrshu/zjctl/releases/tag/v0.1.0
