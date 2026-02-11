use anyhow::{Context, Result};

pub fn default_memory() -> Result<String> {
    let mem_kb = read_mem_total_kb()?;
    let limit_kb = read_cgroup_memory_limit_kb().unwrap_or(mem_kb);
    let effective_kb = mem_kb.min(limit_kb);
    let mem_gb = effective_kb / 1024 / 1024;
    let adjusted = mem_gb.saturating_sub(2).max(1);
    Ok(format!("{}G", adjusted))
}

fn read_mem_total_kb() -> Result<u64> {
    let content =
        std::fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let value = rest.trim().split_whitespace().next().unwrap_or("0");
            let kb = value.parse::<u64>().context("Invalid MemTotal value")?;
            return Ok(kb);
        }
    }
    anyhow::bail!("MemTotal not found in /proc/meminfo")
}

fn read_cgroup_memory_limit_kb() -> Option<u64> {
    // cgroup v2
    if let Ok(content) = std::fs::read_to_string("/sys/fs/cgroup/memory.max") {
        let trimmed = content.trim();
        if trimmed != "max" {
            if let Ok(bytes) = trimmed.parse::<u64>() {
                return bytes.checked_div(1024);
            }
        }
    }

    // cgroup v1
    if let Ok(content) = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes") {
        let trimmed = content.trim();
        if let Ok(bytes) = trimmed.parse::<u64>() {
            if bytes > 0 {
                return bytes.checked_div(1024);
            }
        }
    }

    None
}
