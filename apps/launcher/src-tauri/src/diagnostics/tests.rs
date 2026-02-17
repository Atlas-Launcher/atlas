use super::*;
use crate::models::settings::{AtlasPackLink, InstanceConfig};
use crate::models::{
    AppSettings, AtlasProfile, AtlasSession, AuthSession, InstanceSource, ModLoaderConfig,
    ModLoaderKind, Profile, TroubleshooterFinding,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_settings() -> AppSettings {
    AppSettings {
        ms_client_id: None,
        atlas_hub_url: None,
        default_java_memory_mb: 4096,
        default_jvm_args: None,
        instances: vec![],
        selected_instance_id: None,
        theme_mode: Some("system".to_string()),
        launch_readiness_wizard: Default::default(),
        pending_intent: None,
        first_launch_completed_at: None,
        first_launch_notice_dismissed_at: None,
        default_memory_profile_v1_applied: false,
    }
}

fn atlas_session_with_uuid(uuid: &str) -> AtlasSession {
    AtlasSession {
        access_token: "atlas-token".to_string(),
        profile: AtlasProfile {
            id: "atlas-user".to_string(),
            email: None,
            name: None,
            mojang_username: Some("PlayerOne".to_string()),
            mojang_uuid: Some(uuid.to_string()),
        },
        refresh_token: Some("refresh".to_string()),
        access_token_expires_at: 9_999_999_999,
        client_id: "atlas-client".to_string(),
        auth_base_url: "https://atlas.example.com/api/auth".to_string(),
    }
}

fn auth_session_with_uuid(uuid: &str) -> AuthSession {
    AuthSession {
        access_token: "mc-token".to_string(),
        profile: Profile {
            id: uuid.to_string(),
            name: "PlayerOne".to_string(),
        },
        refresh_token: Some("refresh".to_string()),
        access_token_expires_at: 9_999_999_999,
        client_id: "ms-client".to_string(),
    }
}

fn finding_exists(findings: &[TroubleshooterFinding], code: &str) -> bool {
    findings.iter().any(|finding| finding.code == code)
}

#[test]
fn readiness_marks_missing_auth_and_files() {
    let settings = sample_settings();
    let report = build_launch_readiness(ReadinessContext {
        settings,
        atlas_session: None,
        auth_session: None,
        game_dir: Some("/tmp/atlas-missing".to_string()),
    });

    assert!(!report.ready_to_launch);
    assert!(!report.atlas_logged_in);
    assert!(!report.microsoft_logged_in);
    assert!(!report.accounts_linked);
    assert!(!report.files_installed);
    assert_eq!(report.checklist.len(), 5);
    let java_item = report
        .checklist
        .iter()
        .find(|item| item.key == "javaReady")
        .expect("java readiness checklist entry");
    assert_eq!(java_item.ready, report.java_ready);
    assert!(java_item.ready);
}

#[test]
fn readiness_detects_linked_accounts_even_with_hyphenated_uuid() {
    let settings = sample_settings();
    let atlas = atlas_session_with_uuid("00112233445566778899aabbccddeeff");
    let auth = auth_session_with_uuid("00112233-4455-6677-8899-aabbccddeeff");
    let report = build_launch_readiness(ReadinessContext {
        settings,
        atlas_session: Some(atlas),
        auth_session: Some(auth),
        game_dir: Some("/tmp/atlas-missing".to_string()),
    });

    assert!(report.atlas_logged_in);
    assert!(report.microsoft_logged_in);
    assert!(report.accounts_linked);
    assert!(!report.ready_to_launch);
}

#[test]
fn troubleshooter_classifies_memory_and_runtime_metadata_signals() {
    let readiness = LaunchReadinessReport {
        atlas_logged_in: true,
        microsoft_logged_in: true,
        accounts_linked: true,
        files_installed: true,
        java_ready: true,
        ready_to_launch: true,
        checklist: vec![],
    };
    let report = run_troubleshooter(TroubleshooterInput {
        readiness,
        recent_status: Some("Launch failed: Out of memory".to_string()),
        recent_logs: vec![
            "java heap space".to_string(),
            "Atlas metadata is missing Minecraft version. Try update again.".to_string(),
        ],
    });

    assert!(finding_exists(&report.findings, "memory_pressure"));
    assert!(finding_exists(&report.findings, "runtime_metadata_missing"));
    assert!(finding_exists(
        &report.findings,
        "install_corruption_or_stale"
    ));
}

#[test]
fn troubleshooter_does_not_flag_install_corruption_when_files_not_installed() {
    let readiness = LaunchReadinessReport {
        atlas_logged_in: true,
        microsoft_logged_in: true,
        accounts_linked: true,
        files_installed: false,
        java_ready: true,
        ready_to_launch: false,
        checklist: vec![],
    };
    let report = run_troubleshooter(TroubleshooterInput {
        readiness,
        recent_status: Some("Launch failed".to_string()),
        recent_logs: vec![],
    });

    assert!(finding_exists(&report.findings, "files_missing"));
    assert!(!finding_exists(
        &report.findings,
        "install_corruption_or_stale"
    ));
}

#[test]
fn infer_pack_id_returns_only_for_matching_atlas_instance() {
    let mut settings = sample_settings();
    settings.instances = vec![
        InstanceConfig {
            id: "atlas-1".to_string(),
            name: "Remote".to_string(),
            game_dir: "/tmp/game/instances/atlas-1".to_string(),
            version: None,
            loader: ModLoaderConfig {
                kind: ModLoaderKind::Vanilla,
                loader_version: None,
            },
            java_path: String::new(),
            memory_mb: None,
            jvm_args: None,
            source: InstanceSource::Atlas,
            atlas_pack: Some(AtlasPackLink {
                pack_id: "pack-1".to_string(),
                pack_slug: "remote".to_string(),
                channel: "production".to_string(),
                build_id: None,
                build_version: None,
                artifact_key: None,
            }),
        },
        InstanceConfig {
            id: "local-1".to_string(),
            name: "Local".to_string(),
            game_dir: "/tmp/game/instances/local-1".to_string(),
            version: None,
            loader: ModLoaderConfig {
                kind: ModLoaderKind::Vanilla,
                loader_version: None,
            },
            java_path: String::new(),
            memory_mb: None,
            jvm_args: None,
            source: InstanceSource::Local,
            atlas_pack: None,
        },
    ];

    let atlas_pack_id = infer_pack_id_for_game_dir(&settings, Some("/tmp/game/instances/atlas-1"));
    let local_pack_id = infer_pack_id_for_game_dir(&settings, Some("/tmp/game/instances/local-1"));

    assert_eq!(atlas_pack_id.as_deref(), Some("pack-1"));
    assert!(local_pack_id.is_none());
}

#[test]
fn repair_plan_prefers_local_strategy_for_local_instance_even_with_pack_id() {
    let mut settings = sample_settings();
    settings.instances = vec![InstanceConfig {
        id: "local-1".to_string(),
        name: "Local".to_string(),
        game_dir: "/tmp/game/instances/local-1".to_string(),
        version: None,
        loader: ModLoaderConfig {
            kind: ModLoaderKind::Vanilla,
            loader_version: None,
        },
        java_path: String::new(),
        memory_mb: None,
        jvm_args: None,
        source: InstanceSource::Local,
        atlas_pack: Some(AtlasPackLink {
            pack_id: "pack-should-not-be-used".to_string(),
            pack_slug: "local".to_string(),
            channel: "production".to_string(),
            build_id: None,
            build_version: None,
            artifact_key: None,
        }),
    }];

    let plan = resolve_repair_plan(&RepairInput {
        settings,
        atlas_session: Some(atlas_session_with_uuid("00112233445566778899aabbccddeeff")),
        game_dir: "/tmp/game/instances/local-1".to_string(),
        pack_id: Some("pack-explicit".to_string()),
        channel: Some("staging".to_string()),
        preserve_saves: true,
    });

    assert!(matches!(plan.strategy, RepairStrategy::LocalRuntime));
}

#[test]
fn repair_plan_uses_atlas_strategy_for_atlas_instance() {
    let mut settings = sample_settings();
    settings.instances = vec![InstanceConfig {
        id: "atlas-1".to_string(),
        name: "Atlas".to_string(),
        game_dir: "/tmp/game/instances/atlas-1".to_string(),
        version: None,
        loader: ModLoaderConfig {
            kind: ModLoaderKind::Vanilla,
            loader_version: None,
        },
        java_path: String::new(),
        memory_mb: None,
        jvm_args: None,
        source: InstanceSource::Atlas,
        atlas_pack: Some(AtlasPackLink {
            pack_id: "pack-1".to_string(),
            pack_slug: "atlas".to_string(),
            channel: "production".to_string(),
            build_id: None,
            build_version: None,
            artifact_key: None,
        }),
    }];

    let plan = resolve_repair_plan(&RepairInput {
        settings,
        atlas_session: Some(atlas_session_with_uuid("00112233445566778899aabbccddeeff")),
        game_dir: "/tmp/game/instances/atlas-1".to_string(),
        pack_id: None,
        channel: None,
        preserve_saves: true,
    });

    assert!(matches!(plan.strategy, RepairStrategy::AtlasSync { .. }));
}

#[test]
fn redaction_masks_token_values_in_line_or_json_forms() {
    let log_text =
        "Authorization: Bearer abc.def.ghi access_token=secret-123 refresh_token: refresh-456";
    let redacted = redact_sensitive(log_text);
    assert!(!redacted.contains("abc.def.ghi"));
    assert!(!redacted.contains("secret-123"));
    assert!(!redacted.contains("refresh-456"));
    assert!(redacted.contains("[REDACTED]"));

    let json_text =
        r#"{"access_token":"abc","nested":{"authorization":"Bearer xyz","proof":"p123"}}"#;
    let redacted_json = redact_sensitive(json_text);
    assert!(!redacted_json.contains("\"abc\""));
    assert!(!redacted_json.contains("xyz"));
    assert!(!redacted_json.contains("p123"));
    assert!(redacted_json.contains("[REDACTED]"));
}

#[test]
fn readiness_java_check_rejects_directory_override() {
    let dir = unique_temp_path("java-dir");
    fs::create_dir_all(&dir).expect("create temp dir");

    let mut settings = sample_settings();
    settings.instances = vec![InstanceConfig {
        id: "local-1".to_string(),
        name: "Local".to_string(),
        game_dir: dir.to_string_lossy().to_string(),
        version: None,
        loader: ModLoaderConfig {
            kind: ModLoaderKind::Vanilla,
            loader_version: None,
        },
        java_path: dir.to_string_lossy().to_string(),
        memory_mb: None,
        jvm_args: None,
        source: InstanceSource::Local,
        atlas_pack: None,
    }];

    assert!(!resolve_java_ready(&settings, Some(&dir.to_string_lossy())));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn readiness_java_check_accepts_runtime_binary_layout() {
    let game_dir = unique_temp_path("java-runtime");
    let runtime_bin = game_dir.join("runtimes").join("custom").join("bin");
    fs::create_dir_all(&runtime_bin).expect("create runtime bin");
    let bin = runtime_bin.join(java_file_name());
    fs::write(&bin, b"#!/bin/sh\nexit 0\n").expect("write temp java binary");
    make_executable(&bin);

    let settings = sample_settings();
    assert!(resolve_java_ready(
        &settings,
        Some(&game_dir.to_string_lossy())
    ));
    let _ = fs::remove_dir_all(&game_dir);
}

fn unique_temp_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("atlas-diagnostics-{prefix}-{nanos}"))
}

#[cfg(target_os = "windows")]
fn java_file_name() -> &'static str {
    "java.exe"
}

#[cfg(not(target_os = "windows"))]
fn java_file_name() -> &'static str {
    "java"
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .expect("stat temp java binary")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod temp java binary");
}

#[cfg(not(unix))]
fn make_executable(_: &std::path::Path) {}
