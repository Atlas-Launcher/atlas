use std::io::{self, IsTerminal};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use atlas_client::hub::{HubClient, PackBuild, PackChannel};
use clap::Args;
use dialoguer::{Select, theme::ColorfulTheme};

use crate::auth_store;
use crate::config;

#[derive(Args)]
pub struct PromoteArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long)]
    pack_id: Option<String>,
    #[arg(long)]
    hub_url: Option<String>,
    #[arg(long, value_name = "CHANNEL", value_parser = ["dev", "beta", "production"])]
    channel: Option<String>,
    #[arg(long, value_name = "BUILD_ID")]
    build_id: Option<String>,
}

pub fn run(args: PromoteArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;

    let settings = config::resolve_cli_settings(
        &root,
        args.pack_id.clone(),
        args.hub_url.clone(),
        args.channel.clone(),
    )?;

    let pack_id = settings
        .pack_id
        .clone()
        .context("pack_id is required (use --pack-id or set pack_id in atlas.toml)")?;

    let access_token = auth_store::require_access_token_for_hub(&settings.hub_url)?;
    let mut client = HubClient::new(&settings.hub_url)?;
    client.set_token(access_token);

    let selected_channel = resolve_channel(&args, &settings.channel, &client, &pack_id)?;
    let selected_build = resolve_build_id(&args, &client, &pack_id)?;

    client.blocking_promote_pack_channel(&pack_id, &selected_channel, &selected_build)?;

    println!(
        "Promoted build {} to {} for pack {}.",
        selected_build, selected_channel, pack_id
    );
    Ok(())
}

fn resolve_channel(
    args: &PromoteArgs,
    default_channel: &str,
    client: &HubClient,
    pack_id: &str,
) -> Result<String> {
    if let Some(channel) = args.channel.as_ref() {
        return Ok(channel.clone());
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(default_channel.to_string());
    }

    let current_channels = client
        .blocking_list_pack_channels(pack_id)
        .unwrap_or_default();
    let options = ["dev", "beta", "production"];

    let default_index = if let Some(current) = most_recent_channel(&current_channels) {
        options
            .iter()
            .position(|value| *value == current)
            .unwrap_or(0)
    } else {
        options
            .iter()
            .position(|value| *value == default_channel)
            .unwrap_or(0)
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Promote to channel")
        .items(&options)
        .default(default_index)
        .interact()
        .context("Failed to read channel selection")?;

    Ok(options[selection].to_string())
}

fn resolve_build_id(args: &PromoteArgs, client: &HubClient, pack_id: &str) -> Result<String> {
    if let Some(build_id) = args.build_id.as_ref() {
        return Ok(build_id.clone());
    }

    let builds = client.blocking_list_pack_builds(pack_id)?;
    if builds.is_empty() {
        bail!("No builds found for this pack.");
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("--build-id is required in non-interactive mode.");
    }

    let labels = builds
        .iter()
        .map(format_build_label)
        .collect::<Vec<String>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a build to promote")
        .items(&labels)
        .default(0)
        .interact()
        .context("Failed to read build selection")?;

    builds
        .get(selection)
        .map(|build| build.id.clone())
        .context("Invalid build selection")
}

fn format_build_label(build: &PackBuild) -> String {
    let version = build
        .version
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let commit = build
        .commit_hash
        .as_ref()
        .map(|value| value.chars().take(8).collect::<String>())
        .unwrap_or_else(|| "no-commit".to_string());
    format!("{} | {} | {}", build.id, version, commit)
}

fn most_recent_channel(channels: &[PackChannel]) -> Option<&str> {
    channels
        .iter()
        .max_by_key(|channel| channel.updated_at.as_str())
        .map(|channel| channel.name.as_str())
}
