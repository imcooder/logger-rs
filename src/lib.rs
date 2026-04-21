//! # hourly-file-logger
//!
//! A high-performance hourly-rotating file logger that implements the [`log`]
//! facade.  Behaviour matches the Node.js `@imcooder/node-logger` library:
//!
//! * Active log written to `<log_dir>/app.log`.
//! * Every hour the active file is renamed to `app.log.YYYYMMDDHH`.
//! * Files older than `ttl_hours` (default 72) are deleted automatically.
//! * All I/O is performed on a dedicated background thread via a lock-free
//!   channel, so calling threads are never blocked.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use hourly_file_logger::{Config, init};
//! use log::LevelFilter;
//! use std::path::PathBuf;
//!
//! init(Config {
//!     log_dir: PathBuf::from("/var/log/my-app"),
//!     ttl_hours: 72,
//!     level: LevelFilter::Info,
//!     console: true,
//! }).expect("logger init failed");
//!
//! log::info!("Application started");
//! ```

mod cleaner;
mod formatter;
mod logger;

pub use logger::{Config, HourlyFileLogger};

use log::SetLoggerError;

// ── Module-level singleton handle for shutdown ────────────────────────────────

use std::sync::OnceLock;
static LOGGER: OnceLock<HourlyFileLogger> = OnceLock::new();

/// Initialise the global logger.  Call once at application startup.
///
/// Returns `Err` if the global logger has already been set by another crate.
pub fn init(config: Config) -> Result<(), SetLoggerError> {
    let level = config.level;
    let logger = LOGGER.get_or_init(|| HourlyFileLogger::new(config));
    log::set_logger(logger)?;
    log::set_max_level(level);
    Ok(())
}

/// Flush pending writes and stop the background I/O thread gracefully.
///
/// Call this before process exit (e.g. in Tauri's `on_drop`).
pub fn shutdown() {
    if let Some(logger) = LOGGER.get() {
        logger.shutdown();
    }
}

#[cfg(test)]
mod tests;

