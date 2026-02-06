use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth_store;
mod commands;
mod config;
mod io;
mod version_catalog;

use commands::{auth, ci, completion, deploy, pack};

#[derive(Parser)]
#[command(name = "atlas", version, about = "Atlas pack tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Pack {
        #[command(subcommand)]
        command: pack::PackCommand,
    },
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    Ci {
        #[command(subcommand)]
        command: ci::CiCommand,
    },
    Deploy(deploy::DeployArgs),
    Completion(completion::CompletionArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { command } => pack::run(command),
        Commands::Auth { command } => auth::run(command),
        Commands::Ci { command } => ci::run(command),
        Commands::Deploy(args) => deploy::run(args),
        Commands::Completion(args) => completion::run(args),
    }
}
