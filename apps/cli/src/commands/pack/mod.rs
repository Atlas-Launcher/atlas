use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::{self as stdio, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use dialoguer::{Select, theme::ColorfulTheme};
use mod_resolver::{Provider, SearchCandidate};
use walkdir::WalkDir;

use crate::auth_store;
use crate::commands::{init, pull, push};
use crate::config;
use crate::io;

mod mrpack;

const SEARCH_PAGE_SIZE: usize = 5;

#[derive(Subcommand)]
pub enum PackCommand {
    Init(init::InitArgs),
    Reinit(init::ReinitArgs),
    Channel(ChannelArgs),
    Build(BuildArgs),
    Add(AddArgs),
    Import(ImportArgs),
    #[command(alias = "remove")]
    Rm(RmArgs),
    List(ListArgs),
    Pull(pull::PullArgs),
    Push(push::PushArgs),
    Commit(CommitArgs),
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
    #[arg(long, default_value = "atlas", value_parser = ["atlas", "mrpack"])]
    format: String,
    #[arg(long, default_value_t = protocol::DEFAULT_ZSTD_LEVEL)]
    zstd_level: i32,
}

#[derive(Args)]
pub struct ChannelArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_name = "CHANNEL", value_parser = ["dev", "beta", "production"])]
    channel: String,
}

#[derive(Args)]
pub struct AddArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_parser = ["cf", "mr"])]
    source: String,
    #[arg(value_name = "QUERY", required_unless_present = "slug")]
    query: Option<String>,
    #[arg(long, value_name = "SLUG", conflicts_with = "query")]
    slug: Option<String>,
    #[arg(long)]
    version: Option<String>,
    #[arg(
        long = "type",
        default_value = "mod",
        value_name = "TYPE",
        value_parser = ["mod", "shader", "shaderpack", "resourcepack", "other"]
    )]
    asset_type: String,
}

#[derive(Args)]
pub struct ValidateArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
}

#[derive(Args)]
pub struct ImportArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_name = "MRPACK")]
    file: PathBuf,
}

#[derive(Args)]
pub struct ListArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
}

#[derive(Args)]
pub struct RmArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_name = "QUERY")]
    query: String,
    #[arg(
        long = "type",
        value_name = "TYPE",
        default_value = "any",
        value_parser = ["any", "mod", "resource"]
    )]
    asset_type: String,
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
pub struct CommitArgs {
    #[arg(long, default_value = ".")]
    input: PathBuf,
    #[arg(value_name = "MESSAGE")]
    message: String,
}

pub fn run(command: PackCommand) -> Result<()> {
    match command {
        PackCommand::Init(args) => init::run_init(args),
        PackCommand::Reinit(args) => init::run_reinit(args),
        PackCommand::Channel(args) => set_channel(args),
        PackCommand::Build(args) => build(args),
        PackCommand::Add(args) => add(args),
        PackCommand::Import(args) => mrpack::import(args),
        PackCommand::Rm(args) => rm(args),
        PackCommand::List(args) => list(args),
        PackCommand::Pull(args) => pull::run(args),
        PackCommand::Push(args) => push::run(args),
        PackCommand::Commit(args) => commit(args),
        PackCommand::Validate(args) => validate(args),
    }
}

fn set_channel(args: ChannelArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let atlas_path = root.join("atlas.toml");
    if !atlas_path.exists() {
        bail!("atlas.toml not found at {}", atlas_path.display());
    }

    let config_text = io::read_to_string(&atlas_path)?;
    let mut config = protocol::config::atlas::parse_config(&config_text)
        .map_err(|_| anyhow::anyhow!("atlas.toml is invalid"))?;

    let cli = config.cli.get_or_insert_with(Default::default);
    cli.default_channel = Some(args.channel.clone());

    let contents = toml::to_string(&config).context("Failed to serialize atlas config")?;
    fs::write(&atlas_path, format!("{contents}\n"))
        .with_context(|| format!("Failed to write {}", atlas_path.display()))?;

    println!(
        "Set cli.default_channel={} in {}",
        args.channel,
        atlas_path.display()
    );
    Ok(())
}

fn build(args: BuildArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    match args.format.as_str() {
        "atlas" => {
            let build =
                config::build_pack_bytes(&root, args.pack_id, args.version, args.zstd_level)?;
            io::write_output(&args.output, &build.bytes)?;
            println!("Wrote {}", args.output.display());
        }
        "mrpack" => mrpack::build(&args, &root)?,
        other => bail!("Unsupported build format '{}'. Use atlas or mrpack.", other),
    }
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
    let desired_version = args.version.clone();
    let asset_kind = AssetKind::from_input(&args.asset_type)?;
    let pack_type = asset_kind.resolver_pack_type();

    let provider = Provider::from_short_code(&args.source).context("source must be cf or mr")?;
    let curseforge_auth = match provider {
        Provider::CurseForge => {
            let settings = config::resolve_cli_settings(&root, None, None, None)?;
            let access_token = auth_store::require_access_token_for_hub(&settings.hub_url)?;
            Some(CurseForgeAuth {
                hub_url: settings.hub_url,
                access_token,
            })
        }
        Provider::Modrinth => None,
    };

    let selected = if let Some(slug) = args.slug.as_deref() {
        resolve_slug_candidate(
            provider,
            pack_type,
            slug,
            &loader,
            &minecraft_version,
            curseforge_auth.as_ref(),
        )?
    } else {
        if !stdio::stdin().is_terminal() || !stdio::stdout().is_terminal() {
            bail!(
                "Search UI requires an interactive terminal. Use `--slug=<provider-slug>` in non-interactive mode."
            );
        }

        let query = args
            .query
            .as_deref()
            .context("query is required unless --slug is set")?;
        prompt_search_selection(
            provider,
            pack_type,
            query,
            &loader,
            &minecraft_version,
            curseforge_auth.as_ref(),
        )?
    };
    let Some(selected) = selected else {
        println!("Cancelled.");
        return Ok(());
    };

    let mut existing = load_existing_mod_keys(&root)?;
    let mut visited_projects = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(QueuedResolution {
        project_id: selected.project_id.clone(),
        desired_version: desired_version.clone(),
        preferred_name: Some(selected.title),
        preferred_project_url: selected.project_url,
    });
    visited_projects.insert(selected.project_id);

    let mut added_count = 0usize;
    let mut skipped_existing_count = 0usize;

    while let Some(next) = queue.pop_front() {
        let resolved = resolve_project(
            provider,
            pack_type,
            &next.project_id,
            &loader,
            &minecraft_version,
            next.desired_version.as_deref(),
            curseforge_auth.as_ref(),
        )?;

        let mut entry = resolved.entry;
        if entry.metadata.name.trim().is_empty() {
            if let Some(name) = next.preferred_name.filter(|value| !value.trim().is_empty()) {
                entry.metadata.name = name;
            }
        }
        if entry
            .metadata
            .project_url
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            entry.metadata.project_url = next
                .preferred_project_url
                .filter(|value| !value.trim().is_empty());
        }

        let key = mod_key(&entry.download.source, &entry.download.project_id);
        if existing.insert(key) {
            if asset_kind == AssetKind::Mod {
                io::write_mod_entry(&root, &entry)?;
            } else {
                io::write_resource_entry(&root, &entry, asset_kind.resource_pointer_directory())?;
            }
            added_count += 1;
            println!("Added {}", mod_reference_for_entry(&entry));
        } else {
            skipped_existing_count += 1;
        }

        if asset_kind == AssetKind::Mod {
            for dependency in resolved.dependencies {
                if visited_projects.insert(dependency.project_id.clone()) {
                    queue.push_back(QueuedResolution {
                        project_id: dependency.project_id,
                        desired_version: dependency.desired_version,
                        preferred_name: None,
                        preferred_project_url: None,
                    });
                }
            }
        }
    }

    if added_count == 0 {
        println!("No new mods were added.");
    } else {
        println!("Added {} mod(s).", added_count);
    }
    if skipped_existing_count > 0 {
        println!("Skipped {} existing mod(s).", skipped_existing_count);
    }

    Ok(())
}

fn resolve_slug_candidate(
    provider: Provider,
    pack_type: &str,
    slug: &str,
    loader: &str,
    minecraft_version: &str,
    curseforge_auth: Option<&CurseForgeAuth>,
) -> Result<Option<SearchCandidate>> {
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        bail!("--slug cannot be empty.");
    }

    match provider {
        Provider::Modrinth => Ok(Some(SearchCandidate {
            project_id: trimmed.to_string(),
            slug: trimmed.to_string(),
            title: trimmed.to_string(),
            description: None,
            project_url: Some(modrinth_project_url(pack_type, trimmed)),
        })),
        Provider::CurseForge => {
            let mut offset = 0usize;
            loop {
                let candidates = search_candidates(
                    provider,
                    pack_type,
                    trimmed,
                    loader,
                    minecraft_version,
                    offset,
                    50,
                    curseforge_auth,
                )?;
                if candidates.is_empty() {
                    break;
                }

                if let Some(candidate) = candidates
                    .iter()
                    .find(|candidate| candidate.slug.eq_ignore_ascii_case(trimmed))
                {
                    return Ok(Some(candidate.clone()));
                }

                if candidates.len() < 50 {
                    break;
                }
                offset += 50;
            }

            bail!(
                "No compatible CurseForge mod found with slug '{}' for Minecraft {} ({}).",
                trimmed,
                minecraft_version,
                loader
            );
        }
    }
}

fn prompt_search_selection(
    provider: Provider,
    pack_type: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    curseforge_auth: Option<&CurseForgeAuth>,
) -> Result<Option<SearchCandidate>> {
    let mut offset = 0usize;
    loop {
        let candidates = search_candidates(
            provider,
            pack_type,
            query,
            loader,
            minecraft_version,
            offset,
            SEARCH_PAGE_SIZE + 1,
            curseforge_auth,
        )?;

        if candidates.is_empty() {
            if offset == 0 {
                bail!(
                    "No compatible {} mods found for '{}' on Minecraft {} ({})",
                    provider_label(provider),
                    query,
                    minecraft_version,
                    loader
                );
            }
            offset = offset.saturating_sub(SEARCH_PAGE_SIZE);
            continue;
        }

        let has_next_page = candidates.len() > SEARCH_PAGE_SIZE;
        let visible = candidates
            .into_iter()
            .take(SEARCH_PAGE_SIZE)
            .collect::<Vec<_>>();

        let mut items = visible
            .iter()
            .map(format_candidate_label)
            .collect::<Vec<_>>();
        let mut nav = Vec::new();
        if offset > 0 {
            items.push("Previous results".to_string());
            nav.push(NavigationAction::Previous);
        }
        if has_next_page {
            items.push("Next results".to_string());
            nav.push(NavigationAction::Next);
        }
        items.push("Cancel".to_string());
        nav.push(NavigationAction::Cancel);

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Select a {} mod (page {})",
                provider_label(provider),
                offset / SEARCH_PAGE_SIZE + 1
            ))
            .items(&items)
            .default(0)
            .interact()
            .context("Failed to read mod selection")?;

        if selection < visible.len() {
            return visible
                .get(selection)
                .cloned()
                .context("Invalid search selection")
                .map(Some);
        }

        let action = nav
            .get(selection - visible.len())
            .copied()
            .context("Invalid navigation selection")?;
        match action {
            NavigationAction::Previous => {
                offset = offset.saturating_sub(SEARCH_PAGE_SIZE);
            }
            NavigationAction::Next => {
                offset += SEARCH_PAGE_SIZE;
            }
            NavigationAction::Cancel => return Ok(None),
        }
    }
}

fn search_candidates(
    provider: Provider,
    pack_type: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    offset: usize,
    limit: usize,
    curseforge_auth: Option<&CurseForgeAuth>,
) -> Result<Vec<SearchCandidate>> {
    match provider {
        Provider::Modrinth => mod_resolver::search_blocking(
            provider,
            query,
            loader,
            minecraft_version,
            pack_type,
            offset,
            limit,
        ),
        Provider::CurseForge => {
            let auth = curseforge_auth.context("CurseForge authentication is required")?;
            mod_resolver::search_curseforge_via_proxy_blocking(
                &auth.hub_url,
                &auth.access_token,
                query,
                loader,
                minecraft_version,
                pack_type,
                offset,
                limit,
            )
        }
    }
}

fn resolve_project(
    provider: Provider,
    pack_type: &str,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    curseforge_auth: Option<&CurseForgeAuth>,
) -> Result<mod_resolver::ResolvedMod> {
    match provider {
        Provider::Modrinth => mod_resolver::resolve_by_project_id_blocking(
            provider,
            project_id,
            loader,
            minecraft_version,
            desired_version,
            pack_type,
        ),
        Provider::CurseForge => {
            let auth = curseforge_auth.context("CurseForge authentication is required")?;
            mod_resolver::resolve_curseforge_by_project_id_via_proxy_blocking(
                &auth.hub_url,
                &auth.access_token,
                project_id,
                loader,
                minecraft_version,
                desired_version,
                pack_type,
            )
        }
    }
}

fn load_existing_mod_keys(root: &Path) -> Result<HashSet<String>> {
    let mut keys = HashSet::new();
    for path in pointer_paths(root)? {
        let contents = io::read_to_string(&path)?;
        let parsed = protocol::config::mods::parse_mod_toml(&contents)
            .map_err(|_| anyhow::anyhow!("Invalid pointer file: {}", path.display()))?;
        keys.insert(mod_key(
            &parsed.download.source,
            &parsed.download.project_id,
        ));
    }
    Ok(keys)
}

fn pointer_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if is_excluded_path(&rel) {
            continue;
        }

        if rel.ends_with(".mod.toml") || rel.ends_with(".res.toml") {
            paths.push(path.to_path_buf());
        }
    }
    Ok(paths)
}

fn mod_key(source: &str, project_id: &str) -> String {
    format!(
        "{}:{}",
        source.trim().to_ascii_lowercase(),
        project_id.trim()
    )
}

fn format_candidate_label(candidate: &SearchCandidate) -> String {
    let mut label = format!(
        "{} ({})",
        candidate.title,
        candidate
            .project_url
            .clone()
            .unwrap_or_else(|| candidate.slug.clone())
    );
    if let Some(description) = candidate
        .description
        .as_ref()
        .map(|value| normalize_inline(value))
        .filter(|value| !value.is_empty())
    {
        label.push_str(" - ");
        label.push_str(&truncate_for_ui(&description, 90));
    }
    label
}

fn normalize_inline(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_for_ui(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let take_count = max_chars.saturating_sub(3);
    let prefix = value.chars().take(take_count).collect::<String>();
    format!("{}...", prefix)
}

fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Modrinth => "Modrinth",
        Provider::CurseForge => "CurseForge",
    }
}

fn modrinth_project_url(pack_type: &str, slug_or_id: &str) -> String {
    let base = match pack_type {
        "mod" => "mod",
        "resourcepack" => "resourcepack",
        "shader" => "shader",
        _ => "project",
    };
    format!("https://modrinth.com/{}/{}", base, slug_or_id.trim())
}

fn is_excluded_path(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.starts_with(".git/")
        || lower.starts_with("target/")
        || lower.starts_with("node_modules/")
        || lower.starts_with(".next/")
        || lower.starts_with("dist/")
}

fn is_utf8_file(path: &Path) -> Result<bool> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(std::str::from_utf8(&bytes).is_ok())
}

fn display_name_for_entry(entry: &protocol::config::mods::ModEntry) -> String {
    if !entry.metadata.name.trim().is_empty() {
        entry.metadata.name.trim().to_string()
    } else {
        entry.download.project_id.clone()
    }
}

fn mod_reference_for_entry(entry: &protocol::config::mods::ModEntry) -> String {
    let name = display_name_for_entry(entry);
    let url = entry
        .metadata
        .project_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-");
    format!("{} ({})", name, url)
}

fn rm(args: RmArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let query = args.query.trim();
    if query.is_empty() {
        bail!("query cannot be empty");
    }

    let filter = RemoveAssetFilter::from_input(&args.asset_type)?;
    let pointers = load_pointer_resources(&root)?
        .into_iter()
        .filter(|pointer| filter.matches(pointer.kind))
        .collect::<Vec<_>>();

    if pointers.is_empty() {
        bail!("No {} pointer files found.", filter.label_plural());
    }

    let matches = find_pointer_matches(&pointers, query);
    if matches.is_empty() {
        bail!("No {} matched '{}'.", filter.label_plural(), query);
    }

    let selected = if args.all {
        matches
    } else if matches.len() == 1 {
        matches
    } else if !stdio::stdin().is_terminal() || !stdio::stdout().is_terminal() {
        let candidates = matches
            .iter()
            .map(|index| {
                let pointer = &pointers[*index];
                format!(
                    "{} -> {}",
                    pointer.rel_path,
                    mod_reference_for_entry(&pointer.entry)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "Multiple matches for '{}'. Use --all or run in an interactive terminal.\n{}",
            query,
            candidates
        );
    } else {
        let mut items = matches
            .iter()
            .map(|index| {
                let pointer = &pointers[*index];
                format!(
                    "{} -> {}",
                    pointer.rel_path,
                    mod_reference_for_entry(&pointer.entry)
                )
            })
            .collect::<Vec<_>>();
        items.push("Cancel".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Select {} to remove", filter.label_singular()))
            .items(&items)
            .default(0)
            .interact()
            .context("Failed to read removal selection")?;

        if selection == items.len() - 1 {
            println!("Cancelled.");
            return Ok(());
        }

        vec![matches[selection]]
    };

    let mut removed = 0usize;
    for index in selected {
        let pointer = &pointers[index];
        std::fs::remove_file(&pointer.path)
            .with_context(|| format!("Failed to remove {}", pointer.path.display()))?;
        removed += 1;
        println!(
            "Removed {} -> {}",
            pointer.rel_path,
            mod_reference_for_entry(&pointer.entry)
        );
    }

    println!("Removed {} {}.", removed, filter.label_with_count(removed));
    Ok(())
}

fn load_pointer_resources(root: &Path) -> Result<Vec<PointerResource>> {
    let mut pointers = Vec::new();
    for path in pointer_paths(root)? {
        let rel_path = path
            .strip_prefix(root)
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        let kind = pointer_kind_from_rel_path(&rel_path)?;
        let contents = io::read_to_string(&path)?;
        let entry = protocol::config::mods::parse_mod_toml(&contents)
            .map_err(|_| anyhow::anyhow!("Invalid pointer file: {}", path.display()))?;

        pointers.push(PointerResource {
            path,
            rel_path,
            entry,
            kind,
        });
    }
    Ok(pointers)
}

fn pointer_kind_from_rel_path(rel_path: &str) -> Result<PointerKind> {
    if rel_path.ends_with(".mod.toml") {
        return Ok(PointerKind::Mod);
    }
    if rel_path.ends_with(".res.toml") {
        return Ok(PointerKind::Resource);
    }
    bail!("Unsupported pointer file: {}", rel_path);
}

fn find_pointer_matches(pointers: &[PointerResource], query: &str) -> Vec<usize> {
    let query_lower = query.to_ascii_lowercase();
    let mut exact = Vec::new();
    let mut partial = Vec::new();

    for (index, pointer) in pointers.iter().enumerate() {
        if pointer_matches_exact(pointer, &query_lower) {
            exact.push(index);
            continue;
        }
        if pointer_matches_partial(pointer, &query_lower) {
            partial.push(index);
        }
    }

    if !exact.is_empty() { exact } else { partial }
}

fn pointer_matches_exact(pointer: &PointerResource, query_lower: &str) -> bool {
    let rel_lower = pointer.rel_path.to_ascii_lowercase();
    if rel_lower == query_lower {
        return true;
    }

    if let Some(file_name) = Path::new(&pointer.rel_path)
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
    {
        if file_name == query_lower {
            return true;
        }
    }

    if pointer.entry.download.project_id.to_ascii_lowercase() == query_lower {
        return true;
    }

    if display_name_for_entry(&pointer.entry).to_ascii_lowercase() == query_lower {
        return true;
    }

    let source_and_project = mod_key(
        &pointer.entry.download.source,
        &pointer.entry.download.project_id,
    );
    source_and_project == query_lower
}

fn pointer_matches_partial(pointer: &PointerResource, query_lower: &str) -> bool {
    let rel_lower = pointer.rel_path.to_ascii_lowercase();
    if rel_lower.contains(query_lower) {
        return true;
    }

    if pointer
        .entry
        .download
        .project_id
        .to_ascii_lowercase()
        .contains(query_lower)
    {
        return true;
    }

    if display_name_for_entry(&pointer.entry)
        .to_ascii_lowercase()
        .contains(query_lower)
    {
        return true;
    }

    pointer
        .entry
        .metadata
        .project_url
        .as_deref()
        .map(|value| value.to_ascii_lowercase().contains(query_lower))
        .unwrap_or(false)
}

fn list(args: ListArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;

    let mut resources = Vec::new();
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(&root)
            .context("Failed to compute relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_excluded_path(&rel_str) {
            continue;
        }

        if rel_str.ends_with(".mod.toml") || rel_str.ends_with(".res.toml") {
            let contents = io::read_to_string(path)?;
            let parsed = protocol::config::mods::parse_mod_toml(&contents)
                .map_err(|_| anyhow::anyhow!("Invalid pointer file: {}", path.display()))?;
            resources.push(format!(
                "{}  ->  {}",
                rel_str,
                mod_reference_for_entry(&parsed)
            ));
            continue;
        }

        if is_utf8_file(path)? {
            resources.push(rel_str);
        }
    }

    resources.sort();
    if resources.is_empty() {
        println!("No pack resources found.");
        return Ok(());
    }

    for resource in resources {
        println!("{}", resource);
    }
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

    for path in pointer_paths(&root)? {
        let contents = io::read_to_string(&path)?;
        protocol::config::mods::parse_mod_toml(&contents)
            .map_err(|_| anyhow::anyhow!("Invalid pointer file: {}", path.display()))?;
    }

    println!("Pack config is valid.");
    Ok(())
}

fn commit(args: CommitArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;

    let message = args.message.trim();
    if message.is_empty() {
        bail!("Commit message is required.");
    }

    let status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&root)
        .output()
        .context("Failed to run `git status --porcelain`")?;
    if !status_output.status.success() {
        bail!("Unable to check repository status.");
    }

    let changed = !String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .is_empty();
    if !changed {
        println!("No changes to commit.");
        return Ok(());
    }

    let add_status = Command::new("git")
        .arg("add")
        .arg("-A")
        .current_dir(&root)
        .status()
        .context("Failed to run `git add -A`")?;
    if !add_status.success() {
        bail!("`git add -A` failed.");
    }

    let commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(&root)
        .status()
        .context("Failed to run `git commit`")?;
    if !commit_status.success() {
        bail!("`git commit` failed.");
    }

    Ok(())
}

struct CurseForgeAuth {
    hub_url: String,
    access_token: String,
}

struct QueuedResolution {
    project_id: String,
    desired_version: Option<String>,
    preferred_name: Option<String>,
    preferred_project_url: Option<String>,
}

struct PointerResource {
    path: PathBuf,
    rel_path: String,
    entry: protocol::config::mods::ModEntry,
    kind: PointerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointerKind {
    Mod,
    Resource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoveAssetFilter {
    Any,
    Mod,
    Resource,
}

impl RemoveAssetFilter {
    fn from_input(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "any" => Ok(Self::Any),
            "mod" => Ok(Self::Mod),
            "resource" => Ok(Self::Resource),
            other => bail!("Unsupported type '{}'. Use any, mod, or resource.", other),
        }
    }

    fn matches(self, kind: PointerKind) -> bool {
        match self {
            Self::Any => true,
            Self::Mod => kind == PointerKind::Mod,
            Self::Resource => kind == PointerKind::Resource,
        }
    }

    fn label_singular(self) -> &'static str {
        match self {
            Self::Any => "pointer",
            Self::Mod => "mod",
            Self::Resource => "resource",
        }
    }

    fn label_plural(self) -> &'static str {
        match self {
            Self::Any => "pointers",
            Self::Mod => "mods",
            Self::Resource => "resources",
        }
    }

    fn label_with_count(self, count: usize) -> &'static str {
        if count == 1 {
            self.label_singular()
        } else {
            self.label_plural()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Mod,
    Shader,
    Resourcepack,
    Other,
}

impl AssetKind {
    fn from_input(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "mod" => Ok(Self::Mod),
            "shader" | "shaderpack" => Ok(Self::Shader),
            "resourcepack" => Ok(Self::Resourcepack),
            "other" => Ok(Self::Other),
            other => bail!(
                "Unsupported type '{}'. Use mod, shader, resourcepack, or other.",
                other
            ),
        }
    }

    fn resolver_pack_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Shader => "shader",
            Self::Resourcepack => "resourcepack",
            Self::Other => "other",
        }
    }

    fn resource_pointer_directory(self) -> &'static str {
        match self {
            Self::Shader => "shaderpacks",
            Self::Resourcepack => "resourcepacks",
            Self::Other => "resources",
            Self::Mod => "mods",
        }
    }
}

#[derive(Clone, Copy)]
enum NavigationAction {
    Previous,
    Next,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::AssetKind;

    #[test]
    fn resource_pointer_directories_match_asset_type() {
        assert_eq!(
            AssetKind::Shader.resource_pointer_directory(),
            "shaderpacks"
        );
        assert_eq!(
            AssetKind::Resourcepack.resource_pointer_directory(),
            "resourcepacks"
        );
        assert_eq!(AssetKind::Other.resource_pointer_directory(), "resources");
    }
}
