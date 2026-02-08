use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod hub;
mod fetch;
mod cache;
mod assemble;
mod reconcile;
mod supervisor;
mod rcon;

const DEFAULT_HUB_URL: &str = "https://atlas.nathanm.org";

#[derive(Parser)]
#[command(name = "atlas-runner")]
#[command(about = "Single-server deployment agent for Atlas packs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup and start a new Minecraft server instance
    Launch {
        /// The pack ID to launch
        pack_id: String,
        /// Release channel (prod, beta, nightly)
        #[arg(short, long, default_value = "prod")]
        channel: String,
        /// Memory limit (e.g., 6G)
        #[arg(short, long)]
        memory: Option<String>,
        /// Server port
        #[arg(short, long)]
        port: Option<u16>,
        /// Automatically accept EULA
        #[arg(long)]
        accept_eula: bool,
    },
    /// Start the server in the current directory
    Up {
        /// Force update configs from upstream
        #[arg(long)]
        force_config: bool,
    },
    /// Gracefully stop the server
    Down,
    /// Restart the server
    Restart,
    /// Show server status
    Status,
    /// Stream server logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
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

    match cli.command {
        Commands::Launch { pack_id, channel, memory, port, accept_eula } => {
            commands::launch::exec(DEFAULT_HUB_URL, pack_id, channel, memory, port, accept_eula).await?;
        }
        Commands::Up { force_config } => {
            commands::up::exec(force_config).await?;
        }
        Commands::Down => {
            commands::down::exec().await?;
        }
        Commands::Restart => {
            commands::restart::exec().await?;
        }
        Commands::Status => {
            commands::status::exec().await?;
        }
        Commands::Logs { follow: _ } => {
            println!("Logs command not yet fully integrated with supervisor streaming.");
        }
        Commands::Exec { command, it } => {
            commands::exec::exec(command, it).await?;
        }
    }

    Ok(())
}
