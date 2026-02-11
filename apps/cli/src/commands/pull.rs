use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use atlas_client::hub::HubClient;
use base64::Engine;
use clap::Args;
use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use serde::Deserialize;

use crate::auth_store;

#[derive(Args)]
pub struct PullArgs {
    #[arg(value_name = "PACK_NAME", conflicts_with = "id")]
    query: Option<String>,
    #[arg(long, value_name = "PACK_ID", conflicts_with = "query")]
    id: Option<String>,
    #[arg(long)]
    hub_url: Option<String>,
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePack {
    pack_id: String,
    pack_name: String,
    pack_slug: String,
    repo_url: Option<String>,
}

pub fn run(args: PullArgs) -> Result<()> {
    let hub_url = auth_store::resolve_hub_url(args.hub_url.clone());
    let access_token = auth_store::require_access_token_for_hub(&hub_url)?;
    let mut client = HubClient::new(&hub_url)?;
    client.set_token(access_token.clone());

    let packs = fetch_remote_packs(&client)?;
    let cloneable = packs
        .into_iter()
        .filter(|pack| {
            pack.repo_url
                .as_ref()
                .map(|url| !url.trim().is_empty())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if cloneable.is_empty() {
        bail!("No accessible packs with linked repositories were found.");
    }

    let selected = select_pack(&cloneable, &args)?;
    let repo_url = selected
        .repo_url
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .context("Selected pack does not have an associated repository.")?;

    let github_token = if is_github_https_url(repo_url) {
        request_linked_github_access_token(&client)?
    } else {
        None
    };

    if github_token.is_none() && is_github_https_url(repo_url) {
        println!("No linked GitHub token found. Trying local git credentials.");
    }

    println!(
        "Cloning {} ({}) from {}",
        selected.pack_name, selected.pack_id, repo_url
    );
    run_git_clone(repo_url, args.output.as_ref(), github_token.as_deref())?;
    Ok(())
}

fn fetch_remote_packs(client: &HubClient) -> Result<Vec<RemotePack>> {
    let packs = client.blocking_list_launcher_packs()?;
    Ok(packs
        .into_iter()
        .map(|pack| RemotePack {
            pack_id: pack.pack_id,
            pack_name: pack.pack_name,
            pack_slug: pack.pack_slug,
            repo_url: pack.repo_url,
        })
        .collect())
}

fn request_linked_github_access_token(client: &HubClient) -> Result<Option<String>> {
    client.blocking_get_github_token()
}

fn select_pack(packs: &[RemotePack], args: &PullArgs) -> Result<RemotePack> {
    if let Some(pack_id) = args.id.as_ref() {
        return packs
            .iter()
            .find(|pack| pack.pack_id == *pack_id)
            .cloned()
            .with_context(|| format!("No accessible pack found with id '{}'.", pack_id));
    }

    if let Some(query) = args.query.as_ref() {
        let matches = filter_packs(packs, query);
        if matches.is_empty() {
            bail!("No accessible packs matched '{}'.", query);
        }
        if matches.len() == 1 {
            return Ok(matches[0].clone());
        }
        return prompt_pack_selection(&matches, "Multiple packs matched. Select a pack to clone");
    }

    prompt_pack_selection(packs, "Select a pack to clone")
}

fn filter_packs(packs: &[RemotePack], query: &str) -> Vec<RemotePack> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return packs.to_vec();
    }

    packs
        .iter()
        .filter(|pack| {
            pack.pack_id.eq_ignore_ascii_case(&needle)
                || pack.pack_name.eq_ignore_ascii_case(&needle)
                || pack.pack_slug.eq_ignore_ascii_case(&needle)
                || pack.pack_name.to_ascii_lowercase().contains(&needle)
                || pack.pack_slug.to_ascii_lowercase().contains(&needle)
                || pack.pack_id.to_ascii_lowercase().contains(&needle)
        })
        .cloned()
        .collect::<Vec<_>>()
}

fn prompt_pack_selection(packs: &[RemotePack], prompt: &str) -> Result<RemotePack> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("Multiple packs matched. Re-run with `--id <PACK_ID>` in non-interactive mode.");
    }

    let labels = packs.iter().map(format_pack_label).collect::<Vec<_>>();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&labels)
        .default(0)
        .interact()
        .context("Failed to read pack selection")?;

    packs
        .get(selection)
        .cloned()
        .context("Invalid pack selection")
}

fn format_pack_label(pack: &RemotePack) -> String {
    format!("{} ({}) [{}]", pack.pack_name, pack.pack_slug, pack.pack_id)
}

fn run_git_clone(
    repo_url: &str,
    output: Option<&PathBuf>,
    github_token: Option<&str>,
) -> Result<()> {
    let mut command = Command::new("git");

    if let Some(token) = github_token.filter(|_| is_github_https_url(repo_url)) {
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        command.arg("-c").arg(format!(
            "http.https://github.com/.extraheader=AUTHORIZATION: basic {basic}"
        ));
    }

    command.arg("clone").arg(repo_url);
    if let Some(path) = output {
        command.arg(path);
    }

    let status = command.status().context("Failed to run git clone")?;
    if !status.success() {
        bail!("git clone failed.");
    }
    Ok(())
}

fn is_github_https_url(repo_url: &str) -> bool {
    repo_url
        .to_ascii_lowercase()
        .starts_with("https://github.com/")
}
