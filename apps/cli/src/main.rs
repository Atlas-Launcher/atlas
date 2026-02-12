use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth_store;
mod commands;
mod config;
mod io;
mod version_catalog;

use commands::{auth, ci, completion, deploy, init, pack, promote, pull, push};

#[derive(Parser)]
#[command(name = "atlas", version, about = "Atlas pack tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login(auth::SignInArgs),
    Logout,
    Status,
    Init(init::InitArgs),
    Reinit(init::ReinitArgs),
    Pull(pull::PullArgs),
    Push(push::PushArgs),
    Build(pack::BuildArgs),
    Publish(deploy::DeployArgs),
    Promote(promote::PromoteArgs),
    Validate(pack::ValidateArgs),
    Commit(pack::CommitArgs),
    Mod {
        #[command(subcommand)]
        command: ModCommands,
    },
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    Completion(completion::CompletionArgs),
}

#[derive(Subcommand)]
enum ModCommands {
    Add(pack::AddArgs),
    Remove(pack::RmArgs),
    List(pack::ListArgs),
    Import(pack::ImportArgs),
}

#[derive(Subcommand)]
enum WorkflowCommands {
    Init(ci::CiSyncArgs),
    Update(ci::CiSyncArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login(args) => auth::run(auth::AuthCommand::Signin(args)),
        Commands::Logout => auth::run(auth::AuthCommand::Signout),
        Commands::Status => auth::run(auth::AuthCommand::Status),
        Commands::Init(args) => init::run_init(args),
        Commands::Reinit(args) => init::run_reinit(args),
        Commands::Pull(args) => pull::run(args),
        Commands::Push(args) => push::run(args),
        Commands::Build(args) => pack::run(pack::PackCommand::Build(args)),
        Commands::Publish(args) => deploy::run(args),
        Commands::Promote(args) => promote::run(args),
        Commands::Validate(args) => pack::run(pack::PackCommand::Validate(args)),
        Commands::Commit(args) => pack::run(pack::PackCommand::Commit(args)),
        Commands::Mod { command } => match command {
            ModCommands::Add(args) => pack::run(pack::PackCommand::Add(args)),
            ModCommands::Remove(args) => pack::run(pack::PackCommand::Rm(args)),
            ModCommands::List(args) => pack::run(pack::PackCommand::List(args)),
            ModCommands::Import(args) => pack::run(pack::PackCommand::Import(args)),
        },
        Commands::Workflow { command } => match command {
            WorkflowCommands::Init(args) => ci::run(ci::CiCommand::Init(args)),
            WorkflowCommands::Update(args) => ci::run(ci::CiCommand::Update(args)),
        },
        Commands::Completion(args) => completion::run(args),
    }
}
