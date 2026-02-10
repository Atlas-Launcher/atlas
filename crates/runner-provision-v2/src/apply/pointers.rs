use std::path::{Path, PathBuf};

use crate::errors::ProvisionError;

/// Pointer files are identified by suffix.
#[derive(Debug, Clone, Copy)]
pub enum PointerKind {
    Mod,
    Resource,
}


/// Identify pointer files by suffix.
pub fn is_pointer_path(rel_path: &Path) -> Option<PointerKind> {
    let s = rel_path.to_string_lossy();
    if s.ends_with(".mod.toml") {
        Some(PointerKind::Mod)
    } else if s.ends_with(".res.toml") {
        Some(PointerKind::Resource)
    } else {
        None
    }
}


/// Convert pointer_path + kind + url into the actual destination relative path.
///
/// Rules (mirrors your launcher description):
/// - Strip the pointer suffix (.mod.toml or .res.toml)
/// - If the resulting path has no extension:
///   - Try to take extension from the URL filename
///   - Else default extension:
///     - Mods: `.jar`
///     - Resources: `.zip`
///
/// The output is a *relative path under the game/server directory*.
pub fn destination_relative_path(
    pointer_path: &Path,
    kind: PointerKind,
    url: &str,
) -> Result<PathBuf, ProvisionError> {
    let s = pointer_path
        .to_str()
        .ok_or_else(|| ProvisionError::Invalid("pointer path is not valid utf-8".into()))?;

    let stripped = match kind {
        PointerKind::Mod => s.strip_suffix(".mod.toml"),
        PointerKind::Resource => s.strip_suffix(".res.toml"),
    }
        .ok_or_else(|| ProvisionError::Invalid(format!("pointer path missing expected suffix: {s}")))?;

    let mut rel = PathBuf::from(stripped);

    // If it already has an extension after stripping, keep it.
    if rel.extension().is_some() {
        return Ok(rel);
    }

    // Otherwise, pick an extension.
    let ext = url_filename_extension(url).unwrap_or_else(|| match kind {
        PointerKind::Mod => "jar".to_string(),
        PointerKind::Resource => "zip".to_string(),
    });

    // Add extension by replacing the file name.
    // (If rel ends with a directory somehow, error.)
    let file_name = rel
        .file_name()
        .ok_or_else(|| ProvisionError::Invalid(format!("pointer path has no filename: {s}")))?
        .to_string_lossy()
        .to_string();

    rel.set_file_name(format!("{file_name}.{ext}"));

    Ok(rel)
}

/// Extract extension from the URL’s last path segment, if any.
/// Handles URLs with query strings.
///
/// Examples:
/// - ".../iris-neoforge-1.8.12+mc1.21.1.jar" -> Some("jar")
/// - ".../download" -> None
fn url_filename_extension(url: &str) -> Option<String> {
    // Chop query string
    let url_no_q = url.split('?').next().unwrap_or(url);

    // Take the last path segment
    let fname = url_no_q.rsplit('/').next()?;
    if fname.is_empty() {
        return None;
    }

    // Percent decoding is optional; for extension detection we can just look for last '.'
    let ext = fname.rsplit('.').next()?;
    // If there was no '.', rsplit('.') returns whole string; we need to ensure there was a dot.
    if ext == fname {
        return None;
    }

    // Basic sanity: ext should be alnum-ish and not too long
    let ext_lc = ext.to_ascii_lowercase();
    if ext_lc.len() > 8 || ext_lc.is_empty() {
        return None;
    }

    Some(ext_lc)
}

