use anyhow::{Context, Result};

pub fn default_memory() -> Result<String> {
    let mem_kb = read_mem_total_kb()?;
    let mem_gb = mem_kb / 1024 / 1024;
    let adjusted = mem_gb.saturating_sub(2).max(1);
    Ok(format!("{}G", adjusted))
}

fn read_mem_total_kb() -> Result<u64> {
    let content = std::fs::read_to_string("/proc/meminfo")
        .context("Failed to read /proc/meminfo")?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let value = rest.trim().split_whitespace().next().unwrap_or("0");
            let kb = value.parse::<u64>().context("Invalid MemTotal value")?;
            return Ok(kb);
        }
    }
    anyhow::bail!("MemTotal not found in /proc/meminfo")
}
