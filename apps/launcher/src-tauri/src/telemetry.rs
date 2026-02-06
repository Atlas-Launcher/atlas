use crate::paths::{auth_store_dir, ensure_dir};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn log_mutex() -> &'static Mutex<()> {
    LOG_LOCK.get_or_init(|| Mutex::new(()))
}

fn timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn log_path() -> Result<std::path::PathBuf, String> {
    Ok(auth_store_dir()?.join("launcher.log"))
}

fn append_line(level: &str, message: &str) {
    let Ok(_guard) = log_mutex().lock() else {
        return;
    };

    let Ok(path) = log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if ensure_dir(parent).is_err() {
            return;
        }
    }

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };

    let _ = writeln!(
        file,
        "[{}] [{}] {}",
        timestamp_seconds(),
        level.to_uppercase(),
        message
    );
}

pub fn info(message: impl AsRef<str>) {
    append_line("info", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    append_line("warn", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    append_line("error", message.as_ref());
}
