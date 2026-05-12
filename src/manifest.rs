use crate::errors::UepmError;
use configparser::ini::Ini;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub plugins: HashMap<String, String>,
    pub engine_version: Option<String>,
    pub commit_plugins: bool,
}

fn manifest_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("Config").join("UEPM.ini")
}

pub fn manifest_exists(project_dir: &Path) -> bool {
    manifest_path(project_dir).exists()
}

pub fn read_manifest(project_dir: &Path) -> Result<ProjectManifest, UepmError> {
    let path = manifest_path(project_dir);
    let mut config = Ini::new_cs();
    config
        .load(path.to_str().unwrap())
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;

    let plugins = config
        .get_map_ref()
        .get("Plugins")
        .map(|section| {
            section
                .iter()
                .filter_map(|(k, v)| v.as_ref().map(|v| (k.clone(), v.trim().to_string())))
                .collect()
        })
        .unwrap_or_default();

    let engine_version = config.get("Settings", "EngineVersion");
    let commit_plugins = config
        .get("Settings", "CommitPlugins")
        .map(|v| v.trim() == "true")
        .unwrap_or(false);

    Ok(ProjectManifest {
        plugins,
        engine_version,
        commit_plugins,
    })
}

pub fn write_manifest(project_dir: &Path, manifest: &ProjectManifest) -> Result<(), UepmError> {
    let path = manifest_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut config = Ini::new_cs();

    for (name, range) in &manifest.plugins {
        config.set("Plugins", name, Some(range.clone()));
    }

    if let Some(ref ev) = manifest.engine_version {
        config.set("Settings", "EngineVersion", Some(ev.clone()));
    }
    config.set(
        "Settings",
        "CommitPlugins",
        Some(manifest.commit_plugins.to_string()),
    );

    config
        .write(path.to_str().unwrap())
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;
    Ok(())
}

pub fn create_manifest(
    project_dir: &Path,
    engine_version: Option<&str>,
    commit_plugins: bool,
) -> Result<(), UepmError> {
    let manifest = ProjectManifest {
        plugins: HashMap::new(),
        engine_version: engine_version.map(|s| s.to_string()),
        commit_plugins,
    };
    write_manifest(project_dir, &manifest)
}

pub fn add_plugin(project_dir: &Path, package: &str, range: &str) -> Result<(), UepmError> {
    let mut manifest = read_manifest(project_dir)?;
    manifest.plugins.insert(package.to_string(), range.to_string());
    write_manifest(project_dir, &manifest)
}

pub fn remove_plugin(project_dir: &Path, package: &str) -> Result<(), UepmError> {
    let mut manifest = read_manifest(project_dir)?;
    manifest.plugins.remove(package);
    write_manifest(project_dir, &manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_sample_ini(dir: &Path) {
        fs::create_dir_all(dir.join("Config")).unwrap();
        fs::write(
            dir.join("Config/UEPM.ini"),
            "[Plugins]\n@acme/cool-plugin = ^1.0.0\n@studio/other = ~2.1.0\n\n[Settings]\nEngineVersion = 5.7\nCommitPlugins = false\n",
        )
        .unwrap();
    }

    #[test]
    fn test_parse_manifest() {
        let dir = tempdir().unwrap();
        write_sample_ini(dir.path());
        let m = read_manifest(dir.path()).unwrap();
        assert_eq!(m.plugins.get("@acme/cool-plugin").map(|s| s.as_str()), Some("^1.0.0"));
        assert_eq!(m.plugins.get("@studio/other").map(|s| s.as_str()), Some("~2.1.0"));
        assert_eq!(m.engine_version.as_deref(), Some("5.7"));
        assert!(!m.commit_plugins);
    }

    #[test]
    fn test_write_manifest() {
        let dir = tempdir().unwrap();
        let mut m = ProjectManifest::default();
        m.plugins.insert("@foo/bar".to_string(), "^1.0.0".to_string());
        m.engine_version = Some("5.3".to_string());
        m.commit_plugins = true;
        write_manifest(dir.path(), &m).unwrap();
        let content = fs::read_to_string(dir.path().join("Config/UEPM.ini")).unwrap();
        assert!(content.contains("@foo/bar"));
        assert!(content.contains("^1.0.0"));
        assert!(content.contains("EngineVersion"));
        assert!(content.contains("CommitPlugins=true"));
    }

    #[test]
    fn test_create_manifest() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), Some("5.4"), false).unwrap();
        let m = read_manifest(dir.path()).unwrap();
        assert!(m.plugins.is_empty());
        assert_eq!(m.engine_version.as_deref(), Some("5.4"));
        assert!(!m.commit_plugins);
    }

    #[test]
    fn test_missing_manifest_returns_error() {
        let dir = tempdir().unwrap();
        assert!(read_manifest(dir.path()).is_err());
    }

    #[test]
    fn test_add_and_remove_plugin() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), None, false).unwrap();
        add_plugin(dir.path(), "@acme/plugin", "^1.0.0").unwrap();
        let m = read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@acme/plugin"));
        remove_plugin(dir.path(), "@acme/plugin").unwrap();
        let m = read_manifest(dir.path()).unwrap();
        assert!(!m.plugins.contains_key("@acme/plugin"));
    }
}
