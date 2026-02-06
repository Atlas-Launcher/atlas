use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::auth_store;
use crate::config;

#[derive(Subcommand)]
pub enum CiCommand {
    Init(CiSyncArgs),
    Update(CiSyncArgs),
}

#[derive(Args)]
pub struct CiSyncArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long)]
    pack_id: Option<String>,
    #[arg(long)]
    hub_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowSyncRequest<'a> {
    action: &'a str,
    pack_id: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowSyncResponse {
    repository: String,
    workflow_path: String,
    workflow_updated: bool,
    actions_enabled: bool,
}

pub fn run(command: CiCommand) -> Result<()> {
    match command {
        CiCommand::Init(args) => run_sync("init", args),
        CiCommand::Update(args) => run_sync("update", args),
    }
}

fn run_sync(action: &str, args: CiSyncArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let settings = config::resolve_cli_settings(&root, args.pack_id, args.hub_url, None)?;
    let pack_id = settings
        .pack_id
        .clone()
        .context("pack_id is required (use --pack-id or set pack_id in atlas.toml)")?;
    let token = auth_store::require_access_token_for_hub(&settings.hub_url)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("Failed to create HTTP client")?;
    let endpoint = format!("{}/api/launcher/ci/workflow", settings.hub_url);
    let response = client
        .post(endpoint)
        .bearer_auth(token)
        .json(&WorkflowSyncRequest {
            action,
            pack_id: &pack_id,
        })
        .send()
        .context("Failed to sync CI workflow")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!("CI workflow sync failed (HTTP {}): {}", status, body);
    }

    let payload = response
        .json::<WorkflowSyncResponse>()
        .context("Failed to parse CI workflow sync response")?;

    let action_label = if action == "init" {
        "Initialized"
    } else {
        "Updated"
    };
    println!("{action_label} Atlas CI workflow.");
    println!("Repository: {}", payload.repository);
    println!("Workflow path: {}", payload.workflow_path);
    println!("Workflow updated: {}", payload.workflow_updated);
    println!("Actions enabled: {}", payload.actions_enabled);
    Ok(())
}
