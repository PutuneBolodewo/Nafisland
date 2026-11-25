use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let brand_path = Path::new(&manifest_dir).join("../../brand.toml");
    println!("cargo:rerun-if-changed={}", brand_path.display());
    println!("cargo:rerun-if-changed=res/poni.rc");
    println!("cargo:rerun-if-changed=res/poni.manifest");
    println!("cargo:rerun-if-changed=res/poni.ico");

    // Only Windows targets get the icon, manifest (Per-Monitor-V2 DPI) and version info.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out_rc = generate_rc(&manifest_dir, &brand_path);
        // Panics with a clear message if rc.exe / x86_64-w64-mingw32-windres is missing.
        embed_resource::compile(&out_rc, embed_resource::NONE);
        // MSVC builds: resolve statically imported non-KnownDLLs (d2d1, dwrite,
        // dwmapi, winmm) from System32 only, never from the exe's folder.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
        }
    }
    build_info();
}

fn generate_rc(manifest_dir: &str, brand_path: &Path) -> std::path::PathBuf {
    let text = std::fs::read_to_string(brand_path).expect("failed to read workspace brand.toml");
    let get = |k: &str, fallback: &str| -> String {
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                if key.trim() == k {
                    let v = val.trim();
                    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                        return v[1..v.len() - 1].to_string();
                    }
                }
            }
        }
        fallback.to_string()
    };

    let app_name = get("app_name", "Nafisland");
    let vendor = get("vendor_name", "Nafisland");
    let copyright = get("copyright", "Copyright (C) 2026 Nafisland");
    let description = get("description", "Nafisland - a living notch for Windows");
    let repo_url = get("repo_url", "https://github.com/example/nafisland");
    println!("cargo:rustc-env=BRAND_APP_NAME={app_name}");
    println!("cargo:rustc-env=BRAND_REPO_URL={repo_url}");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.2.30".to_string());
    let parts: Vec<&str> = version.split('.').collect();
    let file_ver_num = format!(
        "{},{},{},0",
        parts.first().copied().unwrap_or("0"),
        parts.get(1).copied().unwrap_or("0"),
        parts.get(2).copied().unwrap_or("0")
    );

    let res_dir = Path::new(manifest_dir).join("res");
    let ico_path = res_dir.join("poni.ico").to_string_lossy().replace('\\', "/");
    let manifest_path = res_dir.join("poni.manifest").to_string_lossy().replace('\\', "/");

    let rc = format!(
        r#"1 ICON "{ico_path}"
1 24 "{manifest_path}"

1 VERSIONINFO
FILEVERSION {file_ver_num}
PRODUCTVERSION {file_ver_num}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "{vendor}\0"
      VALUE "FileDescription", "{description}\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "poni\0"
      VALUE "LegalCopyright", "{copyright}\0"
      VALUE "OriginalFilename", "poni.exe\0"
      VALUE "ProductName", "{app_name}\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_rc = out_dir.join("poni.rc");
    std::fs::write(&out_rc, rc).expect("failed to write generated poni.rc");
    out_rc
}

/// Which build this is, for the menu's About line (src/version.rs): the `ci`
/// run of a release tag builds the release, any other CI run a test build of
/// its commit, and anything else is a local build (no commit or day, so a
/// local rebuild never shows a stale one).
fn build_info() {
    for var in ["GITHUB_ACTIONS", "GITHUB_REF_TYPE", "GITHUB_REF_NAME", "GITHUB_SHA"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let ci = env("GITHUB_ACTIONS") == "true";
    let release = ci && env("GITHUB_REF_TYPE") == "tag" && env("GITHUB_REF_NAME") == format!("v{}", env("CARGO_PKG_VERSION"));
    let kind = if release { "release" } else if ci { "test" } else { "local" };
    let commit: String = if ci { env("GITHUB_SHA").chars().take(7).collect() } else { String::new() };
    let day = if ci { utc_day() } else { String::new() };
    println!("cargo:rustc-env=PONI_BUILD_KIND={kind}");
    println!("cargo:rustc-env=PONI_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=PONI_BUILD_DAY={day}");
}

/// Today in UTC as "25 Oct 2026" (Howard Hinnant's days-to-civil).
fn utc_day() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{d} {} {y}", MONTHS[(m - 1) as usize])
}
