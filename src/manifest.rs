use crate::errors::UepmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Public flat struct — unchanged API for all callers
#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub plugins: HashMap<String, String>,
    pub engine_version: Option<String>,
    pub commit_plugins: bool,
}

// Private TOML representation matching the [Settings] / [Plugins] sections
#[derive(Serialize, Deserialize, Default)]
struct TomlManifest {
    #[serde(rename = "Settings", default)]
    settings: TomlSettings,
    #[serde(rename = "Plugins", default)]
    plugins: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default)]
struct TomlSettings {
    #[serde(rename = "EngineVersion", skip_serializing_if = "Option::is_none")]
    engine_version: Option<String>,
    #[serde(rename = "CommitPlugins", default)]
    commit_plugins: bool,
}

fn manifest_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("Config").join("UEPM.ini")
}

pub fn manifest_exists(project_dir: &Path) -> bool {
    manifest_path(project_dir).exists()
}

pub fn read_manifest(project_dir: &Path) -> Result<ProjectManifest, UepmError> {
    let path = manifest_path(project_dir);
    let content = std::fs::read_to_string(&path)?;
    let toml: TomlManifest = toml::from_str(&content)
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;

    Ok(ProjectManifest {
        plugins: toml.plugins,
        engine_version: toml.settings.engine_version,
        commit_plugins: toml.settings.commit_plugins,
    })
}

pub fn write_manifest(project_dir: &Path, manifest: &ProjectManifest) -> Result<(), UepmError> {
    let path = manifest_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let toml = TomlManifest {
        settings: TomlSettings {
            engine_version: manifest.engine_version.clone(),
            commit_plugins: manifest.commit_plugins,
        },
        plugins: manifest.plugins.clone(),
    };

    let content = toml::to_string_pretty(&toml)
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn create_manifest(
    project_dir: &Path,
    engine_version: Option<&str>,
    commit_plugins: bool,
) -> Result<(), UepmError> {
    write_manifest(
        project_dir,
        &ProjectManifest {
            plugins: HashMap::new(),
            engine_version: engine_version.map(|s| s.to_string()),
            commit_plugins,
        },
    )
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
            r#"[Settings]
EngineVersion = "5.7"
CommitPlugins = false

[Plugins]
"@acme/cool-plugin" = "^1.0.0"
"@studio/other" = "~2.1.0"
"#,
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
        assert!(content.contains("CommitPlugins = true"));
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
