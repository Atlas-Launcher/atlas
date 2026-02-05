use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod io;
mod mods;

use commands::{deploy, pack};

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
    Deploy(deploy::DeployArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { command } => pack::run(command),
        Commands::Deploy(args) => deploy::run(args),
    }
}
