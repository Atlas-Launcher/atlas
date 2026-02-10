use std::path::Path;

use crate::errors::ProvisionError;

pub async fn ensure_whitelist_enforced(runtime_dir: &Path) -> Result<(), ProvisionError> {
    let server_props = runtime_dir.join("server.properties");
    let mut current = tokio::fs::read_to_string(&server_props).await.unwrap_or_default();
    if current.trim().is_empty() {
        current = String::new();
    }

    let mut updated = set_property(&current, "white-list", "true");
    updated = set_property(&updated, "enforce-whitelist", "true");

    tokio::fs::write(&server_props, updated).await?;
    Ok(())
}

fn set_property(contents: &str, key: &str, value: &str) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    let prefix = format!("{}=", key);
    for line in contents.lines() {
        if line.trim_start().starts_with(&prefix) {
            lines.push(format!("{}={}", key, value));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("{}={}", key, value));
    }
    format!("{}\n", lines.join("\n"))
}
