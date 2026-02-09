use fs2::FileExt;
use std::{fs::OpenOptions, path::Path, fs::File};

pub struct LockGuard {
    _file: File,
}

pub fn acquire_lock(path: &Path) -> std::io::Result<LockGuard> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    file.try_lock_exclusive()?;
    Ok(LockGuard { _file: file })
}
