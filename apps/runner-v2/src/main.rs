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
    Shutdown,
    Exec {
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,

        command: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Ping => {
            let resp = client::ping().await?;
            println!("{resp}");
        }
        Cmd::Shutdown => {
            let resp = client::shutdown().await?;
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
    }
    Ok(())
}
