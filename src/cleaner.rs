use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Local};

/// Scan `log_dir` for rotated log files matching `app.log.YYYYMMDDHH`
/// and delete those older than `ttl_hours`.
pub fn cleanup(log_dir: &Path, _app_name: &str, ttl_hours: i64) {
    let cutoff = Local::now() - Duration::hours(ttl_hours);
    let cutoff_tag = cutoff.format("%Y%m%d%H").to_string();

    let entries = match fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_owned(),
            None => continue,
        };
        // Only touch files matching: app.log.YYYYMMDDHH
        if let Some(tag) = name.strip_prefix("app.log.") {
            if tag.len() == 10 && tag.chars().all(|c| c.is_ascii_digit()) && tag < cutoff_tag.as_str() {
                let _ = fs::remove_file(&path);
            }
        }
    }
}
