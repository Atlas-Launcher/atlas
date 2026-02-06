use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::Args;
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

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
    let ci_auth = resolve_ci_auth(args.oidc_token, &settings.hub_url)?;
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

    let client = Client::new();
    let presign = request_presign(&client, &settings.hub_url, &ci_auth, &pack_id)?;

    upload_artifact(&client, &presign.upload_url, bytes)?;
    complete_build(
        &client,
        &settings.hub_url,
        &ci_auth,
        CompleteRequest {
            pack_id: &pack_id,
            build_id: &presign.build_id,
            artifact_key: &presign.artifact_key,
            version: &version,
            commit_hash: Some(commit_hash.as_str()),
            commit_message: commit_message.as_deref(),
            minecraft_version: build_context
                .as_ref()
                .map(|value| value.minecraft_version.as_str()),
            modloader: build_context.as_ref().map(|value| value.modloader.as_str()),
            modloader_version: build_context
                .as_ref()
                .and_then(|value| value.modloader_version.as_deref()),
            artifact_size,
            channel: &settings.channel,
        },
    )?;

    println!(
        "Deployed {} (version {}) to {}",
        pack_id, version, settings.channel
    );
    Ok(())
}

enum CiAuth {
    UserToken(String),
    OidcToken(String),
}

#[derive(Serialize)]
struct PresignRequest<'a> {
    #[serde(rename = "packId")]
    pack_id: &'a str,
}

#[derive(Deserialize)]
struct PresignResponse {
    #[serde(rename = "buildId")]
    build_id: String,
    #[serde(rename = "artifactKey")]
    artifact_key: String,
    #[serde(rename = "uploadUrl")]
    upload_url: String,
}

#[derive(Serialize)]
struct CompleteRequest<'a> {
    #[serde(rename = "packId")]
    pack_id: &'a str,
    #[serde(rename = "buildId")]
    build_id: &'a str,
    #[serde(rename = "artifactKey")]
    artifact_key: &'a str,
    version: &'a str,
    #[serde(rename = "commitHash", skip_serializing_if = "Option::is_none")]
    commit_hash: Option<&'a str>,
    #[serde(rename = "commitMessage", skip_serializing_if = "Option::is_none")]
    commit_message: Option<&'a str>,
    #[serde(rename = "minecraftVersion", skip_serializing_if = "Option::is_none")]
    minecraft_version: Option<&'a str>,
    #[serde(rename = "modloader", skip_serializing_if = "Option::is_none")]
    modloader: Option<&'a str>,
    #[serde(rename = "modloaderVersion", skip_serializing_if = "Option::is_none")]
    modloader_version: Option<&'a str>,
    #[serde(rename = "artifactSize")]
    artifact_size: u64,
    channel: &'a str,
}

fn request_presign(
    client: &Client,
    hub_url: &str,
    ci_auth: &CiAuth,
    pack_id: &str,
) -> Result<PresignResponse> {
    apply_ci_auth(client.post(format!("{}/api/ci/presign", hub_url)), ci_auth)
        .json(&PresignRequest { pack_id })
        .send()
        .context("Failed to request presigned URL")?
        .error_for_status()
        .context("Presign request failed")?
        .json::<PresignResponse>()
        .context("Failed to parse presign response")
}

fn upload_artifact(client: &Client, upload_url: &str, bytes: Vec<u8>) -> Result<()> {
    client
        .put(upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .context("Failed to upload artifact")?
        .error_for_status()
        .context("Upload failed")?;
    Ok(())
}

fn complete_build(
    client: &Client,
    hub_url: &str,
    ci_auth: &CiAuth,
    payload: CompleteRequest<'_>,
) -> Result<()> {
    apply_ci_auth(client.post(format!("{}/api/ci/complete", hub_url)), ci_auth)
        .json(&payload)
        .send()
        .context("Failed to complete build")?
        .error_for_status()
        .context("Complete request failed")?;
    Ok(())
}

fn resolve_ci_auth(oidc_token_override: Option<String>, hub_url: &str) -> Result<CiAuth> {
    let oidc_token = normalize_optional(oidc_token_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_CI_OIDC_TOKEN").ok()));
    if let Some(token) = oidc_token {
        return Ok(CiAuth::OidcToken(token));
    }

    let user_token = auth_store::require_access_token_for_hub(hub_url).with_context(|| {
        "No deploy credential provided. Use `--oidc-token` (or `ATLAS_CI_OIDC_TOKEN`) for CI, or sign in locally with `atlas auth signin`."
    })?;
    Ok(CiAuth::UserToken(user_token))
}

fn apply_ci_auth(builder: RequestBuilder, ci_auth: &CiAuth) -> RequestBuilder {
    match ci_auth {
        CiAuth::UserToken(token) => builder.bearer_auth(token),
        CiAuth::OidcToken(token) => builder.header("x-atlas-oidc-token", token),
    }
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
