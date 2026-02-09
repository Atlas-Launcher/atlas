use clap::{Parser, Subcommand};

mod client;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Ping,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Ping => {
            let resp = client::ping().await?;
            println!("{resp}");
        }
    }
    Ok(())
}
