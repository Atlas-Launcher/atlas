#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod auth;
mod commands;
mod config;
mod diagnostics;
mod launcher;
mod library;
mod models;
mod net;
mod paths;
mod settings;
mod state;
mod telemetry;

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            use tauri_plugin_deep_link::DeepLinkExt;
            app.deep_link().handle_cli_arguments(_args.iter());
            if let Some(window) = app.get_webview_window("main") {
                if window.is_minimized().unwrap_or(false) {
                    let _ = window.unminimize();
                }
                if !window.is_visible().unwrap_or(true) {
                    let _ = window.show();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = window.set_always_on_top(true);
                    let _ = window.set_always_on_top(false);
                }
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_prevent_default::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }

            {
                use tauri::Manager;
                #[cfg(not(target_os = "macos"))]
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(false);
                }
                if let Some(loading) = app.get_webview_window("loading") {
                    let _ = loading.show();
                    let _ = loading.set_focus();
                }
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_default_game_dir,
            commands::settings::get_system_memory_mb,
            commands::library::get_version_manifest_summary,
            commands::library::get_fabric_loader_versions,
            commands::library::get_neoforge_loader_versions,
            commands::library::list_installed_versions,
            commands::library::list_mods,
            commands::library::set_mod_enabled,
            commands::library::delete_mod,
            commands::library::uninstall_instance_data,
            commands::library::resolve_pack_mod,
            commands::library::list_atlas_remote_packs,
            commands::library::sync_atlas_pack,
            commands::auth::start_device_code,
            commands::auth::focus_main_window,
            commands::auth::focus_window,
            commands::auth::begin_deeplink_login,
            commands::auth::complete_loopback_login,
            commands::auth::complete_deeplink_login,
            commands::auth::complete_device_code,
            commands::auth::begin_atlas_login,
            commands::auth::start_atlas_device_code,
            commands::auth::complete_atlas_login,
            commands::auth::complete_atlas_device_code,
            commands::launcher::launch_minecraft,
            commands::launcher::download_minecraft_files,
            commands::auth::restore_session,
            commands::auth::restore_atlas_session,
            commands::auth::sign_out,
            commands::auth::atlas_sign_out,
            commands::auth::create_launcher_link_session,
            commands::auth::complete_launcher_link_session,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::diagnostics::get_launch_readiness,
            commands::diagnostics::run_troubleshooter,
            commands::diagnostics::apply_fix,
            commands::diagnostics::repair_installation,
            commands::diagnostics::create_support_bundle,
            commands::restart::restart_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
