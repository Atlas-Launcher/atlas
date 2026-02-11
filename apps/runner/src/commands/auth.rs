use crate::hub::whitelist::InstanceConfig;
use crate::hub::{HubClient, LauncherPack};
use crate::runner_config;
use anyhow::{Context, Result, bail};
use dialoguer::{FuzzySelect, Input, theme::ColorfulTheme};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::Duration;

pub async fn exec(
    hub_url: &str,
    pack_id: Option<String>,
    channel: String,
    service_token: Option<String>,
    token_name: Option<String>,
    memory: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let mut hub = HubClient::new(hub_url)?;
    let mut selected_pack_id = pack_id;

    let resolved_service_token = if let Some(token) = service_token {
        let pack_id = selected_pack_id
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .context("Pack ID is required when using --token")?;
        hub.set_service_token(token.clone());
        let exchange = hub.validate_service_token().await?;
        if exchange.pack_id != pack_id {
            bail!(
                "Service token pack mismatch: expected {}, got {}",
                pack_id,
                exchange.pack_id
            );
        }
        token
    } else {
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
                Err(e) => bail!("Authentication failed: {}", e),
            }
        }

        let resolved_pack_id = resolve_pack_id(&hub, selected_pack_id.clone()).await?;
        let label = resolve_token_name(token_name)?;
        let created = hub
            .create_runner_service_token(&resolved_pack_id, Some(label))
            .await?;
        selected_pack_id = Some(resolved_pack_id);
        created.token
    };

    let resolved_pack_id = selected_pack_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .context("Pack ID is required to save instance configuration")?;

    let config = InstanceConfig {
        pack_id: resolved_pack_id.to_string(),
        channel,
        hub_url: hub_url.to_string(),
        token: None,
        service_token: Some(resolved_service_token),
        memory: memory.or(Some(runner_config::default_memory()?)),
        port,
        minecraft_version: None,
        java_major: None,
        modloader: None,
        modloader_version: None,
    };

    config.save(&PathBuf::from("instance.toml")).await?;
    println!("Instance configuration saved to instance.toml");

    println!("User credentials are not stored on disk.");
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
