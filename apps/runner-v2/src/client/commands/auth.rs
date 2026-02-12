use anyhow::{bail, Context, Result};
use atlas_client::device_code::{normalize_hub_url, DEFAULT_ATLAS_HUB_URL};
use atlas_client::hub::{HubClient, LauncherPack};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use serde_json::Value;
use std::io::{self, IsTerminal};
use std::time::Duration;

pub async fn exec(
    hub_url: Option<String>,
    pack_id: Option<String>,
    token_name: Option<String>,
    channel: Option<String>,
) -> Result<String> {
    let hub_url = resolve_hub_url(hub_url)?;

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

fn resolve_hub_url(cli_hub_url: Option<String>) -> Result<String> {
    if let Some(value) = cli_hub_url {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(normalize_hub_url(trimmed));
        }
    }

    if let Ok(value) = std::env::var("ATLAS_HUB_URL") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(normalize_hub_url(trimmed));
        }
    }

    if let Some(value) = read_hub_url_from_deploy_config()? {
        return Ok(normalize_hub_url(&value));
    }

    Ok(normalize_hub_url(DEFAULT_ATLAS_HUB_URL))
}

fn read_hub_url_from_deploy_config() -> Result<Option<String>> {
    let base = match dirs::data_dir().or_else(dirs::home_dir) {
        Some(value) => value,
        None => return Ok(None),
    };
    let path = base.join("atlas").join("runnerd").join("deploy.json");
    let content = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let value: Value = serde_json::from_str(&content)
        .context("Failed to parse existing runner deploy config while resolving hub URL")?;
    let hub_url = value.get("hub_url").and_then(|v| v.as_str()).map(str::trim);
    Ok(hub_url
        .filter(|value| !value.is_empty())
        .map(ToString::to_string))
}

async fn save_deploy_key(
    hub_url: &str,
    pack_id: &str,
    channel: &str,
    deploy_key: &str,
    prefix: &str,
) -> Result<()> {
    write_deploy_key_file(hub_url, pack_id, channel, deploy_key, prefix)?;
    Ok(())
}

fn normalize_channel(channel: Option<String>) -> String {
    channel
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "production".to_string())
}

fn write_deploy_key_file(
    hub_url: &str,
    pack_id: &str,
    channel: &str,
    deploy_key: &str,
    prefix: &str,
) -> Result<()> {
    let base = dirs::data_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("Unable to resolve a writable data directory"))?;
    let dir = base.join("atlas").join("runnerd");
    std::fs::create_dir_all(&dir)
        .map_err(|err| anyhow::anyhow!("Failed to create runnerd config dir: {err}"))?;
    let path = dir.join("deploy.json");

    let payload = serde_json::json!({
        "hub_url": hub_url,
        "pack_id": pack_id,
        "channel": channel,
        "deploy_key": deploy_key,
        "prefix": prefix,
    });
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|err| anyhow::anyhow!("Failed to serialize deploy key config: {err}"))?;
    std::fs::write(&path, content)
        .map_err(|err| anyhow::anyhow!("Failed to write deploy key config: {err}"))?;

    Ok(())
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
    format!("{} ({}) [{}]", pack.pack_name, pack.pack_slug, pack.pack_id)
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
