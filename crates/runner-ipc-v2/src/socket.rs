use std::path::Path;
use tokio::net::{UnixListener, UnixStream};

pub async fn connect(path: &Path) -> std::io::Result<UnixStream> {
    UnixStream::connect(path).await
}

pub async fn bind(path: &Path) -> std::io::Result<UnixListener> {
    UnixListener::bind(path)
}

pub fn remove_stale_socket(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn socket_alive(path: &Path) -> bool {
    UnixStream::connect(path).await.is_ok()
}
