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

pub fn insert_config_dir(files: &mut BTreeMap<String, Vec<u8>>, root: &Path) -> Result<()> {
    let config_dir = root.join("config");
    if !config_dir.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(&config_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .context("Failed to compute config relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        files.insert(rel_str, bytes);
    }

    Ok(())
}

pub fn write_mod_entry(root: &Path, entry: &crate::mods::ModEntry) -> Result<()> {
    let mods_dir = root.join("mods");
    fs::create_dir_all(&mods_dir).context("Failed to create mods directory")?;
    let file_name = format!("{}.mod.toml", entry.project_id);
    let file_path = mods_dir.join(file_name);
    let content = entry
        .to_toml_string()
        .context("Failed to serialize mod entry")?;
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write {}", file_path.display()))?;
    Ok(())
}
