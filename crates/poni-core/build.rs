use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let brand_path = Path::new(&manifest_dir).join("../../brand.toml");
    println!("cargo:rerun-if-changed={}", brand_path.display());

    let text = std::fs::read_to_string(&brand_path).expect("failed to read workspace brand.toml");
    let keys = [
        ("app_name", "BRAND_APP_NAME"),
        ("app_short_name", "BRAND_APP_SHORT_NAME"),
        ("mascot_name", "BRAND_MASCOT_NAME"),
        ("vendor_name", "BRAND_VENDOR_NAME"),
        ("copyright", "BRAND_COPYRIGHT"),
        ("description", "BRAND_DESCRIPTION"),
        ("support_email", "BRAND_SUPPORT_EMAIL"),
        ("repo_url", "BRAND_REPO_URL"),
        ("update_feed_url", "BRAND_UPDATE_FEED_URL"),
        ("drop_server_url", "BRAND_DROP_SERVER_URL"),
        ("places_base_url", "BRAND_PLACES_BASE_URL"),
        ("addons_releases_url", "BRAND_ADDONS_RELEASES_URL"),
    ];

    for (toml_key, env_key) in keys {
        let val = parse_toml_str(&text, toml_key).unwrap_or_default();
        println!("cargo:rustc-env={env_key}={val}");
    }
}

fn parse_toml_str(text: &str, key: &str) -> Option<String> {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once('=')?;
        if k.trim() == key {
            let v = v.trim();
            if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                return Some(v[1..v.len() - 1].to_string());
            }
        }
    }
    None
}
