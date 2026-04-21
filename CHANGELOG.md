# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-21

### Added
- Initial release
- Hourly file rotation: `app.log` → `app.log.YYYYMMDDHH`
- TTL-based cleanup (default 72 hours)
- Non-blocking async I/O via `crossbeam-channel` + background thread
- Implements `log::Log` trait — integrates with the standard `log` facade
- Console mirroring (stderr) — enabled by default in debug builds
- `init(Config)` / `shutdown()` public API
- Log format matches `@imcooder/node-logger`: `[time] [LEVEL] [target] message`
