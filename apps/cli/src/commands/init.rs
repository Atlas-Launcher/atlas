use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use protocol::config::atlas::{AtlasConfig, CliConfig, MetadataConfig, VersionsConfig};

#[derive(Args)]
pub struct InitArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
}

#[derive(Args)]
pub struct ReinitArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
}

pub fn run_init(args: InitArgs) -> Result<()> {
    let root = resolve_root(&args.input, true)?;
    let atlas_path = root.join("atlas.toml");

    if atlas_path.exists() && !confirm_overwrite(&atlas_path)? {
        println!("Aborted.");
        return Ok(());
    }

    let theme = ColorfulTheme::default();
    let metadata = prompt_metadata(&theme, &root)?;
    let versions = prompt_versions(&theme, None)?;
    let cli = prompt_cli(&theme, &metadata.name)?;

    let config = AtlasConfig {
        metadata,
        versions,
        cli,
    };

    write_atlas_config(&atlas_path, &config)?;
    ensure_seed_files(&root)?;

    println!("Initialized {}", atlas_path.display());
    Ok(())
}

pub fn run_reinit(args: ReinitArgs) -> Result<()> {
    let root = resolve_root(&args.input, false)?;
    let atlas_path = root.join("atlas.toml");
    if !atlas_path.exists() {
        bail!("atlas.toml not found at {}", atlas_path.display());
    }

    let mut config = crate::config::load_atlas_config(&root)?;
    let theme = ColorfulTheme::default();
    config.versions = prompt_versions(&theme, Some(&config.versions))?;

    write_atlas_config(&atlas_path, &config)?;
    println!("Updated versions in {}", atlas_path.display());
    Ok(())
}

fn resolve_root(input: &Path, create_if_missing: bool) -> Result<PathBuf> {
    if create_if_missing && !input.exists() {
        fs::create_dir_all(input)
            .with_context(|| format!("Failed to create input path {}", input.display()))?;
    }

    let root = input
        .canonicalize()
        .with_context(|| format!("Failed to resolve input path {}", input.display()))?;

    if !root.is_dir() {
        bail!("Input path must be a directory: {}", root.display());
    }

    Ok(root)
}

fn confirm_overwrite(path: &Path) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{} already exists. Overwrite?", path.display()))
        .default(false)
        .interact()
        .context("Failed to read confirmation")
}

fn prompt_metadata(theme: &ColorfulTheme, root: &Path) -> Result<MetadataConfig> {
    let default_name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Atlas Pack".to_string());

    let name: String = Input::with_theme(theme)
        .with_prompt("Pack name")
        .default(default_name.clone())
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("Pack name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to read pack name")?;

    let description: String = Input::with_theme(theme)
        .with_prompt("Description")
        .default("Atlas modpack".to_string())
        .interact_text()
        .context("Failed to read pack description")?;

    let version: String = Input::with_theme(theme)
        .with_prompt("Pack version")
        .default("0.1.0".to_string())
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("Version cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to read pack version")?;

    Ok(MetadataConfig {
        name: name.trim().to_string(),
        version: Some(version.trim().to_string()),
        description: non_empty(Some(description)),
    })
}

fn prompt_versions(
    theme: &ColorfulTheme,
    existing: Option<&VersionsConfig>,
) -> Result<VersionsConfig> {
    let loader_options = ["Fabric", "Forge", "NeoForge"];
    let default_loader = existing
        .map(|v| normalize_loader(&v.modloader))
        .unwrap_or("fabric");
    let default_loader_index = loader_options
        .iter()
        .position(|option| normalize_loader(option) == default_loader)
        .unwrap_or(0);

    let loader_selection = Select::with_theme(theme)
        .with_prompt("Select modloader")
        .items(&loader_options)
        .default(default_loader_index)
        .interact()
        .context("Failed to read modloader selection")?;

    let modloader = normalize_loader(loader_options[loader_selection]).to_string();
    let mc = prompt_minecraft_version(theme, existing.map(|v| v.mc.as_str()))?;

    let suggested_loader_version = existing
        .and_then(|v| {
            if normalize_loader(&v.modloader) == modloader {
                Some(v.modloader_version.clone())
            } else {
                None
            }
        })
        .and_then(|value| non_empty(Some(value)))
        .unwrap_or_else(|| default_loader_version(&modloader, &mc).to_string());

    let modloader_version: String = Input::with_theme(theme)
        .with_prompt(format!("{} loader version for MC {}", modloader, mc))
        .default(suggested_loader_version)
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("Modloader version cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to read modloader version")?;

    Ok(VersionsConfig {
        mc,
        modloader,
        modloader_version: modloader_version.trim().to_string(),
    })
}

fn prompt_minecraft_version(theme: &ColorfulTheme, existing: Option<&str>) -> Result<String> {
    let options = ["1.21.4", "1.21.1", "1.20.6", "1.20.1", "Custom"];
    let default_version = existing.unwrap_or("1.20.1").trim();
    let default_index = options
        .iter()
        .position(|candidate| *candidate == default_version)
        .unwrap_or(options.len() - 1);

    let selection = Select::with_theme(theme)
        .with_prompt("Select Minecraft version")
        .items(&options)
        .default(default_index)
        .interact()
        .context("Failed to read Minecraft version selection")?;

    if options[selection] == "Custom" {
        let custom_version: String = Input::with_theme(theme)
            .with_prompt("Minecraft version")
            .default(default_version.to_string())
            .validate_with(|input: &String| -> std::result::Result<(), &str> {
                if input.trim().is_empty() {
                    Err("Minecraft version cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact_text()
            .context("Failed to read Minecraft version")?;
        Ok(custom_version.trim().to_string())
    } else {
        Ok(options[selection].to_string())
    }
}

fn prompt_cli(theme: &ColorfulTheme, pack_name: &str) -> Result<Option<CliConfig>> {
    let configure_cli = Confirm::with_theme(theme)
        .with_prompt("Configure CLI defaults (pack_id/channel/hub_url)?")
        .default(true)
        .interact()
        .context("Failed to read CLI configuration choice")?;
    if !configure_cli {
        return Ok(None);
    }

    let default_pack_id = slugify(pack_name);
    let pack_id: String = Input::with_theme(theme)
        .with_prompt("Pack ID (optional)")
        .default(default_pack_id)
        .allow_empty(true)
        .interact_text()
        .context("Failed to read pack ID")?;

    let default_channel: String = Input::with_theme(theme)
        .with_prompt("Default deploy channel")
        .default("dev".to_string())
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("Channel cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .context("Failed to read default channel")?;

    let hub_url: String = Input::with_theme(theme)
        .with_prompt("Atlas Hub URL (optional)")
        .default("https://atlas.nathanm.org".to_string())
        .allow_empty(true)
        .interact_text()
        .context("Failed to read hub URL")?;

    let cli = CliConfig {
        pack_id: non_empty(Some(pack_id)),
        hub_url: non_empty(Some(hub_url)),
        default_channel: non_empty(Some(default_channel)),
    };

    if cli.pack_id.is_none() && cli.hub_url.is_none() && cli.default_channel.is_none() {
        Ok(None)
    } else {
        Ok(Some(cli))
    }
}

fn default_loader_version(modloader: &str, mc_version: &str) -> &'static str {
    match modloader {
        "forge" => {
            if mc_version.starts_with("1.20") {
                "47.2.0"
            } else {
                "latest"
            }
        }
        "neoforge" => {
            if mc_version.starts_with("1.21") {
                "21.1.0"
            } else {
                "latest"
            }
        }
        _ => "0.15.11",
    }
}

fn normalize_loader(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "forge" => "forge",
        "neo" | "neoforge" => "neoforge",
        _ => "fabric",
    }
}

fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            result.push(normalized);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
    }

    while result.starts_with('-') {
        result.remove(0);
    }
    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "atlas-pack".to_string()
    } else {
        result
    }
}

fn write_atlas_config(path: &Path, config: &AtlasConfig) -> Result<()> {
    let contents = toml::to_string(config).context("Failed to serialize atlas config")?;
    fs::write(path, format!("{}\n", contents))
        .with_context(|| format!("Failed to write {}", path.display()))
}

fn ensure_seed_files(root: &Path) -> Result<()> {
    let mods_toml = root.join("mods.toml");
    if !mods_toml.exists() {
        let starter = "# Add mod entries with `atlas pack add`.\n";
        fs::write(&mods_toml, starter)
            .with_context(|| format!("Failed to write {}", mods_toml.display()))?;
    }

    let mods_dir = root.join("mods");
    if !mods_dir.exists() {
        fs::create_dir_all(&mods_dir)
            .with_context(|| format!("Failed to create {}", mods_dir.display()))?;
    }

    let config_dir = root.join("config");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create {}", config_dir.display()))?;
    }

    Ok(())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
