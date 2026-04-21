use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

// ── formatter ────────────────────────────────────────────────────────────────

#[test]
fn format_contains_level_and_message() {
    let record = log::Record::builder()
        .args(format_args!("hello world"))
        .level(log::Level::Info)
        .target("my_crate")
        .build();

    let line = crate::formatter::format_record(&record, "my-app");
    assert!(line.contains("[INFO]"),    "missing level: {line}");
    assert!(line.contains("my-app"),    "missing app_name: {line}");
    assert!(line.contains("hello world"), "missing message: {line}");
    assert!(line.ends_with('\n'),       "line should end with newline");
}

#[test]
fn format_matches_node_logger_style() {
    let record = log::Record::builder()
        .args(format_args!("ts test"))
        .level(log::Level::Warn)
        .target("t")
        .build();
    let line = crate::formatter::format_record(&record, "myapp");
    // Expected: [2026-04-21 10:28:35.123] [WARN] myapp - ts test\n
    assert!(line.starts_with('['),         "should start with '[': {line}");
    assert!(line.contains("] [WARN] myapp - ts test"), "format mismatch: {line}");
}

// ── cleaner ───────────────────────────────────────────────────────────────────

#[test]
fn cleaner_removes_old_files() {
    let dir = TempDir::new().unwrap();
    let old   = dir.path().join("app.log.2020010100");
    let fresh = dir.path().join("app.log.2099010100");
    std::fs::write(&old,   b"old").unwrap();
    std::fs::write(&fresh, b"fresh").unwrap();

    crate::cleaner::cleanup(dir.path(), "app", 72);

    assert!(!old.exists(),  "old file should have been deleted");
    assert!(fresh.exists(), "fresh file should be kept");
}

#[test]
fn cleaner_ignores_non_matching_files() {
    let dir = TempDir::new().unwrap();
    let other = dir.path().join("something_else.txt");
    std::fs::write(&other, b"data").unwrap();

    crate::cleaner::cleanup(dir.path(), "app", 72);

    assert!(other.exists(), "unrelated file should not be touched");
}

#[test]
fn cleaner_respects_app_name() {
    let dir = TempDir::new().unwrap();
    // Belongs to "other-app", should NOT be touched when cleaning "app"
    let other_app = dir.path().join("other-app.log.2020010100");
    std::fs::write(&other_app, b"other").unwrap();

    crate::cleaner::cleanup(dir.path(), "app", 72);

    assert!(other_app.exists(), "other app's log should not be touched");
}

// ── integration ───────────────────────────────────────────────────────────────

#[test]
fn writes_log_file_on_init() {
    let dir = TempDir::new().unwrap();
    let log_dir: PathBuf = dir.path().to_path_buf();

    let logger = crate::HourlyFileLogger::new(crate::Config {
        app_name: "myapp".to_string(),
        log_dir: log_dir.clone(),
        ttl_hours: 72,
        level: log::LevelFilter::Info,
        console: false,
    });

    use log::Log;
    let record = log::Record::builder()
        .args(format_args!("integration test message"))
        .level(log::Level::Info)
        .target("test_target")
        .build();
    logger.log(&record);

    thread::sleep(Duration::from_millis(200));
    logger.shutdown();

    let log_path = log_dir.join("myapp.log");
    assert!(log_path.exists(), "myapp.log should be created");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("integration test message"), "message not in log: {content}");
    assert!(content.contains("myapp"),                   "app_name not in log: {content}");
}

#[test]
fn debug_messages_below_info_not_written() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().to_path_buf();

    let logger = crate::HourlyFileLogger::new(crate::Config {
        app_name: "filtertest".to_string(),
        log_dir: log_dir.clone(),
        ttl_hours: 72,
        level: log::LevelFilter::Info,
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

    let log_path = log_dir.join("filtertest.log");
    if log_path.exists() {
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(!content.contains("this is a debug message"), "debug msg should be filtered");
    }
}

#[test]
fn config_convenience_fn() {
    // Just verify the convenience constructor compiles and sets fields correctly
    let cfg = crate::config("hello", "/tmp/hello-logs");
    assert_eq!(cfg.app_name, "hello");
    assert_eq!(cfg.log_dir, std::path::PathBuf::from("/tmp/hello-logs"));
    assert_eq!(cfg.ttl_hours, 72);
}
