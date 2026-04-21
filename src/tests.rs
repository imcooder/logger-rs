use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

// ── formatter ────────────────────────────────────────────────────────────────

#[test]
fn format_contains_level_and_message() {
    // Build a minimal log Record and verify the formatted output.
    let record = log::Record::builder()
        .args(format_args!("hello world"))
        .level(log::Level::Info)
        .target("my_crate")
        .build();

    let line = crate::formatter::format_record(&record);
    assert!(line.contains("[INFO]"),  "missing level: {line}");
    assert!(line.contains("[my_crate]"), "missing target: {line}");
    assert!(line.contains("hello world"), "missing message: {line}");
    assert!(line.ends_with('\n'), "line should end with newline");
}

#[test]
fn format_timestamp_format() {
    let record = log::Record::builder()
        .args(format_args!("ts test"))
        .level(log::Level::Warn)
        .target("t")
        .build();
    let line = crate::formatter::format_record(&record);
    // Expect: [2026-04-21 10:28:35.123] …
    assert!(line.starts_with('['), "should start with '[': {line}");
    assert!(line.contains("] [WARN]"), "warn level missing: {line}");
}

// ── cleaner ───────────────────────────────────────────────────────────────────

#[test]
fn cleaner_removes_old_files() {
    let dir = TempDir::new().unwrap();
    // Create a file with a tag well in the past (year 2020).
    let old = dir.path().join("app.log.2020010100");
    std::fs::write(&old, b"old").unwrap();
    // Create a file with a future-safe tag (year 2099).
    let fresh = dir.path().join("app.log.2099010100");
    std::fs::write(&fresh, b"fresh").unwrap();

    crate::cleaner::cleanup(dir.path(), 72);

    assert!(!old.exists(),   "old file should have been deleted");
    assert!(fresh.exists(),  "fresh file should be kept");
}

#[test]
fn cleaner_ignores_non_matching_files() {
    let dir = TempDir::new().unwrap();
    let other = dir.path().join("something_else.txt");
    std::fs::write(&other, b"data").unwrap();

    crate::cleaner::cleanup(dir.path(), 72);

    assert!(other.exists(), "unrelated file should not be touched");
}

// ── integration ───────────────────────────────────────────────────────────────

#[test]
fn writes_log_file_on_init() {
    // Each test gets its own temp dir so they can run in parallel.
    let dir = TempDir::new().unwrap();
    let log_dir: PathBuf = dir.path().to_path_buf();

    let logger = crate::HourlyFileLogger::new(crate::Config {
        log_dir: log_dir.clone(),
        ttl_hours: 72,
        level: log::LevelFilter::Info,
        console: false,
    });

    // Use the logger directly (without setting global) to avoid conflicts.
    use log::Log;
    let record = log::Record::builder()
        .args(format_args!("integration test message"))
        .level(log::Level::Info)
        .target("test_target")
        .build();
    logger.log(&record);

    // Give background thread a moment to flush.
    thread::sleep(Duration::from_millis(200));
    logger.shutdown();

    let log_path = log_dir.join("app.log");
    assert!(log_path.exists(), "app.log should be created");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("integration test message"), "message not in log");
    assert!(content.contains("[test_target]"),            "target not in log");
}

#[test]
fn debug_messages_below_info_not_written() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().to_path_buf();

    let logger = crate::HourlyFileLogger::new(crate::Config {
        log_dir: log_dir.clone(),
        ttl_hours: 72,
        level: log::LevelFilter::Info, // Debug messages should be filtered
        console: false,
    });

    use log::Log;
    let record = log::Record::builder()
        .args(format_args!("this is a debug message"))
        .level(log::Level::Debug)
        .target("test")
        .build();
    logger.log(&record);

    thread::sleep(Duration::from_millis(200));
    logger.shutdown();

    let log_path = log_dir.join("app.log");
    if log_path.exists() {
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(!content.contains("this is a debug message"), "debug msg should be filtered");
    }
    // If file doesn't exist at all, that's also fine.
}
