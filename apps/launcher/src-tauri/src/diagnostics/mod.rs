use crate::models::{
    AppSettings, AtlasSession, AuthSession, FixAction, FixResult, InstanceSource, LaunchOptions,
    LaunchReadinessReport, ModLoaderConfig, ReadinessItem, RepairResult, SupportBundleResult,
    TroubleshooterFinding, TroubleshooterReport,
};
use crate::paths::{auth_store_dir, normalize_path};
use crate::{launcher, library};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReadinessContext {
    pub settings: AppSettings,
    pub atlas_session: Option<AtlasSession>,
    pub auth_session: Option<AuthSession>,
    pub game_dir: Option<String>,
}

pub struct TroubleshooterInput {
    pub readiness: LaunchReadinessReport,
    pub recent_status: Option<String>,
    pub recent_logs: Vec<String>,
}

pub struct ApplyFixInput {
    pub action: FixAction,
    pub settings: AppSettings,
    pub atlas_session: Option<AtlasSession>,
    pub game_dir: Option<String>,
    pub pack_id: Option<String>,
    pub channel: Option<String>,
}

pub struct RepairInput {
    pub settings: AppSettings,
    pub atlas_session: Option<AtlasSession>,
    pub game_dir: String,
    pub pack_id: Option<String>,
    pub channel: Option<String>,
    pub preserve_saves: bool,
}

pub struct SupportBundleInput {
    pub readiness: LaunchReadinessReport,
    pub game_dir: Option<String>,
    pub recent_status: Option<String>,
    pub recent_logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepairStrategy {
    AtlasSync {
        pack_id: String,
        channel: Option<String>,
    },
    LocalRuntime,
}

#[derive(Debug, Clone)]
struct RepairPlan {
    strategy: RepairStrategy,
    decision_reason: String,
}

pub fn build_launch_readiness(input: ReadinessContext) -> LaunchReadinessReport {
    let atlas_logged_in = input.atlas_session.is_some();
    let microsoft_logged_in = input.auth_session.is_some();
    let accounts_linked = resolve_accounts_linked(
        input
            .atlas_session
            .as_ref()
            .and_then(|session| session.profile.mojang_uuid.as_deref()),
        input
            .auth_session
            .as_ref()
            .map(|session| session.profile.id.as_str()),
    );
    let files_installed = input
        .game_dir
        .as_deref()
        .map(resolve_files_installed)
        .unwrap_or(false);
    let java_ready = resolve_java_ready(&input.settings, input.game_dir.as_deref());
    let ready_to_launch =
        atlas_logged_in && microsoft_logged_in && accounts_linked && files_installed && java_ready;
    let checklist = vec![
        ReadinessItem {
            key: "atlasLogin".to_string(),
            label: "Atlas login".to_string(),
            ready: atlas_logged_in,
            detail: (!atlas_logged_in).then_some("Sign in to Atlas Hub.".to_string()),
        },
        ReadinessItem {
            key: "microsoftLogin".to_string(),
            label: "Microsoft login".to_string(),
            ready: microsoft_logged_in,
            detail: (!microsoft_logged_in).then_some("Sign in with Microsoft.".to_string()),
        },
        ReadinessItem {
            key: "accountLink".to_string(),
            label: "Account link".to_string(),
            ready: accounts_linked,
            detail: (!accounts_linked).then_some(
                "Atlas and Microsoft Minecraft accounts are not linked to the same UUID."
                    .to_string(),
            ),
        },
        ReadinessItem {
            key: "filesInstalled".to_string(),
            label: "Files installed".to_string(),
            ready: files_installed,
            detail: (!files_installed).then_some("Install or sync profile files.".to_string()),
        },
        ReadinessItem {
            key: "javaReady".to_string(),
            label: "Java ready".to_string(),
            ready: java_ready,
            detail: (!java_ready)
                .then_some("Java runtime has not been detected for this profile.".to_string()),
        },
    ];

    LaunchReadinessReport {
        atlas_logged_in,
        microsoft_logged_in,
        accounts_linked,
        files_installed,
        java_ready,
        ready_to_launch,
        checklist,
    }
}

pub fn run_troubleshooter(input: TroubleshooterInput) -> TroubleshooterReport {
    let mut findings = Vec::<TroubleshooterFinding>::new();
    let status = input.recent_status.unwrap_or_default().to_ascii_lowercase();
    let joined_logs = input.recent_logs.join("\n").to_ascii_lowercase();
    let haystack = format!("{status}\n{joined_logs}");

    if !input.readiness.atlas_logged_in {
        findings.push(TroubleshooterFinding {
            code: "atlas_not_signed_in".to_string(),
            title: "Atlas sign-in required".to_string(),
            detail: "Atlas Hub session is missing or expired.".to_string(),
            confidence: 100,
            suggested_actions: vec![FixAction::RelinkAccount],
        });
    }
    if !input.readiness.microsoft_logged_in {
        findings.push(TroubleshooterFinding {
            code: "microsoft_not_signed_in".to_string(),
            title: "Microsoft sign-in required".to_string(),
            detail: "Minecraft account session is missing or expired.".to_string(),
            confidence: 100,
            suggested_actions: vec![FixAction::RelinkAccount],
        });
    }
    if input.readiness.atlas_logged_in
        && input.readiness.microsoft_logged_in
        && !input.readiness.accounts_linked
    {
        findings.push(TroubleshooterFinding {
            code: "account_link_mismatch".to_string(),
            title: "Account link mismatch".to_string(),
            detail: "Atlas linked Mojang UUID does not match the active Microsoft profile."
                .to_string(),
            confidence: 100,
            suggested_actions: vec![FixAction::RelinkAccount],
        });
    }
    if !input.readiness.files_installed {
        findings.push(TroubleshooterFinding {
            code: "files_missing".to_string(),
            title: "Profile files are missing".to_string(),
            detail: "The selected profile does not appear to have installed game files."
                .to_string(),
            confidence: 95,
            suggested_actions: vec![FixAction::ResyncPack, FixAction::FullRepair],
        });
    }
    if !input.readiness.java_ready {
        findings.push(TroubleshooterFinding {
            code: "java_missing".to_string(),
            title: "Java runtime not ready".to_string(),
            detail: "A usable Java runtime could not be detected.".to_string(),
            confidence: 90,
            suggested_actions: vec![FixAction::RepairRuntime, FixAction::FullRepair],
        });
    }

    if haystack.contains("out of memory") || haystack.contains("java heap space") {
        findings.push(TroubleshooterFinding {
            code: "memory_pressure".to_string(),
            title: "Memory settings may be too low".to_string(),
            detail: "Recent logs suggest JVM memory pressure.".to_string(),
            confidence: 85,
            suggested_actions: vec![FixAction::SetSafeMemory],
        });
    }
    if haystack.contains("missing minecraft version")
        || haystack.contains("missing neoforge loader version")
    {
        findings.push(TroubleshooterFinding {
            code: "runtime_metadata_missing".to_string(),
            title: "Pack runtime metadata missing".to_string(),
            detail: "Atlas sync did not return complete runtime metadata.".to_string(),
            confidence: 90,
            suggested_actions: vec![FixAction::ResyncPack, FixAction::FullRepair],
        });
    }
    if haystack.contains("client jar is missing")
        || haystack.contains("launch failed")
        || haystack.contains("pack update failed")
    {
        findings.push(TroubleshooterFinding {
            code: "install_corruption_or_stale".to_string(),
            title: "Installation may be stale or corrupted".to_string(),
            detail: "Recent status/logs indicate incomplete or corrupted install assets."
                .to_string(),
            confidence: 75,
            suggested_actions: vec![FixAction::FullRepair, FixAction::ResyncPack],
        });
    }

    TroubleshooterReport {
        readiness: input.readiness,
        findings,
    }
}

pub fn collect_troubleshooter_logs(
    game_dir: Option<&str>,
    recent_logs: Option<Vec<String>>,
) -> Vec<String> {
    const MAX_LINES: usize = 160;
    let mut merged = recent_logs.unwrap_or_default();

    if let Some(dir) = game_dir {
        let latest_launch_log = normalize_path(dir).join("latest_launch.log");
        if let Some(text) = read_text_if_exists(&latest_launch_log) {
            let mut tail = text
                .lines()
                .rev()
                .take(MAX_LINES)
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            tail.reverse();
            merged.extend(tail);
        }
    }

    if merged.len() > MAX_LINES {
        merged = merged.split_off(merged.len() - MAX_LINES);
    }

    merged
}

pub async fn apply_fix(window: &tauri::Window, input: ApplyFixInput) -> Result<FixResult, String> {
    let action = input.action.clone();
    let output = match input.action {
        FixAction::RelinkAccount => FixResult {
            action,
            applied: false,
            message: "Relink account is a user-auth flow. Prompt sign-in/link UI.".to_string(),
        },
        FixAction::SetSafeMemory => {
            let mut next = input.settings.clone();
            let current = next.default_java_memory_mb;
            if current < 4096 {
                next.default_java_memory_mb = 4096;
            }
            FixResult {
                action,
                applied: next.default_java_memory_mb != current,
                message: format!("Default memory set to {} MB.", next.default_java_memory_mb),
            }
        }
        FixAction::ResyncPack => {
            let game_dir = input
                .game_dir
                .as_deref()
                .ok_or_else(|| "gameDir is required for resync.".to_string())?;
            let pack_id = input
                .pack_id
                .as_deref()
                .ok_or_else(|| "packId is required for resync.".to_string())?;
            let atlas_session = input
                .atlas_session
                .as_ref()
                .ok_or_else(|| "Atlas session required for resync.".to_string())?;
            let hub_url = crate::config::resolve_atlas_hub_url(&input.settings);
            let result = library::sync_atlas_pack(
                window,
                &hub_url,
                &atlas_session.access_token,
                pack_id,
                input.channel.as_deref(),
                game_dir,
            )
            .await
            .map_err(|err| err.to_string())?;
            FixResult {
                action,
                applied: true,
                message: format!(
                    "Resynced Atlas pack {} ({} files, {} assets).",
                    result.pack_id, result.bundled_files, result.hydrated_assets
                ),
            }
        }
        FixAction::RepairRuntime => {
            let options =
                build_launch_options_for_game_dir(&input.settings, input.game_dir.as_deref())
                    .ok_or_else(|| {
                        "Unable to build launch options for runtime repair.".to_string()
                    })?;
            launcher::download_minecraft_files(window, &options)
                .await
                .map_err(|err| err.to_string())?;
            FixResult {
                action,
                applied: true,
                message: "Runtime repair completed.".to_string(),
            }
        }
        FixAction::FullRepair => {
            let game_dir = input
                .game_dir
                .as_deref()
                .ok_or_else(|| "gameDir is required for full repair.".to_string())?;
            library::uninstall_instance_data(game_dir, true).map_err(|err| err.to_string())?;
            if let (Some(pack_id), Some(atlas_session)) =
                (input.pack_id.as_deref(), input.atlas_session.as_ref())
            {
                let hub_url = crate::config::resolve_atlas_hub_url(&input.settings);
                let _ = library::sync_atlas_pack(
                    window,
                    &hub_url,
                    &atlas_session.access_token,
                    pack_id,
                    input.channel.as_deref(),
                    game_dir,
                )
                .await
                .map_err(|err| err.to_string())?;
            } else {
                let options = build_launch_options_for_game_dir(&input.settings, Some(game_dir))
                    .ok_or_else(|| "Unable to build launch options for full repair.".to_string())?;
                launcher::download_minecraft_files(window, &options)
                    .await
                    .map_err(|err| err.to_string())?;
            }
            FixResult {
                action,
                applied: true,
                message: "Full repair completed while preserving saves.".to_string(),
            }
        }
    };
    Ok(output)
}

pub async fn repair_installation(
    window: &tauri::Window,
    input: RepairInput,
) -> Result<RepairResult, String> {
    let mut details = Vec::<String>::new();
    let game_dir = input.game_dir.trim();
    if game_dir.is_empty() {
        return Err("gameDir is required.".to_string());
    }

    let plan = resolve_repair_plan(&input);
    details.push(plan.decision_reason.clone());

    let cleanup = cleanup_transient_paths(game_dir).map_err(|err| err.to_string())?;
    if !cleanup.removed.is_empty() {
        details.push(format!(
            "Removed transient paths: {}.",
            cleanup.removed.join(", ")
        ));
    }
    if !cleanup.skipped.is_empty() {
        details.push(format!(
            "Skipped transient cleanup due to errors: {}.",
            cleanup.skipped.join(", ")
        ));
    }

    library::uninstall_instance_data(game_dir, input.preserve_saves).map_err(|err| err.to_string())?;
    details.push(format!(
        "Uninstalled instance data (preserve saves: {}).",
        input.preserve_saves
    ));

    if let RepairStrategy::AtlasSync { pack_id, channel } = plan.strategy {
        let atlas_session = input
            .atlas_session
            .as_ref()
            .ok_or_else(|| "Atlas session required for Atlas repair.".to_string())?;
        let hub_url = crate::config::resolve_atlas_hub_url(&input.settings);
        let result = library::sync_atlas_pack(
            window,
            &hub_url,
            &atlas_session.access_token,
            &pack_id,
            channel.as_deref(),
            game_dir,
        )
        .await
        .map_err(|err| err.to_string())?;
        details.push(format!(
            "Resynced Atlas pack {} ({} files, {} assets).",
            result.pack_id, result.bundled_files, result.hydrated_assets
        ));
    } else {
        let options = build_launch_options_for_game_dir(&input.settings, Some(game_dir))
            .ok_or_else(|| "Unable to build launch options for repair.".to_string())?;
        launcher::download_minecraft_files(window, &options)
            .await
            .map_err(|err| err.to_string())?;
        details.push("Downloaded base Minecraft files/runtime.".to_string());
    }

    Ok(RepairResult {
        repaired: true,
        message: "Repair completed successfully.".to_string(),
        details,
    })
}

pub fn create_support_bundle(input: SupportBundleInput) -> Result<SupportBundleResult, String> {
    let base = auth_store_dir()?.join("support");
    fs::create_dir_all(&base).map_err(|err| format!("Failed to create support dir: {err}"))?;
    let stamp = unix_timestamp();
    let bundle_dir = base.join(format!("bundle-{stamp}"));
    fs::create_dir_all(&bundle_dir).map_err(|err| format!("Failed to create bundle dir: {err}"))?;

    let launcher_log = auth_store_dir()?.join("launcher.log");
    let launcher_log_text = read_text_if_exists(&launcher_log).unwrap_or_default();

    let latest_launch_log = input
        .game_dir
        .as_deref()
        .map(|dir| normalize_path(dir).join("latest_launch.log"))
        .unwrap_or_else(|| PathBuf::from(""));
    let latest_launch_log_text = if latest_launch_log.as_os_str().is_empty() {
        String::new()
    } else {
        read_text_if_exists(&latest_launch_log).unwrap_or_default()
    };

    let redacted_launcher_log = redact_sensitive(&launcher_log_text);
    let redacted_latest_launch_log = redact_sensitive(&latest_launch_log_text);

    let diagnostics = run_troubleshooter(TroubleshooterInput {
        readiness: input.readiness.clone(),
        recent_status: input.recent_status.clone(),
        recent_logs: input.recent_logs.clone(),
    });
    let root_cause = diagnostics.findings.first().map(|finding| {
        json!({
            "code": finding.code,
            "title": finding.title,
            "detail": finding.detail,
            "confidence": finding.confidence
        })
    });
    let attempted_fixes = summarize_attempted_fixes(
        input.recent_status.as_deref(),
        &input.recent_logs,
    );

    let report = json!({
        "generatedAtUnix": stamp,
        "readiness": input.readiness,
        "recentStatus": input.recent_status,
        "recentLogs": input.recent_logs,
        "analysis": {
            "rootCause": root_cause,
            "suggestedActions": diagnostics
                .findings
                .iter()
                .flat_map(|finding| finding.suggested_actions.iter())
                .collect::<Vec<_>>(),
            "attemptedFixes": attempted_fixes,
        },
        "logs": {
            "launcherLog": redacted_launcher_log,
            "latestLaunchLog": redacted_latest_launch_log,
        }
    });
    let report_json_path = bundle_dir.join("report.json");
    fs::write(
        &report_json_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|err| format!("Failed to encode report: {err}"))?,
    )
    .map_err(|err| format!("Failed to write report: {err}"))?;

    let summary = build_summary(
        report["readiness"]["readyToLaunch"]
            .as_bool()
            .unwrap_or(false),
        report["recentStatus"].as_str().unwrap_or(""),
        &report["analysis"],
    );
    let summary_path = bundle_dir.join("summary.md");
    fs::write(&summary_path, &summary).map_err(|err| format!("Failed to write summary: {err}"))?;

    Ok(SupportBundleResult {
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        report_json_path: report_json_path.to_string_lossy().to_string(),
        summary_path: summary_path.to_string_lossy().to_string(),
        summary,
    })
}

fn resolve_accounts_linked(atlas_uuid: Option<&str>, launcher_uuid: Option<&str>) -> bool {
    let atlas = normalize_uuid(atlas_uuid);
    let launcher = normalize_uuid(launcher_uuid);
    !atlas.is_empty() && atlas == launcher
}

fn normalize_uuid(value: Option<&str>) -> String {
    value
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .replace('-', "")
}

fn resolve_files_installed(game_dir: &str) -> bool {
    library::list_installed_versions(game_dir)
        .map(|versions| !versions.is_empty())
        .unwrap_or(false)
}

fn resolve_java_ready(settings: &AppSettings, game_dir: Option<&str>) -> bool {
    if let Some(configured) = find_instance_java_path(settings, game_dir) {
        let trimmed = configured.trim();
        if !trimmed.is_empty() {
            if trimmed.eq_ignore_ascii_case("java") {
                return find_java_on_path().is_some();
            }
            if is_usable_java_binary(Path::new(trimmed)) {
                return true;
            }
        }
    }

    if find_java_on_path().is_some() {
        return true;
    }

    let Some(game_dir) = game_dir else {
        return false;
    };
    find_runtime_java_binary(&normalize_path(game_dir))
        .map(|candidate| is_usable_java_binary(&candidate))
        .unwrap_or(false)
}

fn find_instance_java_path(settings: &AppSettings, game_dir: Option<&str>) -> Option<String> {
    let game_dir = game_dir.map(normalize_path);
    settings
        .instances
        .iter()
        .find(|instance| {
            if let Some(target_dir) = game_dir.as_ref() {
                normalize_path(&instance.game_dir) == *target_dir
            } else {
                false
            }
        })
        .map(|instance| instance.java_path.clone())
}

fn build_launch_options_for_game_dir(
    settings: &AppSettings,
    game_dir: Option<&str>,
) -> Option<LaunchOptions> {
    let game_dir = game_dir?;
    let normalized_game_dir = normalize_path(game_dir);
    let instance = settings
        .instances
        .iter()
        .find(|candidate| normalize_path(&candidate.game_dir) == normalized_game_dir)?;

    let memory_mb = instance
        .memory_mb
        .unwrap_or(settings.default_java_memory_mb)
        .max(1024);
    let jvm_args = instance
        .jvm_args
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(settings.default_jvm_args.as_deref())
        .unwrap_or("")
        .to_string();
    let loader = ModLoaderConfig {
        kind: instance.loader.kind.clone(),
        loader_version: instance.loader.loader_version.clone(),
    };

    Some(LaunchOptions {
        game_dir: instance.game_dir.clone(),
        java_path: instance.java_path.clone(),
        memory_mb,
        jvm_args,
        version: instance.version.clone(),
        loader,
    })
}

fn find_runtime_java_binary(game_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();
    let runtime_root = resolve_runtime_root(game_dir);
    candidates.push(runtime_root);
    candidates.push(game_dir.join("runtimes"));

    for root in candidates {
        let hits = find_files_named(&root, &["java", "java.exe", "javaw.exe"], 6);
        if let Some(path) = hits.into_iter().next() {
            return Some(path);
        }
    }
    None
}

fn find_java_on_path() -> Option<PathBuf> {
    let path_value = env::var_os("PATH")?;
    #[cfg(target_os = "windows")]
    let executable_names = ["java.exe", "javaw.exe", "java.cmd", "java.bat", "java"];
    #[cfg(not(target_os = "windows"))]
    let executable_names = ["java"];

    env::split_paths(&path_value)
        .flat_map(|dir| executable_names.iter().map(move |name| dir.join(name)))
        .find(|candidate| is_usable_java_binary(candidate))
}

fn is_usable_java_binary(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            return metadata.permissions().mode() & 0o111 != 0;
        }
        false
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn resolve_runtime_root(game_dir: &Path) -> PathBuf {
    for ancestor in game_dir.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "instances") {
            if let Some(parent) = ancestor.parent() {
                return parent.join("runtimes");
            }
        }
    }
    game_dir.join("runtimes")
}

fn find_files_named(root: &Path, names: &[&str], max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth || !dir.exists() {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push((path, depth + 1));
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());
            if let Some(name) = file_name {
                if names
                    .iter()
                    .any(|expected| name == expected.to_ascii_lowercase())
                {
                    out.push(path);
                }
            }
        }
    }

    out
}

fn read_text_if_exists(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn redact_sensitive(input: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<Value>(input) {
        redact_json_value(&mut value);
        if let Ok(text) = serde_json::to_string_pretty(&value) {
            return text;
        }
    }

    input.lines().map(redact_line).collect::<Vec<_>>().join("\n")
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if let Some(next) = map.get_mut(&key) {
                    if is_sensitive_key(&key) {
                        *next = Value::String("[REDACTED]".to_string());
                    } else {
                        redact_json_value(next);
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_json_value(item);
            }
        }
        Value::String(text) => {
            *text = redact_line(text);
        }
        _ => {}
    }
}

fn redact_line(line: &str) -> String {
    let mut redacted = line.to_string();
    redacted = redact_bearer_token(&redacted);
    redacted = redact_kv_pairs(&redacted);
    redacted
}

fn redact_bearer_token(line: &str) -> String {
    let marker = "bearer ";
    let lower = line.to_ascii_lowercase();
    let Some(start) = lower.find(marker) else {
        return line.to_string();
    };
    let value_start = start + marker.len();
    let token_end = line[value_start..]
        .find(|c: char| c.is_whitespace() || c == ',' || c == '"' || c == '\'')
        .map(|offset| value_start + offset)
        .unwrap_or(line.len());
    let mut out = String::new();
    out.push_str(&line[..value_start]);
    out.push_str("[REDACTED]");
    out.push_str(&line[token_end..]);
    out
}

fn redact_kv_pairs(line: &str) -> String {
    let mut out = line.to_string();
    for key in sensitive_keys() {
        out = redact_kv_value(&out, key);
    }
    out
}

fn redact_kv_value(line: &str, key: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let key_lower = key.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut out = String::new();

    while let Some(found) = lower[cursor..].find(&key_lower) {
        let key_start = cursor + found;
        let key_end = key_start + key_lower.len();
        out.push_str(&line[cursor..key_end]);

        let mut value_start = key_end;
        while let Some(ch) = line[value_start..].chars().next() {
            if ch.is_whitespace() {
                value_start += ch.len_utf8();
                continue;
            }
            if ch == ':' || ch == '=' {
                value_start += ch.len_utf8();
                break;
            }
            out.push_str(&line[key_end..]);
            return out;
        }

        while let Some(ch) = line[value_start..].chars().next() {
            if ch.is_whitespace() {
                value_start += ch.len_utf8();
            } else {
                break;
            }
        }

        let value_end = line[value_start..]
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '"' || c == '\'')
            .map(|offset| value_start + offset)
            .unwrap_or(line.len());
        out.push_str(&line[key_end..value_start]);
        out.push_str("[REDACTED]");
        cursor = value_end;
    }

    out.push_str(&line[cursor..]);
    out
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    sensitive_keys()
        .iter()
        .any(|candidate| lower.contains(candidate))
}

fn sensitive_keys() -> &'static [&'static str] {
    &[
        "access_token",
        "refresh_token",
        "authorization",
        "id_token",
        "service_token",
        "deploy_key",
        "proof",
        "linkcode",
        "password",
        "secret",
        "apikey",
        "api_key",
    ]
}

fn build_summary(ready_to_launch: bool, recent_status: &str, analysis: &Value) -> String {
    let readiness = if ready_to_launch {
        "Ready to launch"
    } else {
        "Not ready to launch"
    };
    let root_cause = analysis
        .get("rootCause")
        .and_then(|value| value.as_object())
        .map(|value| {
            let title = value
                .get("title")
                .and_then(|field| field.as_str())
                .unwrap_or("Unknown");
            let confidence = value
                .get("confidence")
                .and_then(|field| field.as_u64())
                .unwrap_or(0);
            format!("{title} ({confidence}% confidence)")
        })
        .unwrap_or_else(|| "No clear root cause detected".to_string());
    let attempted_fixes = analysis
        .get("attemptedFixes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let attempted_line = if attempted_fixes.is_empty() {
        "None detected in recent logs/status.".to_string()
    } else {
        attempted_fixes
            .iter()
            .map(|entry| {
                let action = entry
                    .get("action")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                let status = entry
                    .get("status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                format!("{action} ({status})")
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "# Atlas Support Summary\n\n- Launch readiness: {readiness}\n- Recent status: {recent_status}\n- Root cause: {root_cause}\n- Attempted fixes: {attempted_line}\n- Generated by diagnostics bundle."
    )
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn infer_pack_id_for_game_dir(
    settings: &AppSettings,
    game_dir: Option<&str>,
) -> Option<String> {
    let game_dir = game_dir.map(normalize_path)?;
    settings
        .instances
        .iter()
        .find(|instance| normalize_path(&instance.game_dir) == game_dir)
        .and_then(|instance| {
            if matches!(instance.source, InstanceSource::Atlas) {
                instance
                    .atlas_pack
                    .as_ref()
                    .map(|pack| pack.pack_id.clone())
            } else {
                None
            }
        })
}

fn resolve_repair_plan(input: &RepairInput) -> RepairPlan {
    let normalized_game_dir = normalize_path(&input.game_dir);
    let matching_instance = input
        .settings
        .instances
        .iter()
        .find(|instance| normalize_path(&instance.game_dir) == normalized_game_dir);

    if let Some(instance) = matching_instance {
        if matches!(instance.source, InstanceSource::Local) {
            return RepairPlan {
                strategy: RepairStrategy::LocalRuntime,
                decision_reason:
                    "Repair strategy: local runtime (instance source is local).".to_string(),
            };
        }

        let pack_id = input.pack_id.clone().or_else(|| {
            instance
                .atlas_pack
                .as_ref()
                .map(|pack| pack.pack_id.clone())
        });
        let channel = input.channel.clone().or_else(|| {
            instance
                .atlas_pack
                .as_ref()
                .map(|pack| pack.channel.clone())
        });

        if pack_id.is_some() && input.atlas_session.is_some() {
            return RepairPlan {
                strategy: RepairStrategy::AtlasSync {
                    pack_id: pack_id.unwrap_or_default(),
                    channel,
                },
                decision_reason:
                    "Repair strategy: Atlas pack sync (instance source is Atlas).".to_string(),
            };
        }

        return RepairPlan {
            strategy: RepairStrategy::LocalRuntime,
            decision_reason:
                "Repair strategy fallback: local runtime (Atlas session/pack unavailable)."
                    .to_string(),
        };
    }

    if let (Some(pack_id), Some(_)) = (input.pack_id.clone(), input.atlas_session.as_ref()) {
        return RepairPlan {
            strategy: RepairStrategy::AtlasSync {
                pack_id,
                channel: input.channel.clone(),
            },
            decision_reason: "Repair strategy: Atlas pack sync (explicit pack id provided)."
                .to_string(),
        };
    }

    RepairPlan {
        strategy: RepairStrategy::LocalRuntime,
        decision_reason: "Repair strategy: local runtime (no Atlas instance match).".to_string(),
    }
}

struct CleanupResult {
    removed: Vec<String>,
    skipped: Vec<String>,
}

fn cleanup_transient_paths(game_dir: &str) -> Result<CleanupResult, String> {
    let base_dir = normalize_path(game_dir);
    let minecraft_dir = base_dir.join(".minecraft");
    let mut result = CleanupResult {
        removed: Vec::new(),
        skipped: Vec::new(),
    };
    let candidates = vec![
        base_dir.join("latest_launch.log"),
        base_dir.join("logs"),
        base_dir.join("crash-reports"),
        base_dir.join("tmp"),
        base_dir.join("downloads"),
        minecraft_dir.join("logs"),
        minecraft_dir.join("crash-reports"),
        minecraft_dir.join("tmp"),
        minecraft_dir.join("downloads"),
    ];

    for path in candidates {
        if !path.exists() {
            continue;
        }
        let outcome = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match outcome {
            Ok(_) => result.removed.push(path.to_string_lossy().to_string()),
            Err(err) => result
                .skipped
                .push(format!("{} ({err})", path.to_string_lossy())),
        }
    }

    cleanup_transient_error_reports(&base_dir, &mut result);
    cleanup_transient_error_reports(&minecraft_dir, &mut result);
    Ok(result)
}

fn cleanup_transient_error_reports(root: &Path, result: &mut CleanupResult) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|value| value.to_str()) {
            Some(value) => value.to_ascii_lowercase(),
            None => continue,
        };
        if !(name.starts_with("hs_err_pid") || name.starts_with("replay_pid")) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(_) => result.removed.push(path.to_string_lossy().to_string()),
            Err(err) => result
                .skipped
                .push(format!("{} ({err})", path.to_string_lossy())),
        }
    }
}

fn summarize_attempted_fixes(recent_status: Option<&str>, recent_logs: &[String]) -> Vec<Value> {
    let mut haystack = recent_status.unwrap_or_default().to_ascii_lowercase();
    if !recent_logs.is_empty() {
        haystack.push('\n');
        haystack.push_str(&recent_logs.join("\n").to_ascii_lowercase());
    }

    let mut fixes = Vec::<Value>::new();
    if haystack.contains("relink")
        || haystack.contains("link")
        || haystack.contains("account")
        || haystack.contains("sign in")
    {
        fixes.push(json!({"action": "relinkAccount", "status": "detected"}));
    }
    if haystack.contains("default memory set to")
        || haystack.contains("safe memory")
        || haystack.contains("heap")
    {
        fixes.push(json!({"action": "setSafeMemory", "status": "detected"}));
    }
    if haystack.contains("resynced atlas pack") || haystack.contains("sync") {
        fixes.push(json!({"action": "resyncPack", "status": "detected"}));
    }
    if haystack.contains("runtime repair completed")
        || haystack.contains("downloaded base minecraft files")
    {
        fixes.push(json!({"action": "repairRuntime", "status": "detected"}));
    }
    if haystack.contains("full repair completed")
        || haystack.contains("repair completed successfully")
    {
        fixes.push(json!({"action": "fullRepair", "status": "detected"}));
    }
    fixes
}

#[cfg(test)]
mod tests;
