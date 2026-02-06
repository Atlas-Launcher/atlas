use std::collections::HashMap;
use std::collections::HashSet;

use super::manifest::{ArgValue, Argument, Rule, VersionData};

pub fn build_arguments(
    version: &VersionData,
    replacements: &HashMap<&str, String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    if let Some(arguments) = &version.arguments {
        let jvm = expand_args(&arguments.jvm, replacements);
        let game = expand_args(&arguments.game, replacements);
        return Ok((jvm, game));
    }

    let raw = version
        .minecraft_arguments
        .clone()
        .ok_or_else(|| "Missing arguments in version metadata".to_string())?;
    let game = raw
        .split_whitespace()
        .map(|arg| replace_tokens(arg, replacements))
        .collect::<Vec<_>>();

    Ok((Vec::new(), game))
}

pub fn split_jvm_args(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn unresolved_tokens(args: &[String]) -> Vec<String> {
    let mut out = HashSet::new();
    for arg in args {
        for token in extract_tokens(arg) {
            out.insert(token);
        }
    }
    let mut values = out.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn expand_args(args: &[Argument], replacements: &HashMap<&str, String>) -> Vec<String> {
    let mut expanded = Vec::new();
    for arg in args {
        match arg {
            Argument::String(value) => expanded.push(replace_tokens(value, replacements)),
            Argument::Rule { rules, value } => {
                if rules_allow(&Some(rules.clone())) {
                    match value {
                        ArgValue::String(value) => {
                            expanded.push(replace_tokens(value, replacements))
                        }
                        ArgValue::List(list) => {
                            for item in list {
                                expanded.push(replace_tokens(item, replacements));
                            }
                        }
                    }
                }
            }
        }
    }
    expanded
}

fn replace_tokens(input: &str, replacements: &HashMap<&str, String>) -> String {
    let mut cursor = 0usize;
    let mut output = String::new();
    while let Some(start_rel) = input[cursor..].find("${") {
        let start = cursor + start_rel;
        output.push_str(&input[cursor..start]);

        let token_start = start + 2;
        let Some(end_rel) = input[token_start..].find('}') else {
            output.push_str(&input[start..]);
            cursor = input.len();
            break;
        };
        let end = token_start + end_rel;
        let key = &input[token_start..end];
        if let Some(value) = replacements.get(key) {
            output.push_str(value);
        } else {
            output.push_str(&input[start..=end]);
        }
        cursor = end + 1;
    }
    if cursor < input.len() {
        output.push_str(&input[cursor..]);
    }
    output
}

fn extract_tokens(input: &str) -> Vec<String> {
    let mut cursor = 0usize;
    let mut tokens = Vec::new();
    while let Some(start_rel) = input[cursor..].find("${") {
        let start = cursor + start_rel + 2;
        let Some(end_rel) = input[start..].find('}') else {
            break;
        };
        let end = start + end_rel;
        if end > start {
            tokens.push(input[start..end].to_string());
        }
        cursor = end + 1;
    }
    tokens
}

pub fn rules_allow(rules: &Option<Vec<Rule>>) -> bool {
    let Some(rules) = rules else {
        return true;
    };

    let mut allowed = false;
    for rule in rules {
        let os_applies = rule
            .os
            .as_ref()
            .and_then(|os| os.name.as_ref())
            .map(|name| name == current_os_key())
            .unwrap_or(true);

        let features_applies = features_match(rule.features.as_ref());
        let applies = os_applies && features_applies;

        if applies {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn features_match(features: Option<&HashMap<String, bool>>) -> bool {
    let Some(features) = features else {
        return true;
    };

    let supported = current_features();
    for (key, expected) in features {
        let actual = supported.get(key).copied().unwrap_or(false);
        if actual != *expected {
            return false;
        }
    }
    true
}

fn current_features() -> HashMap<String, bool> {
    let mut features = HashMap::new();
    features.insert("is_demo_user".to_string(), false);
    features.insert("has_custom_resolution".to_string(), false);
    features.insert("has_quick_plays_support".to_string(), false);
    features.insert("is_quick_play_singleplayer".to_string(), false);
    features.insert("is_quick_play_multiplayer".to_string(), false);
    features.insert("is_quick_play_realms".to_string(), false);
    features
}

fn current_os_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}
