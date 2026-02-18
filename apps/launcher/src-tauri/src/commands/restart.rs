#[tauri::command]
pub fn restart_app(app: tauri::AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve launcher executable path: {err}"))?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    std::process::Command::new(&exe)
        .args(&args)
        .spawn()
        .map_err(|err| {
            format!(
                "Failed to spawn launcher process from {}: {err}",
                exe.display()
            )
        })?;

    app.exit(0);
    Ok(())
}
