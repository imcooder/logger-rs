//! # logger-nx
//!
//! A high-performance hourly-rotating file logger for Rust, implementing the
//! [`log`] facade.  Behaviorally equivalent to the Node.js
//! [`@imcooder/node-logger`](https://github.com/imcooder/node-logger) library.
//!
//! ## Features
//!
//! - Active log written to `<app_name>.log`
//! - Every hour the active file is renamed to `<app_name>.log.YYYYMMDDHH`
//! - Files older than `ttl_hours` (default **72 h**) are deleted automatically
//! - All I/O runs on a dedicated background thread (lock-free channel) —
//!   calling threads are **never** blocked
//! - Zero unsafe code
//!
//! ## Log format
//!
//! ```text
//! [2026-04-21 10:28:35.123] [INFO] my-app - Application started
//! ```
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use logger_nx::{Config, init};
//! use log::LevelFilter;
//! use std::path::PathBuf;
//!
//! init(Config {
//!     app_name: "my-app".to_string(),
//!     log_dir: PathBuf::from("/var/log/my-app"),
//!     ttl_hours: 72,
//!     level: LevelFilter::Info,
//!     console: true,
//! }).expect("logger init failed");
//!
//! log::info!("Application started");
//! log::warn!("Low disk space");
//! log::error!("Connection failed: {}", "timeout");
//!
//! // Before process exit:
//! logger_nx::shutdown();
//! ```

mod cleaner;
mod formatter;
mod logger;

pub use logger::{Config, Logger};

use log::SetLoggerError;
use std::sync::OnceLock;

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initialise the global logger.
///
/// Call **once** at application startup.  Returns `Err` if another logger has
/// already been registered via the `log` crate.
///
/// # Example
/// ```rust,no_run
/// use logger_nx::{Config, init};
/// use log::LevelFilter;
/// use std::path::PathBuf;
///
/// init(Config {
///     app_name: "my-app".to_string(),
///     log_dir: PathBuf::from("/tmp/my-app-logs"),
///     ttl_hours: 72,
///     level: LevelFilter::Info,
///     console: false,
/// }).unwrap();
/// ```
pub fn init(config: Config) -> Result<(), SetLoggerError> {
    let level = config.level;
    let logger = LOGGER.get_or_init(|| Logger::new(config));
    log::set_logger(logger)?;
    log::set_max_level(level);
    Ok(())
}

/// Flush pending writes and stop the background I/O thread gracefully.
///
/// Waits up to 2 seconds for the writer thread to drain.  Call this before
/// process exit.
///
/// # Example
/// ```rust,no_run
/// logger_nx::shutdown();
/// ```
pub fn shutdown() {
    if let Some(logger) = LOGGER.get() {
        logger.shutdown();
    }
}

/// Convenience: build a [`Config`] with sensible defaults and a single call.
///
/// ```rust,no_run
/// logger_nx::init(logger_nx::config("my-app", "/var/log/my-app")).unwrap();
/// ```
pub fn config(app_name: impl Into<String>, log_dir: impl Into<std::path::PathBuf>) -> Config {
    Config {
        app_name: app_name.into(),
        log_dir: log_dir.into(),
        ..Config::default()
    }
}

#[cfg(test)]
mod tests;
