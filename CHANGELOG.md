# Changelog

All notable changes to this project are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-08

### Added

- `mise` tasks for building, testing, linting, formatting and packaging.
- Tooling to rebuild `localproxy`: `mise run localproxy:build` for Linux, using
  Docker, and `mise run localproxy:windows` for Windows, using GitHub Actions.
- Tests covering the real tunnel connect and disconnect logic.

### Changed

- Upgraded to Freya 0.5 and updated all dependencies.
- Disconnecting now closes the AWS tunnel as well as stopping `localproxy`.
- `aws sso login` now gives up after 5 minutes.
- Raised the pinned Rust toolchain to 1.99.1.

### Fixed

- `localproxy` no longer keeps running after the app is closed.
- Error popups now show the actual error instead of an empty message.
- Pressing Connect repeatedly no longer starts multiple proxies.
- A missing `localproxy` or an incomplete AWS response now shows an error
  instead of causing a panic.
- Network failures no longer launch `aws sso login`.
- The UI now resets if `localproxy` exits unexpectedly.
- Packaging no longer fails on AppImage because of wrong icon paths.

### Removed

- Unused configuration module and its `serde` and `toml` dependencies.
- Tests that only exercised mocks or relied on timing or live AWS access.

### Security

- Dropped outdated `rustls`, `rustls-webpki` and `h2` versions affected by
  published advisories.

## [0.1.0] - 2025-11-07

### Added

- Desktop app to open an AWS IoT Secure Tunnel to a device and run `localproxy`
  for SSH and GORT access.
- AWS SSO login prompt when credentials are missing.
- Packaging with bundled `localproxy` binaries for Linux and Windows.

[0.2.0]: https://github.com/rosterloh/tunnel_manager/compare/81da572...v0.2.0
[0.1.0]: https://github.com/rosterloh/tunnel_manager/tree/81da572
