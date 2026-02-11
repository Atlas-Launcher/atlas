#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerKind {
    Mod,
    Resource,
}

impl PointerKind {
    fn suffix(self) -> &'static str {
        match self {
            Self::Mod => ".mod.toml",
            Self::Resource => ".res.toml",
        }
    }

    fn default_extension(self) -> &'static str {
        match self {
            Self::Mod => ".jar",
            Self::Resource => ".zip",
        }
    }
}

pub fn is_pointer_path(path: &str) -> Option<PointerKind> {
    if path.ends_with(".mod.toml") {
        Some(PointerKind::Mod)
    } else if path.ends_with(".res.toml") {
        Some(PointerKind::Resource)
    } else {
        None
    }
}

pub fn resolve_pointer_path(pointer_path: &str, kind: PointerKind, url: &str) -> String {
    let trimmed = pointer_path.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    let base = url_filename_stem(url).unwrap_or_else(|| "asset".to_string());
    let slug = slugify_filename(&base);

    match kind {
        PointerKind::Mod => format!("mods/{}.mod.toml", slug),
        PointerKind::Resource => format!("resources/{}.res.toml", slug),
    }
}

pub fn destination_relative_path(pointer_path: &str, kind: PointerKind, url: &str) -> String {
    let stripped = pointer_path
        .strip_suffix(kind.suffix())
        .unwrap_or(pointer_path)
        .to_string();

    if stripped.trim().is_empty() {
        return format!(
            "{}{}",
            match kind {
                PointerKind::Mod => "mods/asset",
                PointerKind::Resource => "resources/asset",
            },
            kind.default_extension()
        );
    }

    if std::path::Path::new(&stripped).extension().is_some() {
        return stripped;
    }

    let extension = extension_from_url(url).unwrap_or_else(|| kind.default_extension().to_string());
    format!("{}{}", stripped, extension)
}

fn extension_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let last = parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or_default();
    if last.is_empty() {
        return None;
    }

    let ext = std::path::Path::new(last)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 10 || !ext.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    Some(format!(".{}", ext))
}

fn url_filename_stem(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let last = parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or_default();
    if last.is_empty() {
        return None;
    }

    let stem = std::path::Path::new(last)
        .file_stem()?
        .to_str()?
        .to_string();
    if stem.trim().is_empty() {
        None
    } else {
        Some(stem)
    }
}

fn slugify_filename(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            out.push(normalized);
            last_dash = false;
            continue;
        }
        if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
