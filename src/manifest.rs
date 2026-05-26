use crate::errors::UepmError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Metadata written by plugin *authors* in the `[Plugin]` section of `Config/UEPM.ini`.
/// All fields are required for `uepm publish`.
#[derive(Debug, Default, Clone)]
pub struct PackageMetadata {
    /// Scoped npm package name, e.g. `@acme/my-plugin`
    pub name: String,
    /// SemVer version string, e.g. `1.2.0`
    pub version: String,
    pub description: String,
    pub author: String,
    /// SPDX license identifier, e.g. `MIT`
    pub license: String,
    /// SemVer range of compatible Unreal Engine versions, e.g. `>=5.3.0, <6.0.0`
    pub engine_range: String,
    /// `.uplugin` filename, e.g. `MyPlugin.uplugin`
    pub main: String,
}

// Public flat struct — unchanged API for all callers
#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub plugins: HashMap<String, String>,
    pub engine_version: Option<String>,
    pub commit_plugins: bool,
    /// Present only in plugin source trees that have run `uepm init`.
    pub package: Option<PackageMetadata>,
}

// Private TOML representation matching all sections
#[derive(Serialize, Deserialize, Default)]
struct TomlManifest {
    #[serde(rename = "Settings", default)]
    settings: TomlSettings,
    #[serde(rename = "Plugin", skip_serializing_if = "Option::is_none")]
    package: Option<TomlPackage>,
    #[serde(rename = "Dependencies", default)]
    plugins: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default)]
struct TomlSettings {
    #[serde(rename = "EngineVersion", skip_serializing_if = "Option::is_none")]
    engine_version: Option<String>,
    #[serde(rename = "CommitPlugins", default)]
    commit_plugins: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct TomlPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Description", default)]
    description: String,
    #[serde(rename = "Author", default)]
    author: String,
    #[serde(rename = "License", default)]
    license: String,
    #[serde(rename = "EngineRange", default)]
    engine_range: String,
    #[serde(rename = "Main")]
    main: String,
}

/// Returns `true` if `Config/UEPM.ini` already exists in `project_dir`.
pub fn manifest_exists(project_dir: &Path) -> bool {
    manifest_path(project_dir).exists()
}

fn manifest_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("Config").join("UEPM.ini")
}

/// Parse `Config/UEPM.ini` into a [`ProjectManifest`].
pub fn read_manifest(project_dir: &Path) -> Result<ProjectManifest, UepmError> {
    let path = manifest_path(project_dir);
    let content = std::fs::read_to_string(&path)?;
    let toml: TomlManifest = toml::from_str(&content)
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;

    Ok(ProjectManifest {
        plugins: toml.plugins,
        engine_version: toml.settings.engine_version,
        commit_plugins: toml.settings.commit_plugins,
        package: toml.package.map(|p| PackageMetadata {
            name: p.name,
            version: p.version,
            description: p.description,
            author: p.author,
            license: p.license,
            engine_range: p.engine_range,
            main: p.main,
        }),
    })
}

/// Serialize `manifest` and write it to `Config/UEPM.ini`, creating `Config/` if needed.
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
        package: manifest.package.as_ref().map(|p| TomlPackage {
            name: p.name.clone(),
            version: p.version.clone(),
            description: p.description.clone(),
            author: p.author.clone(),
            license: p.license.clone(),
            engine_range: p.engine_range.clone(),
            main: p.main.clone(),
        }),
        plugins: manifest.plugins.clone(),
    };

    let content = toml::to_string_pretty(&toml)
        .map_err(|e| UepmError::ManifestParse(e.to_string()))?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Create a fresh `Config/UEPM.ini` with no plugins. Safe to call when the file doesn't exist yet.
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
            package: None,
        },
    )
}

/// Read only the `[Plugin]` section from `Config/UEPM.ini`.
/// Returns `Err(UepmError::NoPackageMetadata)` if the section is absent.
pub fn read_package_metadata(project_dir: &Path) -> Result<PackageMetadata, UepmError> {
    let manifest = read_manifest(project_dir)?;
    manifest.package.ok_or(UepmError::NoPackageMetadata)
}

/// Write (or overwrite) the `[Plugin]` section in `Config/UEPM.ini`,
/// leaving `[Settings]` and `[Dependencies]` untouched.
pub fn write_package_metadata(project_dir: &Path, meta: &PackageMetadata) -> Result<(), UepmError> {
    let mut manifest = if manifest_exists(project_dir) {
        read_manifest(project_dir)?
    } else {
        ProjectManifest::default()
    };
    manifest.package = Some(meta.clone());
    write_manifest(project_dir, &manifest)
}

/// Add or update a plugin entry in `Config/UEPM.ini`.
pub fn add_plugin(project_dir: &Path, package: &str, range: &str) -> Result<(), UepmError> {
    let mut manifest = read_manifest(project_dir)?;
    manifest.plugins.insert(package.to_string(), range.to_string());
    write_manifest(project_dir, &manifest)
}

/// Remove a plugin entry from `Config/UEPM.ini`. No-ops if the package isn't listed.
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

[Dependencies]
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

    fn sample_package_meta() -> PackageMetadata {
        PackageMetadata {
            name: "@acme/my-plugin".to_string(),
            version: "1.2.0".to_string(),
            description: "Does cool things".to_string(),
            author: "ACME Studio".to_string(),
            license: "MIT".to_string(),
            engine_range: ">=5.3.0, <6.0.0".to_string(),
            main: "MyPlugin.uplugin".to_string(),
        }
    }

    #[test]
    fn test_package_metadata_round_trip() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), Some("5.3"), false).unwrap();
        let meta = sample_package_meta();
        write_package_metadata(dir.path(), &meta).unwrap();

        let got = read_package_metadata(dir.path()).unwrap();
        assert_eq!(got.name, "@acme/my-plugin");
        assert_eq!(got.version, "1.2.0");
        assert_eq!(got.description, "Does cool things");
        assert_eq!(got.author, "ACME Studio");
        assert_eq!(got.license, "MIT");
        assert_eq!(got.engine_range, ">=5.3.0, <6.0.0");
        assert_eq!(got.main, "MyPlugin.uplugin");
    }

    #[test]
    fn test_package_section_does_not_clutter_project_manifests() {
        // A manifest with no [Package] section should round-trip without one.
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), Some("5.4"), false).unwrap();
        let content = fs::read_to_string(dir.path().join("Config/UEPM.ini")).unwrap();
        assert!(!content.contains("[Plugin]"));

        // Reading it also returns None for .package
        let m = read_manifest(dir.path()).unwrap();
        assert!(m.package.is_none());
    }

    #[test]
    fn test_write_package_preserves_plugins() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), None, false).unwrap();
        add_plugin(dir.path(), "@studio/dep", "^2.0.0").unwrap();
        write_package_metadata(dir.path(), &sample_package_meta()).unwrap();

        let m = read_manifest(dir.path()).unwrap();
        assert!(m.plugins.contains_key("@studio/dep"));
        assert!(m.package.is_some());
    }

    #[test]
    fn test_missing_package_section_returns_no_package_metadata_error() {
        let dir = tempdir().unwrap();
        create_manifest(dir.path(), None, false).unwrap();
        let err = read_package_metadata(dir.path()).unwrap_err();
        assert!(matches!(err, crate::errors::UepmError::NoPackageMetadata));
    }
}
