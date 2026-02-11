use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod assemble;
mod backup;
mod cache;
mod commands;
mod fetch;
mod hub;
mod java;
mod rcon;
mod reconcile;
mod runner_config;
mod supervisor;

const DEFAULT_HUB_URL: &str = "https://atlas.nathanm.org";
pub const RUNNER_BASE_DIR: &str = "/var/lib/atlas-runner";

#[derive(Parser)]
#[command(name = "atlas-runner")]
#[command(about = "Single-server deployment agent for Atlas packs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure runner auth (service token or device code)
    Auth {
        /// Hub base URL
        #[arg(long, default_value = DEFAULT_HUB_URL)]
        hub_url: String,
        /// The pack ID to manage
        pack_id: Option<String>,
        /// Release channel (production, beta, dev)
        #[arg(short, long, default_value = "production")]
        channel: String,
        /// Runner service token (if omitted, device code flow is used)
        #[arg(long)]
        token: Option<String>,
        /// Optional label for the created service token
        #[arg(long)]
        name: Option<String>,
        /// Memory limit (e.g., 6G)
        #[arg(short, long)]
        memory: Option<String>,
        /// Server port
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Start the server in the runner base directory
    Up {
        /// Force update configs from upstream
        #[arg(long)]
        force_config: bool,
        /// Keep running in the foreground after setup
        #[arg(long)]
        attach: bool,
        /// Skip setup and use existing runtime (internal)
        #[arg(long, hide = true)]
        skip_setup: bool,
    },
    /// Gracefully stop the server
    Down,
    /// Update runner configuration
    Config {
        /// Max memory (e.g., 6G). Defaults to system RAM minus 2G
        #[arg(long)]
        memory: Option<String>,
        /// Server port
        #[arg(long)]
        port: Option<u16>,
        /// Override Java major version (must be >= minimum required)
        #[arg(long = "java-major")]
        java_major: Option<u32>,
    },
    /// Restart the server
    Restart,
    /// Show server status
    Status,
    /// View runner logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
    /// Install runner service for auto-start
    Install {
        /// Override the systemd service user
        #[arg(long)]
        user: Option<String>,
    },
    /// Execute a command via RCON
    Exec {
        /// Command to execute
        command: String,
        /// Interactive shell mode
        #[arg(short, long)]
        it: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    ensure_runner_base_dir()?;

    match cli.command {
        Commands::Auth {
            hub_url,
            pack_id,
            channel,
            token,
            name,
            memory,
            port,
        } => {
            commands::auth::exec(&hub_url, pack_id, channel, token, name, memory, port).await?;
        }
        Commands::Up {
            force_config,
            attach,
            skip_setup,
        } => {
            commands::up::exec(force_config, attach, skip_setup).await?;
        }
        Commands::Down => {
            commands::down::exec().await?;
        }
        Commands::Config {
            memory,
            port,
            java_major,
        } => {
            commands::config::exec(memory, port, java_major).await?;
        }
        Commands::Restart => {
            commands::restart::exec().await?;
        }
        Commands::Status => {
            commands::status::exec().await?;
        }
        Commands::Logs { follow } => {
            commands::logs::exec(follow).await?;
        }
        Commands::Install { user } => {
            commands::install::exec(user).await?;
        }
        Commands::Exec { command, it } => {
            commands::exec::exec(command, it).await?;
        }
    }

    Ok(())
}

fn ensure_runner_base_dir() -> anyhow::Result<()> {
    std::fs::create_dir_all(RUNNER_BASE_DIR)
        .with_context(|| format!("Failed to create {}", RUNNER_BASE_DIR))?;
    std::env::set_current_dir(RUNNER_BASE_DIR)
        .with_context(|| format!("Failed to enter {}", RUNNER_BASE_DIR))?;
    Ok(())
}
