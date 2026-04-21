use chrono::Local;
use log::Record;

/// Format a log record into a string line.
///
/// Output format (matches @imcooder/node-logger):
///   [2026-04-21 10:28:35.123] [INFO] [target] message\n
pub fn format_record(record: &Record) -> String {
    let now = Local::now();
    let ts = now.format("%Y-%m-%d %H:%M:%S%.3f");
    let level = record.level();
    let target = record.target();
    let msg = record.args();
    format!("[{ts}] [{level}] [{target}] {msg}\n")
}
