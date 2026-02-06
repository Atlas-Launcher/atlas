use super::*;
use crate::launcher::manifest::{
    ArgValue, Argument, Arguments, AssetIndex, Download, Library, Rule, VersionData,
    VersionDownloads,
};
use std::collections::HashMap;
use std::path::PathBuf;

fn base_version() -> VersionData {
    VersionData {
        id: "1.0".to_string(),
        kind: "release".to_string(),
        main_class: "Main".to_string(),
        arguments: None,
        minecraft_arguments: None,
        asset_index: Some(AssetIndex {
            id: "idx".to_string(),
            url: "https://example.com".to_string(),
            sha1: None,
            size: None,
        }),
        downloads: Some(VersionDownloads {
            client: Download {
                path: None,
                url: "https://example.com".to_string(),
                sha1: None,
                size: None,
            },
        }),
        libraries: Vec::<Library>::new(),
        java_version: None,
        inherits_from: None,
    }
}

#[test]
fn replaces_tokens_in_legacy_args() {
    let mut version = base_version();
    version.minecraft_arguments = Some("--username ${auth_player_name}".to_string());
    let mut replacements = HashMap::new();
    replacements.insert("auth_player_name", "Steve".to_string());

    let (_jvm, game) = args::build_arguments(&version, &replacements).unwrap();
    assert_eq!(game, vec!["--username".to_string(), "Steve".to_string()]);
}

#[test]
fn filters_feature_gated_args() {
    let mut version = base_version();
    version.arguments = Some(Arguments {
        jvm: vec![],
        game: vec![
            Argument::String("--demo".to_string()),
            Argument::Rule {
                rules: vec![Rule {
                    action: "allow".to_string(),
                    os: None,
                    features: Some(HashMap::from([(
                        "has_quick_plays_support".to_string(),
                        true,
                    )])),
                }],
                value: ArgValue::String("--quickPlay".to_string()),
            },
        ],
    });

    let (jvm, game) = args::build_arguments(&version, &HashMap::new()).unwrap();
    assert!(jvm.is_empty());
    assert!(game.contains(&"--demo".to_string()));
    assert!(!game.contains(&"--quickPlay".to_string()));
}

#[test]
fn unresolved_tokens_are_reported() {
    let args = vec![
        "-Dfoo=${known}".to_string(),
        "--bar".to_string(),
        "${missing_token}".to_string(),
    ];
    let unresolved = crate::launcher::args::unresolved_tokens(&args);
    assert_eq!(
        unresolved,
        vec!["known".to_string(), "missing_token".to_string()]
    );
}

#[test]
fn library_path_from_maven_coords() {
    let path = libraries::library_path_from_name("com.example:demo:1.2.3");
    assert_eq!(path, "com/example/demo/1.2.3/demo-1.2.3.jar");
}

#[test]
fn classpath_joins_with_separator() {
    let libs = vec![PathBuf::from("/tmp/a.jar"), PathBuf::from("/tmp/b.jar")];
    let classpath = libraries::build_classpath(&libs, PathBuf::from("/tmp/c.jar").as_path());
    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    assert!(classpath.contains(sep));
    assert!(classpath.contains("a.jar"));
    assert!(classpath.contains("c.jar"));
}

#[test]
fn classpath_deduplicates_library_entries() {
    let libs = vec![
        PathBuf::from("/tmp/a.jar"),
        PathBuf::from("/tmp/a.jar"),
        PathBuf::from("/tmp/b.jar"),
    ];
    let classpath = libraries::build_classpath(&libs, PathBuf::from("/tmp/c.jar").as_path());
    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let entries: Vec<&str> = classpath.split(sep).collect();
    let a_count = entries
        .iter()
        .filter(|entry| entry.ends_with("a.jar"))
        .count();
    assert_eq!(a_count, 1);
}

#[test]
fn selects_fallback_component() {
    let mut map = serde_json::Map::new();
    map.insert("java-runtime-gamma".to_string(), serde_json::json!([{}]));
    let chosen = java::select_java_component(&map, "java-runtime-delta");
    assert_eq!(chosen, "java-runtime-gamma");
}

#[test]
fn locate_java_binary_prefers_manifest_entry() {
    let manifest = java::JavaRuntimeFiles {
        files: HashMap::from([(
            "runtime/bin/java".to_string(),
            java::JavaRuntimeFile {
                kind: "file".to_string(),
                executable: true,
                downloads: None,
                target: None,
            },
        )]),
    };

    let runtime_home = PathBuf::from("/tmp/runtime");
    let path = java::locate_java_binary(&runtime_home, &manifest);
    assert!(path.to_string_lossy().ends_with("runtime/bin/java"));
}
