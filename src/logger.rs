use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Local;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{LevelFilter, Metadata, Record};

use crate::cleaner;
use crate::formatter;

// ── Messages sent to the writer thread ───────────────────────────────────────

enum Msg {
    Line(String),
    Shutdown,
}

// ── Public config ─────────────────────────────────────────────────────────────

/// Configuration for [`Logger`].
///
/// Mirrors `@imcooder/node-logger` options.
///
/// # Example
/// ```rust,no_run
/// use imcooder_logger::Config;
/// use log::LevelFilter;
/// use std::path::PathBuf;
///
/// let cfg = Config {
///     app_name: "my-app".to_string(),
///     log_dir: PathBuf::from("/var/log/my-app"),
///     ttl_hours: 72,
///     level: LevelFilter::Info,
///     console: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Application / category name.  Used in the log line and as the base
    /// filename: `<app_name>.log`, `<app_name>.log.YYYYMMDDHH`.
    ///
    /// Equivalent to the `app` argument of `NodeLogger.getLogger(app)`.
    pub app_name: String,

    /// Directory where log files are written.
    pub log_dir: PathBuf,

    /// How many hours of rotated log files to retain (default: **72**).
    ///
    /// Files older than `ttl_hours` are deleted automatically on startup and
    /// at every hourly rotation (and at most every 30 min).
    pub ttl_hours: i64,

    /// Minimum level written to the log file (default: **Info**).
    pub level: LevelFilter,

    /// Also print log lines to stderr.
    ///
    /// Defaults to `true` in debug builds, `false` in release builds.
    pub console: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "app".to_string(),
            log_dir: std::env::temp_dir().join("app-logs"),
            ttl_hours: 72,
            level: LevelFilter::Info,
            #[cfg(debug_assertions)]
            console: true,
            #[cfg(not(debug_assertions))]
            console: false,
        }
    }
}

// ── Logger ────────────────────────────────────────────────────────────────────

/// A `log::Log` implementation that writes to hourly-rotating files.
///
/// All file I/O is performed on a dedicated background thread via a lock-free
/// channel, so calling threads are never blocked.
pub struct Logger {
    sender: Sender<Msg>,
    console: bool,
    level: LevelFilter,
    app_name: String,
    shutdown_flag: Arc<AtomicBool>,
}

impl Logger {
    /// Create a logger and spawn the background writer thread.
    pub fn new(config: Config) -> Self {
        let log_dir = config.log_dir.clone();
        let ttl_hours = config.ttl_hours;
        let app_name = config.app_name.clone();

        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = bounded(8192);
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown_flag);
        let app_name_thread = app_name.clone();

        thread::Builder::new()
            .name(format!("logger-rs/{}", app_name))
            .spawn(move || {
                writer_thread(rx, &log_dir, &app_name_thread, ttl_hours, shutdown_clone);
            })
            .expect("failed to spawn logger thread");

        Self {
            sender: tx,
            console: config.console,
            level: config.level,
            app_name,
            shutdown_flag,
        }
    }

    /// Flush pending writes and stop the background thread gracefully.
    ///
    /// Waits up to 2 s for the thread to drain.  Call this before process exit.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let _ = self.sender.send(Msg::Shutdown);
        thread::sleep(Duration::from_millis(2000));
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = formatter::format_record(record, &self.app_name);
        if self.console {
            eprint!("{line}");
        }
        // Non-blocking: drop the message if the channel is full (8 192 entries)
        // rather than stalling the caller.
        let _ = self.sender.try_send(Msg::Line(line));
    }

    fn flush(&self) {}
}

// ── Background writer thread ──────────────────────────────────────────────────

struct FileState {
    writer: BufWriter<File>,
    current_hour_tag: String, // YYYYMMDDHH
}

fn current_hour_tag() -> String {
    Local::now().format("%Y%m%d%H").to_string()
}

fn active_log_path(log_dir: &Path, app_name: &str) -> PathBuf {
    log_dir.join(format!("{app_name}.log"))
}

fn archive_log_path(log_dir: &Path, app_name: &str, tag: &str) -> PathBuf {
    log_dir.join(format!("{app_name}.log.{tag}"))
}

fn open_active(log_dir: &Path, app_name: &str) -> std::io::Result<FileState> {
    let path = active_log_path(log_dir, app_name);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(FileState {
        writer: BufWriter::with_capacity(64 * 1024, file),
        current_hour_tag: current_hour_tag(),
    })
}

fn rotate(log_dir: &Path, app_name: &str, state: &mut FileState) {
    let _ = state.writer.flush();
    let old_tag = state.current_hour_tag.clone();
    let src = active_log_path(log_dir, app_name);
    let dst = archive_log_path(log_dir, app_name, &old_tag);
    let _ = fs::rename(&src, &dst);
    match open_active(log_dir, app_name) {
        Ok(new_state) => *state = new_state,
        Err(e) => eprintln!("[logger-rs] Failed to open new log file: {e}"),
    }
}

fn writer_thread(
    rx: Receiver<Msg>,
    log_dir: &Path,
    app_name: &str,
    ttl_hours: i64,
    shutdown: Arc<AtomicBool>,
) {
    let _ = fs::create_dir_all(log_dir);

    let mut state = match open_active(log_dir, app_name) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[logger-rs] Cannot open log file: {e}");
            return;
        }
    };

    // Initial cleanup on startup (like node-logger's 1-min-then-30-min schedule;
    // we just clean on startup + every rotate + every 30 min).
    cleaner::cleanup(log_dir, app_name, ttl_hours);

    let mut last_cleanup = std::time::Instant::now();
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(30 * 60);

    for msg in &rx {
        match msg {
            Msg::Line(line) => {
                let tag = current_hour_tag();
                if tag != state.current_hour_tag {
                    rotate(log_dir, app_name, &mut state);
                    cleaner::cleanup(log_dir, app_name, ttl_hours);
                    last_cleanup = std::time::Instant::now();
                }

                if last_cleanup.elapsed() > CLEANUP_INTERVAL {
                    cleaner::cleanup(log_dir, app_name, ttl_hours);
                    last_cleanup = std::time::Instant::now();
                }

                let _ = state.writer.write_all(line.as_bytes());
                let _ = state.writer.flush();
            }
            Msg::Shutdown => {
                let _ = state.writer.flush();
                break;
            }
        }

        if shutdown.load(Ordering::Relaxed) {
            let _ = state.writer.flush();
            break;
        }
    }
}
