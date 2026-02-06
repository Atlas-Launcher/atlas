use std::io::{self, IsTerminal, Write};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use atlas_auth::device_code::{
    DEFAULT_ATLAS_DEVICE_SCOPE, DeviceCodeRequest, DeviceCodeResponse, DeviceTokenPollStatus,
    DeviceTokenRequest, StandardDeviceTokenResponse, hub_device_code_endpoint,
    hub_device_token_endpoint, parse_device_token_poll_body,
};
use clap::{Args, Subcommand};
use reqwest::blocking::Client;

use crate::auth_store::{self, CliAuthSession};

#[derive(Subcommand)]
pub enum AuthCommand {
    #[command(alias = "login")]
    Signin(SignInArgs),
    #[command(alias = "logout")]
    Signout,
    Status,
}

#[derive(Args)]
pub struct SignInArgs {
    #[arg(long)]
    hub_url: Option<String>,
    #[arg(long)]
    client_id: Option<String>,
}

pub fn run(command: AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Signin(args) => signin(args),
        AuthCommand::Signout => signout(),
        AuthCommand::Status => status(),
    }
}

fn signin(args: SignInArgs) -> Result<()> {
    let hub_url = auth_store::resolve_hub_url(args.hub_url);
    let client_id = auth_store::resolve_device_client_id(args.client_id);
    let client = Client::new();

    let code = request_device_code(&client, &hub_url, &client_id)?;
    let verification_url = code
        .verification_uri_complete
        .clone()
        .unwrap_or_else(|| code.verification_uri.clone());

    prompt_open_browser(&verification_url)?;
    if code.verification_uri_complete.is_none() {
        println!("If prompted, paste the one-time code in the browser.");
    }
    if let Some(message) = code
        .message
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        println!("{}", message.trim());
    }
    println!("Waiting for authorization...");

    let started = Instant::now();
    let timeout = Duration::from_secs(code.expires_in.max(1));
    let mut interval = Duration::from_secs(code.interval.max(1));

    loop {
        if started.elapsed() > timeout {
            bail!("Device code expired before authorization completed.");
        }

        match request_device_token(&client, &hub_url, &client_id, &code.device_code)? {
            DeviceTokenPollStatus::Success(token) => {
                let now = auth_store::unix_timestamp();
                let session = CliAuthSession {
                    access_token: token.access_token,
                    token_type: token.token_type,
                    expires_at: now.saturating_add(token.expires_in),
                    hub_url: hub_url.clone(),
                    client_id: client_id.clone(),
                    scope: token.scope,
                    refresh_token: token.refresh_token,
                    created_at: now,
                };
                auth_store::save_cli_auth_session(&session)?;
                println!("Signed in to {}.", hub_url);
                return Ok(());
            }
            DeviceTokenPollStatus::AuthorizationPending => sleep(interval),
            DeviceTokenPollStatus::SlowDown => {
                interval += Duration::from_secs(5);
                sleep(interval);
            }
            DeviceTokenPollStatus::AccessDenied => {
                bail!("Authorization was denied in the browser.")
            }
            DeviceTokenPollStatus::ExpiredToken => {
                bail!("Device code expired. Run `atlas auth signin` again.")
            }
            DeviceTokenPollStatus::Fatal(message) => bail!(message),
        }
    }
}

fn signout() -> Result<()> {
    auth_store::remove_cli_auth_session()?;
    println!("Signed out.");
    Ok(())
}

fn status() -> Result<()> {
    let session = auth_store::load_cli_auth_session()?;
    let Some(session) = session else {
        println!("Not signed in.");
        return Ok(());
    };

    let now = auth_store::unix_timestamp();
    let remaining = session.expires_at.saturating_sub(now);
    let state = if remaining > 0 { "active" } else { "expired" };
    println!("Status: {}", state);
    println!("Hub: {}", session.hub_url);
    println!("Client ID: {}", session.client_id);
    println!("Expires in: {}s", remaining);
    Ok(())
}

fn request_device_code(
    client: &Client,
    hub_url: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse> {
    let endpoint = hub_device_code_endpoint(hub_url);
    client
        .post(endpoint)
        .json(&DeviceCodeRequest {
            client_id,
            scope: DEFAULT_ATLAS_DEVICE_SCOPE,
        })
        .send()
        .context("Failed to request device code")?
        .error_for_status()
        .context("Device code request failed")?
        .json::<DeviceCodeResponse>()
        .context("Failed to parse device code response")
}

fn request_device_token(
    client: &Client,
    hub_url: &str,
    client_id: &str,
    device_code: &str,
) -> Result<DeviceTokenPollStatus<StandardDeviceTokenResponse>> {
    let endpoint = hub_device_token_endpoint(hub_url);
    let response = client
        .post(endpoint)
        .json(&DeviceTokenRequest::new(client_id, device_code))
        .send()
        .context("Failed polling device token")?;

    let status = response.status().as_u16();
    let body = response.text().unwrap_or_default();
    parse_device_token_poll_body::<StandardDeviceTokenResponse>(status, &body)
        .context("Failed to parse device token response")
}

fn prompt_open_browser(url: &str) -> Result<()> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        print!("Press Enter to open {url} in your browser...");
        io::stdout().flush().context("Failed to flush stdout")?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .context("Failed to read confirmation input")?;
        println!();
    } else {
        println!("Open this URL in your browser: {url}");
        return Ok(());
    }

    if !open_browser(url) {
        println!("Could not open a browser automatically.");
        println!("Open this URL manually: {url}");
    }

    Ok(())
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return command_succeeds("open", &[url]);
    }

    #[cfg(target_os = "linux")]
    {
        return command_succeeds("xdg-open", &[url]);
    }

    #[cfg(target_os = "windows")]
    {
        return command_succeeds("cmd", &["/C", "start", "", url]);
    }

    #[allow(unreachable_code)]
    false
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
