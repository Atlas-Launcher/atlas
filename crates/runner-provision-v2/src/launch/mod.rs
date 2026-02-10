use std::fs;
use std::path::Path;
use crate::errors::ProvisionError;
use protocol::PackBlob;

mod plan;
pub use plan::LaunchPlan;

pub fn derive_launch_plan(_pack: &PackBlob, staging_current: &Path) -> Result<LaunchPlan, ProvisionError> {
    let run_sh = staging_current.join("run.sh");
    if run_sh.exists() {
        let mut argv = extract_run_sh_command(&run_sh)?;
        if !argv.iter().any(|arg| arg.eq_ignore_ascii_case("nogui")) {
            argv.push("nogui".to_string());
        }
        return Ok(LaunchPlan {
            cwd_rel: ".".into(),
            argv,
        });
    }

    let fabric_launch = staging_current.join("fabric-server-launch.jar");
    if fabric_launch.exists() {
        return Ok(LaunchPlan {
            cwd_rel: ".".into(),
            argv: vec![
                "java".into(),
                "-jar".into(),
                "fabric-server-launch.jar".into(),
                "nogui".into(),
            ],
        });
    }

    let server_jar = staging_current.join("server.jar");
    if server_jar.exists() {
        return Ok(LaunchPlan {
            cwd_rel: ".".into(),
            argv: vec![
                "java".into(),
                "-jar".into(),
                "server.jar".into(),
                "nogui".into(),
            ],
        });
    }

    Err(ProvisionError::Invalid(
        "missing server launch files (run.sh, fabric-server-launch.jar, or server.jar)"
            .to_string(),
    ))
}

fn extract_run_sh_command(path: &Path) -> Result<Vec<String>, ProvisionError> {
    let contents = fs::read_to_string(path)?;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if !line.contains("java") && !line.contains("$JAVA") && !line.contains("${JAVA}") {
            continue;
        }

        if !line.contains("@user_jvm_args.txt")
            && !line.contains("unix_args.txt")
            && !line.contains("server.jar")
            && !line.contains("-jar")
        {
            continue;
        }

        let tokens = split_shell_words(line);
        if tokens.is_empty() {
            continue;
        }

        let mut argv = Vec::new();
        for token in tokens {
            let normalized = token.trim_matches('"').trim_matches('\'');
            if normalized.is_empty() || normalized == "$@" || normalized == "\"$@\"" {
                continue;
            }

            let normalized = match normalized {
                "${JAVA}" | "$JAVA" => "java",
                "exec" => continue,
                other => other,
            };
            argv.push(normalized.to_string());
        }

        if argv.is_empty() {
            continue;
        }

        return Ok(argv);
    }

    Err(ProvisionError::Invalid(
        "failed to extract command from run.sh".to_string(),
    ))
}

fn split_shell_words(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    out.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

pub async fn write_launch_plan_to_dir(staging_current: &Path, plan: &LaunchPlan) -> Result<(), ProvisionError> {
    let dir = staging_current.join(".runner");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("launch.json");
    let bytes = serde_json::to_vec_pretty(plan)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

pub async fn read_launch_plan(server_root: &Path) -> Result<LaunchPlan, ProvisionError> {
    let path = server_root.join("current").join(".runner").join("launch.json");
    let bytes = tokio::fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
