# logger-rs

[![Crates.io](https://img.shields.io/crates/v/logger-rs.svg)](https://crates.io/crates/logger-rs)
[![Docs.rs](https://docs.rs/logger-rs/badge.svg)](https://docs.rs/logger-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A high-performance, hourly-rotating file logger for Rust that implements the [`log`](https://crates.io/crates/log) facade.

Inspired by and behaviorally equivalent to the Node.js [`@imcooder/node-logger`](https://www.npmjs.com/package/@imcooder/node-logger) package — same file naming convention, same TTL-based cleanup, same log format.

---

## Features

- 📁 **Hourly rotation** — active log written to `app.log`, rotated to `app.log.YYYYMMDDHH` every hour
- 🗑️ **Auto TTL cleanup** — files older than N hours (default 72) are deleted automatically
- ⚡ **Non-blocking I/O** — all file writes happen on a dedicated background thread via a lock-free channel
- 🔌 **`log` facade** — works with any crate that uses `log::info!`, `log::error!`, etc.
- 🖥️ **Console mirroring** — optionally echo all log lines to stderr (enabled by default in debug builds)
- 🦀 **Zero unsafe code**

---

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
logger-rs = "0.1"
log = "0.4"
```

Initialize once at startup:

```rust
use hourly_file_logger::Config;
use log::LevelFilter;
use std::path::PathBuf;

fn main() {
    hourly_file_logger::init(Config {
        log_dir: PathBuf::from("/var/log/my-app"),
        ttl_hours: 72,
        level: LevelFilter::Info,
        console: true,
    }).expect("logger init failed");

    log::info!("[App] Application started");
    log::warn!("[App] Something looks off");
    log::error!("[App] Something went wrong");

    // Flush and stop the background thread before exit
    hourly_file_logger::shutdown();
}
```

---

## Log Format

```
[2026-04-21 10:28:35.123] [INFO]  [my_crate::module] message
[2026-04-21 10:28:35.124] [WARN]  [my_crate::module] something looks off
[2026-04-21 10:28:35.125] [ERROR] [my_crate::module] something went wrong
```

Matches the format of `@imcooder/node-logger` for cross-language log consistency.

---

## File Rotation & Cleanup

```
~/.local/share/my-app/logs/
  app.log               ← current hour (active)
  app.log.2026042110    ← previous hours (YYYYMMDDHH)
  app.log.2026042109
  app.log.2026042108
  ...                   ← files older than ttl_hours are deleted
```

Rotation happens automatically when the hour changes. Cleanup runs on startup and after each rotation (and every 30 minutes as a safety net).

---

## Configuration

```rust
pub struct Config {
    /// Directory where log files are written. Created automatically if missing.
    pub log_dir: PathBuf,

    /// Delete rotated files older than this many hours. Default: 72.
    pub ttl_hours: i64,

    /// Minimum log level recorded. Default: Info.
    pub level: LevelFilter,

    /// Mirror log lines to stderr. Default: true in debug builds, false in release.
    pub console: bool,
}
```

---

## Comparison with @imcooder/node-logger

| Feature | node-logger (JS) | logger-rs (Rust) |
|---------|-----------------|--------------------------|
| Hourly rotation | ✅ | ✅ |
| File naming `app.log.YYYYMMDDHH` | ✅ | ✅ |
| TTL auto-cleanup | ✅ | ✅ |
| Async / non-blocking writes | ✅ | ✅ |
| Console mirroring | ✅ | ✅ |
| Log format | `[time] [LEVEL] cat - msg` | `[time] [LEVEL] [target] msg` |
| Integration | Custom API | Standard `log` facade |

---

## License

MIT — same as `@imcooder/node-logger`.
