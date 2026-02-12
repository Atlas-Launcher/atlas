use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use atlas_client::hub::HubClient;
use clap::{Args, Subcommand};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::auth_store;

#[derive(Subcommand)]
pub enum CiCommand {
    Init(CiSyncArgs),
    Update(CiSyncArgs),
}

#[derive(Args)]
pub struct CiSyncArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long, hide = true)]
    pack_id: Option<String>,
    #[arg(long)]
    hub_url: Option<String>,
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
    let hub_url = auth_store::resolve_hub_url(args.hub_url);

    let mut client = HubClient::new(&hub_url)?;
    if let Ok(token) = auth_store::require_access_token_for_hub(&hub_url) {
        client.set_token(token);
    }

    let workflow_response = client.blocking_download_ci_workflow()?;

    let workflow_path = workflow_response.workflow_path;
    let relative_path = sanitize_relative_path(&workflow_path)?;
    let target_path = root.join(relative_path);
    let content = workflow_response.content;

    let updated = std::fs::read_to_string(&target_path)
        .map(|existing| existing != content)
        .unwrap_or(true);
    if updated {
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        std::fs::write(&target_path, content)
            .with_context(|| format!("Failed to write {}", target_path.display()))?;
    }

    if action == "init" {
        if let Err(error) = enable_github_workflows_if_needed(&client, &root, &hub_url) {
            println!("Skipped GitHub workflow auto-enable: {error}");
        }
    }

    let action_label = if action == "init" {
        "Initialized"
    } else {
        "Updated"
    };
    println!("{action_label} Atlas CI workflow file.");
    println!("Workflow path: {}", target_path.display());
    println!("Workflow updated: {}", updated);
    println!(
        "Next: run `atlas commit \"{} Atlas CI workflow\"` and `atlas push`.",
        action_label
    );
    Ok(())
}

fn sanitize_relative_path(value: &str) -> Result<std::path::PathBuf> {
    let normalized = value.replace('\\', "/");
    if normalized.trim().is_empty() {
        anyhow::bail!("Invalid empty workflow path from download endpoint.");
    }

    let mut out = std::path::PathBuf::new();
    for component in std::path::Path::new(&normalized).components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            _ => anyhow::bail!("Invalid workflow path from download endpoint: {}", value),
        }
    }

    if out.as_os_str().is_empty() {
        anyhow::bail!("Invalid workflow path from download endpoint: {}", value);
    }

    Ok(out)
}

#[derive(Deserialize)]
struct GithubWorkflowListResponse {
    workflows: Vec<GithubWorkflow>,
}

#[derive(Deserialize)]
struct GithubWorkflow {
    id: u64,
    state: Option<String>,
}

fn enable_github_workflows_if_needed(client: &HubClient, root: &Path, hub_url: &str) -> Result<()> {
    let remote_url = resolve_origin_remote_url(root)?;
    let (owner, repo) = parse_github_owner_repo(&remote_url).with_context(|| {
        format!(
            "Git remote origin is not a supported GitHub URL: {}",
            remote_url
        )
    })?;

    let _hub_token = auth_store::require_access_token_for_hub(hub_url).with_context(
        || "Atlas auth is required to enable workflows. Run `atlas login` and retry.",
    )?;
    let github_token = request_linked_github_access_token(client)?.context(
        "No linked GitHub token found. Link your GitHub account in Atlas Hub and retry.",
    )?;

    let github_client = Client::new();
    set_repository_actions_permissions(&github_client, &github_token, &owner, &repo)?;
    let workflows = list_repository_workflows(&github_client, &github_token, &owner, &repo)?;
    if workflows.is_empty() {
        println!(
            "No GitHub workflows found for {}/{}. Push this repository, then run `atlas workflow init` again to auto-enable workflows if needed.",
            owner, repo
        );
        return Ok(());
    }

    let mut enabled_count = 0usize;
    for workflow in workflows {
        if workflow
            .state
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("active"))
            .unwrap_or(false)
        {
            continue;
        }

        enable_repository_workflow(&github_client, &github_token, &owner, &repo, workflow.id)?;
        enabled_count += 1;
    }

    if enabled_count > 0 {
        println!(
            "Enabled {} GitHub workflow(s) for {}/{}.",
            enabled_count, owner, repo
        );
    } else {
        println!("GitHub workflows already active for {}/{}.", owner, repo);
    }
    Ok(())
}

fn resolve_origin_remote_url(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(root)
        .output()
        .context("Failed to run `git remote get-url origin`")?;
    if !output.status.success() {
        bail!(
            "Unable to resolve git remote 'origin'. Set a GitHub origin before running `atlas workflow init`."
        );
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if remote_url.is_empty() {
        bail!("Git remote 'origin' does not have a URL configured.");
    }

    Ok(remote_url)
}

fn parse_github_owner_repo(remote_url: &str) -> Option<(String, String)> {
    if let Some(path) = remote_url.strip_prefix("git@github.com:") {
        return parse_owner_repo_path(path);
    }
    if let Some(path) = remote_url.strip_prefix("ssh://git@github.com/") {
        return parse_owner_repo_path(path);
    }

    if let Ok(parsed) = reqwest::Url::parse(remote_url) {
        let host = parsed.host_str()?.to_ascii_lowercase();
        if host == "github.com" || host == "www.github.com" {
            return parse_owner_repo_path(parsed.path());
        }
    }

    None
}

fn parse_owner_repo_path(path: &str) -> Option<(String, String)> {
    let clean = path.trim().trim_start_matches('/').trim_end_matches('/');
    let clean = clean.strip_suffix(".git").unwrap_or(clean);
    let mut parts = clean.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

fn request_linked_github_access_token(client: &HubClient) -> Result<Option<String>> {
    client.blocking_get_github_token()
}

fn set_repository_actions_permissions(
    client: &Client,
    github_token: &str,
    owner: &str,
    repo: &str,
) -> Result<()> {
    let endpoint = format!(
        "https://api.github.com/repos/{}/{}/actions/permissions",
        owner, repo
    );
    let response = client
        .put(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "atlas-cli")
        .bearer_auth(github_token)
        .json(&serde_json::json!({
            "enabled": true,
            "allowed_actions": "all",
        }))
        .send()
        .context("Failed to configure GitHub Actions permissions")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!(
            "GitHub Actions permissions update failed for {}/{} (HTTP {}): {}",
            owner,
            repo,
            status,
            body
        );
    }

    Ok(())
}

fn list_repository_workflows(
    client: &Client,
    github_token: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<GithubWorkflow>> {
    let endpoint = format!(
        "https://api.github.com/repos/{}/{}/actions/workflows?per_page=100",
        owner, repo
    );
    let response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "atlas-cli")
        .bearer_auth(github_token)
        .send()
        .context("Failed to list GitHub workflows")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!(
            "GitHub workflow list failed for {}/{} (HTTP {}): {}",
            owner,
            repo,
            status,
            body
        );
    }

    let payload = response
        .json::<GithubWorkflowListResponse>()
        .context("Failed to parse GitHub workflow list response")?;
    Ok(payload.workflows)
}

fn enable_repository_workflow(
    client: &Client,
    github_token: &str,
    owner: &str,
    repo: &str,
    workflow_id: u64,
) -> Result<()> {
    let endpoint = format!(
        "https://api.github.com/repos/{}/{}/actions/workflows/{}/enable",
        owner, repo, workflow_id
    );
    let response = client
        .put(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "atlas-cli")
        .bearer_auth(github_token)
        .send()
        .context("Failed to enable GitHub workflow")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!(
            "Failed enabling workflow {} for {}/{} (HTTP {}): {}",
            workflow_id,
            owner,
            repo,
            status,
            body
        );
    }

    Ok(())
}
