use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod io;
mod mods;

use commands::{deploy, init, pack};

#[derive(Parser)]
#[command(name = "atlas", version, about = "Atlas pack tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init(init::InitArgs),
    Reinit(init::ReinitArgs),
    Pack {
        #[command(subcommand)]
        command: pack::PackCommand,
    },
    Deploy(deploy::DeployArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => init::run_init(args),
        Commands::Reinit(args) => init::run_reinit(args),
        Commands::Pack { command } => pack::run(command),
        Commands::Deploy(args) => deploy::run(args),
    }
}
