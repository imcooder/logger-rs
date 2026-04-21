use chrono::Local;
use log::Record;

/// Format a log record.
///
/// Output format (matches `@imcooder/node-logger`):
///
/// ```text
/// [2026-04-21 10:28:35.123] [INFO] my-app - message
/// ```
pub fn format_record(record: &Record, app_name: &str) -> String {
    let now = Local::now();
    let ts = now.format("%Y-%m-%d %H:%M:%S%.3f");
    let level = record.level();
    let msg = record.args();
    format!("[{ts}] [{level}] {app_name} - {msg}\n")
}
