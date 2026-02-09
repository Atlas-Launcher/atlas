use std::path::{Path, PathBuf};

pub struct RuntimePaths {
    pub runtime_dir: PathBuf,
    pub socket_path: PathBuf,
    pub lock_path: PathBuf,
}

/// Runtime namespace for v2 so it never collides with v1.
const APP_ID_V2: &str = "runner2";

pub fn runtime_paths_v2() -> RuntimePaths {
    // Linux: prefer XDG_RUNTIME_DIR if present.
    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        let dir = PathBuf::from(xdg).join(APP_ID_V2);
        return mk(dir);
    }

    // macOS: use TMPDIR. (Also fine as Linux fallback.)
    if let Some(tmp) = std::env::var_os("TMPDIR") {
        let dir = PathBuf::from(tmp).join(APP_ID_V2);
        return mk(dir);
    }

    // Last resort fallback
    mk(std::env::temp_dir().join(APP_ID_V2))
}

fn mk(runtime_dir: PathBuf) -> RuntimePaths {
    RuntimePaths {
        socket_path: runtime_dir.join("runnerd.sock"),
        lock_path: runtime_dir.join("runnerd.lock"),
        runtime_dir,
    }
}

pub fn ensure_dir(p: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(p)
}
