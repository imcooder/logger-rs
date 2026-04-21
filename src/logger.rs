use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Local;
use crossbeam_channel::{bounded, Sender, Receiver};
use log::{LevelFilter, Metadata, Record};

use crate::cleaner;
use crate::formatter;

// ── Messages sent to the writer thread ───────────────────────────────────────

enum Msg {
    /// A pre-formatted log line ready to be written.
    Line(String),
    /// Flush all buffers and exit the thread.
    Shutdown,
}

// ── Public config ─────────────────────────────────────────────────────────────

/// Configuration for `HourlyFileLogger`.
pub struct Config {
    /// Directory where log files are created.
    pub log_dir: PathBuf,
    /// Number of hours to retain rotated log files (default: 72).
    pub ttl_hours: i64,
    /// Minimum log level recorded to file (default: Info).
    pub level: LevelFilter,
    /// Whether to also print to stderr (default: true in debug, false in release).
    pub console: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
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

// ── Logger implementation ─────────────────────────────────────────────────────

pub struct HourlyFileLogger {
    sender: Sender<Msg>,
    console: bool,
    level: LevelFilter,
    /// Signals the background thread to stop accepting new messages.
    shutdown: Arc<AtomicBool>,
}

impl HourlyFileLogger {
    /// Create a new logger and spawn the background writer thread.
    pub fn new(config: Config) -> Self {
        let log_dir = config.log_dir.clone();
        let ttl_hours = config.ttl_hours;

        // Channel with a generous buffer so callers are (almost) never blocked.
        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = bounded(8192);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        thread::Builder::new()
            .name("hourly-file-logger".into())
            .spawn(move || {
                writer_thread(rx, &log_dir, ttl_hours, shutdown_clone);
            })
            .expect("failed to spawn logger thread");

        Self {
            sender: tx,
            console: config.console,
            level: config.level,
            shutdown,
        }
    }

    /// Flush pending writes and stop the background thread gracefully.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = self.sender.send(Msg::Shutdown);
        // Give the thread up to 2 s to drain.
        thread::sleep(Duration::from_millis(2000));
    }
}

impl log::Log for HourlyFileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = formatter::format_record(record);
        if self.console {
            eprint!("{line}");
        }
        // Non-blocking send: if the channel is full, we drop the message rather
        // than stalling the calling thread. The buffer (8192 entries) is large
        // enough that this should never happen under normal load.
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

fn active_log_path(log_dir: &Path) -> PathBuf {
    log_dir.join("app.log")
}

fn archive_log_path(log_dir: &Path, tag: &str) -> PathBuf {
    log_dir.join(format!("app.log.{tag}"))
}

/// Open (or create) `app.log` for appending, record the current hour tag.
fn open_active(log_dir: &Path) -> std::io::Result<FileState> {
    let path = active_log_path(log_dir);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(FileState {
        writer: BufWriter::with_capacity(64 * 1024, file),
        current_hour_tag: current_hour_tag(),
    })
}

/// Rotate: flush + rename `app.log` → `app.log.YYYYMMDDHH`, open fresh `app.log`.
fn rotate(log_dir: &Path, state: &mut FileState) {
    let _ = state.writer.flush();
    let old_tag = state.current_hour_tag.clone();
    let src = active_log_path(log_dir);
    let dst = archive_log_path(log_dir, &old_tag);
    let _ = fs::rename(&src, &dst);
    match open_active(log_dir) {
        Ok(new_state) => *state = new_state,
        Err(e) => eprintln!("[HourlyLogger] Failed to open new log file: {e}"),
    }
}

fn writer_thread(
    rx: Receiver<Msg>,
    log_dir: &Path,
    ttl_hours: i64,
    shutdown: Arc<AtomicBool>,
) {
    let _ = fs::create_dir_all(log_dir);

    let mut state = match open_active(log_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[HourlyLogger] Cannot open log file: {e}");
            return;
        }
    };

    // Initial TTL cleanup on startup.
    cleaner::cleanup(log_dir, ttl_hours);

    // Track when we last ran cleanup (every 30 min is enough).
    let mut last_cleanup = std::time::Instant::now();
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(30 * 60);

    for msg in &rx {
        match msg {
            Msg::Line(line) => {
                // Check hourly rotation.
                let tag = current_hour_tag();
                if tag != state.current_hour_tag {
                    rotate(log_dir, &mut state);
                    // After rotate, run cleanup.
                    cleaner::cleanup(log_dir, ttl_hours);
                    last_cleanup = std::time::Instant::now();
                }

                // Periodic cleanup even if no rotation happened.
                if last_cleanup.elapsed() > CLEANUP_INTERVAL {
                    cleaner::cleanup(log_dir, ttl_hours);
                    last_cleanup = std::time::Instant::now();
                }

                let _ = state.writer.write_all(line.as_bytes());
                // Flush immediately so tail -f works in dev; in prod the BufWriter
                // batches writes efficiently anyway.
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
