use std::path::PathBuf;

/// A detected Unreal Engine installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UeInstall {
    /// Clean version string, e.g. `5.7.4`
    pub version: String,
    /// Root directory of the engine install
    pub path: PathBuf,
}

/// Scan for locally installed Unreal Engine builds.
///
/// On macOS/Linux this reads Epic's `LauncherInstalled.dat` JSON file.
/// On Windows it additionally checks the registry (cfg-gated).
///
/// Returns a sorted list (ascending version order). Empty if none found or
/// the launcher data file is absent.
pub fn find_installed_engines() -> Vec<UeInstall> {
    let mut installs = Vec::new();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    scan_launcher_dat(&mut installs);

    #[cfg(target_os = "windows")]
    scan_registry(&mut installs);

    // Sort ascending by version string components
    installs.sort_by(|a, b| version_cmp(&a.version, &b.version));
    installs.dedup_by(|a, b| a.path == b.path);
    installs
}

// ── macOS / Linux ─────────────────────────────────────────────────────────────

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn launcher_dat_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = dirs_next::data_dir(); // ~/Library/Application Support

    #[cfg(target_os = "linux")]
    let base = dirs_next::config_dir(); // ~/.config

    base.map(|d| {
        d.join("Epic")
            .join("UnrealEngineLauncher")
            .join("LauncherInstalled.dat")
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn scan_launcher_dat(out: &mut Vec<UeInstall>) {
    let path = match launcher_dat_path() {
        Some(p) if p.exists() => p,
        _ => return,
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };
    parse_installation_list(&json, out);
}

/// Parse a `LauncherInstalled.dat` JSON value into `UeInstall` entries.
/// Extracted so tests can call this directly instead of duplicating the logic.
pub(crate) fn parse_installation_list(json: &serde_json::Value, out: &mut Vec<UeInstall>) {
    let list = match json.get("InstallationList").and_then(|v| v.as_array()) {
        Some(l) => l,
        None => return,
    };

    for entry in list {
        // Only engine entries; skip plugins like FabPlugin_*, QuixelBridge_*
        let app_name = entry
            .get("AppName")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !app_name.starts_with("UE_") {
            continue;
        }

        let location = match entry.get("InstallLocation").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => PathBuf::from(s),
            _ => continue,
        };

        // AppVersion looks like "5.7.4-51494982+++UE5+Release-5.7-Mac"
        // Take everything before the first '-' as the clean version.
        let version = entry
            .get("AppVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split('-')
            .next()
            .unwrap_or("")
            .to_string();

        if !version.is_empty() {
            out.push(UeInstall { version, path: location });
        }
    }
}

// ── Windows registry ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn scan_registry(out: &mut Vec<UeInstall>) {
    use winreg::RegKey;
    use winreg::enums::*;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let base = match hklm.open_subkey("SOFTWARE\\EpicGames\\Unreal Engine") {
        Ok(k) => k,
        Err(_) => return,
    };

    for key_name in base.enum_keys().flatten() {
        if let Ok(sub) = base.open_subkey(&key_name) {
            if let Ok(location) = sub.get_value::<String, _>("InstalledDirectory") {
                out.push(UeInstall {
                    version: key_name.clone(),
                    path: PathBuf::from(location),
                });
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compare two dotted version strings numerically, component by component.
/// Falls back to lexicographic for non-numeric parts.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts = a.split('.').collect::<Vec<_>>();
    let b_parts = b.split('.').collect::<Vec<_>>();
    let len = a_parts.len().max(b_parts.len());
    for i in 0..len {
        let av: u64 = a_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        let bv: u64 = b_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        let ord = av.cmp(&bv);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    std::cmp::Ordering::Equal
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dat(entries: &[(&str, &str, &str)]) -> String {
        let items: Vec<String> = entries
            .iter()
            .map(|(app_name, location, app_version)| {
                format!(
                    r#"{{"AppName":"{app_name}","InstallLocation":"{location}","AppVersion":"{app_version}"}}"#
                )
            })
            .collect();
        format!("{{\"InstallationList\":[{}]}}", items.join(","))
    }

    fn parse_dat(json: &str) -> Vec<UeInstall> {
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut out = Vec::new();
        parse_installation_list(&value, &mut out);
        out.sort_by(|a, b| version_cmp(&a.version, &b.version));
        out
    }

    #[test]
    fn test_filters_non_engine_entries() {
        let json = make_dat(&[
            ("FabPlugin_5.7", "/Games/UE_5.7", "5.7.0-xxx"),
            ("QuixelBridge_5.7", "/Games/UE_5.7", "5.7.0-xxx"),
            ("UE_5.7", "/Games/UE_5.7", "5.7.4-51494982+++UE5+Release-5.7-Mac"),
        ]);
        let installs = parse_dat(&json);
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].version, "5.7.4");
    }

    #[test]
    fn test_version_extracted_from_app_version() {
        let json = make_dat(&[("UE_5.3", "/Games/UE_5.3", "5.3.2-12345+++UE5+Release-5.3-Mac")]);
        let installs = parse_dat(&json);
        assert_eq!(installs[0].version, "5.3.2");
    }

    #[test]
    fn test_sorted_ascending() {
        let json = make_dat(&[
            ("UE_5.7", "/Games/UE_5.7", "5.7.0-xxx"),
            ("UE_5.3", "/Games/UE_5.3", "5.3.0-xxx"),
            ("UE_5.5", "/Games/UE_5.5", "5.5.0-xxx"),
        ]);
        let installs = parse_dat(&json);
        let versions: Vec<&str> = installs.iter().map(|i| i.version.as_str()).collect();
        assert_eq!(versions, ["5.3.0", "5.5.0", "5.7.0"]);
    }

    #[test]
    fn test_empty_list() {
        let json = r#"{"InstallationList":[]}"#;
        let installs = parse_dat(json);
        assert!(installs.is_empty());
    }

    #[test]
    fn test_find_installed_engines_does_not_panic_when_file_absent() {
        // Just verify it returns without panicking in any environment.
        let _ = find_installed_engines();
    }

    #[test]
    fn test_version_cmp() {
        use std::cmp::Ordering::*;
        assert_eq!(version_cmp("5.3.0", "5.7.0"), Less);
        assert_eq!(version_cmp("5.7.0", "5.3.0"), Greater);
        assert_eq!(version_cmp("5.3.0", "5.3.0"), Equal);
        assert_eq!(version_cmp("5.10.0", "5.9.0"), Greater); // numeric, not lex
    }
}
