use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use runner_core_v2::proto::{LogLine, LogStream};

mod client;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Auth {
        #[arg(long, value_name = "HUB_URL")]
        hub_url: Option<String>,

        #[arg(long, value_name = "PACK_ID")]
        pack_id: Option<String>,

        #[arg(long, value_name = "TOKEN_NAME")]
        token_name: Option<String>,

        #[arg(long, value_name = "CHANNEL", default_value = "production")]
        channel: String,
    },
    Ping,
    Shutdown,
    Down {
        #[arg(long)]
        force: bool,
    },
    Up {
        #[arg(long, value_name = "PROFILE", default_value = "default")]
        profile: String,

        #[arg(long, value_name = "PACK_BLOB")]
        pack_blob: Option<PathBuf>,

        #[arg(long, value_name = "SERVER_ROOT")]
        server_root: Option<PathBuf>,
    },
    Exec {
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,

        command: Option<String>,
    },
    Logs {
        #[arg(short = 'n', long = "lines", default_value_t = 200)]
        lines: usize,

        #[arg(short = 'f', long = "follow")]
        follow: bool,

        #[arg(long = "daemon-logs")]
        daemon_logs: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Auth {
            hub_url,
            pack_id,
            token_name,
            channel,
        } => {
            let resp = client::auth(hub_url, pack_id, token_name, Some(channel)).await?;
            println!("{resp}");
        }
        Cmd::Ping => {
            let resp = client::ping().await?;
            println!("{resp}");
        }
        Cmd::Shutdown => {
            let resp = client::shutdown().await?;
            println!("{resp}");
        }
        Cmd::Down { force } => {
            let resp = client::stop(force).await?;
            println!("stopped at {}", resp.stopped_at_ms);
        }
        Cmd::Up {
            profile,
            pack_blob,
            server_root,
        } => {
            if profile != "default" {
                eprintln!("Ignoring --profile; runner uses a single profile: default");
            }
            let resp = client::up("default".to_string(), pack_blob, server_root).await?;
            println!("{resp}");
        }
        Cmd::Exec { interactive, command } => {
            let framed = client::connect_or_start().await?;
            if interactive {
                client::rcon_interactive(framed).await.map_err(|e| anyhow::anyhow!("interactive rcon failed: {e}"))?;
            } else {
                let cmd = command.ok_or_else(|| anyhow::anyhow!("command required for non-interactive exec"))?;
                client::rcon_exec(framed, cmd).await?;
            }
        }
        Cmd::Logs {
            lines,
            follow,
            daemon_logs,
        } => {
            if follow {
                follow_logs(lines, daemon_logs).await?;
            } else {
                let resp = if daemon_logs {
                    client::daemon_logs_tail(lines).await?
                } else {
                    client::logs_tail(lines).await?
                };
                for line in resp.lines {
                    print_log_line(&line);
                }
            }
        }
    }
    Ok(())
}

async fn follow_logs(lines: usize, daemon_logs: bool) -> anyhow::Result<()> {
    let mut last_at_ms = 0u64;
    let mut last_lines: Vec<String> = Vec::new();

    loop {
        let resp = if daemon_logs {
            client::daemon_logs_tail(lines).await?
        } else {
            client::logs_tail(lines).await?
        };
        for line in resp.lines {
            if line.at_ms > last_at_ms {
                last_at_ms = line.at_ms;
                last_lines.clear();
                last_lines.push(line.line.clone());
                print_log_line(&line);
                continue;
            }

            if line.at_ms == last_at_ms && !last_lines.contains(&line.line) {
                last_lines.push(line.line.clone());
                print_log_line(&line);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

fn print_log_line(line: &LogLine) {
    let stream = match line.stream {
        LogStream::Stdout => "stdout",
        LogStream::Stderr => "stderr",
    };
    println!("[{}] {}", stream, line.line.trim_end());
}
