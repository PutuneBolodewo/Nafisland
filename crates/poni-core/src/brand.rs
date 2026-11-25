//! Centralized brand identity and endpoint configuration.
//!
//! Values are injected at compile time from the workspace `brand.toml` via `build.rs`.

pub const APP_NAME: &str = env!("BRAND_APP_NAME");
pub const APP_SHORT_NAME: &str = env!("BRAND_APP_SHORT_NAME");
pub const MASCOT_NAME: &str = env!("BRAND_MASCOT_NAME");
pub const VENDOR_NAME: &str = env!("BRAND_VENDOR_NAME");
pub const COPYRIGHT: &str = env!("BRAND_COPYRIGHT");
pub const DESCRIPTION: &str = env!("BRAND_DESCRIPTION");
pub const SUPPORT_EMAIL: &str = env!("BRAND_SUPPORT_EMAIL");
pub const REPO_URL: &str = env!("BRAND_REPO_URL");

const RAW_UPDATE_FEED_URL: &str = env!("BRAND_UPDATE_FEED_URL");
const RAW_DROP_SERVER_URL: &str = env!("BRAND_DROP_SERVER_URL");
const RAW_PLACES_BASE_URL: &str = env!("BRAND_PLACES_BASE_URL");
const RAW_ADDONS_RELEASES_URL: &str = env!("BRAND_ADDONS_RELEASES_URL");

fn non_empty(s: &'static str) -> Option<&'static str> {
    let trimmed = s.trim();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

/// Signed update feed JSON URL, or `None` when auto-update is disabled.
pub fn update_feed_url() -> Option<&'static str> {
    non_empty(RAW_UPDATE_FEED_URL)
}

/// Default paired-drop meeting server URL, or `None` when disabled.
pub fn drop_server_url() -> Option<&'static str> {
    non_empty(RAW_DROP_SERVER_URL)
}

/// Base URL for world prayer places dataset, or `None` when disabled.
pub fn places_base_url() -> Option<&'static str> {
    non_empty(RAW_PLACES_BASE_URL)
}

/// Base URL for add-on release downloads, or `None` when disabled.
pub fn addons_releases_url() -> Option<&'static str> {
    non_empty(RAW_ADDONS_RELEASES_URL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_constants_are_populated() {
        assert!(!APP_NAME.is_empty());
        assert!(!VENDOR_NAME.is_empty());
        assert!(!COPYRIGHT.is_empty());
    }
}
