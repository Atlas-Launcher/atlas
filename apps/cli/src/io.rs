use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

pub fn read_bytes(path: &PathBuf) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("Failed to read {}", path.display()))
}

pub fn write_output(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }
    fs::write(path, bytes).context("Failed to write output file")
}

pub fn insert_file(files: &mut BTreeMap<String, Vec<u8>>, root: &Path, name: &str) -> Result<()> {
    let path = root.join(name);
    if !path.exists() {
        return Ok(());
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    files.insert(name.to_string(), bytes);
    Ok(())
}

pub fn insert_repo_text_files(files: &mut BTreeMap<String, Vec<u8>>, root: &Path) -> Result<()> {
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .context("Failed to compute repo relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_excluded_path(&rel_str) {
            continue;
        }

        let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        if std::str::from_utf8(&bytes).is_ok() {
            files.insert(rel_str, bytes);
        }
    }
    Ok(())
}

pub fn write_mod_entry(root: &Path, entry: &protocol::config::mods::ModEntry) -> Result<()> {
    write_pointer_entry(root, entry, "mods", ".mod.toml")
}

pub fn write_resource_entry(root: &Path, entry: &protocol::config::mods::ModEntry) -> Result<()> {
    write_pointer_entry(root, entry, "resources", ".res.toml")
}

fn write_pointer_entry(
    root: &Path,
    entry: &protocol::config::mods::ModEntry,
    directory: &str,
    extension: &str,
) -> Result<()> {
    let out_dir = root.join(directory);
    fs::create_dir_all(&out_dir).context("Failed to create pointer directory")?;
    let name_slug = slugify_mod_name(entry.metadata.name.as_str());
    let version_slug = short_version_slug(entry);
    let base = if name_slug.is_empty() {
        version_slug
    } else {
        format!("{}-{}", name_slug, version_slug)
    };
    let file_path = unique_pointer_entry_path(&out_dir, &base, extension);
    let content = entry
        .to_toml_string()
        .context("Failed to serialize mod entry")?;
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write {}", file_path.display()))?;
    Ok(())
}

fn unique_pointer_entry_path(directory: &Path, base: &str, extension: &str) -> PathBuf {
    let normalized = if base.trim().is_empty() {
        "mod"
    } else {
        base.trim()
    };
    let candidate = directory.join(format!("{}{}", normalized, extension));
    if !candidate.exists() {
        return candidate;
    }

    let mut index = 2usize;
    loop {
        let next = directory.join(format!("{}-{}{}", normalized, index, extension));
        if !next.exists() {
            return next;
        }
        index += 1;
    }
}

fn short_version_slug(entry: &protocol::config::mods::ModEntry) -> String {
    let from_version = entry
        .download
        .version
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .take(3)
        .collect::<String>();
    if !from_version.is_empty() {
        return from_version;
    }

    let from_project = entry
        .download
        .project_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .take(3)
        .collect::<String>();
    if !from_project.is_empty() {
        return from_project;
    }

    "mod".to_string()
}

fn slugify_mod_name(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            slug.push(normalized);
            last_dash = false;
            continue;
        }

        if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn is_excluded_path(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.starts_with(".git/")
        || lower.starts_with("target/")
        || lower.starts_with("node_modules/")
        || lower.starts_with(".next/")
        || lower.starts_with("dist/")
}
