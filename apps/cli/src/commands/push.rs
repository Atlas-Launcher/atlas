use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;

#[derive(Args)]
pub struct PushArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long, default_value = "origin")]
    remote: String,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    set_upstream: bool,
    #[arg(long)]
    force_with_lease: bool,
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

    println!(
        "Pushing {}{} via system git credentials ({})",
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
    )?;
    Ok(())
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
) -> Result<()> {
    let mut command = Command::new("git");
    command.current_dir(root);

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
