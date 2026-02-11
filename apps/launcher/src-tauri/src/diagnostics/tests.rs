use super::*;
use crate::models::settings::{AtlasPackLink, InstanceConfig};
use crate::models::{
    AppSettings, AtlasProfile, AtlasSession, AuthSession, InstanceSource, ModLoaderConfig,
    ModLoaderKind, Profile, TroubleshooterFinding,
};

fn sample_settings() -> AppSettings {
    AppSettings {
        ms_client_id: None,
        atlas_hub_url: None,
        default_java_memory_mb: 4096,
        default_jvm_args: None,
        instances: vec![],
        selected_instance_id: None,
        theme_mode: Some("system".to_string()),
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
    assert!(!report.java_ready);
    assert_eq!(report.checklist.len(), 5);
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
