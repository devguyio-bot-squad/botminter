use std::fs;
use std::io::Write as _;

use super::config::DaemonPaths;

/// Maximum log file size before rotation (10 MB).
pub const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

/// Writes a log entry to the daemon's log file.
///
/// This function uses `eprint!` because the daemon process runs detached with
/// stderr redirected to the log file by the parent process. The direct file
/// write serves as a backup when stderr is not redirected.
pub fn daemon_log(paths: &DaemonPaths, level: &str, message: &str) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let line = format!("[{}] [{}] {}\n", timestamp, level, message);

    // Write to stderr (redirected to log file by the parent process)
    eprint!("{}", line);

    // Direct file write as backup
    if let Ok(log_file) = paths.log() {
        // Rotate if too large
        if let Ok(meta) = fs::metadata(&log_file) {
            if meta.len() > MAX_LOG_SIZE {
                let rotated = log_file.with_extension("log.old");
                let _ = fs::rename(&log_file, rotated);
            }
        }
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }
}
