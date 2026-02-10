use runner_core_v2::proto::{LogLine, LogStream};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
pub struct LogStore {
    inner: Arc<Mutex<LogState>>,
}

struct LogState {
    server: VecDeque<LogLine>,
    daemon: VecDeque<LogLine>,
    server_tx: broadcast::Sender<LogLine>,
    daemon_tx: broadcast::Sender<LogLine>,
    max_lines: usize,
}

impl LogStore {
    pub fn new(max_lines: usize) -> Self {
        let (server_tx, _) = broadcast::channel(1024);
        let (daemon_tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Mutex::new(LogState {
                server: VecDeque::with_capacity(max_lines.min(1024)),
                daemon: VecDeque::with_capacity(max_lines.min(1024)),
                server_tx,
                daemon_tx,
                max_lines: max_lines.max(100),
            })),
        }
    }

    pub fn push_server(&self, stream: LogStream, line: String) {
        let mut guard = self.inner.lock().expect("log lock poisoned");
        let entry = LogLine {
            at_ms: now_millis(),
            stream,
            line,
        };
        push_bounded(&mut guard.server, guard.max_lines, entry.clone());
        let _ = guard.server_tx.send(entry);
    }

    pub fn push_daemon(&self, line: String) {
        let mut guard = self.inner.lock().expect("log lock poisoned");
        let entry = LogLine {
            at_ms: now_millis(),
            stream: LogStream::Stdout,
            line,
        };
        push_bounded(&mut guard.daemon, guard.max_lines, entry.clone());
        let _ = guard.daemon_tx.send(entry);
    }

    pub fn tail_server(&self, lines: usize) -> Vec<LogLine> {
        let guard = self.inner.lock().expect("log lock poisoned");
        tail(&guard.server, lines)
    }

    pub fn tail_daemon(&self, lines: usize) -> Vec<LogLine> {
        let guard = self.inner.lock().expect("log lock poisoned");
        tail(&guard.daemon, lines)
    }

    pub fn server_subscribe(&self) -> broadcast::Receiver<LogLine> {
        let guard = self.inner.lock().expect("log lock poisoned");
        guard.server_tx.subscribe()
    }

    pub fn daemon_subscribe(&self) -> broadcast::Receiver<LogLine> {
        let guard = self.inner.lock().expect("log lock poisoned");
        guard.daemon_tx.subscribe()
    }

    pub fn daemon_writer(&self) -> LogWriterFactory {
        LogWriterFactory { store: self.clone() }
    }
}

fn push_bounded(buf: &mut VecDeque<LogLine>, max_lines: usize, entry: LogLine) {
    while buf.len() >= max_lines {
        buf.pop_front();
    }
    buf.push_back(entry);
}

fn tail(buf: &VecDeque<LogLine>, lines: usize) -> Vec<LogLine> {
    let count = lines.min(buf.len());
    buf.iter().skip(buf.len().saturating_sub(count)).cloned().collect()
}

pub struct LogWriterFactory {
    store: LogStore,
}

impl<'a> MakeWriter<'a> for LogWriterFactory {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            store: self.store.clone(),
            buffer: Vec::new(),
        }
    }
}

pub struct LogWriter {
    store: LogStore,
    buffer: Vec<u8>,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        while let Some(pos) = self.buffer.iter().position(|b| *b == b'\n') {
            let line_bytes: Vec<u8> = self.buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end_matches('\n').to_string();
            if !line.trim().is_empty() {
                self.store.push_daemon(line);
            }
        }

        io::stdout().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
