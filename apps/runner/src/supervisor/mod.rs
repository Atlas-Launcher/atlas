use anyhow::{Result, Context};
use std::process::Stdio;
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, BufReader};
use std::path::PathBuf;
// PackBlob removed
use tokio::fs;

pub struct Supervisor {
    runtime_dir: PathBuf,
    args: Vec<String>,
    pid_file: PathBuf,
    command: String,
    envs: Vec<(String, String)>,
}

impl Supervisor {
    pub fn new(
        runtime_dir: PathBuf,
        command: String,
        args: Vec<String>,
        envs: Vec<(String, String)>,
    ) -> Self {
        let pid_file = runtime_dir.join("server.pid");
        Self {
            runtime_dir,
            args,
            pid_file,
            command,
            envs,
        }
    }

    pub async fn is_running(&self) -> bool {
        if !self.pid_file.exists() {
            return false;
        }
        
        if let Ok(pid_str) = fs::read_to_string(&self.pid_file).await {
            let pid_str: String = pid_str;
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                let pid: u32 = pid;
                // Check if process exists (platform specific)
                // For simplicity on Linux/macOS:
                let status = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                
                return status.map(|s| s.success()).unwrap_or(false);
            }
        }
        false
    }

    pub async fn spawn(&self) -> Result<Child> {
        let mut cmd = Command::new(&self.command);
        
        cmd.current_dir(&self.runtime_dir)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &self.envs {
            cmd.env(key, value);
        }

        let child = cmd.spawn().context("Failed to spawn Java process")?;
        let pid = child.id().context("Failed to get process ID")?;
        
        let _: () = fs::write(&self.pid_file, pid.to_string()).await?;
        
        let mut child = child;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn log capture tasks
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                println!("[STDOUT] {}", line);
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                eprintln!("[STDERR] {}", line);
            }
        });

        Ok(child)
    }

    pub async fn stop(&self, child: &mut Child) -> Result<()> {
        // Ideally we send "stop" via RCON first
        // If that fails or timeout, we kill the process
        child.kill().await.context("Failed to kill process")?;
        Ok(())
    }
}
