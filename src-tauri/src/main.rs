mod auth;
mod commands;
mod config;
mod launcher;
mod library;
mod models;
mod net;
mod paths;
mod settings;
mod state;

use crate::state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_default_game_dir,
            commands::library::get_version_manifest_summary,
            commands::library::get_fabric_loader_versions,
            commands::library::get_neoforge_loader_versions,
            commands::library::list_installed_versions,
            commands::library::list_mods,
            commands::library::set_mod_enabled,
            commands::library::delete_mod,
            commands::auth::start_device_code,
            commands::auth::begin_deeplink_login,
            commands::auth::complete_deeplink_login,
            commands::auth::complete_device_code,
            commands::launcher::launch_minecraft,
            commands::launcher::download_minecraft_files,
            commands::auth::restore_session,
            commands::auth::sign_out,
            commands::settings::get_settings,
            commands::settings::update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
