use crate::errors::UepmError;
use crate::lockfile::read_lockfile;
use crate::manifest::read_manifest;
use std::path::Path;

#[derive(Debug)]
pub struct PluginEntry {
    pub name: String,
    pub resolved_version: Option<String>,
    pub engine_range: String,
    pub compatible: Option<bool>,
}

pub async fn run() -> Result<(), UepmError> {
    let project_dir = std::env::current_dir()?;
    let plugins = list_plugins(&project_dir)?;

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

pub fn list_plugins(project_dir: &Path) -> Result<Vec<PluginEntry>, UepmError> {
    let manifest = read_manifest(project_dir)?;
    let lock = read_lockfile(project_dir)?.unwrap_or_default();

    let engine_req: Option<semver::VersionReq> = manifest
        .engine_version
        .as_deref()
        .and_then(|v| semver::VersionReq::parse(v).ok());

    let mut entries = Vec::new();

    for (name, engine_range) in &manifest.plugins {
        let resolved = lock.plugins.get(name).map(|lp| lp.resolved.clone());

        let compatible = resolved.as_deref().and_then(|v| {
            let ver = semver::Version::parse(v).ok()?;
            let req = semver::VersionReq::parse(engine_range).ok()?;
            let _ = engine_req.as_ref()?;
            Some(req.matches(&ver))
        });

        entries.push(PluginEntry {
            name: name.clone(),
            resolved_version: resolved,
            engine_range: engine_range.clone(),
            compatible,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "[plugins]\n@acme/cool-plugin = ^1.0.0\n[settings]\nengine_version = 5.7\n",
        )
        .unwrap();

        let plugins = list_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "@acme/cool-plugin");
    }
}
