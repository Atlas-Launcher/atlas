use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

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
    api_key: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    commit_hash: Option<String>,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    input_file: Option<PathBuf>,
    #[arg(long, default_value_t = protocol::DEFAULT_ZSTD_LEVEL)]
    zstd_level: i32,
}

pub fn run(args: DeployArgs) -> Result<()> {
    let root = args.input.canonicalize().context("Failed to resolve input path")?;
    let settings = config::resolve_cli_settings(&root, args.pack_id, args.hub_url, args.channel)?;
    let api_key = config::resolve_api_key(args.api_key)?;

    let (bytes, pack_id, version) = if let Some(input_file) = args.input_file.clone() {
        let bytes = io::read_bytes(&input_file)?;
        let config = config::load_atlas_config(&root)?;
        let pack_id = settings
            .pack_id
            .clone()
            .context("pack_id is required (use --pack-id or ATLAS_PACK_ID)")?;
        let version = args
            .version
            .or(config.metadata.version)
            .context("version is required (use --version)")?;
        (bytes, pack_id, version)
    } else {
        let build = config::build_pack_bytes(
            &root,
            settings.pack_id.clone(),
            args.version.clone(),
            args.zstd_level,
        )?;
        (
            build.bytes,
            build.metadata.pack_id,
            build.metadata.version,
        )
    };

    let commit_hash = args
        .commit_hash
        .or_else(|| std::env::var("GITHUB_SHA").ok());
    let artifact_size = bytes.len() as u64;

    let client = Client::new();
    let presign = request_presign(
        &client,
        &settings.hub_url,
        &api_key,
        &pack_id,
    )?;

    upload_artifact(&client, &presign.upload_url, bytes)?;
    complete_build(
        &client,
        &settings.hub_url,
        &api_key,
        CompleteRequest {
            pack_id: &pack_id,
            build_id: &presign.build_id,
            artifact_key: &presign.artifact_key,
            version: &version,
            commit_hash: commit_hash.as_deref(),
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
    #[serde(rename = "artifactSize")]
    artifact_size: u64,
    channel: &'a str,
}

fn request_presign(
    client: &Client,
    hub_url: &str,
    api_key: &str,
    pack_id: &str,
) -> Result<PresignResponse> {
    client
        .post(format!("{}/api/ci/presign", hub_url))
        .bearer_auth(api_key)
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
    api_key: &str,
    payload: CompleteRequest<'_>,
) -> Result<()> {
    client
        .post(format!("{}/api/ci/complete", hub_url))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .context("Failed to complete build")?
        .error_for_status()
        .context("Complete request failed")?;
    Ok(())
}
