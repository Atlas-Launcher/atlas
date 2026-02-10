use std::path::{Path, PathBuf};

use crate::errors::ProvisionError;

// Adjust to your prost module layout.
use protocol::{Dependency, DependencyKind, DependencySide, PackBlob};

use super::pointers::{destination_relative_path, is_pointer_path};

pub struct InlineFileOp {
    pub rel_path: PathBuf,
    pub bytes: Vec<u8>,
}

pub struct DepOp {
    pub dep: Dependency,
    pub dest_rel_path: PathBuf,
}

pub struct ApplyPlan {
    pub inline_files: Vec<InlineFileOp>,
    pub deps: Vec<DepOp>,
}

/// Build an install plan for a *server*:
/// - Platform filters are ignored.
/// - Target path for a dependency is derived from its pointer path.
/// - Client-only dependencies are skipped.
pub fn build_apply_plan(pack: &PackBlob) -> Result<ApplyPlan, ProvisionError> {
    let mut deps = Vec::new();
    for dep in &pack.manifest.dependencies {
        if dep.side == DependencySide::Client {
            continue;
        }

        let pointer_path = dependency_pointer_path(dep);
        let kind = dependency_kind_to_pointer_kind(dep.kind);
        let dest = destination_relative_path(&pointer_path, kind, &dep.url)?;

        deps.push(DepOp {
            dep: dep.clone(),
            dest_rel_path: dest,
        });
    }

    let mut inline_files = Vec::new();
    for (rel_path_str, bytes) in &pack.files {
        let rel_path = sanitize_rel_path(rel_path_str)?;

        if is_pointer_path(&rel_path).is_some() {
            continue;
        }

        inline_files.push(InlineFileOp {
            rel_path,
            bytes: bytes.clone(),
        });
    }

    Ok(ApplyPlan { inline_files, deps })
}

fn sanitize_rel_path(rel: &str) -> Result<PathBuf, ProvisionError> {
    let p = Path::new(rel);

    if p.is_absolute() {
        return Err(ProvisionError::Invalid(format!(
            "absolute path in files map: {rel}"
        )));
    }

    // Prevent traversal
    if rel.split('/').any(|seg| seg == "..") {
        return Err(ProvisionError::Invalid(format!(
            "path traversal in files map: {rel}"
        )));
    }

    Ok(p.to_path_buf())
}

pub(crate) async fn write_dependency_bytes(op: &DepOp, bytes: &[u8], staging_current: &PathBuf) -> Result<(), ProvisionError> {
    // Destination path inside the staging current dir
    let dest = staging_current.join(&op.dest_rel_path);

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(dest, bytes).await?;

    Ok(())
}

pub(crate) async fn write_inline_files(pack: &PackBlob, staging_current: &Path) -> Result<(), ProvisionError> {
    for (rel_path_str, bytes) in &pack.files {
        let rel_path = sanitize_rel_path(&*rel_path_str)?;

        // Skip pointer files; they are handled separately
        if is_pointer_path(&rel_path).is_some() {
            continue;
        }

        let dest = staging_current.join(&rel_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(dest, bytes).await?;
    }

    Ok(())
}

fn dependency_pointer_path(dep: &Dependency) -> PathBuf {
    let trimmed = dep.pointer_path.trim();
    if !trimmed.is_empty() {
        return PathBuf::from(trimmed);
    }
    let kind = dependency_kind_to_pointer_kind(dep.kind);
    let resolved = mod_resolver::pointer::resolve_pointer_path("", kind, &dep.url);
    PathBuf::from(resolved)
}

fn dependency_kind_to_pointer_kind(kind: DependencyKind) -> super::pointers::PointerKind {
    match kind {
        DependencyKind::Mod => super::pointers::PointerKind::Mod,
        DependencyKind::Resource => super::pointers::PointerKind::Resource,
    }
}