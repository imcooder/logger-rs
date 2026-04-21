use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Local};

/// Scan `log_dir` for rotated log files matching `<app_name>.log.YYYYMMDDHH`
/// and delete those older than `ttl_hours`.
pub fn cleanup(log_dir: &Path, app_name: &str, ttl_hours: i64) {
    let cutoff = Local::now() - Duration::hours(ttl_hours);
    let cutoff_tag = cutoff.format("%Y%m%d%H").to_string();
    let prefix = format!("{app_name}.log.");

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
        // Only touch files that look like our rotated archives: <app_name>.log.YYYYMMDDHH
        if let Some(tag) = name.strip_prefix(&prefix) {
            if tag.len() == 10 && tag.chars().all(|c| c.is_ascii_digit()) && tag < cutoff_tag.as_str() {
                let _ = fs::remove_file(&path);
            }
        }
    }
}
