use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use atlas_client::hub::{CiCompleteRequest, HubClient};
use clap::Args;
use reqwest::blocking::Client;

use crate::auth_store;
use crate::config;
use crate::io;

#[derive(Args)]
pub struct DeployArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long)]
    pack_id: Option<String>,
    #[arg(long)]
    hub_url: Option<String>,
    #[arg(long)]
    oidc_token: Option<String>,
    #[arg(long)]
    deploy_token: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    commit_hash: Option<String>,
    #[arg(long)]
    input_file: Option<PathBuf>,
    #[arg(long, default_value_t = protocol::DEFAULT_ZSTD_LEVEL)]
    zstd_level: i32,
}

pub fn run(args: DeployArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let settings = config::resolve_cli_settings(&root, args.pack_id, args.hub_url, args.channel)?;
    let ci_auth = resolve_ci_auth(args.oidc_token, args.deploy_token, &settings.hub_url)?;
    let commit_hash = resolve_commit_hash(&root, args.commit_hash)?;
    let commit_message = resolve_commit_message(&root, &commit_hash);
    let build_context = resolve_build_context(&root);
    let derived_version = commit_hash.clone();

    let (bytes, pack_id, version) = if let Some(input_file) = args.input_file.clone() {
        let bytes = io::read_bytes(&input_file)?;
        let pack_id = settings
            .pack_id
            .clone()
            .context("pack_id is required (use --pack-id or ATLAS_PACK_ID)")?;
        (bytes, pack_id, derived_version.clone())
    } else {
        let build = config::build_pack_bytes(
            &root,
            settings.pack_id.clone(),
            Some(derived_version.clone()),
            args.zstd_level,
        )?;
        (build.bytes, build.metadata.pack_id, build.metadata.version)
    };
    let artifact_size = bytes.len() as u64;

    let mut hub_client = HubClient::new(&settings.hub_url)?;
    apply_ci_auth_to_client(&mut hub_client, &ci_auth)?;
    let presign = hub_client.blocking_presign_ci_upload(&pack_id)?;

    let upload_client = Client::new();
    upload_artifact(
        &upload_client,
        &presign.upload_url,
        &presign.upload_headers,
        bytes,
    )?;

    hub_client.blocking_complete_ci_build(&CiCompleteRequest {
        pack_id: pack_id.clone(),
        build_id: presign.build_id.clone(),
        artifact_key: presign.artifact_key.clone(),
        version: version.clone(),
        commit_hash: Some(commit_hash.clone()),
        commit_message: commit_message.clone(),
        minecraft_version: build_context
            .as_ref()
            .map(|value| value.minecraft_version.clone()),
        modloader: build_context.as_ref().map(|value| value.modloader.clone()),
        modloader_version: build_context
            .as_ref()
            .and_then(|value| value.modloader_version.clone()),
        artifact_size,
        channel: settings.channel.clone(),
    })?;

    println!(
        "Deployed {} (version {}) to {}",
        pack_id, version, settings.channel
    );
    Ok(())
}

enum CiAuth {
    UserToken(String),
    OidcToken(String),
    PackDeployToken(String),
}

fn upload_artifact(
    client: &Client,
    upload_url: &str,
    upload_headers: &std::collections::HashMap<String, String>,
    bytes: Vec<u8>,
) -> Result<()> {
    let mut request = client.put(upload_url);

    if upload_headers.is_empty() {
        request = request.header("Content-Type", "application/octet-stream");
    } else {
        for (name, value) in upload_headers {
            request = request.header(name, value);
        }
    }

    request
        .body(bytes)
        .send()
        .context("Failed to upload artifact")?
        .error_for_status()
        .context("Upload failed")?;
    Ok(())
}

fn resolve_ci_auth(
    oidc_token_override: Option<String>,
    deploy_token_override: Option<String>,
    hub_url: &str,
) -> Result<CiAuth> {
    let oidc_token = normalize_optional(oidc_token_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_CI_OIDC_TOKEN").ok()));
    if let Some(token) = oidc_token {
        return Ok(CiAuth::OidcToken(token));
    }

    let deploy_token = normalize_optional(deploy_token_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_PACK_DEPLOY_TOKEN").ok()));
    if let Some(token) = deploy_token {
        return Ok(CiAuth::PackDeployToken(token));
    }

    let user_token = auth_store::require_access_token_for_hub(hub_url).with_context(|| {
        "No deploy credential provided. Use `--oidc-token` (`ATLAS_CI_OIDC_TOKEN`), `--deploy-token` (`ATLAS_PACK_DEPLOY_TOKEN`), or sign in locally with `atlas auth signin`."
    })?;
    Ok(CiAuth::UserToken(user_token))
}

fn apply_ci_auth_to_client(client: &mut HubClient, ci_auth: &CiAuth) -> Result<()> {
    match ci_auth {
        CiAuth::UserToken(token) => client.set_token(token.clone()),
        CiAuth::OidcToken(token) => client.set_service_token(token.clone()),
        CiAuth::PackDeployToken(token) => client.set_pack_deploy_token(token.clone()),
    }
    Ok(())
}

fn resolve_commit_hash(root: &std::path::Path, commit_hash_arg: Option<String>) -> Result<String> {
    if let Some(commit_hash) = normalize_optional(commit_hash_arg) {
        return Ok(commit_hash);
    }

    if let Some(commit_hash) = normalize_optional(std::env::var("GITHUB_SHA").ok()) {
        return Ok(commit_hash);
    }

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(root)
        .output()
        .context("Failed to run git rev-parse HEAD")?;

    if !output.status.success() {
        anyhow::bail!(
            "commit hash is required (use --commit-hash, GITHUB_SHA, or run deploy from a git repository)"
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    normalize_optional(Some(stdout)).context(
        "commit hash is required (use --commit-hash, GITHUB_SHA, or run deploy from a git repository)",
    )
}

fn resolve_commit_message(root: &std::path::Path, commit_hash: &str) -> Option<String> {
    if let Some(message) = normalize_optional(std::env::var("GITHUB_COMMIT_MESSAGE").ok()) {
        return Some(message);
    }

    let output = Command::new("git")
        .arg("show")
        .arg("-s")
        .arg("--format=%s")
        .arg(commit_hash)
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    normalize_optional(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

struct BuildContext {
    minecraft_version: String,
    modloader: String,
    modloader_version: Option<String>,
}

fn resolve_build_context(root: &std::path::Path) -> Option<BuildContext> {
    let config = config::load_atlas_config(root).ok()?;
    Some(BuildContext {
        minecraft_version: config.versions.mc,
        modloader: config.versions.modloader,
        modloader_version: normalize_optional(Some(config.versions.modloader_version)),
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
