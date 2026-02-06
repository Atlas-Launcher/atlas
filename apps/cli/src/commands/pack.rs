use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use mod_resolver::Provider;

use crate::auth_store;
use crate::config;
use crate::io;

#[derive(Subcommand)]
pub enum PackCommand {
    Build(BuildArgs),
    Add(AddArgs),
    Validate(ValidateArgs),
}

#[derive(Args)]
pub struct BuildArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(long)]
    pack_id: Option<String>,
    #[arg(long)]
    version: Option<String>,
    #[arg(long, default_value = "dist/atlas-pack.atlas")]
    output: PathBuf,
    #[arg(long, default_value_t = protocol::DEFAULT_ZSTD_LEVEL)]
    zstd_level: i32,
}

#[derive(Args)]
pub struct AddArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_parser = ["cf", "mr"])]
    source: String,
    query: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long, default_value = "mod")]
    pack_type: String,
}

#[derive(Args)]
pub struct ValidateArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
}

pub fn run(command: PackCommand) -> Result<()> {
    match command {
        PackCommand::Build(args) => build(args),
        PackCommand::Add(args) => add(args),
        PackCommand::Validate(args) => validate(args),
    }
}

fn build(args: BuildArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let build = config::build_pack_bytes(&root, args.pack_id, args.version, args.zstd_level)?;
    io::write_output(&args.output, &build.bytes)?;
    println!("Wrote {}", args.output.display());
    Ok(())
}

fn add(args: AddArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let config = config::load_atlas_config(&root)?;
    let loader = config.versions.modloader;
    let minecraft_version = config.versions.mc;
    let desired_version = args.version;

    let pack_type = args.pack_type.to_lowercase();
    if !["mod", "shader", "resourcepack"].contains(&pack_type.as_str()) {
        bail!("pack type must be one of: mod, shader, resourcepack");
    }

    let provider = Provider::from_short_code(&args.source).context("source must be cf or mr")?;
    let entry = match provider {
        Provider::Modrinth => mod_resolver::resolve_blocking(
            provider,
            &args.query,
            &loader,
            &minecraft_version,
            desired_version.as_deref(),
            &pack_type,
        )?,
        Provider::CurseForge => {
            let settings = config::resolve_cli_settings(&root, None, None, None)?;
            let token = auth_store::require_access_token_for_hub(&settings.hub_url)?;
            mod_resolver::resolve_curseforge_via_proxy_blocking(
                &settings.hub_url,
                &token,
                &args.query,
                &loader,
                &minecraft_version,
                desired_version.as_deref(),
                &pack_type,
            )?
        }
    };

    io::write_mod_entry(&root, &entry)?;
    println!("Added {}", entry.project_id);
    Ok(())
}

fn validate(args: ValidateArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let config_text = io::read_to_string(&root.join("atlas.toml"))?;
    let _config = protocol::config::atlas::parse_config(&config_text)
        .map_err(|_| anyhow::anyhow!("atlas.toml is invalid"))?;

    let mods_dir = root.join("mods");
    if mods_dir.exists() {
        for entry in std::fs::read_dir(&mods_dir).context("Failed to read mods directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let contents = io::read_to_string(&path)?;
                protocol::config::mods::parse_mod_toml(&contents)
                    .map_err(|_| anyhow::anyhow!("Invalid mod file: {}", path.display()))?;
            }
        }
    }

    println!("Pack config is valid.");
    Ok(())
}
