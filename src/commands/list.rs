use crate::context::UEPMContext;
use crate::errors::UepmError;
use crate::lockfile::read_lockfile;
use crate::manifest::read_manifest;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct PluginEntry {
    pub name: String,
    #[serde(rename = "version")]
    pub resolved_version: Option<String>,
    pub engine_range: String,
    pub compatible: Option<bool>,
}

/// Entry point for `uepm list`. Prints all plugins and their engine compatibility.
pub async fn run(ctx: &UEPMContext) -> Result<(), UepmError> {
    let plugins = list_plugins(&ctx.project_dir)?;

    if ctx.output_mode == crate::context::OutputMode::Json {
        crate::output::emit_json(&plugins);
        return Ok(());
    }

    if plugins.is_empty() {
        crate::output::print_info(
            "No plugins installed. Run 'uepm install @scope/plugin' to add one.",
        );
        return Ok(());
    }

    for p in &plugins {
        let version = p.resolved_version.as_deref().unwrap_or("unknown");
        let compat = match p.compatible {
            Some(true) => "✓ compatible",
            Some(false) => "✗ incompatible",
            None => "? unknown",
        };
        crate::output::print_info(&format!(
            "{} @ {} — engine: {} [{}]",
            p.name, version, p.engine_range, compat
        ));
    }
    Ok(())
}

/// Build the list of plugins from `Config/UEPM.ini` and `uepm.lock`,
/// annotating each with its resolved version and engine compatibility.
pub fn list_plugins(project_dir: &Path) -> Result<Vec<PluginEntry>, UepmError> {
    let manifest = read_manifest(project_dir)?;
    let lock = read_lockfile(project_dir)?.unwrap_or_default();

    let engine_ver: Option<semver::Version> = manifest
        .engine_version
        .as_deref()
        .and_then(|v| semver::Version::parse(v).ok());

    let entries = manifest
        .plugins
        .iter()
        .map(|(name, engine_range)| {
            let resolved = lock.plugins.get(name).map(|lp| lp.resolved.clone());
            let compatible = engine_ver.as_ref().and_then(|ev| {
                let req = semver::VersionReq::parse(engine_range).ok()?;
                Some(req.matches(ev))
            });
            PluginEntry {
                name: name.clone(),
                resolved_version: resolved,
                engine_range: engine_range.clone(),
                compatible,
            }
        })
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{LockedPlugin, LockFile, write_lockfile};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_returns_installed_plugins() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();
        std::fs::create_dir(uepm_dir.join("cool-plugin")).unwrap();
        std::fs::write(
            uepm_dir.join("cool-plugin").join("CoolPlugin.uplugin"),
            "{}",
        )
        .unwrap();

        std::fs::create_dir(dir.path().join("Config")).unwrap();
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Settings]\nEngineVersion = \"5.7\"\n\n[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
        )
        .unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "@acme/cool-plugin");
    }

    #[test]
    fn test_list_compatible_when_engine_version_satisfies_plugin_range() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        // Project is on engine 5.3, plugin requires >=5.0.0, <6.0.0
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Settings]\nEngineVersion = \"5.3.0\"\n\n[Dependencies]\n\"@acme/my-plugin\" = \">=5.0.0, <6.0.0\"\n",
        )
        .unwrap();

        // Write a lockfile so resolved version is known
        let mut lock = LockFile::default();
        lock.plugins.insert(
            "@acme/my-plugin".to_string(),
            LockedPlugin {
                resolved: "1.2.0".to_string(),
                tarball: "https://example.com/tarball.tgz".to_string(),
                sha512: "sha512-abc".to_string(),
                dependencies: HashMap::new(),
            },
        );
        write_lockfile(dir.path(), &lock).unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].compatible, Some(true),
            "engine 5.3.0 should satisfy >=5.0.0, <6.0.0");
    }

    #[test]
    fn test_list_incompatible_when_engine_version_fails_plugin_range() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        // Project is on engine 4.27, plugin requires >=5.0.0
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Settings]\nEngineVersion = \"4.27.0\"\n\n[Dependencies]\n\"@acme/my-plugin\" = \">=5.0.0, <6.0.0\"\n",
        )
        .unwrap();

        let mut lock = LockFile::default();
        lock.plugins.insert(
            "@acme/my-plugin".to_string(),
            LockedPlugin {
                resolved: "1.2.0".to_string(),
                tarball: "https://example.com/tarball.tgz".to_string(),
                sha512: "sha512-abc".to_string(),
                dependencies: HashMap::new(),
            },
        );
        write_lockfile(dir.path(), &lock).unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].compatible, Some(false),
            "engine 4.27.0 should NOT satisfy >=5.0.0, <6.0.0");
    }

    #[test]
    fn test_list_unknown_compat_when_no_engine_version_set() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Config")).unwrap();
        // No EngineVersion in Settings
        std::fs::write(
            dir.path().join("Config/UEPM.ini"),
            "[Dependencies]\n\"@acme/my-plugin\" = \">=5.0.0, <6.0.0\"\n",
        )
        .unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins[0].compatible, None,
            "no engine version means compatibility is unknown");
    }
}
