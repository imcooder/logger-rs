# logger-rs

[![Crates.io](https://img.shields.io/crates/v/logger-rs.svg)](https://crates.io/crates/imcooder-logger)
[![docs.rs](https://docs.rs/imcooder-logger/badge.svg)](https://docs.rs/imcooder-logger)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A high-performance hourly-rotating file logger for Rust, implementing the [`log`](https://crates.io/crates/log) facade.

Behaviorally equivalent to the Node.js [`@imcooder/node-logger`](https://github.com/imcooder/node-logger) library — same log format, same file naming convention, same TTL-based cleanup.

## Features

- 📁 Writes to `<app_name>.log`, rotates to `<app_name>.log.YYYYMMDDHH` every hour
- 🧹 Auto-deletes files older than `ttl_hours` (default **72 h**)
- ⚡ All I/O on a dedicated background thread — **calling threads never block**
- 🔒 Zero `unsafe` code
- 🎯 Drop-in with the standard `log` crate — no changes to existing `log::info!` calls

## Log Format

```
[2026-04-21 10:28:35.123] [INFO] my-app - Application started
[2026-04-21 10:28:35.124] [WARN] my-app - Low disk space
[2026-04-21 10:28:35.125] [ERROR] my-app - Connection failed: timeout
```

## Installation

```toml
[dependencies]
logger-rs = "0.1"
log = "0.4"
```

## Quick Start

```rust
use logger_rs::{Config, init};
use log::LevelFilter;
use std::path::PathBuf;

fn main() {
    // Option 1: convenience constructor
    logger_rs::init(logger_rs::config("my-app", "/var/log/my-app")).unwrap();

    // Option 2: full config
    logger_rs::init(Config {
        app_name: "my-app".to_string(),
        log_dir:  PathBuf::from("/var/log/my-app"),
        ttl_hours: 72,
        level:    LevelFilter::Info,
        console:  false,
    }).unwrap();

    log::info!("Application started");
    log::warn!("Low disk space");
    log::error!("Connection failed: {}", "timeout");

    // Flush & stop background thread before exit
    logger_rs::shutdown();
}
```

## Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `app_name` | `String` | `"app"` | App / category name. Used in log lines and filenames |
| `log_dir` | `PathBuf` | system temp | Directory where log files are created |
| `ttl_hours` | `i64` | `72` | Hours to retain rotated log files |
| `level` | `LevelFilter` | `Info` | Minimum log level written to file |
| `console` | `bool` | `true` (debug) / `false` (release) | Also print to stderr |

## File Naming

| File | Description |
|------|-------------|
| `my-app.log` | Current log file (active) |
| `my-app.log.2026042110` | Rotated archive for the 10:00–11:00 slot on 2026-04-21 |

## Comparison with @imcooder/node-logger

| Feature | `@imcooder/node-logger` | `logger-rs` |
|---------|------------------------|-------------|
| Log format | `[time] [LEVEL] app - msg` | `[time] [LEVEL] app - msg` ✅ |
| File naming | `app.log.YYYYMMDDHH` | `app.log.YYYYMMDDHH` ✅ |
| Hourly rotation | ✅ | ✅ |
| TTL cleanup | ✅ (72 h default) | ✅ (72 h default) |
| Async I/O | ✅ (Node streams) | ✅ (background thread + channel) |
| Console output | ✅ | ✅ |
| Zero blocking | ✅ | ✅ |

## License

MIT © [imcooder](mailto:imcooder@gmail.com)
