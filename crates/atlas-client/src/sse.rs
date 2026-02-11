#[derive(Debug, Default)]
pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn push_chunk(&mut self, chunk: &[u8]) -> Vec<String> {
        let text = String::from_utf8_lossy(chunk);
        self.buffer.push_str(&text);

        let mut out = Vec::new();
        loop {
            let Some(split_at) = self.buffer.find("\n\n") else {
                break;
            };
            let raw = self.buffer[..split_at].to_string();
            self.buffer.drain(..split_at + 2);

            if let Some(payload) = extract_sse_payload(&raw) {
                out.push(payload);
            }
        }
        out
    }
}

pub fn extract_sse_payload(raw: &str) -> Option<String> {
    let data_lines = raw.lines().filter_map(|line| {
        if line.starts_with(":") {
            return None;
        }
        line.strip_prefix("data:").map(|rest| rest.trim_start())
    });

    let payload = data_lines.collect::<Vec<_>>().join("\n");
    if payload.trim().is_empty() {
        None
    } else {
        Some(payload)
    }
}
