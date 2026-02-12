fn normalize_build_version(value: &str) -> String {
    value.trim().trim_start_matches('v').to_string()
}

fn main() {
    let value = std::env::var("ATLAS_BUILD_VERSION").unwrap_or_default();
    let resolved = if value.trim().is_empty() {
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string())
    } else {
        normalize_build_version(&value)
    };

    println!("cargo:rustc-env=ATLAS_BUILD_VERSION={resolved}");
}
