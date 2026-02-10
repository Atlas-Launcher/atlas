use std::path::{Path, PathBuf};

use crate::errors::ProvisionError;
use mod_resolver::pointer as resolver_pointer;

pub use resolver_pointer::PointerKind;


pub fn is_pointer_path(rel_path: &Path) -> Option<PointerKind> {
    let s = rel_path.to_string_lossy();
    resolver_pointer::is_pointer_path(&s)
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
    Ok(PathBuf::from(resolver_pointer::destination_relative_path(
        s, kind, url,
    )))
}

