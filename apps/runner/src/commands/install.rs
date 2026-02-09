use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;

const SERVICE_PATH: &str = "/etc/systemd/system/atlas-runner.service";
const RUNNER_DIR: &str = "/var/lib/atlas-runner";

pub async fn exec(user: Option<String>) -> Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("Runner install is only supported on Linux systemd hosts.");
    }

    #[cfg(target_os = "linux")]
    {
        if unsafe { libc::geteuid() } != 0 {
            bail!("Runner install requires root. Re-run with sudo.");
        }
    }

    let exe = std::env::current_exe().context("Failed to resolve atlas-runner path")?;
    let resolved_user = resolve_service_user(user);

    fs::create_dir_all(RUNNER_DIR).with_context(|| format!("Failed to create {RUNNER_DIR}"))?;

    if let Some(user) = &resolved_user {
        let _ = Command::new("chown")
            .arg("-R")
            .arg(format!("{user}:{user}"))
            .arg(RUNNER_DIR)
            .status();
    }

    let unit = build_unit_file(&exe.to_string_lossy(), resolved_user.as_deref());
    fs::write(SERVICE_PATH, unit).with_context(|| format!("Failed to write {SERVICE_PATH}"))?;

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .context("Failed to reload systemd")?;
    Command::new("systemctl")
        .args(["enable", "--now", "atlas-runner"])
        .status()
        .context("Failed to enable atlas-runner service")?;

    println!("atlas-runner service installed and started.");
    Ok(())
}

fn resolve_service_user(explicit: Option<String>) -> Option<String> {
    if let Some(value) = explicit.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Some(value);
        }
    }

    if let Ok(value) = std::env::var("SUDO_USER") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(value) = std::env::var("USER") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn build_unit_file(exec_path: &str, user: Option<&str>) -> String {
    let mut unit = String::new();
    unit.push_str("[Unit]\n");
    unit.push_str("Description=Atlas Runner\n");
    unit.push_str("After=network-online.target\n");
    unit.push_str("Wants=network-online.target\n\n");
    unit.push_str("[Service]\n");
    unit.push_str("Type=simple\n");
    if let Some(user) = user {
        unit.push_str(&format!("User={}\n", user));
    }
    unit.push_str(&format!("WorkingDirectory={}\n", RUNNER_DIR));
    unit.push_str(&format!("ExecStart={} up --attach\n", exec_path));
    unit.push_str("Restart=always\n");
    unit.push_str("RestartSec=5\n");
    unit.push_str("Environment=RUST_LOG=info\n\n");
    unit.push_str("[Install]\n");
    unit.push_str("WantedBy=multi-user.target\n");
    unit
}
