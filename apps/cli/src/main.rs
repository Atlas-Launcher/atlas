use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth_store;
mod commands;
mod config;
mod io;
mod version_catalog;

use commands::{auth, ci, deploy, init, pack, pull};

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
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    Ci {
        #[command(subcommand)]
        command: ci::CiCommand,
    },
    Pull(pull::PullArgs),
    Deploy(deploy::DeployArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => init::run_init(args),
        Commands::Reinit(args) => init::run_reinit(args),
        Commands::Pack { command } => pack::run(command),
        Commands::Auth { command } => auth::run(command),
        Commands::Ci { command } => ci::run(command),
        Commands::Pull(args) => pull::run(args),
        Commands::Deploy(args) => deploy::run(args),
    }
}
