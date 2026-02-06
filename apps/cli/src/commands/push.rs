use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use base64::Engine;
use clap::Args;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::auth_store;

#[derive(Args)]
pub struct PushArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long)]
    hub_url: Option<String>,
    #[arg(long, default_value = "origin")]
    remote: String,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    set_upstream: bool,
    #[arg(long)]
    force_with_lease: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubTokenResponse {
    access_token: String,
}

pub fn run(args: PushArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let remote_url = resolve_remote_url(&root, &args.remote)?;

    let branch = match (args.set_upstream, args.branch.clone()) {
        (true, Some(value)) => Some(value),
        (true, None) => Some(resolve_current_branch(&root)?),
        (false, value) => value,
    };

    let mut github_token = None;
    if is_github_https_url(&remote_url) {
        let hub_url = auth_store::resolve_hub_url(args.hub_url.clone());
        match auth_store::require_access_token_for_hub(&hub_url) {
            Ok(access_token) => {
                let client = Client::builder()
                    .timeout(Duration::from_secs(20))
                    .build()
                    .context("Failed to create HTTP client")?;
                match request_linked_github_access_token(&client, &hub_url, &access_token) {
                    Ok(token) => github_token = token,
                    Err(error) => {
                        println!(
                            "Could not fetch linked GitHub token ({error}). Trying local git credentials."
                        );
                    }
                }
            }
            Err(_) => {
                println!(
                    "No valid Atlas session for linked GitHub token. Trying local git credentials."
                );
            }
        }
    }

    if github_token.is_none() && is_github_https_url(&remote_url) {
        println!("No linked GitHub token found. Trying local git credentials.");
    }

    println!(
        "Pushing {}{} via {}",
        args.remote,
        branch
            .as_ref()
            .map(|value| format!(" ({value})"))
            .unwrap_or_default(),
        remote_url
    );
    run_git_push(
        &root,
        &args.remote,
        branch.as_deref(),
        args.set_upstream,
        args.force_with_lease,
        github_token.as_deref(),
        &remote_url,
    )?;
    Ok(())
}

fn request_linked_github_access_token(
    client: &Client,
    hub_url: &str,
    access_token: &str,
) -> Result<Option<String>> {
    let endpoint = format!("{}/api/launcher/github/token", hub_url);
    let response = client
        .get(endpoint)
        .bearer_auth(access_token)
        .send()
        .context("Failed to request linked GitHub credentials")?;

    let status = response.status().as_u16();
    if status == 401 {
        bail!(
            "Atlas Hub rejected your saved token. Run `atlas auth signout` then `atlas auth signin --hub-url {}`.",
            hub_url
        );
    }
    if status == 404 || status == 409 {
        return Ok(None);
    }

    if !(200..300).contains(&status) {
        let body = response.text().unwrap_or_default();
        bail!(
            "Failed to fetch linked GitHub credentials (HTTP {}): {}",
            status,
            body
        );
    }

    let payload = response
        .json::<GithubTokenResponse>()
        .context("Failed to parse linked GitHub credentials response")?;
    Ok(Some(payload.access_token))
}

fn resolve_remote_url(root: &Path, remote: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg(remote)
        .current_dir(root)
        .output()
        .context("Failed to run git remote get-url")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("Unable to resolve git remote '{}'.", remote);
        }
        bail!("Unable to resolve git remote '{}': {}", remote, stderr);
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if remote_url.is_empty() {
        bail!("Git remote '{}' does not have a URL configured.", remote);
    }
    Ok(remote_url)
}

fn resolve_current_branch(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(root)
        .output()
        .context("Failed to run git rev-parse --abbrev-ref HEAD")?;

    if !output.status.success() {
        bail!("Unable to determine current branch. Pass `--branch` explicitly.");
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        bail!("Unable to determine current branch. Pass `--branch` explicitly.");
    }
    Ok(branch)
}

fn run_git_push(
    root: &Path,
    remote: &str,
    branch: Option<&str>,
    set_upstream: bool,
    force_with_lease: bool,
    github_token: Option<&str>,
    remote_url: &str,
) -> Result<()> {
    let mut command = Command::new("git");
    command.current_dir(root);

    if let Some(token) = github_token.filter(|_| is_github_https_url(remote_url)) {
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        command.arg("-c").arg(format!(
            "http.https://github.com/.extraheader=AUTHORIZATION: basic {basic}"
        ));
    }

    command.arg("push");
    if force_with_lease {
        command.arg("--force-with-lease");
    }
    if set_upstream {
        command.arg("--set-upstream");
    }
    command.arg(remote);
    if let Some(value) = branch {
        command.arg(value);
    }

    let status = command.status().context("Failed to run git push")?;
    if !status.success() {
        bail!("git push failed.");
    }

    Ok(())
}

fn is_github_https_url(repo_url: &str) -> bool {
    repo_url
        .to_ascii_lowercase()
        .starts_with("https://github.com/")
}
