use anyhow::{Context, Result, bail};
use atlas_client::device_code::{DEFAULT_ATLAS_HUB_URL, normalize_hub_url};
use atlas_client::hub::{HubClient, LauncherPack};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use std::io::{self, IsTerminal};
use std::time::Duration;
use crate::client::connect_or_start;
use runner_core_v2::proto::{Envelope, Request, Response};
use runner_ipc_v2::framing;

pub async fn exec(
    hub_url: Option<String>,
    pack_id: Option<String>,
    token_name: Option<String>,
    channel: Option<String>,
) -> Result<String> {
    let hub_url = normalize_hub_url(
        hub_url
            .as_deref()
            .unwrap_or(DEFAULT_ATLAS_HUB_URL),
    );

    let mut hub = HubClient::new(&hub_url)?;
    let device_code = hub.login().await?;
    println!(
        "To authorize, please visit: {}",
        device_code
            .verification_uri_complete
            .as_ref()
            .unwrap_or(&device_code.verification_uri)
    );
    println!("User code: {}", device_code.user_code);

    let mut interval = Duration::from_secs(device_code.interval);
    if interval.as_secs() == 0 {
        interval = Duration::from_secs(5);
    }

    loop {
        tokio::time::sleep(interval).await;
        match hub.poll_token(&device_code.device_code).await {
            Ok(Some(_)) => break,
            Ok(None) => continue,
            Err(err) => bail!("Authentication failed: {err}"),
        }
    }

    let resolved_pack_id = resolve_pack_id(&hub, pack_id).await?;
    let label = resolve_token_name(token_name)?;
    let created = hub
        .create_runner_service_token(&resolved_pack_id, Some(label))
        .await?;

    let channel = normalize_channel(channel);

    save_deploy_key(
        &hub_url,
        &resolved_pack_id,
        &channel,
        &created.token,
        created.prefix.as_str(),
    )
    .await?;

    Ok(format!(
        "Saved deploy key for pack {} (channel {}, prefix {}).",
        resolved_pack_id, channel, created.prefix
    ))
}

async fn save_deploy_key(
    hub_url: &str,
    pack_id: &str,
    channel: &str,
    deploy_key: &str,
    prefix: &str,
) -> Result<()> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::SaveDeployKey {
            hub_url: hub_url.to_string(),
            pack_id: pack_id.to_string(),
            channel: channel.to_string(),
            deploy_key: deploy_key.to_string(),
            prefix: Some(prefix.to_string()),
        },
    };

    framing::send_request(&mut framed, &req).await?;
    let resp = framing::read_response(&mut framed).await?;

    match resp.payload {
        Response::DeployKeySaved {} => Ok(()),
        Response::Error(err) => Err(anyhow::anyhow!("save deploy key failed: {}", err.message)),
        other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
    }
}

fn normalize_channel(channel: Option<String>) -> String {
    channel
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "production".to_string())
}

async fn resolve_pack_id(hub: &HubClient, pack_id: Option<String>) -> Result<String> {
    if let Some(value) = pack_id.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Ok(value);
        }
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("Pack ID is required in non-interactive mode.");
    }

    let packs = hub.list_launcher_packs().await?;
    if packs.is_empty() {
        bail!("No accessible packs found for this account.");
    }

    prompt_pack_selection(&packs)
}

fn prompt_pack_selection(packs: &[LauncherPack]) -> Result<String> {
    let labels = packs.iter().map(format_pack_label).collect::<Vec<_>>();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a pack to authorize")
        .items(&labels)
        .default(0)
        .interact()
        .context("Failed to read pack selection")?;

    packs
        .get(selection)
        .map(|pack| pack.pack_id.clone())
        .context("Invalid pack selection")
}

fn format_pack_label(pack: &LauncherPack) -> String {
    format!(
        "{} ({}) [{}]",
        pack.pack_name, pack.pack_slug, pack.pack_id
    )
}

fn resolve_token_name(token_name: Option<String>) -> Result<String> {
    if let Some(value) = token_name.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Ok(value);
        }
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok("atlas-runner".to_string());
    }

    Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Service token name")
        .default("atlas-runner".to_string())
        .interact_text()
        .context("Failed to read token name")
}
