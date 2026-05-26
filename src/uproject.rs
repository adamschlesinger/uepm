use crate::errors::UepmError;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Find the first `.uproject` file in `dir` alphabetically.
/// Returns an error if none is found.
pub fn find_uproject(dir: &Path) -> Result<PathBuf, UepmError> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "uproject"))
        .collect();

    found.sort();

    found.into_iter().next().ok_or_else(|| UepmError::UprojectNotFound {
        directory: dir.display().to_string(),
    })
}

/// Read `EngineAssociation` from a `.uproject` file.
pub fn get_engine_association(uproject_path: &Path) -> Result<String, UepmError> {
    let content = std::fs::read_to_string(uproject_path)?;
    let value: Value = serde_json::from_str(&content)?;
    value["EngineAssociation"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| UepmError::ManifestParse("Missing EngineAssociation".to_string()))
}

/// Append `dir_name` to `AdditionalPluginDirectories` in the `.uproject` file.
/// Idempotent — does nothing if the entry already exists.
pub fn add_plugin_directory(uproject_path: &Path, dir_name: &str) -> Result<(), UepmError> {
    let content = std::fs::read_to_string(uproject_path)?;
    let mut value: Value = serde_json::from_str(&content)?;

    let dirs = value
        .as_object_mut()
        .and_then(|obj| {
            obj.entry("AdditionalPluginDirectories")
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
        })
        .ok_or_else(|| UepmError::ManifestParse("Invalid uproject structure".to_string()))?;

    if !dirs.iter().any(|d| d.as_str() == Some(dir_name)) {
        dirs.push(Value::String(dir_name.to_string()));
    }

    let output = serde_json::to_string_pretty(&value)?;
    std::fs::write(uproject_path, output + "\n")?;
    Ok(())
}

/// Returns `true` if `s` looks like a launcher-installed engine GUID (`{...}`).
pub fn is_guid(s: &str) -> bool {
    s.starts_with('{') && s.ends_with('}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_uproject(dir: &Path, content: serde_json::Value) {
        let path = dir.join("TestProject.uproject");
        std::fs::write(path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
    }

    #[test]
    fn test_find_uproject() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), json!({ "EngineAssociation": "5.3" }));
        let path = find_uproject(dir.path()).unwrap();
        assert!(path.ends_with("TestProject.uproject"));
    }

    #[test]
    fn test_find_uproject_missing() {
        let dir = tempdir().unwrap();
        assert!(find_uproject(dir.path()).is_err());
    }

    #[test]
    fn test_add_plugin_directory() {
        let dir = tempdir().unwrap();
        write_uproject(
            dir.path(),
            json!({ "EngineAssociation": "5.3", "AdditionalPluginDirectories": [] }),
        );
        let path = find_uproject(dir.path()).unwrap();
        add_plugin_directory(&path, "UEPMPlugins").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("UEPMPlugins"));
    }

    #[test]
    fn test_add_plugin_directory_idempotent() {
        let dir = tempdir().unwrap();
        write_uproject(
            dir.path(),
            json!({ "EngineAssociation": "5.3", "AdditionalPluginDirectories": ["UEPMPlugins"] }),
        );
        let path = find_uproject(dir.path()).unwrap();
        add_plugin_directory(&path, "UEPMPlugins").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches("UEPMPlugins").count(), 1);
    }

    #[test]
    fn test_is_guid() {
        assert!(is_guid("{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}"));
        assert!(!is_guid("5.3"));
        assert!(!is_guid("5.7"));
    }

    #[test]
    fn test_get_engine_association() {
        let dir = tempdir().unwrap();
        write_uproject(dir.path(), json!({ "EngineAssociation": "5.7" }));
        let path = find_uproject(dir.path()).unwrap();
        assert_eq!(get_engine_association(&path).unwrap(), "5.7");
    }

    #[test]
    fn test_multiple_uprojects_sorted_alphabetically() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Beta.uproject"), "{}").unwrap();
        std::fs::write(dir.path().join("Alpha.uproject"), "{}").unwrap();
        let path = find_uproject(dir.path()).unwrap();
        assert!(path.ends_with("Alpha.uproject"));
    }
}
