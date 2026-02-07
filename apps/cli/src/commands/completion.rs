use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, CommandFactory, Subcommand, ValueEnum};
use clap_complete::{generate, shells};

#[derive(Args)]
pub struct CompletionArgs {
    #[command(subcommand)]
    command: Option<CompletionCommand>,

    #[arg(value_enum)]
    shell: Option<CompletionShell>,
}

#[derive(Subcommand)]
pub enum CompletionCommand {
    Install(InstallArgs),
}

#[derive(Args)]
pub struct InstallArgs {
    #[arg(value_enum)]
    shell: Option<CompletionShell>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    #[value(name = "powershell", alias = "pwsh")]
    Powershell,
}

pub fn run(args: CompletionArgs) -> Result<()> {
    if let Some(command) = args.command {
        if args.shell.is_some() {
            bail!("Pass either a shell (`atlas completion <shell>`) or `install`, not both.");
        }
        return match command {
            CompletionCommand::Install(install_args) => install(install_args),
        };
    }

    let shell = args.shell.context(
        "Missing shell. Use `atlas completion <bash|zsh|powershell>` or `atlas completion install`.",
    )?;
    emit_to_stdout(shell)
}

fn emit_to_stdout(shell: CompletionShell) -> Result<()> {
    let script = render_completion(shell)?;
    io::stdout()
        .write_all(script.as_bytes())
        .context("Failed to write completion script to stdout")?;
    Ok(())
}

fn install(args: InstallArgs) -> Result<()> {
    let shell = args.shell.or_else(detect_current_shell).context(
        "Could not detect your shell. Use `atlas completion install <bash|zsh|powershell>`.",
    )?;

    let script = render_completion(shell)?;
    match shell {
        CompletionShell::Bash => install_bash(&script)?,
        CompletionShell::Zsh => install_zsh(&script)?,
        CompletionShell::Powershell => install_powershell(&script)?,
    }

    println!(
        "Installed {} completion for `{}`.",
        shell.as_str(),
        command_name()
    );
    println!("Open a new shell session (or reload your profile) to start using it.");
    Ok(())
}

fn render_completion(shell: CompletionShell) -> Result<String> {
    let mut cmd = crate::Cli::command();
    let command_name = cmd.get_name().to_owned();
    let mut out = Vec::new();

    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut cmd, command_name, &mut out),
        CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, command_name, &mut out),
        CompletionShell::Powershell => {
            generate(shells::PowerShell, &mut cmd, command_name, &mut out)
        }
    }

    String::from_utf8(out).context("Generated completion script was not valid UTF-8")
}

fn detect_current_shell() -> Option<CompletionShell> {
    if let Some(shell_path) = env::var_os("SHELL") {
        if let Some(shell) = detect_shell_from_name(&shell_path.to_string_lossy()) {
            return Some(shell);
        }
    }

    if is_powershell_session() {
        return Some(CompletionShell::Powershell);
    }

    None
}

fn detect_shell_from_name(value: &str) -> Option<CompletionShell> {
    let name = Path::new(value)
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase();

    if name.contains("zsh") {
        return Some(CompletionShell::Zsh);
    }
    if name.contains("bash") {
        return Some(CompletionShell::Bash);
    }
    if name.contains("pwsh") || name.contains("powershell") {
        return Some(CompletionShell::Powershell);
    }

    None
}

fn is_powershell_session() -> bool {
    [
        "PSModulePath",
        "POWERSHELL_DISTRIBUTION_CHANNEL",
        "PSExecutionPolicyPreference",
    ]
    .iter()
    .any(|key| env::var_os(key).is_some())
}

fn install_bash(script: &str) -> Result<()> {
    let completion_path = bash_completion_path()?;
    write_text_file(&completion_path, script)?;

    let bashrc = home_dir()?.join(".bashrc");
    let quoted = bash_single_quoted_path(&completion_path);
    let source_line = format!("[[ -f {quoted} ]] && source {quoted}");
    ensure_profile_block(&bashrc, "# atlas-cli completion", &[source_line.as_str()])?;

    println!("Completion script: {}", completion_path.display());
    println!("Profile updated: {}", bashrc.display());
    Ok(())
}

fn install_zsh(script: &str) -> Result<()> {
    if let Some(path) = try_write_to_zsh_fpath(script) {
        println!("Completion script: {}", path.display());
        return Ok(());
    }

    let zfunc_dir = home_dir()?.join(".zfunc");
    let completion_path = zfunc_dir.join(format!("_{}", command_name()));
    write_text_file(&completion_path, script)?;

    let zshrc = zshrc_path();
    ensure_profile_block(
        &zshrc,
        "# atlas-cli completion",
        &[r#"fpath=("$HOME/.zfunc" $fpath)"#],
    )?;

    println!("Completion script: {}", completion_path.display());
    println!("Profile updated: {}", zshrc.display());
    Ok(())
}

fn install_powershell(script: &str) -> Result<()> {
    let completion_path = powershell_completion_path()?;
    write_text_file(&completion_path, script)?;

    let profile_path = resolve_powershell_profile_path()?;
    let escaped = powershell_escape_double_quotes(&completion_path.display().to_string());
    let source_line = format!(r#"if (Test-Path "{escaped}") {{ . "{escaped}" }}"#);
    ensure_profile_block(
        &profile_path,
        "# atlas-cli completion",
        &[source_line.as_str()],
    )?;

    println!("Completion script: {}", completion_path.display());
    println!("Profile updated: {}", profile_path.display());
    Ok(())
}

fn try_write_to_zsh_fpath(script: &str) -> Option<PathBuf> {
    let fpath = env::var_os("FPATH")?;
    for dir in env::split_paths(&fpath) {
        if !dir.is_dir() {
            continue;
        }

        let candidate = dir.join(format!("_{}", command_name()));
        if fs::write(&candidate, script).is_ok() {
            return Some(candidate);
        }
    }

    None
}

fn bash_completion_path() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("BASH_COMPLETION_USER_DIR") {
        return Ok(PathBuf::from(dir).join("completions").join(command_name()));
    }

    if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg)
            .join("bash-completion")
            .join("completions")
            .join(command_name()));
    }

    Ok(home_dir()?
        .join(".local")
        .join("share")
        .join("bash-completion")
        .join("completions")
        .join(command_name()))
}

fn powershell_completion_path() -> Result<PathBuf> {
    let base = if let Some(config_dir) = dirs::config_dir() {
        config_dir
    } else {
        home_dir()?.join(".config")
    };

    Ok(base
        .join("powershell")
        .join("completions")
        .join(format!("{}.ps1", command_name())))
}

fn resolve_powershell_profile_path() -> Result<PathBuf> {
    if let Some(path) = query_profile_path("pwsh") {
        return Ok(path);
    }
    if let Some(path) = query_profile_path("powershell") {
        return Ok(path);
    }

    let home = home_dir()?;

    #[cfg(target_os = "windows")]
    {
        return Ok(home
            .join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(home
            .join(".config")
            .join("powershell")
            .join("Microsoft.PowerShell_profile.ps1"))
    }
}

fn query_profile_path(program: &str) -> Option<PathBuf> {
    let output = Command::new(program)
        .arg("-NoProfile")
        .arg("-Command")
        .arg("$PROFILE")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }

    Some(PathBuf::from(value))
}

fn ensure_profile_block(path: &Path, marker: &str, lines: &[&str]) -> Result<()> {
    if lines.is_empty() {
        bail!("No profile lines were provided.");
    }

    let mut existing = fs::read_to_string(path).unwrap_or_default();
    if existing.contains(marker) {
        return Ok(());
    }

    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(marker);
    existing.push('\n');
    for line in lines {
        existing.push_str(line);
        existing.push('\n');
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    fs::write(path, existing).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn write_text_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("Failed to write {}", path.display()))
}

fn bash_single_quoted_path(path: &Path) -> String {
    let value = path.display().to_string().replace('\'', "'\"'\"'");
    format!("'{value}'")
}

fn powershell_escape_double_quotes(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

fn command_name() -> &'static str {
    "atlas"
}

fn zshrc_path() -> PathBuf {
    if let Some(zdotdir) = env::var_os("ZDOTDIR") {
        PathBuf::from(zdotdir).join(".zshrc")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".zshrc")
    }
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("Could not resolve home directory.")
}

impl CompletionShell {
    fn as_str(self) -> &'static str {
        match self {
            CompletionShell::Bash => "bash",
            CompletionShell::Zsh => "zsh",
            CompletionShell::Powershell => "powershell",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompletionShell, detect_shell_from_name};

    #[test]
    fn detects_shell_from_binary_name() {
        assert_eq!(
            detect_shell_from_name("/bin/bash"),
            Some(CompletionShell::Bash)
        );
        assert_eq!(
            detect_shell_from_name("/bin/zsh"),
            Some(CompletionShell::Zsh)
        );
        assert_eq!(
            detect_shell_from_name(r"C:\Program Files\PowerShell\7\pwsh.exe"),
            Some(CompletionShell::Powershell)
        );
    }
}
